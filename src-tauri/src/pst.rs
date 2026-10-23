//! Importazione di archivi: file di dati di Outlook (PST) e MBOX.
//! Ogni archivio diventa un "file di dati" nel riquadro cartelle, come in Outlook.
//!
//! I messaggi dei PST sono salvati come proprietà MAPI, non come email: qui si
//! ricostruisce un messaggio MIME completo per ognuno (intestazioni originali
//! quando ci sono, testo, HTML e allegati), così da trattarlo come ogni altra
//! email dell'archivio locale.

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use mail_builder::headers::address::Address;
use mail_builder::headers::date::Date;
use mail_builder::headers::message_id::MessageId;
use mail_builder::headers::text::Text;
use mail_builder::mime::MimePart;
use mail_builder::MessageBuilder;
use outlook_pst::ltp::prop_context::PropertyValue;
use outlook_pst::ltp::table_context::TableContext;
use outlook_pst::messaging::attachment::{AnsiAttachment, Attachment as PstAttachment, AttachmentData, UnicodeAttachment};
use outlook_pst::messaging::folder::Folder as PstFolder;
use outlook_pst::messaging::message::{AnsiMessage, Message as PstMessage, UnicodeMessage};
use outlook_pst::messaging::store::{AnsiStore, EntryId, Store, UnicodeStore};
use outlook_pst::ndb::node_id::NodeId;
use outlook_pst::{AnsiPstFile, UnicodePstFile};
use tauri::State;

use crate::db::NewMessage;
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::mime;
use crate::model::*;
use crate::rtf;

fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Archivio".into())
}

/// Stato di un messaggio importato.
#[derive(Default, Clone, Copy)]
pub struct Flags {
    pub unread: bool,
    pub flagged: bool,
    pub answered: bool,
    pub draft: bool,
}

/// Inserisce un messaggio completo in una cartella di un file di dati.
pub fn insert_raw(engine: &Engine, account_id: &str, folder_id: i64, raw: Vec<u8>, fallback_date: i64, flags: Flags) -> Result<()> {
    let mut nm = match mime::parse(&raw) {
        Some(p) => p.to_new_message(fallback_date, true, raw.len() as i64),
        None => NewMessage { date: fallback_date, ..Default::default() },
    };
    nm.unread = flags.unread;
    nm.flagged = flags.flagged;
    nm.answered = flags.answered;
    nm.draft = flags.draft;
    nm.focused = true;
    nm.raw = Some(raw);
    engine.db.insert_message(account_id, folder_id, &nm)?;
    Ok(())
}

pub fn import_mbox_file(engine: &Engine, path: &Path) -> Result<Imported> {
    let file = std::fs::File::open(path).map_err(|_| Error::user("Impossibile aprire il file MBOX."))?;
    let name = file_stem(path);
    let acct = engine.create_local_account(&name, Some(path.to_string_lossy().to_string()))?;
    let folder = engine.db.upsert_folder(&acct.id, &name, &name, None, Some("/"), true)?;
    let mut n = 0;
    let mut skipped = 0;
    for m in mail_parser::mailbox::mbox::MessageIterator::new(BufReader::new(file)) {
        let Ok(m) = m else {
            skipped += 1;
            continue;
        };
        let date = m.internal_date() as i64;
        insert_raw(engine, &acct.id, folder, m.unwrap_contents(), date, Flags::default())?;
        n += 1;
        if n % 200 == 0 {
            engine.import_progress(&name, n, 0);
        }
    }
    engine.import_progress(&name, n, n);
    if n == 0 {
        engine.remove_account(&acct.id)?;
        return Err(Error::user("Il file non contiene messaggi in formato MBOX."));
    }
    engine.notify_changed(&acct.id);
    Ok(Imported { account: AccountInfo { status: engine.status(&acct.id), config: acct }, skipped, lost_attachments: 0 })
}

#[tauri::command]
pub async fn import_mbox(e: State<'_, Arc<Engine>>, path: PathBuf) -> Result<Imported> {
    let engine = e.inner().clone();
    tokio::task::spawn_blocking(move || import_mbox_file(&engine, &path)).await.map_err(|e| Error::user(e.to_string()))?
}

#[tauri::command]
pub async fn import_pst(e: State<'_, Arc<Engine>>, path: PathBuf) -> Result<Imported> {
    let engine = e.inner().clone();
    tokio::task::spawn_blocking(move || import_pst_file(&engine, &path)).await.map_err(|e| Error::user(e.to_string()))?
}

// ---------- PST ----------

