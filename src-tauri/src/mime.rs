//! Lettura e scrittura dei messaggi nel formato standard della posta (MIME).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use lettre::message::header::{ContentTransferEncoding, ContentType, HeaderName, HeaderValue};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
use base64::Engine;
use mail_parser::{Address, HeaderValue as PHeader, Message, MessageParser, MimeHeaders};

use crate::db::{Envelope, NewMessage};
use crate::error::{Error, Result};
use crate::model::*;

pub const DRAFT_HEADER: &str = "X-Better-Look-Draft-Id";
/// Destinatari di una bozza scritti in modo incompleto (per esempio solo un
/// nome): non sono indirizzi validi, ma la bozza deve conservarli.
pub const PENDING_HEADER: &str = "X-Better-Look-Draft-Pending";

/// Identificativo casuale (non crittografico) per Message-ID e bozze.
pub fn random_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut a = RandomState::new().build_hasher();
    a.write_u128(t);
    a.write_u64(n);
    let mut b = RandomState::new().build_hasher();
    b.write_u64(a.finish());
    b.write_u64(n.rotate_left(17));
    format!("{:016x}{:016x}", a.finish(), b.finish())
}

fn persons(a: Option<&Address>) -> Vec<Person> {
    let Some(a) = a else { return Vec::new() };
    a.iter()
        .map(|x| Person { name: x.name().unwrap_or("").trim().trim_matches('"').to_string(), email: x.address().unwrap_or("").trim().to_string() })
        .filter(|p| !p.email.is_empty() || !p.name.is_empty())
        .collect()
}