// Proprietà MAPI usate qui (MS-OXPROPS).
const SUBJECT: u16 = 0x0037;
const CLIENT_SUBMIT_TIME: u16 = 0x0039;
const MESSAGE_CLASS: u16 = 0x001A;
const TRANSPORT_HEADERS: u16 = 0x007D;
const SENDER_NAME: u16 = 0x0C1A;
const SENDER_EMAIL: u16 = 0x0C1F;
const SENDER_ADDRTYPE: u16 = 0x0C1E;
const SENDER_SMTP: u16 = 0x5D01;
const SENT_REPR_NAME: u16 = 0x0042;
const SENT_REPR_EMAIL: u16 = 0x0065;
const SENT_REPR_SMTP: u16 = 0x5D02;
const DELIVERY_TIME: u16 = 0x0E06;
const MESSAGE_FLAGS: u16 = 0x0E07;
const BODY: u16 = 0x1000;
const RTF_COMPRESSED: u16 = 0x1009;
const HTML: u16 = 0x1013;
const INTERNET_MESSAGE_ID: u16 = 0x1035;
const REFERENCES: u16 = 0x1039;
const IN_REPLY_TO: u16 = 0x1042;
const LAST_VERB: u16 = 0x1081;
const FLAG_STATUS: u16 = 0x1090;
const CREATION_TIME: u16 = 0x3007;
const DISPLAY_NAME: u16 = 0x3001;
const INTERNET_CPID: u16 = 0x3FDE;
const MESSAGE_CODEPAGE: u16 = 0x3FFD;
const CONTAINER_CLASS: u16 = 0x3613;
// Destinatari.
const RECIPIENT_TYPE: u16 = 0x0C15;
const EMAIL_ADDRESS: u16 = 0x3003;
const ADDRTYPE: u16 = 0x3002;
const SMTP_ADDRESS: u16 = 0x39FE;
// Allegati.
const ATTACH_FILENAME: u16 = 0x3704;
const ATTACH_LONG_FILENAME: u16 = 0x3707;
const ATTACH_MIME: u16 = 0x370E;
const ATTACH_CONTENT_ID: u16 = 0x3712;
const ATTACH_HIDDEN: u16 = 0x7FFE;

const MSGFLAG_READ: i32 = 0x01;
const MSGFLAG_UNSENT: i32 = 0x08;

/// Messaggio aperto, i suoi allegati e quanti allegati non si sono potuti leggere.
type OpenedMessage = (Rc<dyn PstMessage>, Vec<Rc<dyn PstAttachment>>, u32);

enum Pst {
    Unicode(Rc<UnicodeStore>),
    Ansi(Rc<AnsiStore>),
}

impl Pst {
    fn open(path: &Path) -> std::io::Result<Self> {
        if let Ok(f) = UnicodePstFile::open(path) {
            return Ok(Self::Unicode(UnicodeStore::read(Rc::new(f))?));
        }
        let f = AnsiPstFile::open(path)?;
        Ok(Self::Ansi(AnsiStore::read(Rc::new(f))?))
    }

    fn store(&self) -> Rc<dyn Store> {
        match self {
            Self::Unicode(s) => s.clone(),
            Self::Ansi(s) => s.clone(),
        }
    }

    /// Messaggio completo con i suoi allegati, e quanti allegati non si sono potuti leggere.
    fn message(&self, id: &EntryId) -> std::io::Result<OpenedMessage> {
        fn collect<T: PstAttachment + 'static>(read: impl Iterator<Item = std::io::Result<Rc<T>>>) -> (Vec<Rc<dyn PstAttachment>>, u32) {
            let mut lost = 0;
            let mut out = Vec::new();
            for a in read {
                match a {
                    Ok(a) => out.push(a as Rc<dyn PstAttachment>),
                    Err(_) => lost += 1,
                }
            }
            (out, lost)
        }
        match self {
            Self::Unicode(s) => {
                let m = UnicodeMessage::read(s.clone(), id, None)?;
                let (atts, lost) = collect(attachment_nodes(m.as_ref()).into_iter().map(|n| UnicodeAttachment::read(m.clone(), n, None)));
                Ok((m, atts, lost))
            }
            Self::Ansi(s) => {
                let m = AnsiMessage::read(s.clone(), id, None)?;
                let (atts, lost) = collect(attachment_nodes(m.as_ref()).into_iter().map(|n| AnsiAttachment::read(m.clone(), n, None)));
                Ok((m, atts, lost))
            }
        }
    }
}

fn attachment_nodes(m: &dyn PstMessage) -> Vec<NodeId> {
    match m.attachment_table() {
        Some(t) => t.rows_matrix().map(|row| NodeId::from(u32::from(row.id()))).collect(),
        None => Vec::new(),
    }
}

/// Proprietà di una riga di tabella (destinatari, sottocartelle...).
fn row_props(table: &dyn TableContext, row: &outlook_pst::ltp::table_context::TableRowData) -> HashMap<u16, PropertyValue> {
    let ctx = table.context();
    let Ok(values) = row.columns(ctx) else { return HashMap::new() };
    ctx.columns()
        .iter()
        .zip(values)
        .filter_map(|(col, v)| Some((col.prop_id(), table.read_column(v.as_ref()?, col.prop_type()).ok()?)))
        .collect()
}

fn as_string(v: Option<&PropertyValue>, enc: &'static encoding_rs::Encoding) -> Option<String> {
    let s = match v? {
        PropertyValue::Unicode(u) => String::from_utf16_lossy(u.buffer()),
        PropertyValue::String8(s) => enc.decode_without_bom_handling(s.buffer()).0.into_owned(),
        _ => return None,
    };
    let s = s.trim_end_matches('\0').to_string();
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn as_i32(v: Option<&PropertyValue>) -> Option<i32> {
    match v? {
        PropertyValue::Integer32(n) => Some(*n),
        PropertyValue::Integer16(n) => Some(*n as i32),
        _ => None,
    }
}

fn as_bool(v: Option<&PropertyValue>) -> bool {
    matches!(v, Some(PropertyValue::Boolean(true)))
}

/// Data MAPI (centinaia di nanosecondi dal 1601) in secondi Unix.
fn as_unix_secs(v: Option<&PropertyValue>) -> Option<i64> {
    match v? {
        PropertyValue::Time(t) if *t > 0 => Some(t / 10_000_000 - 11_644_473_600),
        _ => None,
    }
}

fn as_bytes(v: Option<&PropertyValue>) -> Option<&[u8]> {
    match v? {
        PropertyValue::Binary(b) => Some(b.buffer()),
        _ => None,
    }
}

/// Lettura delle proprietà di un messaggio, con la sua codifica per i testi a 8 bit.
struct Props<'a> {
    get: Box<dyn Fn(u16) -> Option<&'a PropertyValue> + 'a>,
    enc: &'static encoding_rs::Encoding,
}

impl<'a> Props<'a> {
    fn of_message(m: &'a dyn PstMessage) -> Self {
        let p = m.properties();
        let cp = as_i32(p.get(MESSAGE_CODEPAGE)).or(as_i32(p.get(INTERNET_CPID))).unwrap_or(1252);
        Self { get: Box::new(move |id| p.get(id)), enc: rtf::encoding_for_codepage(cp as u32) }
    }
    fn str(&self, id: u16) -> Option<String> {
        as_string((self.get)(id), self.enc)
    }
    fn i32(&self, id: u16) -> Option<i32> {
        as_i32((self.get)(id))
    }
    fn time(&self, id: u16) -> Option<i64> {
        as_unix_secs((self.get)(id))
    }
}

fn subject_text(s: Option<String>) -> String {
    // Outlook può salvare l'oggetto con un prefisso di due caratteri (0x01 + lunghezza).
    let s = s.unwrap_or_default();
    if s.starts_with('\u{1}') {
        s.chars().skip(2).collect()
    } else {
        s
    }
}

/// Indirizzo email leggibile: quello SMTP se c'è, altrimenti l'indirizzo
/// salvato quando è già nel formato giusto (non quelli interni di Exchange).
fn smtp_address(smtp: Option<String>, email: Option<String>, addrtype: Option<String>) -> String {
    if let Some(s) = smtp.filter(|s| s.contains('@')) {
        return s;
    }
    match (email, addrtype) {
        (Some(e), t) if e.contains('@') && t.as_deref().map(|t| t.eq_ignore_ascii_case("SMTP")).unwrap_or(true) => e,
        _ => String::new(),
    }
}

fn address<'x>(name: Option<String>, email: String) -> Address<'x> {
    let name = name.filter(|n| n != &email);
    Address::new_address(name, email)
}

/// Intestazioni originali ricevute dal server, senza quelle sul formato del
/// contenuto (che viene ricostruito). Oggetto, mittente e destinatari con
/// caratteri non ASCII si rifanno dai dati di Outlook: nelle intestazioni
/// salvate sono spesso rovinati. Restituisce anche quali vanno rifatti.
fn filtered_headers(headers: &str) -> Option<(String, Vec<&'static str>)> {
    const REBUILD: [&str; 4] = ["subject", "from", "to", "cc"];
    let mut fields: Vec<(String, String)> = Vec::new();
    for line in headers.split_inclusive('\n') {
        let l = line.trim_end_matches(['\r', '\n']);
        if l.is_empty() {
            break;
        }
        if l.starts_with([' ', '\t']) {
            if let Some(last) = fields.last_mut() {
                last.1.push_str(l);
                last.1.push_str("\r\n");
            }
            continue;
        }
        match l.split_once(':') {
            Some((name, _)) if !name.trim().is_empty() && !name.contains(' ') => fields.push((name.trim().to_ascii_lowercase(), format!("{l}\r\n"))),
            // Riga non valida: si scarta, con le sue continuazioni.
            _ => fields.push((String::new(), String::new())),
        }
    }
    if !fields.iter().any(|(n, _)| n == "from") {
        return None;
    }
    let mut out = String::new();
    let mut rebuild = Vec::new();
    for (name, text) in fields {
        if name.is_empty() || name.starts_with("content-") || name == "mime-version" {
            continue;
        }
        if let Some(r) = REBUILD.iter().find(|r| **r == name) {
            if !text.is_ascii() {
                if !rebuild.contains(r) {
                    rebuild.push(*r);
                }
                continue;
            }
        }
        out.push_str(&text);
    }
    Some((out, rebuild))
}