fn text_list(h: &PHeader) -> Vec<String> {
    match h {
        PHeader::Text(t) => vec![t.to_string()],
        PHeader::TextList(l) => l.iter().map(|t| t.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn clean_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Euristica per la scheda "Altre": newsletter, notifiche automatiche, mittenti
/// che non leggono le risposte.
fn is_focused(m: &Message) -> bool {
    if !m.list_unsubscribe().is_empty() || !m.list_id().is_empty() {
        return false;
    }
    if let Some(p) = m.header_raw("Precedence") {
        let p = p.trim().to_ascii_lowercase();
        if p == "bulk" || p == "list" || p == "junk" {
            return false;
        }
    }
    if let Some(a) = m.header_raw("Auto-Submitted") {
        if !a.trim().eq_ignore_ascii_case("no") {
            return false;
        }
    }
    if m.header_raw("X-Campaign").is_some() || m.header_raw("X-Mailgun-Tag").is_some() || m.header_raw("Feedback-ID").is_some() {
        return false;
    }
    let from = m.from().and_then(|a| a.first()).and_then(|a| a.address()).unwrap_or("").to_ascii_lowercase();
    let local = from.split('@').next().unwrap_or("");
    const BULK: [&str; 10] = ["noreply", "no-reply", "donotreply", "do-not-reply", "newsletter", "news", "marketing", "promo", "notifiche", "notifications"];
    !BULK.iter().any(|b| local == *b || local.starts_with(&format!("{b}.")) || local.starts_with(&format!("{b}-")) || local.starts_with(&format!("{b}_")))
}

/// Allegato vero o immagine dentro il testo?
fn part_is_inline(p: &mail_parser::MessagePart) -> bool {
    let disp_attachment = p.content_disposition().map(|d| d.ctype().eq_ignore_ascii_case("attachment")).unwrap_or(false);
    if disp_attachment {
        return false;
    }
    let is_image = p.content_type().map(|c| c.ctype().eq_ignore_ascii_case("image")).unwrap_or(false);
    p.content_id().is_some() && is_image
}

fn part_mime(p: &mail_parser::MessagePart) -> String {
    match p.content_type() {
        Some(c) => match c.subtype() {
            Some(s) => format!("{}/{}", c.ctype(), s).to_ascii_lowercase(),
            None => c.ctype().to_ascii_lowercase(),
        },
        None => "application/octet-stream".into(),
    }
}

fn part_name(p: &mail_parser::MessagePart, index: usize) -> String {
    if let Some(n) = p.attachment_name() {
        let n = n.trim().replace(['/', '\\'], "_");
        if !n.is_empty() {
            return n;
        }
    }
    if p.is_message() {
        if let Some(sub) = p.message().and_then(|m| m.subject()) {
            return format!("{}.eml", sub.replace(['/', '\\', ':'], "_"));
        }
        return "Messaggio allegato.eml".into();
    }
    let ext = match part_mime(p).as_str() {
        "application/pdf" => "pdf",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "text/calendar" => "ics",
        "text/plain" => "txt",
        _ => "bin",
    };
    format!("allegato-{}.{ext}", index + 1)
}

pub struct Parsed<'a> {
    pub msg: Message<'a>,
}

pub fn parse(raw: &[u8]) -> Option<Parsed<'_>> {
    MessageParser::default().parse(raw).map(|msg| Parsed { msg })
}

impl Parsed<'_> {
    pub fn attachments(&self) -> Vec<AttachmentInfo> {
        self.msg
            .attachments()
            .enumerate()
            .map(|(i, p)| AttachmentInfo {
                index: i as u32,
                name: part_name(p, i),
                size: p.len() as u64,
                mime: part_mime(p),
                inline: part_is_inline(p),
            })
            .collect()
    }

    /// Testo iCalendar di un invito: la parte text/calendar (Outlook, Google)
    /// o un allegato .ics. Si preferisce quella con il metodo (REQUEST, CANCEL…).
    pub fn calendar(&self) -> Option<String> {
        let mut found: Option<String> = None;
        for p in &self.msg.parts {
            let mime = part_mime(p);
            let named_ics = p.attachment_name().is_some_and(|n| n.to_ascii_lowercase().ends_with(".ics"));
            if mime != "text/calendar" && mime != "application/ics" && !named_ics {
                continue;
            }
            let text = match p.text_contents() {
                Some(t) => t.to_string(),
                None => String::from_utf8_lossy(p.contents()).to_string(),
            };
            if !text.to_ascii_uppercase().contains("BEGIN:VEVENT") {
                continue;
            }
            let method = p.content_type().and_then(|c| c.attribute("method")).is_some() || text.to_ascii_uppercase().contains("\nMETHOD:");
            if method {
                return Some(text);
            }
            found.get_or_insert(text);
        }
        found
    }

    pub fn has_real_attachments(&self) -> bool {
        self.msg.attachments().any(|p| !part_is_inline(p))
    }

    pub fn attachment(&self, index: u32) -> Option<(String, String, Vec<u8>)> {
        let (i, p) = self.msg.attachments().enumerate().nth(index as usize)?;
        Some((part_name(p, i), part_mime(p), p.contents().to_vec()))
    }

    /// Immagini incorporate nel testo HTML, per Content-ID.
    pub fn inline_images(&self) -> Vec<(String, String, Vec<u8>)> {
        self.msg
            .attachments()
            .filter_map(|p| {
                let cid = p.content_id()?.trim().trim_start_matches('<').trim_end_matches('>').to_string();
                Some((cid, part_mime(p), p.contents().to_vec()))
            })
            .collect()
    }

    pub fn text(&self) -> String {
        self.msg.body_text(0).map(|t| t.to_string()).unwrap_or_default()
    }

    pub fn html(&self) -> Option<String> {
        if self.msg.html_body_count() == 0 {
            return None;
        }
        // mail-parser converte il testo semplice in HTML quando manca la parte
        // HTML: in quel caso conviene mostrare il testo così com'è.
        let part = self.msg.html_part(0)?;
        if !part.is_text_html() {
            return None;
        }
        self.msg.body_html(0).map(|h| h.to_string())
    }

    pub fn recipients(&self) -> Recipients {
        Recipients {
            to: persons(self.msg.to()),
            cc: persons(self.msg.cc()),
            bcc: persons(self.msg.bcc()),
            reply_to: persons(self.msg.reply_to()),
        }
    }

    pub fn references(&self) -> Vec<String> {
        text_list(self.msg.references())
    }

    pub fn in_reply_to(&self) -> Vec<String> {
        text_list(self.msg.in_reply_to())
    }

    /// Si può mandare una risposta automatica a questo messaggio? No alle
    /// newsletter, alle notifiche, alle altre risposte automatiche, ai messaggi
    /// di errore dei server e a quelli mandati da sé stessi. Restituisce
    /// l'indirizzo a cui rispondere.
    pub fn auto_reply_to(&self, own: &str) -> Option<Person> {
        if !is_focused(&self.msg) {
            return None;
        }
        if let Some(s) = self.msg.header_raw("X-Auto-Response-Suppress") {
            let s = s.to_ascii_lowercase();
            if s.contains("all") || s.contains("oof") || s.contains("autoreply") {
                return None;
            }
        }
        if self.msg.header_raw("Return-Path").is_some_and(|r| r.trim().trim_matches(|c| c == '<' || c == '>').is_empty()) {
            return None;
        }
        let from = persons(self.msg.from()).into_iter().next()?;
        let target = persons(self.msg.reply_to()).into_iter().next().unwrap_or_else(|| from.clone());
        let addr = target.email.trim().to_ascii_lowercase();
        let local = addr.split('@').next().unwrap_or("");
        if addr.is_empty() || !addr.contains('@') || addr == own.trim().to_ascii_lowercase() || from.email.trim().eq_ignore_ascii_case(own.trim()) {
            return None;
        }
        if ["mailer-daemon", "postmaster", "bounce", "bounces"].iter().any(|b| local == *b || local.starts_with(&format!("{b}+")) || local.starts_with(&format!("{b}-"))) {
            return None;
        }
        Some(target)
    }

    pub fn subject(&self) -> String {
        self.msg.subject().map(clean_ws).unwrap_or_default()
    }

    pub fn message_id(&self) -> Option<String> {
        self.msg.message_id().map(|m| m.to_string())
    }

    pub fn draft_key(&self) -> Option<String> {
        self.msg.header_raw(DRAFT_HEADER).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    pub fn preview(&self) -> String {
        let p = self.msg.body_preview(400).map(|p| clean_ws(&p)).unwrap_or_default();
        p.chars().take(220).collect()
    }

    /// Campi per l'archivio. `date` è il ripiego se l'intestazione manca.
    pub fn to_new_message(&self, fallback_date: i64, full: bool, size: i64) -> NewMessage {
        let from = persons(self.msg.from()).into_iter().next().unwrap_or_default();
        let date = self.msg.date().map(|d| d.to_timestamp()).filter(|d| *d > 0).unwrap_or(fallback_date);
        let body_text: String = if full { self.text().chars().take(100_000).collect() } else { String::new() };
        let has_atts = if full {
            self.has_real_attachments()
        } else {
            self.msg
                .content_type()
                .map(|c| c.ctype().eq_ignore_ascii_case("multipart") && c.subtype().map(|s| s.eq_ignore_ascii_case("mixed")).unwrap_or(false))
                .unwrap_or(false)
        };
        NewMessage {
            message_id: self.msg.message_id().map(|s| s.to_string()),
            draft_key: self.draft_key(),
            date,
            from,
            recipients: self.recipients(),
            subject: clean_ws(self.msg.subject().unwrap_or("")),
            preview: if full { self.preview() } else { String::new() },
            has_atts,
            focused: is_focused(&self.msg),
            size,
            refs: self.references().join(" "),
            in_reply_to: self.in_reply_to().join(" "),
            body_text,
            ..Default::default()
        }
    }

    /// Testo del messaggio da citare in una risposta o in un inoltro, come
    /// frammento HTML ripulito (senza script né stili globali).
    pub fn quote_fragment(&self) -> String {
        match self.html() {
            // Firma e citazione di un messaggio scritto con Better-Look restano testo
            // normale: i segni servono solo per il messaggio che si sta scrivendo.
            Some(h) => crate::sanitize::clean_fragment(&h, &self.inline_images()).replace(" id=\"better-look-firma\"", "").replace(" id=\"better-look-citazione\"", ""),
            None => text_fragment(&self.text()),
        }
    }

    /// Ricostruisce una bozza da un messaggio salvato.
    pub fn to_draft(&self, account_id: &str, local_id: i64) -> Draft {
        let mut rc = self.recipients();
        if let Some(p) = self.msg.header_raw(PENDING_HEADER) {
            let compact: String = p.split_whitespace().collect();
            if let Some(extra) = base64::engine::general_purpose::STANDARD.decode(compact).ok().and_then(|b| serde_json::from_slice::<Recipients>(&b).ok()) {
                rc.to.extend(extra.to);
                rc.cc.extend(extra.cc);
                rc.bcc.extend(extra.bcc);
            }
        }
        let attachments = self
            .attachments()
            .into_iter()
            .filter(|a| !a.inline)
            .map(|a| DraftAttachment {
                name: a.name,
                size: a.size,
                mime: Some(a.mime),
                path: None,
                source: Some(AttachmentRef { message_id: local_id, index: a.index }),
            })
            .collect();
        Draft {
            key: self.draft_key().unwrap_or_else(random_id),
            account_id: account_id.into(),
            to: rc.to,
            cc: rc.cc,
            bcc: rc.bcc,
            subject: clean_ws(self.msg.subject().unwrap_or("")),
            body: self.text(),
            html: self.html().map(|h| crate::sanitize::clean_fragment(&h, &self.inline_images())),
            plain: self.html().is_none(),
            attachments,
            in_reply_to: self.in_reply_to().into_iter().next(),
            references: self.references(),
            reply_to_message: None,
        }
    }
}

#[derive(Clone)]
struct DraftIdHeader(String);

impl lettre::message::header::Header for DraftIdHeader {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str(DRAFT_HEADER)
    }
    fn parse(s: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(DraftIdHeader(s.to_string()))
    }
    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

#[derive(Clone)]
struct PendingHeader(String);

impl lettre::message::header::Header for PendingHeader {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str(PENDING_HEADER)
    }
    fn parse(s: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(PendingHeader(s.to_string()))
    }
    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// Intestazioni delle risposte automatiche, ognuna con il suo tipo: lettre
/// riconosce le intestazioni dal tipo.
macro_rules! fixed_header {
    ($t:ident, $name:literal, $value:literal) => {
        #[derive(Clone)]
        struct $t;

        impl lettre::message::header::Header for $t {
            fn name() -> HeaderName {
                HeaderName::new_from_ascii_str($name)
            }
            fn parse(_: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
                Ok($t)
            }
            fn display(&self) -> HeaderValue {
                HeaderValue::new(Self::name(), $value.to_string())
            }
        }
    };
}

fixed_header!(AutoSubmitted, "Auto-Submitted", "auto-replied");
fixed_header!(AutoResponseSuppress, "X-Auto-Response-Suppress", "All");

#[derive(Clone)]
struct XMailer;

impl lettre::message::header::Header for XMailer {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("X-Mailer")
    }
    fn parse(_: &str) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(XMailer)
    }
    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), format!("Better-Look {}", env!("CARGO_PKG_VERSION")))
    }
}