fn mime_for(name: &str, declared: Option<String>) -> String {
    if let Some(m) = declared.filter(|m| m.contains('/')) {
        return m.to_ascii_lowercase();
    }
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "txt" => "text/plain",
        "htm" | "html" => "text/html",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "eml" => "message/rfc822",
        "ics" => "text/calendar",
        "csv" => "text/csv",
        "p7m" => "application/pkcs7-mime",
        "xml" => "application/xml",
        _ => "application/octet-stream",
    }
    .to_string()
}

struct Converted {
    raw: Vec<u8>,
    date: i64,
    flags: Flags,
}

/// Ricostruisce un messaggio MIME da un messaggio del PST.
fn convert_message(m: &dyn PstMessage, atts: &[Rc<dyn PstAttachment>], depth: u8) -> Converted {
    let p = Props::of_message(m);
    let date = p.time(CLIENT_SUBMIT_TIME).or(p.time(DELIVERY_TIME)).or(p.time(CREATION_TIME)).unwrap_or_else(now_secs);
    let msg_flags = p.i32(MESSAGE_FLAGS).unwrap_or(MSGFLAG_READ);
    let flags = Flags {
        unread: msg_flags & MSGFLAG_READ == 0,
        flagged: p.i32(FLAG_STATUS) == Some(2),
        answered: matches!(p.i32(LAST_VERB), Some(102) | Some(103)),
        draft: msg_flags & MSGFLAG_UNSENT != 0,
    };

    // Contenuto: HTML se c'è (anche dentro l'RTF), poi il testo semplice.
    let mut html = match (p.get)(HTML) {
        Some(PropertyValue::Binary(b)) => {
            let cp = p.i32(INTERNET_CPID).unwrap_or(65001);
            Some(rtf::encoding_for_codepage(cp as u32).decode_without_bom_handling(b.buffer()).0.into_owned())
        }
        v => as_string(v, p.enc),
    };
    let mut text = p.str(BODY);
    if html.is_none() {
        if let Some(body) = as_bytes((p.get)(RTF_COMPRESSED)).and_then(rtf::from_compressed) {
            match body {
                rtf::Body::Html(h) => html = Some(h),
                rtf::Body::Text(t) if text.is_none() && !t.is_empty() => text = Some(t),
                _ => {}
            }
        }
    }
    let html = html.map(|h| h.trim_end_matches('\0').to_string()).filter(|h| !h.trim().is_empty());

    // Parti del messaggio.
    let body_part = match (text, html.clone()) {
        (Some(t), Some(h)) => MimePart::new("multipart/alternative", vec![MimePart::new("text/plain", t), MimePart::new("text/html", h)]),
        (None, Some(h)) => MimePart::new("text/html", h),
        (Some(t), None) => MimePart::new("text/plain", t),
        (None, None) => MimePart::new("text/plain", String::new()),
    };
    let html_lower = html.as_deref().unwrap_or("").to_ascii_lowercase();
    let mut inline_parts = Vec::new();
    let mut file_parts = Vec::new();
    for a in atts {
        let ap = a.properties();
        let enc = p.enc;
        let name = as_string(ap.get(ATTACH_LONG_FILENAME), enc)
            .or_else(|| as_string(ap.get(ATTACH_FILENAME), enc))
            .or_else(|| as_string(ap.get(DISPLAY_NAME), enc))
            .unwrap_or_else(|| "allegato".into());
        let cid = as_string(ap.get(ATTACH_CONTENT_ID), enc).map(|c| c.trim_matches(['<', '>']).to_string());
        match a.data() {
            Some(AttachmentData::Binary(b)) => {
                let ct = mime_for(&name, as_string(ap.get(ATTACH_MIME), enc));
                let bytes = b.buffer().to_vec();
                match cid {
                    Some(c) if html_lower.contains(&format!("cid:{}", c.to_ascii_lowercase())) || (as_bool(ap.get(ATTACH_HIDDEN)) && ct.starts_with("image/")) => {
                        inline_parts.push(MimePart::new(ct, bytes).inline().cid(c));
                    }
                    _ => file_parts.push(MimePart::new(ct, bytes).attachment(name)),
                }
            }
            Some(AttachmentData::Message(inner)) if depth < 3 => {
                // Messaggio allegato: si ricostruisce anche quello (senza i suoi allegati).
                let c = convert_message(inner.as_ref(), &[], depth + 1);
                let ip = Props::of_message(inner.as_ref());
                let title = subject_text(ip.str(SUBJECT));
                let file = if title.trim().is_empty() { "messaggio.eml".to_string() } else { format!("{}.eml", title.replace(['/', '\\', ':', '"', '*', '?', '<', '>', '|'], "_")) };
                file_parts.push(MimePart::new("message/rfc822", c.raw).attachment(file));
            }
            _ => {}
        }
    }
    let content = if inline_parts.is_empty() {
        body_part
    } else {
        let mut parts = vec![body_part];
        parts.extend(inline_parts);
        MimePart::new("multipart/related", parts)
    };
    let root = if file_parts.is_empty() {
        content
    } else {
        let mut parts = vec![content];
        parts.extend(file_parts);
        MimePart::new("multipart/mixed", parts)
    };

    // Mittente, destinatari e oggetto come li ha decodificati Outlook.
    let subject = subject_text(p.str(SUBJECT));
    let sender_email = smtp_address(p.str(SENDER_SMTP), p.str(SENDER_EMAIL), p.str(SENDER_ADDRTYPE));
    let repr_email = smtp_address(p.str(SENT_REPR_SMTP), p.str(SENT_REPR_EMAIL), None);
    let (from_name, from_email) = if !repr_email.is_empty() || p.str(SENT_REPR_NAME).is_some() {
        (p.str(SENT_REPR_NAME), repr_email)
    } else {
        (p.str(SENDER_NAME), sender_email)
    };
    let from = (from_name.is_some() || !from_email.is_empty()).then(|| address(from_name, from_email));
    let (mut to, mut cc, mut bcc) = (Vec::new(), Vec::new(), Vec::new());
    if let Some(t) = m.recipient_table() {
        for row in t.rows_matrix() {
            let r = row_props(t.as_ref(), row);
            let email = smtp_address(as_string(r.get(&SMTP_ADDRESS), p.enc), as_string(r.get(&EMAIL_ADDRESS), p.enc), as_string(r.get(&ADDRTYPE), p.enc));
            let name = as_string(r.get(&DISPLAY_NAME), p.enc);
            if email.is_empty() && name.is_none() {
                continue;
            }
            let a = address(name, email);
            match as_i32(r.get(&RECIPIENT_TYPE)).unwrap_or(1) & 0x0F {
                2 => cc.push(a),
                3 => bcc.push(a),
                _ => to.push(a),
            }
        }
    }

    // Intestazioni: quelle originali se il PST le conserva, altrimenti si ricostruiscono.
    if let Some((h, rebuild)) = p.str(TRANSPORT_HEADERS).and_then(|h| filtered_headers(&h)) {
        use mail_builder::headers::Header;
        let mut raw = h.into_bytes();
        for name in rebuild {
            match name {
                "subject" => {
                    raw.extend_from_slice(b"Subject: ");
                    Text::new(subject.clone()).write_header(&mut raw, 9);
                }
                "from" => {
                    if let Some(a) = &from {
                        raw.extend_from_slice(b"From: ");
                        a.write_header(&mut raw, 6);
                    }
                }
                "to" if !to.is_empty() => {
                    raw.extend_from_slice(b"To: ");
                    Address::new_list(to.clone()).write_header(&mut raw, 4);
                }
                "cc" if !cc.is_empty() => {
                    raw.extend_from_slice(b"Cc: ");
                    Address::new_list(cc.clone()).write_header(&mut raw, 4);
                }
                _ => {}
            }
        }
        let mut body = Vec::new();
        let _ = MessageBuilder::new().body(root).write_body(&mut body);
        raw.extend_from_slice(b"MIME-Version: 1.0\r\n");
        raw.extend_from_slice(&body);
        return Converted { raw, date, flags };
    }

    let mut b = MessageBuilder::new().date(Date::new(date)).subject(Text::new(subject));
    if let Some(a) = from {
        b = b.from(a);
    }
    if !to.is_empty() {
        b = b.to(Address::new_list(to));
    }
    if !cc.is_empty() {
        b = b.cc(Address::new_list(cc));
    }
    if !bcc.is_empty() {
        b = b.bcc(Address::new_list(bcc));
    }
    let clean_id = |s: String| s.trim().trim_matches(['<', '>']).to_string();
    b = b.message_id(MessageId::new(p.str(INTERNET_MESSAGE_ID).map(clean_id).unwrap_or_else(|| format!("{}@pst.better-look", mime::random_id()))));
    if let Some(r) = p.str(IN_REPLY_TO) {
        b = b.in_reply_to(MessageId::new(clean_id(r)));
    }
    if let Some(refs) = p.str(REFERENCES) {
        let ids: Vec<String> = refs.split_whitespace().map(|s| clean_id(s.to_string())).filter(|s| !s.is_empty()).collect();
        if !ids.is_empty() {
            b = b.references(MessageId::new_list(ids.into_iter()));
        }
    }
    let raw = b.body(root).write_to_vec().unwrap_or_default();
    Converted { raw, date, flags }
}

/// Cartelle di posta: si saltano calendario, contatti, attività e note.
fn is_mail_folder(class: Option<&str>) -> bool {
    match class {
        None => true,
        Some(c) => c.is_empty() || c.starts_with("IPF.Note") || c.starts_with("IPF.Imap"),
    }
}

fn is_mail_message(class: &str) -> bool {
    let c = class.to_ascii_uppercase();
    c.is_empty() || c == "IPM" || c.starts_with("IPM.NOTE") || c.starts_with("IPM.SCHEDULE.MEETING") || c.starts_with("IPM.POST") || c.starts_with("REPORT.")
}

fn pst_role(name: &str, top_level: bool, is_trash: bool) -> Option<&'static str> {
    if is_trash {
        return Some("deleted");
    }
    if !top_level {
        return None;
    }
    let n = name.trim().to_lowercase();
    if ["inbox", "posta in arrivo"].contains(&n.as_str()) {
        return Some("inbox");
    }
    crate::imap::role_from_name(name, None)
}

struct Walk<'a> {
    engine: &'a Engine,
    pst: Pst,
    account_id: String,
    title: String,
    trash: Option<NodeId>,
    total: u32,
    done: u32,
    imported: u32,
    /// Messaggi e allegati che non si sono potuti leggere.
    skipped: u32,
    lost_attachments: u32,
    used_roles: Vec<&'static str>,
    used_paths: std::collections::HashSet<String>,
}