pub fn mailbox(p: &Person) -> Result<Mailbox> {
    let email = p.email.trim();
    let addr = email.parse().map_err(|_| Error::user(format!("L'indirizzo \"{}\" non è valido.", if email.is_empty() { p.name.as_str() } else { email })))?;
    let name = p.name.trim();
    Ok(Mailbox::new(if name.is_empty() || name == email { None } else { Some(name.to_string()) }, addr))
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Testo semplice come frammento HTML: una riga per riga, righe vuote comprese.
pub fn text_fragment(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    let trimmed = normalized.trim_end();
    trimmed
        .split('\n')
        .map(|l| if l.trim().is_empty() { "<div><br></div>".to_string() } else { format!("<div>{}</div>", escape_html(l)) })
        .collect::<Vec<_>>()
        .join("")
}

/// Documento completo dal frammento scritto nell'editor.
pub fn wrap_html(fragment: &str) -> String {
    format!("<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body>{fragment}</body></html>")
}

/// Immagine messa nel testo (incollata o inserita), da spedire come parte
/// incorporata del messaggio.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineImage {
    pub cid: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

fn parse_data_url(v: &str) -> Option<(String, Vec<u8>)> {
    let rest = v.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    let mime = meta.strip_suffix(";base64")?.trim().to_ascii_lowercase();
    if !matches!(mime.as_str(), "image/png" | "image/jpeg" | "image/jpg" | "image/gif" | "image/webp" | "image/bmp") {
        return None;
    }
    let compact: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(compact).ok()?;
    Some((if mime == "image/jpg" { "image/jpeg".into() } else { mime }, bytes))
}

/// Le immagini scritte come dati nel testo (`src="data:image/..."`) diventano
/// parti incorporate con un Content-ID: molti programmi di posta non mostrano
/// le immagini come dati. La stessa immagine ripetuta viaggia una volta sola.
pub fn extract_data_images(html: &str) -> (String, Vec<InlineImage>) {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut images: Vec<InlineImage> = Vec::new();
    let mut seen: std::collections::HashMap<&str, String> = std::collections::HashMap::new();
    let tag = random_id();
    let mut i = 0;
    while let Some(pos) = lower[i..].find("src=") {
        let at = i + pos + 4;
        let q = match html.as_bytes().get(at) {
            Some(b'"') => '"',
            Some(b'\'') => '\'',
            _ => {
                out.push_str(&html[i..at]);
                i = at;
                continue;
            }
        };
        let start = at + 1;
        let Some(len) = html[start..].find(q) else { break };
        let end = start + len;
        let value = &html[start..end];
        if !lower[start..end].starts_with("data:image/") {
            out.push_str(&html[i..end]);
            i = end;
            continue;
        }
        let cid = if let Some(c) = seen.get(value) {
            Some(c.clone())
        } else if let Some((mime, bytes)) = parse_data_url(value) {
            let cid = format!("img{}.{}@better-look", images.len() + 1, &tag[..12]);
            images.push(InlineImage { cid: cid.clone(), mime, bytes });
            seen.insert(value, cid.clone());
            Some(cid)
        } else {
            None
        };
        match cid {
            Some(c) => {
                out.push_str(&html[i..start]);
                out.push_str("cid:");
                out.push_str(&c);
            }
            None => out.push_str(&html[i..end]),
        }
        i = end;
    }
    out.push_str(&html[i..]);
    (out, images)
}

/// Versione HTML del testo scritto: paragrafi e a capo, senza altro.
pub fn text_to_html(body: &str) -> String {
    let mut out = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head><body><div style=\"font-family: Aptos, Calibri, Arial, Helvetica, sans-serif; font-size: 11pt; color: #000000;\">",
    );
    let normalized = body.replace("\r\n", "\n");
    for para in normalized.split("\n\n") {
        if para.trim().is_empty() {
            out.push_str("<p style=\"margin:0\">&nbsp;</p>");
            continue;
        }
        out.push_str("<p style=\"margin:0 0 1em 0\">");
        out.push_str(&para.lines().map(escape_html).collect::<Vec<_>>().join("<br>"));
        out.push_str("</p>");
    }
    out.push_str("</div></body></html>");
    out
}

pub struct BuiltMessage {
    /// Da spedire: senza l'intestazione Ccn.
    pub wire: Vec<u8>,
    /// Da conservare (Posta inviata, Bozze): con Ccn.
    pub copy: Vec<u8>,
    pub envelope: Envelope,
    pub message_id: String,
}

/// Contenuto degli allegati, già letto dal disco o dall'archivio.
pub struct AttachmentData {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub fn build(account: &AccountConfig, d: &Draft, atts: &[AttachmentData], as_draft: bool) -> Result<BuiltMessage> {
    build_message(account, d, atts, as_draft, false, None)
}

/// Invito, risposta o annullamento di una riunione: il testo per le persone e
/// la parte text/calendar per i programmi (come fanno Outlook e Google).
pub fn build_calendar(account: &AccountConfig, d: &Draft, method: &str, ics: &str) -> Result<BuiltMessage> {
    build_message(account, d, &[], false, false, Some((method, ics)))
}

/// Risposta automatica: un messaggio come gli altri, con le intestazioni che
/// dicono agli altri programmi di non rispondere a loro volta (RFC 3834).
pub fn build_auto_reply(account: &AccountConfig, d: &Draft) -> Result<BuiltMessage> {
    build_message(account, d, &[], false, true, None)
}

fn build_message(account: &AccountConfig, d: &Draft, atts: &[AttachmentData], as_draft: bool, auto_reply: bool, calendar: Option<(&str, &str)>) -> Result<BuiltMessage> {
    let from = mailbox(&Person::new(account.name.clone(), account.email.clone()))?;
    let domain = account.email.split('@').nth(1).unwrap_or("better-look.local").to_string();
    let message_id = format!("{}@{}", random_id(), domain);

    // In una bozza gli indirizzi non validi non fermano il salvataggio:
    // si conservano a parte e si ritrovano quando la bozza si riapre.
    let mut pending = Recipients::default();
    let valid = |list: &[Person], out: &mut Vec<Person>| -> Result<Vec<Mailbox>> {
        let mut v = Vec::new();
        for p in list {
            match mailbox(p) {
                Ok(m) => v.push(m),
                Err(e) if !as_draft => return Err(e),
                Err(_) => out.push(p.clone()),
            }
        }
        Ok(v)
    };
    let to = valid(&d.to, &mut pending.to)?;
    let cc = valid(&d.cc, &mut pending.cc)?;
    let bcc = valid(&d.bcc, &mut pending.bcc)?;
    let pending_json = if pending.to.is_empty() && pending.cc.is_empty() && pending.bcc.is_empty() {
        None
    } else {
        Some(base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(&pending).unwrap_or_default()))
    };