impl Walk<'_> {
    fn count(&self, folder: &Rc<dyn PstFolder>, depth: u8) -> u32 {
        let class = folder.properties().get(CONTAINER_CLASS).and_then(|v| as_string(Some(v), encoding_rs::WINDOWS_1252));
        if !is_mail_folder(class.as_deref()) || depth > 30 {
            return 0;
        }
        let own = folder.properties().content_count().unwrap_or(0).max(0) as u32;
        own + self.children(folder).iter().map(|c| self.count(c, depth + 1)).sum::<u32>()
    }

    fn children(&self, folder: &Rc<dyn PstFolder>) -> Vec<Rc<dyn PstFolder>> {
        let store = self.pst.store();
        let Some(t) = folder.hierarchy_table() else { return Vec::new() };
        t.rows_matrix()
            .filter_map(|row| {
                let id = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))).ok()?;
                store.open_folder(&id).ok()
            })
            .collect()
    }

    fn folder(&mut self, folder: &Rc<dyn PstFolder>, parent: &str, depth: u8) -> Result<()> {
        let props = folder.properties();
        let class = props.get(CONTAINER_CLASS).and_then(|v| as_string(Some(v), encoding_rs::WINDOWS_1252));
        if !is_mail_folder(class.as_deref()) || depth > 30 {
            return Ok(());
        }
        let name = props.display_name().unwrap_or_else(|_| "Cartella".into());
        let safe = name.replace('/', "\u{2215}");
        let base = if parent.is_empty() { safe } else { format!("{parent}/{safe}") };
        // Due cartelle con lo stesso nome nello stesso punto restano separate.
        let mut path = base.clone();
        let mut n = 2;
        while !self.used_paths.insert(path.clone()) {
            path = format!("{base} ({n})");
            n += 1;
        }
        let is_trash = self.trash == Some(props.node_id());
        let mut role = pst_role(&name, parent.is_empty(), is_trash);
        if let Some(r) = role {
            if self.used_roles.contains(&r) {
                role = None;
            } else {
                self.used_roles.push(r);
            }
        }
        let folder_id = self.engine.db.upsert_folder(&self.account_id, &path, &name, role, Some("/"), true)?;
        let store = self.pst.store();
        if let Some(t) = folder.contents_table() {
            let ids: Vec<NodeId> = t.rows_matrix().map(|row| NodeId::from(u32::from(row.id()))).collect();
            for node in ids {
                self.done += 1;
                if self.done % 50 == 0 {
                    self.engine.import_progress(&self.title, self.done, self.total.max(self.done));
                }
                let Ok(entry) = store.properties().make_entry_id(node) else { continue };
                // Un messaggio rovinato non deve fermare l'importazione degli altri.
                let Ok((m, atts, lost)) = self.pst.message(&entry) else {
                    self.skipped += 1;
                    continue;
                };
                let class = Props::of_message(m.as_ref()).str(MESSAGE_CLASS).unwrap_or_default();
                if !is_mail_message(&class) {
                    continue;
                }
                let c = convert_message(m.as_ref(), &atts, 0);
                insert_raw(self.engine, &self.account_id, folder_id, c.raw, c.date, c.flags)?;
                self.imported += 1;
                self.lost_attachments += lost;
            }
        }
        self.engine.notify_changed(&self.account_id);
        for child in self.children(folder) {
            self.folder(&child, &path, depth + 1)?;
        }
        Ok(())
    }
}