    // Testo formattato dall'editor, oppure costruito dal testo semplice.
    let source = match d.html.as_deref() {
        Some(h) if !h.trim().is_empty() => wrap_html(h),
        _ => text_to_html(&d.body),
    };
    let (html_doc, images) = extract_data_images(&source);

    let make = |keep_bcc: bool| -> Result<Vec<u8>> {
        let mut b = lettre::Message::builder()
            .from(from.clone())
            .subject(d.subject.clone())
            .message_id(Some(format!("<{message_id}>")))
            .date(SystemTime::now())
            .header(XMailer);
        for m in &to {
            b = b.to(m.clone());
        }
        for m in &cc {
            b = b.cc(m.clone());
        }
        for m in &bcc {
            b = b.bcc(m.clone());
        }
        if let Some(irt) = &d.in_reply_to {
            b = b.in_reply_to(format!("<{}>", irt.trim_matches(|c| c == '<' || c == '>')));
        }
        if !d.references.is_empty() {
            let refs = d.references.iter().map(|r| format!("<{}>", r.trim_matches(|c| c == '<' || c == '>'))).collect::<Vec<_>>().join(" ");
            b = b.references(refs);
        }
        if as_draft {
            b = b.header(DraftIdHeader(d.key.clone()));
            if let Some(p) = &pending_json {
                b = b.header(PendingHeader(p.clone()));
            }
        }
        if keep_bcc {
            b = b.keep_bcc();
        }
        if auto_reply {
            b = b.header(AutoSubmitted).header(AutoResponseSuppress);
        }
        let text = d.body.replace("\r\n", "\n");
        let with_atts = |mut mixed: MultiPart| {
            for a in atts {
                let ct = ContentType::parse(&a.mime).unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("tipo valido"));
                mixed = mixed.singlepart(Attachment::new(a.name.clone()).body(a.bytes.clone(), ct));
            }
            mixed
        };
        // Testo normale: solo il testo, come il formato "Testo normale" di Outlook.
        if d.plain {
            let msg = if atts.is_empty() { b.singlepart(SinglePart::plain(text)) } else { b.multipart(with_atts(MultiPart::mixed().singlepart(SinglePart::plain(text)))) }
                .map_err(|e| Error::user(format!("Impossibile preparare il messaggio: {e}")))?;
            return Ok(msg.formatted());
        }
        let mut alt = MultiPart::alternative().singlepart(SinglePart::plain(text));
        alt = if images.is_empty() {
            alt.singlepart(SinglePart::html(html_doc.clone()))
        } else {
            let mut related = MultiPart::related().singlepart(SinglePart::html(html_doc.clone()));
            for img in &images {
                let ct = ContentType::parse(&img.mime).unwrap_or_else(|_| ContentType::parse("application/octet-stream").expect("tipo valido"));
                related = related.singlepart(Attachment::new_inline(img.cid.clone()).body(img.bytes.clone(), ct));
            }
            alt.multipart(related)
        };
        if let Some((method, ics)) = calendar {
            let ct = ContentType::parse(&format!("text/calendar; method={method}; charset=utf-8")).map_err(|e| Error::user(format!("Impossibile preparare l'invito: {e}")))?;
            alt = alt.singlepart(SinglePart::builder().header(ct).header(ContentTransferEncoding::Base64).body(ics.as_bytes().to_vec()));
        }
        let msg = if atts.is_empty() { b.multipart(alt) } else { b.multipart(with_atts(MultiPart::mixed().multipart(alt))) }
        .map_err(|e| Error::user(format!("Impossibile preparare il messaggio: {e}")))?;
        Ok(msg.formatted())
    };

    let all: Vec<String> = d.to.iter().chain(&d.cc).chain(&d.bcc).map(|p| p.email.trim().to_string()).collect();
    Ok(BuiltMessage {
        wire: if as_draft { Vec::new() } else { make(false)? },
        copy: make(true)?,
        envelope: Envelope { from: account.email.clone(), to: all },
        message_id,
    })
}

/// Una singola parte di testo, usata nei test e per i messaggi di sistema.
#[allow(dead_code)]
pub fn plain_part(text: &str) -> SinglePart {
    SinglePart::plain(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> AccountConfig {
        AccountConfig {
            id: "a".into(),
            name: "Chiara Valli".into(),
            email: "chiara@valliarredi.it".into(),
            kind: AccountKind::Imap,
            incoming: None,
            outgoing: None,
            sync: SyncOptions::default(),
            server_saves_sent: false,
            source_file: None,
            signature_new: None,
            signature_reply: None,
            auto_reply: None,
        }
    }

    fn draft() -> Draft {
        Draft {
            key: "k1".into(),
            account_id: "a".into(),
            to: vec![Person::new("Davide Orlandi", "d.orlandi@studiorlandi.it")],
            cc: vec![Person::new("", "marco@valliarredi.it")],
            bcc: vec![Person::new("Giorgio", "giorgio@valliarredi.it")],
            subject: "Campioni del rovere: però è urgente".into(),
            body: "Ciao Davide,\n\nriesci a portarli giovedì?\nGrazie\n\nChiara".into(),
            html: None,
            plain: false,
            attachments: vec![],
            in_reply_to: Some("orig@studiorlandi.it".into()),
            references: vec!["orig@studiorlandi.it".into()],
            reply_to_message: None,
        }
    }

    #[test]
    fn build_and_parse_roundtrip() {
        let atts = [AttachmentData { name: "Preventivo rev3.pdf".into(), mime: "application/pdf".into(), bytes: b"%PDF-1.4 prova".to_vec() }];
        let b = build(&account(), &draft(), &atts, false).unwrap();
        assert_eq!(b.envelope.to.len(), 3);
        let wire = String::from_utf8_lossy(&b.wire);
        assert!(!wire.contains("giorgio@valliarredi.it"), "il Ccn non deve viaggiare con il messaggio");
        let p = parse(&b.copy).unwrap();
        assert_eq!(p.msg.subject(), Some("Campioni del rovere: però è urgente"));
        let rc = p.recipients();
        assert_eq!(rc.to[0], Person::new("Davide Orlandi", "d.orlandi@studiorlandi.it"));
        assert_eq!(rc.bcc[0].email, "giorgio@valliarredi.it");
        assert!(p.text().contains("riesci a portarli giovedì?"));
        let a = p.attachments();
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].name, "Preventivo rev3.pdf");
        assert!(!a[0].inline);
        assert_eq!(p.attachment(0).unwrap().2, b"%PDF-1.4 prova");
        assert_eq!(p.in_reply_to(), vec!["orig@studiorlandi.it".to_string()]);
        assert_eq!(p.msg.message_id(), Some(b.message_id.as_str()));
        let nm = p.to_new_message(0, true, b.copy.len() as i64);
        assert!(nm.has_atts && nm.focused);
        assert!(nm.preview.starts_with("Ciao Davide"));
    }

    #[test]
    fn draft_header_roundtrip() {
        let b = build(&account(), &draft(), &[], true).unwrap();
        let p = parse(&b.copy).unwrap();
        let d = p.to_draft("a", 5);
        assert_eq!(d.key, "k1");
        assert_eq!(d.to.len(), 1);
        assert_eq!(d.bcc.len(), 1);
        assert!(d.body.contains("giovedì"));
    }

    #[test]
    fn newsletters_go_to_other() {
        let raw = b"From: Arredo & Contract <newsletter@arredoecontract.it>\r\nTo: chiara@valliarredi.it\r\nSubject: Uffici 2027\r\nList-Unsubscribe: <mailto:u@x.it>\r\n\r\nciao\r\n";
        assert!(!parse(raw).unwrap().to_new_message(0, true, 0).focused);
        let raw = b"From: Trasporti <no-reply@trasporti.it>\r\nSubject: Consegna\r\n\r\nciao\r\n";
        assert!(!parse(raw).unwrap().to_new_message(0, true, 0).focused);
        let raw = b"From: Davide <d.orlandi@studiorlandi.it>\r\nSubject: Preventivo\r\n\r\nciao\r\n";
        assert!(parse(raw).unwrap().to_new_message(0, true, 0).focused);
    }

    #[test]
    fn drafts_keep_incomplete_recipients() {
        let mut d = draft();
        d.to.push(Person::new("Mario Rossi", ""));
        d.bcc = vec![Person::new("", "non-valido")];
        let b = build(&account(), &d, &[], true).unwrap();
        let back = parse(&b.copy).unwrap().to_draft("a", 1);
        assert_eq!(back.to, d.to);
        assert_eq!(back.bcc, d.bcc);
        assert!(build(&account(), &d, &[], false).is_err());
    }

    #[test]
    fn invalid_address_is_explained() {
        let mut d = draft();
        d.to = vec![Person::new("Mario Rossi", "")];
        let e = build(&account(), &d, &[], false).err().unwrap();
        assert!(e.to_string().contains("Mario Rossi"));
    }

    #[test]
    fn formatted_text_with_images_roundtrip() {
        let mut d = draft();
        let png = base64::engine::general_purpose::STANDARD.encode([137u8, 80, 78, 71, 1, 2, 3]);
        d.html = Some(format!(
            "<div class=\"better-look-corpo\" style=\"font-family: Aptos; font-size: 11pt\"><div><b>Ciao</b> Davide</div><img src=\"data:image/png;base64,{png}\" width=\"120\"><div id=\"better-look-firma\"><img src='data:image/png;base64,{png}'></div><img src=\"https://x.it/a.png\"></div>"
        ));
        let b = build(&account(), &d, &[], false).unwrap();
        let p = parse(&b.copy).unwrap();
        let html = p.html().unwrap();
        assert!(html.contains("<b>Ciao</b> Davide"));
        assert!(!html.contains("data:image"), "le immagini viaggiano come parti incorporate");
        assert!(html.contains("https://x.it/a.png"));
        let inline = p.inline_images();
        assert_eq!(inline.len(), 1, "la stessa immagine viaggia una volta");
        assert_eq!(inline[0].2, vec![137u8, 80, 78, 71, 1, 2, 3]);
        assert!(!p.has_real_attachments());
        assert!(p.text().contains("riesci a portarli"), "resta la versione in testo semplice");

        // La bozza riaperta ritrova il testo formattato con le immagini.
        let saved = build(&account(), &d, &[], true).unwrap();
        let back = parse(&saved.copy).unwrap().to_draft("a", 1);
        let h = back.html.unwrap();
        assert!(h.contains("<b>Ciao</b>"));
        assert!(h.contains("id=\"better-look-firma\""));
        assert!(h.contains(&format!("data:image/png;base64,{png}")));
    }

    #[test]
    fn plain_text_messages_have_no_html() {
        let mut d = draft();
        d.plain = true;
        d.html = Some("<div><b>ignorato</b></div>".into());
        let b = build(&account(), &d, &[], false).unwrap();
        let p = parse(&b.copy).unwrap();
        assert!(p.html().is_none() || !p.html().unwrap().contains("<b>"));
        assert!(p.text().contains("riesci a portarli"));
        assert!(!String::from_utf8_lossy(&b.copy).to_ascii_lowercase().contains("text/html"));
        let back = p.to_draft("a", 1);
        assert!(back.plain, "la bozza riaperta resta in testo normale");
        let with = build(&account(), &d, &[AttachmentData { name: "a.txt".into(), mime: "text/plain".into(), bytes: b"x".to_vec() }], false).unwrap();
        assert_eq!(parse(&with.copy).unwrap().attachments().len(), 1);
    }

    #[test]
    fn quoting_a_better_look_message_drops_its_markers() {
        let mut d = draft();
        d.html = Some("<div>Ciao</div><div id=\"better-look-firma\"><div>Davide</div></div><div id=\"better-look-citazione\"><div>Prima</div></div>".into());
        let b = build(&account(), &d, &[], false).unwrap();
        let q = parse(&b.copy).unwrap().quote_fragment();
        assert!(q.contains("Davide") && q.contains("Prima"));
        assert!(!q.contains("better-look-firma") && !q.contains("better-look-citazione"));
    }

    #[test]
    fn data_images_need_a_known_type() {
        let (out, imgs) = extract_data_images("<img src=\"data:image/svg+xml;base64,PHN2Zz4=\"> src=\"x\"");
        assert!(imgs.is_empty());
        assert!(out.contains("data:image/svg+xml"));
        assert_eq!(text_fragment("a < b\n\nfine"), "<div>a &lt; b</div><div><br></div><div>fine</div>");
    }

    #[test]
    fn html_version_escapes_text() {
        let h = text_to_html("a < b & c\n\nnuovo");
        assert!(h.contains("a &lt; b &amp; c"));
        assert!(h.contains("<p style=\"margin:0 0 1em 0\">nuovo</p>"));
    }
}