/// Risultato di un'importazione: il nuovo file di dati e quanti messaggi e
/// allegati non si sono potuti leggere (per dirlo all'utente invece di tacerlo).
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    #[serde(flatten)]
    pub account: AccountInfo,
    pub skipped: u32,
    pub lost_attachments: u32,
}

const GENERIC_NAMES: [&str; 6] = ["personal folders", "cartelle personali", "outlook data file", "file di dati di outlook", "outlook", "archive"];

pub fn import_pst_file(engine: &Engine, path: &Path) -> Result<Imported> {
    if path.extension().map(|e| e.eq_ignore_ascii_case("ost")).unwrap_or(false) {
        return Err(Error::user("I file .ost non si possono aprire: sono copie temporanee di una casella che sta sul server. Aggiungi quella casella come account, oppure esporta la posta in un file .pst e apri quello."));
    }
    let unreadable = || Error::user("Impossibile aprire il file. Controlla che sia un file di dati di Outlook (.pst) e che non sia aperto in un altro programma.");
    let pst = Pst::open(path).map_err(|_| unreadable())?;
    let store = pst.store();
    let root_id = store.properties().ipm_sub_tree_entry_id().map_err(|_| unreadable())?;
    let root = store.open_folder(&root_id).map_err(|_| unreadable())?;
    let trash = store.properties().ipm_wastebasket_entry_id().ok().map(|e| e.node_id());
    let name = store
        .properties()
        .display_name()
        .ok()
        .map(|n| n.trim_end_matches('\0').trim().to_string())
        // I nomi predefiniti non aiutano a distinguere più archivi: meglio il nome del file.
        .filter(|n| !n.is_empty() && !GENERIC_NAMES.contains(&n.to_lowercase().as_str()))
        .unwrap_or_else(|| file_stem(path));
    let acct = engine.create_local_account(&name, Some(path.to_string_lossy().to_string()))?;
    let mut walk = Walk { engine, pst, account_id: acct.id.clone(), title: name.clone(), trash, total: 0, done: 0, imported: 0, skipped: 0, lost_attachments: 0, used_roles: Vec::new(), used_paths: Default::default() };
    walk.total = walk.children(&root).iter().map(|c| walk.count(c, 1)).sum();
    engine.import_progress(&name, 0, walk.total);
    let result = (|| {
        for child in walk.children(&root) {
            walk.folder(&child, "", 1)?;
        }
        Ok::<(), Error>(())
    })();
    engine.import_progress(&name, walk.total, walk.total);
    if let Err(e) = result {
        let _ = engine.remove_account(&acct.id);
        return Err(e);
    }
    if walk.imported == 0 {
        let _ = engine.remove_account(&acct.id);
        return Err(Error::user("Il file non contiene messaggi di posta da importare."));
    }
    Ok(Imported { account: AccountInfo { status: engine.status(&acct.id), config: acct }, skipped: walk.skipped, lost_attachments: walk.lost_attachments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_original_headers_without_content_type() {
        let h = "Received: from x\r\n\tby y\r\nFrom: Anna <anna@x.it>\r\nContent-Type: multipart/mixed;\r\n\tboundary=\"abc\"\r\nSubject: Ciao\r\nMIME-Version: 1.0\r\n\r\n";
        let (f, rebuild) = filtered_headers(h).unwrap();
        assert_eq!(f, "Received: from x\r\n\tby y\r\nFrom: Anna <anna@x.it>\r\nSubject: Ciao\r\n");
        assert!(rebuild.is_empty());
        assert!(filtered_headers("Subject: senza mittente\r\n").is_none());
        let (f, rebuild) = filtered_headers("From: Ã¨ <a@b.it>\r\nSubject: caff\u{e8}\r\n\tdue\r\nTo: c@d.it\r\n").unwrap();
        assert_eq!(f, "To: c@d.it\r\n");
        assert_eq!(rebuild, vec!["from", "subject"]);
    }

    #[test]
    fn exchange_addresses_are_not_used_as_email() {
        assert_eq!(smtp_address(None, Some("/O=ACME/OU=EXCHANGE/CN=RECIPIENTS/CN=ANNA".into()), Some("EX".into())), "");
        assert_eq!(smtp_address(Some("anna@acme.it".into()), Some("/O=ACME".into()), Some("EX".into())), "anna@acme.it");
        assert_eq!(smtp_address(None, Some("anna@acme.it".into()), Some("SMTP".into())), "anna@acme.it");
    }

    #[test]
    fn folder_roles() {
        assert_eq!(pst_role("Posta in arrivo", true, false), Some("inbox"));
        assert_eq!(pst_role("Posta inviata", true, false), Some("sent"));
        assert_eq!(pst_role("Elementi eliminati", true, false), Some("deleted"));
        assert_eq!(pst_role("Clienti", true, false), None);
        assert_eq!(pst_role("Bozze", false, false), None);
        assert_eq!(pst_role("Cestino", false, true), Some("deleted"));
    }

    #[test]
    fn mail_classes() {
        assert!(is_mail_message("IPM.Note"));
        assert!(is_mail_message("IPM.Note.SMIME.MultipartSigned"));
        assert!(is_mail_message("IPM.Schedule.Meeting.Request"));
        assert!(!is_mail_message("IPM.Contact"));
        assert!(!is_mail_message("IPM.Appointment"));
        assert!(is_mail_folder(Some("IPF.Note")));
        assert!(!is_mail_folder(Some("IPF.Appointment")));
    }

    /// Prova su file PST veri: BETTERLOOK_PST=percorso cargo test pst_reale -- --nocapture
    #[test]
    fn pst_reale() {
        let Ok(path) = std::env::var("BETTERLOOK_PST") else { return };
        let pst = Pst::open(Path::new(&path)).expect("PST leggibile");
        let store = pst.store();
        let root = store.open_folder(&store.properties().ipm_sub_tree_entry_id().unwrap()).unwrap();
        fn walk(pst: &Pst, f: &Rc<dyn PstFolder>, indent: usize, out: &mut u32) {
            let store = pst.store();
            let name = f.properties().display_name().unwrap_or_default();
            let class = f.properties().get(CONTAINER_CLASS).and_then(|v| as_string(Some(v), encoding_rs::WINDOWS_1252)).unwrap_or_default();
            println!("{}{} [{}] {}", " ".repeat(indent), name, class, f.properties().content_count().unwrap_or(0));
            if let Some(t) = f.contents_table() {
                for row in t.rows_matrix().take(3) {
                    let id = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))).unwrap();
                    match pst.message(&id) {
                        Ok((m, atts, _)) => {
                            let c = convert_message(m.as_ref(), &atts, 0);
                            let parsed = mime::parse(&c.raw).expect("MIME valido");
                            let nm = parsed.to_new_message(0, true, 0);
                            println!("{}  - {:?} | {} <{}> | {} allegati | html: {} | {} byte", " ".repeat(indent), nm.subject, nm.from.name, nm.from.email, parsed.attachments().len(), parsed.html().is_some(), c.raw.len());
                            *out += 1;
                        }
                        Err(e) => println!("{}  ! {e}", " ".repeat(indent)),
                    }
                }
            }
            if let Some(t) = f.hierarchy_table() {
                for row in t.rows_matrix() {
                    let id = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))).unwrap();
                    if let Ok(c) = store.open_folder(&id) {
                        walk(pst, &c, indent + 2, out);
                    }
                }
            }
        }
        let mut n = 0;
        walk(&pst, &root, 0, &mut n);
        if let Ok(dir) = std::env::var("BETTERLOOK_PST_EML") {
            if let Some(t) = root.hierarchy_table() {
                let mut k = 0;
                for row in t.rows_matrix() {
                    let id = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))).unwrap();
                    let Ok(f) = store.open_folder(&id) else { continue };
                    let Some(ct) = f.contents_table() else { continue };
                    for row in ct.rows_matrix().take(5) {
                        let id = store.properties().make_entry_id(NodeId::from(u32::from(row.id()))).unwrap();
                        if let Ok((m, atts, _)) = pst.message(&id) {
                            k += 1;
                            std::fs::write(format!("{dir}/{k}.eml"), convert_message(m.as_ref(), &atts, 0).raw).unwrap();
                        }
                    }
                }
            }
        }
        println!("messaggi letti: {n}");
    }
}
