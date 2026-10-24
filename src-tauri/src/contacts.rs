//! Rubrica: lettura dei contatti esportati dagli altri programmi e
//! scrittura in vCard.
//!
//! - vCard (.vcf) 2.1, 3.0 e 4.0: Outlook, Apple Contatti, Google, Thunderbird;
//! - CSV: Outlook (anche in italiano), Google, Thunderbird e fogli di calcolo;
//! - LDIF: rubrica di Thunderbird.

use base64::Engine as _;

use crate::model::{Contact, Phone};

/// Testo del file: UTF-8 (con o senza BOM), UTF-16 con BOM, altrimenti
/// Windows-1252 (i CSV di Outlook per Windows).
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(rest).into_owned();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return encoding_rs::UTF_16LE.decode(bytes).0.into_owned();
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return encoding_rs::UTF_16BE.decode(bytes).0.into_owned();
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

/// Riconosce il formato dal nome del file e dal contenuto.
pub fn parse_file(bytes: &[u8], file_name: &str) -> Vec<Contact> {
    let text = decode(bytes);
    let ext = file_name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    let head = text.trim_start().get(..200.min(text.trim_start().len())).unwrap_or("").to_ascii_uppercase();
    if ext == "vcf" || ext == "vcard" || head.starts_with("BEGIN:VCARD") {
        parse_vcard(&text)
    } else if ext == "ldif" || ext == "ldi" || head.starts_with("DN:") || head.starts_with("VERSION: 1") {
        parse_ldif(&text)
    } else {
        parse_csv(&text)
    }
}

// ----------------------------------------------------------------------
// Pezzi comuni
// ----------------------------------------------------------------------

fn add_email(c: &mut Contact, e: &str) {
    let e = e.trim().trim_start_matches("mailto:").trim();
    if e.contains('@') && !c.emails.iter().any(|x| x.eq_ignore_ascii_case(e)) {
        c.emails.push(e.to_string());
    }
}

fn add_phone(c: &mut Contact, kind: &str, number: &str) {
    let n = number.trim().trim_start_matches("tel:").trim();
    if n.is_empty() || c.phones.iter().any(|p| digits(&p.number) == digits(n)) {
        return;
    }
    c.phones.push(Phone { kind: kind.to_string(), number: n.to_string() });
}

fn digits(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Tipo di telefono dalle parole dell'etichetta (vCard, CSV, Google).
fn phone_kind(label: &str) -> &'static str {
    let l = label.to_lowercase();
    if l.contains("fax") {
        "fax"
    } else if l.contains("cell") || l.contains("mobile") {
        "cellulare"
    } else if l.contains("work") || l.contains("business") || l.contains("ufficio") || l.contains("lavoro") || l.contains("uff.") || l.contains("company") {
        "lavoro"
    } else if l.contains("home") || l.contains("casa") || l.contains("abitazione") || l.contains("ab.") {
        "casa"
    } else {
        "altro"
    }
}

/// Indirizzo su più righe: via, "CAP Città (Provincia)", paese.
fn format_address(street: &str, city: &str, region: &str, postal: &str, country: &str) -> String {
    let mut line2 = [postal.trim(), city.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    if !region.trim().is_empty() {
        line2 = if line2.is_empty() { region.trim().to_string() } else { format!("{line2} ({})", region.trim()) };
    }
    [street.trim(), line2.as_str(), country.trim()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("\n")
}

/// Compleanno in "AAAA-MM-GG". `day_first`: 14/05/1980 (italiano) invece di 5/14/1980.
fn normalize_birthday(s: &str, day_first: bool) -> String {
    let s = s.trim();
    if s.is_empty() || s.starts_with("0/0/") || s == "00/00/00" {
        return String::new();
    }
    let date = s.split('T').next().unwrap_or(s);
    if let Some(md) = date.strip_prefix("--") {
        let d = digits(md);
        if d.len() == 4 {
            return format!("--{}-{}", &d[..2], &d[2..]);
        }
    }
    let d = digits(date);
    if d.len() == 8 && (date.contains('-') || !date.contains('/')) && date.find('-').map_or(true, |i| i == 4) {
        return format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]);
    }
    let parts: Vec<&str> = date.split(['/', '.', '-']).map(|p| p.trim()).collect();
    if parts.len() == 3 {
        let n: Vec<u32> = parts.iter().filter_map(|p| p.parse().ok()).collect();
        if n.len() == 3 {
            let (a, b, mut y) = (n[0], n[1], n[2]);
            if y < 100 {
                y += if y > 30 { 1900 } else { 2000 };
            }
            let (day, month) = if a > 12 { (a, b) } else if b > 12 { (b, a) } else if day_first { (a, b) } else { (b, a) };
            if (1..=12).contains(&month) && (1..=31).contains(&day) {
                return format!("{y:04}-{month:02}-{day:02}");
            }
        }
    }
    String::new()
}

fn finish(mut c: Contact) -> Option<Contact> {
    for f in [&mut c.first_name, &mut c.last_name, &mut c.display_name, &mut c.company, &mut c.job_title, &mut c.website, &mut c.notes] {
        *f = f.trim().to_string();
    }
    c.address = c.address.trim().to_string();
    if c.is_empty() {
        None
    } else {
        Some(c)
    }
}

// ----------------------------------------------------------------------
// vCard
// ----------------------------------------------------------------------

/// Righe logiche: le righe che iniziano con uno spazio continuano la
/// precedente; nel quoted-printable un "=" finale fa lo stesso.
fn vcard_lines(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut qp_open = false;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if qp_open {
            if let Some(last) = out.last_mut() {
                last.pop(); // il "=" di continuazione
                last.push_str(line);
                qp_open = last.ends_with('=') && is_qp(last);
                continue;
            }
        }
        if (line.starts_with(' ') || line.starts_with('\t')) && !out.is_empty() {
            out.last_mut().unwrap().push_str(&line[1..]);
            continue;
        }
        if line.is_empty() {
            continue;
        }
        out.push(line.to_string());
        qp_open = line.ends_with('=') && is_qp(line);
    }
    out
}

fn is_qp(line: &str) -> bool {
    let head = line.split(':').next().unwrap_or("").to_ascii_uppercase();
    head.contains("QUOTED-PRINTABLE")
}

fn qp_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' && i + 2 < b.len() + 1 {
            if let (Some(h), Some(l)) = (b.get(i + 1).and_then(|c| (*c as char).to_digit(16)), b.get(i + 2).and_then(|c| (*c as char).to_digit(16))) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Separa una riga in nome, parametri e valore.
fn split_prop(line: &str) -> Option<(String, Vec<String>, String)> {
    let mut in_quote = false;
    let mut cut = None;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            ':' if !in_quote => {
                cut = Some(i);
                break;
            }
            _ => {}
        }
    }
    let i = cut?;
    let (head, value) = (&line[..i], &line[i + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.rsplit('.').next()?.trim().to_ascii_uppercase();
    let params = parts.map(|p| p.trim().to_string()).collect();
    Some((name, params, value.to_string()))
}

fn param<'a>(params: &'a [String], key: &str) -> Option<&'a str> {
    params.iter().find_map(|p| {
        let (k, v) = p.split_once('=')?;
        k.trim().eq_ignore_ascii_case(key).then(|| v.trim().trim_matches('"'))
    })
}

/// Tipi di una proprietà: "TYPE=work,voice", "TYPE=work;TYPE=voice" o "WORK;VOICE" (2.1).
fn types(params: &[String]) -> String {
    params
        .iter()
        .map(|p| match p.split_once('=') {
            Some((k, v)) if k.trim().eq_ignore_ascii_case("type") => v.to_string(),
            Some(_) => String::new(),
            None => p.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
        .to_lowercase()
}

/// Divide un valore strutturato (N, ADR, ORG) sui ";" non preceduti da "\".
fn components(v: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut esc = false;
    for ch in v.chars() {
        if esc {
            out.last_mut().unwrap().push('\\');
            out.last_mut().unwrap().push(ch);
            esc = false;
        } else if ch == '\\' {
            esc = true;
        } else if ch == ';' {
            out.push(String::new());
        } else {
            out.last_mut().unwrap().push(ch);
        }
    }
    out.into_iter().map(|s| unescape(&s)).collect()
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(ch) = it.next() {
        if ch == '\\' {
            match it.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(c) => out.push(c),
                None => {}
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn parse_vcard(text: &str) -> Vec<Contact> {
    let mut out = Vec::new();
    let mut cur: Option<Contact> = None;
    let mut addr: Option<(bool, String)> = None;
    for line in vcard_lines(text) {
        let Some((name, params, raw)) = split_prop(&line) else { continue };
        let enc = param(&params, "ENCODING").unwrap_or("").to_ascii_uppercase();
        let plain_qp = params.iter().any(|p| p.eq_ignore_ascii_case("QUOTED-PRINTABLE"));
        let value = if enc == "QUOTED-PRINTABLE" || plain_qp {
            let bytes = qp_decode(&raw);
            let label = param(&params, "CHARSET").unwrap_or("utf-8");
            let enc = encoding_rs::Encoding::for_label(label.as_bytes()).unwrap_or(encoding_rs::UTF_8);
            enc.decode(&bytes).0.into_owned()
        } else if enc == "B" || enc == "BASE64" {
            if name == "PHOTO" || name == "LOGO" || name == "SOUND" || name == "KEY" {
                continue;
            }
            String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(raw.trim()).unwrap_or_default()).into_owned()
        } else {
            raw.clone()
        };
        match name.as_str() {
            "BEGIN" if value.trim().eq_ignore_ascii_case("VCARD") => {
                cur = Some(Contact::default());
                addr = None;
            }
            "END" if value.trim().eq_ignore_ascii_case("VCARD") => {
                if let Some(mut c) = cur.take() {
                    if let Some((_, a)) = addr.take() {
                        c.address = a;
                    }
                    if let Some(c) = finish(c) {
                        out.push(c);
                    }
                }
            }
            _ => {
                let Some(c) = cur.as_mut() else { continue };
                match name.as_str() {
                    "FN" => c.display_name = unescape(&value),
                    "N" => {
                        let n = components(&value);
                        c.last_name = n.first().cloned().unwrap_or_default();
                        c.first_name = [n.get(1), n.get(2)].iter().flatten().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
                    }
                    "EMAIL" => add_email(c, &unescape(&value)),
                    "TEL" => add_phone(c, phone_kind(&types(&params)), &unescape(&value)),
                    "ORG" => c.company = components(&value).into_iter().find(|s| !s.trim().is_empty()).unwrap_or_default(),
                    "TITLE" => c.job_title = unescape(&value),
                    "URL" if c.website.is_empty() => c.website = unescape(&value),
                    "BDAY" => c.birthday = normalize_birthday(&value, true),
                    "NOTE" => c.notes = unescape(&value),
                    "ADR" => {
                        let a = components(&value);
                        let get = |i: usize| a.get(i).map(|s| s.as_str()).unwrap_or("");
                        let street = [get(0), get(1), get(2)].iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
                        let text = format_address(&street, get(3), get(4), get(5), get(6));
                        let work = types(&params).contains("work");
                        // Prima l'indirizzo di lavoro, come Outlook.
                        if !text.is_empty() && addr.as_ref().map_or(true, |(was_work, _)| work && !was_work) {
                            addr = Some((work, text));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

/// vCard 3.0, leggibile da Outlook, Apple, Google e Thunderbird.
pub fn to_vcard(list: &[Contact]) -> String {
    fn esc(s: &str) -> String {
        s.replace('\\', "\\\\").replace('\n', "\\n").replace(',', "\\,").replace(';', "\\;")
    }
    /// Righe di al massimo 75 byte, come vuole lo standard.
    fn fold(line: &str, out: &mut String) {
        let mut cur = 0;
        let mut first = true;
        let mut start = 0;
        for (i, ch) in line.char_indices() {
            let limit = if first { 75 } else { 74 };
            if cur + ch.len_utf8() > limit {
                if !first {
                    out.push(' ');
                }
                out.push_str(&line[start..i]);
                out.push_str("\r\n");
                start = i;
                cur = 0;
                first = false;
            }
            cur += ch.len_utf8();
        }
        if !first {
            out.push(' ');
        }
        out.push_str(&line[start..]);
        out.push_str("\r\n");
    }
    let mut out = String::new();
    for c in list {
        let mut lines = vec!["BEGIN:VCARD".to_string(), "VERSION:3.0".to_string()];
        lines.push(format!("N:{};{};;;", esc(&c.last_name), esc(&c.first_name)));
        lines.push(format!("FN:{}", esc(&c.name())));
        if !c.company.is_empty() {
            lines.push(format!("ORG:{}", esc(&c.company)));
        }
        if !c.job_title.is_empty() {
            lines.push(format!("TITLE:{}", esc(&c.job_title)));
        }
        for (i, e) in c.emails.iter().enumerate() {
            lines.push(format!("EMAIL;TYPE=INTERNET{}:{}", if i == 0 { ",PREF" } else { "" }, esc(e)));
        }
        for p in &c.phones {
            let t = match p.kind.as_str() {
                "cellulare" => "CELL",
                "lavoro" => "WORK,VOICE",
                "casa" => "HOME,VOICE",
                "fax" => "WORK,FAX",
                _ => "VOICE",
            };
            lines.push(format!("TEL;TYPE={t}:{}", esc(&p.number)));
        }
        if !c.address.is_empty() {
            lines.push(format!("ADR;TYPE=WORK:;;{};;;;", esc(&c.address)));
        }
        if !c.website.is_empty() {
            lines.push(format!("URL:{}", esc(&c.website)));
        }
        if !c.birthday.is_empty() {
            lines.push(format!("BDAY:{}", c.birthday));
        }
        if !c.notes.is_empty() {
            lines.push(format!("NOTE:{}", esc(&c.notes)));
        }
        lines.push("END:VCARD".into());
        for l in lines {
            fold(&l, &mut out);
        }
    }
    out
}

// ----------------------------------------------------------------------
// CSV
// ----------------------------------------------------------------------

/// Righe e campi di un CSV (RFC 4180: virgolette, a capo dentro i campi).
pub fn csv_rows(text: &str, sep: char) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(ch);
            }
        } else if ch == '"' && field.is_empty() {
            quoted = true;
        } else if ch == sep {
            row.push(std::mem::take(&mut field));
        } else if ch == '\n' || ch == '\r' {
            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut field));
            if row.iter().any(|f| !f.is_empty()) {
                rows.push(std::mem::take(&mut row));
            } else {
                row.clear();
            }
        } else {
            field.push(ch);
        }
    }
    row.push(field);
    if row.iter().any(|f| !f.is_empty()) {
        rows.push(row);
    }
    rows
}

#[derive(Debug, Clone, PartialEq)]
enum Col {
    First,
    Middle,
    Last,
    Display,
    Email,
    /// Telefono con il tipo già noto dall'intestazione.
    Phone(&'static str),
    /// Telefono numerato di Google: tipo e valore in colonne separate.
    GPhoneType(String),
    GPhoneValue(String),
    Company,
    Title,
    /// Parte di un indirizzo: (lavoro?, parte).
    Addr(bool, &'static str),
    AddrFormatted(String),
    AddrLabel(String),
    Website,
    Birthday,
    BirthYear,
    BirthMonth,
    BirthDay,
    Notes,
}

fn classify(h: &str) -> Option<Col> {
    let h = h.trim().trim_start_matches('\u{feff}').trim().to_lowercase();
    // Intestazioni numerate di Google: "Phone 1 - Value", "E-mail 2 - Label"...
    if let Some((left, right)) = h.split_once(" - ") {
        let mut w = left.rsplitn(2, ' ');
        let n = w.next().unwrap_or("").to_string();
        let what = w.next().unwrap_or("");
        let right = right.trim();
        if n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty() {
            return match (what, right) {
                ("e-mail" | "email", "value") => Some(Col::Email),
                ("phone", "type" | "label") => Some(Col::GPhoneType(n)),
                ("phone", "value") => Some(Col::GPhoneValue(n)),
                ("address", "formatted") => Some(Col::AddrFormatted(n)),
                ("address", "type" | "label") => Some(Col::AddrLabel(n)),
                ("organization", "name") => Some(Col::Company),
                ("organization", "title") => Some(Col::Title),
                ("website", "value") => Some(Col::Website),
                _ => None,
            };
        }
    }
    let is = |list: &[&str]| list.contains(&h.as_str());
    if is(&["first name", "given name", "nome", "first", "nome di battesimo"]) {
        return Some(Col::First);
    }
    if is(&["middle name", "additional name", "secondo nome", "altro nome"]) {
        return Some(Col::Middle);
    }
    if is(&["last name", "family name", "cognome", "last", "surname"]) {
        return Some(Col::Last);
    }
    if is(&["name", "display name", "nome visualizzato", "nome completo", "full name", "nome e cognome", "nominativo"]) {
        return Some(Col::Display);
    }
    if is(&["company", "società", "societa", "organization", "organizzazione", "azienda", "organization name", "ditta", "ragione sociale"]) {
        return Some(Col::Company);
    }
    if is(&["job title", "posizione", "qualifica", "organization title", "mansione", "ruolo", "work title"]) {
        return Some(Col::Title);
    }
    if is(&["notes", "note", "notes 1", "annotazioni"]) {
        return Some(Col::Notes);
    }
    if is(&["birthday", "compleanno", "data di nascita", "birthdate"]) {
        return Some(Col::Birthday);
    }
    if is(&["birth year"]) {
        return Some(Col::BirthYear);
    }
    if is(&["birth month"]) {
        return Some(Col::BirthMonth);
    }
    if is(&["birth day"]) {
        return Some(Col::BirthDay);
    }
    let has = |w: &[&str]| w.iter().any(|x| h.contains(x));
    if has(&["e-mail", "email", "posta elettronica", "mail"]) {
        if has(&["type", "tipo", "display", "visualizzat", "label", "etichetta"]) {
            return None;
        }
        return Some(Col::Email);
    }
    if has(&["web page", "pagina web", "website", "web site", "sito", "url"]) {
        return Some(Col::Website);
    }
    if has(&["phone", "telefono", "cellulare", "mobile", "fax"]) && !has(&["pager", "cercapersone", "radiotelefono", "isdn", "telex"]) {
        return Some(Col::Phone(phone_kind(&h)));
    }
    let work = has(&["business", "work", "(uff.)", "ufficio", "lavoro"]);
    let home = has(&["home", "(ab.)", "abitazione", "casa"]);
    if work || home || has(&["other street", "altra via", "altro indirizzo"]) {
        let part = if has(&["street", "via", "address", "indirizzo"]) {
            Some("street")
        } else if has(&["city", "città", "citta"]) {
            Some("city")
        } else if has(&["postal", "zip", "cap"]) {
            Some("postal")
        } else if has(&["country", "paese"]) {
            Some("country")
        } else if has(&["state", "provincia", "region"]) {
            Some("region")
        } else {
            None
        };
        return part.map(|p| Col::Addr(work, p));
    }
    None
}

pub fn parse_csv(text: &str) -> Vec<Contact> {
    let first_line = text.lines().next().unwrap_or("");
    let sep = [',', ';', '\t'].into_iter().max_by_key(|s| first_line.matches(*s).count()).unwrap_or(',');
    let rows = csv_rows(text, sep);
    let Some((header, data)) = rows.split_first() else { return Vec::new() };
    let cols: Vec<Option<Col>> = header.iter().map(|h| classify(h)).collect();
    // Intestazioni in italiano: date scritte giorno/mese/anno.
    let italian = header.iter().any(|h| {
        let h = h.to_lowercase();
        h == "nome" || h == "cognome" || h.contains("posta elettronica")
    });
    let mut out = Vec::new();
    for row in data {
        let mut c = Contact::default();
        let mut gphone: Vec<(String, String, String)> = Vec::new(); // (n, tipo, valore)
        let mut gaddr: Vec<(String, String, String)> = Vec::new();
        let mut parts: [[String; 5]; 2] = Default::default(); // [lavoro, casa] x [via, città, cap, prov, paese]
        let mut birth = (String::new(), String::new(), String::new());
        let mut middle = String::new();
        for (i, v) in row.iter().enumerate() {
            let v = v.trim();
            if v.is_empty() {
                continue;
            }
            let Some(Some(col)) = cols.get(i) else { continue };
            match col {
                Col::First => c.first_name = v.into(),
                Col::Middle => middle = v.into(),
                Col::Last => c.last_name = v.into(),
                Col::Display => c.display_name = v.into(),
                Col::Email => {
                    for e in v.split(":::") {
                        add_email(&mut c, e);
                    }
                }
                Col::Phone(kind) => {
                    for n in v.split(":::") {
                        add_phone(&mut c, kind, n);
                    }
                }
                Col::GPhoneType(n) => upsert(&mut gphone, n, Some(v), None),
                Col::GPhoneValue(n) => upsert(&mut gphone, n, None, Some(v)),
                Col::Company if c.company.is_empty() => c.company = v.into(),
                Col::Title if c.job_title.is_empty() => c.job_title = v.into(),
                Col::Addr(work, part) => {
                    let k = if *work { 0 } else { 1 };
                    let j = ["street", "city", "postal", "region", "country"].iter().position(|p| p == part).unwrap_or(0);
                    let slot = &mut parts[k][j];
                    *slot = if slot.is_empty() { v.to_string() } else { format!("{slot}, {v}") };
                }
                Col::AddrFormatted(n) => upsert(&mut gaddr, n, None, Some(v)),
                Col::AddrLabel(n) => upsert(&mut gaddr, n, Some(v), None),
                Col::Website if c.website.is_empty() => c.website = v.into(),
                Col::Birthday => c.birthday = normalize_birthday(v, italian),
                Col::BirthYear => birth.0 = v.into(),
                Col::BirthMonth => birth.1 = v.into(),
                Col::BirthDay => birth.2 = v.into(),
                Col::Notes => c.notes = v.into(),
                _ => {}
            }
        }
        if !middle.is_empty() {
            c.first_name = format!("{} {middle}", c.first_name).trim().to_string();
        }
        for (_, kind, value) in &gphone {
            for n in value.split(":::") {
                add_phone(&mut c, phone_kind(kind), n);
            }
        }
        if c.birthday.is_empty() && !birth.1.is_empty() && !birth.2.is_empty() {
            c.birthday = match birth.0.parse::<u32>() {
                Ok(y) => format!("{y:04}-{:0>2}-{:0>2}", birth.1, birth.2),
                Err(_) => format!("--{:0>2}-{:0>2}", birth.1, birth.2),
            };
        }
        let built = |p: &[String; 5]| format_address(&p[0], &p[1], &p[3], &p[2], &p[4]);
        c.address = [built(&parts[0]), built(&parts[1])].into_iter().find(|a| !a.is_empty()).unwrap_or_else(|| {
            // Google: prima l'indirizzo di lavoro, altrimenti il primo.
            gaddr.iter().find(|(_, l, v)| phone_kind(l) == "lavoro" && !v.is_empty()).or_else(|| gaddr.iter().find(|(_, _, v)| !v.is_empty())).map(|(_, _, v)| v.clone()).unwrap_or_default()
        });
        if let Some(c) = finish(c) {
            out.push(c);
        }
    }
    out
}

fn upsert(list: &mut Vec<(String, String, String)>, n: &str, kind: Option<&str>, value: Option<&str>) {
    let i = match list.iter().position(|(k, _, _)| k == n) {
        Some(i) => i,
        None => {
            list.push((n.to_string(), String::new(), String::new()));
            list.len() - 1
        }
    };
    if let Some(k) = kind {
        list[i].1 = k.to_string();
    }
    if let Some(v) = value {
        list[i].2 = v.to_string();
    }
}

// ----------------------------------------------------------------------
// LDIF (Thunderbird)
// ----------------------------------------------------------------------

pub fn parse_ldif(text: &str) -> Vec<Contact> {
    // Righe logiche: quelle che iniziano con uno spazio continuano la precedente.
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(rest) = line.strip_prefix(' ') {
            if let Some(last) = lines.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        lines.push(line.to_string());
    }
    let mut out = Vec::new();
    let mut rec: Vec<(String, String)> = Vec::new();
    let flush = |rec: &mut Vec<(String, String)>, out: &mut Vec<Contact>| {
        if rec.is_empty() {
            return;
        }
        let get = |k: &str| rec.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone()).unwrap_or_default();
        let is_list = rec.iter().any(|(a, v)| a == "objectclass" && v.eq_ignore_ascii_case("groupOfNames"));
        if !is_list {
            let mut c = Contact { display_name: get("cn"), first_name: get("givenname"), last_name: get("sn"), company: get("o"), job_title: get("title"), notes: get("description"), ..Default::default() };
            for (a, v) in rec.iter() {
                match a.as_str() {
                    "mail" | "mozillasecondemail" => add_email(&mut c, v),
                    "telephonenumber" => add_phone(&mut c, "lavoro", v),
                    "homephone" => add_phone(&mut c, "casa", v),
                    "mobile" => add_phone(&mut c, "cellulare", v),
                    "facsimiletelephonenumber" | "fax" => add_phone(&mut c, "fax", v),
                    _ => {}
                }
            }
            let street = [get("street"), get("mozillaworkstreet2")].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", ");
            c.address = format_address(&street, &get("l"), &get("st"), &get("postalcode"), &get("c"));
            if c.address.is_empty() {
                let street = [get("mozillahomestreet"), get("mozillahomestreet2")].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(", ");
                c.address = format_address(&street, &get("mozillahomelocalityname"), &get("mozillahomestate"), &get("mozillahomepostalcode"), &get("mozillahomecountryname"));
            }
            c.website = [get("mozillaworkurl"), get("workurl"), get("mozillahomeurl"), get("homeurl")].into_iter().find(|s| !s.is_empty()).unwrap_or_default();
            let (y, m, d) = (get("birthyear"), get("birthmonth"), get("birthday"));
            if !m.is_empty() && !d.is_empty() {
                c.birthday = if y.is_empty() { format!("--{m:0>2}-{d:0>2}") } else { format!("{y:0>4}-{m:0>2}-{d:0>2}") };
            }
            if let Some(c) = finish(c) {
                out.push(c);
            }
        }
        rec.clear();
    };
    for line in lines {
        if line.trim().is_empty() {
            flush(&mut rec, &mut out);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some((attr, rest)) = line.split_once(':') else { continue };
        let attr = attr.trim().to_lowercase();
        let value = if let Some(b64) = rest.strip_prefix(':') {
            String::from_utf8_lossy(&base64::engine::general_purpose::STANDARD.decode(b64.trim()).unwrap_or_default()).into_owned()
        } else {
            rest.trim().to_string()
        };
        rec.push((attr, value));
    }
    flush(&mut rec, &mut out);
    out
}

// ----------------------------------------------------------------------
// Unione con la rubrica
// ----------------------------------------------------------------------

/// Completa `into` con i dati di `from` senza cancellare nulla. Vero se è
/// cambiato qualcosa.
pub fn merge(into: &mut Contact, from: &Contact) -> bool {
    let before = into.clone();
    // Il nome da mostrare serve solo se dice qualcosa di diverso.
    if into.display_name.trim().is_empty() && !from.display_name.trim().is_empty() && from.display_name.trim() != into.name() {
        into.display_name = from.display_name.clone();
    }
    for (dst, src) in [
        (&mut into.first_name, &from.first_name),
        (&mut into.last_name, &from.last_name),
        (&mut into.company, &from.company),
        (&mut into.job_title, &from.job_title),
        (&mut into.address, &from.address),
        (&mut into.website, &from.website),
        (&mut into.birthday, &from.birthday),
    ] {
        if dst.trim().is_empty() && !src.trim().is_empty() {
            *dst = src.clone();
        }
    }
    if !from.notes.trim().is_empty() && !into.notes.contains(from.notes.trim()) {
        into.notes = if into.notes.trim().is_empty() { from.notes.clone() } else { format!("{}\n{}", into.notes.trim_end(), from.notes) };
    }
    for e in &from.emails {
        add_email(into, e);
    }
    for p in &from.phones {
        add_phone(into, &p.kind, &p.number);
    }
    *into != before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcard_from_outlook_apple_and_google() {
        // Outlook: vCard 2.1, quoted-printable in Windows-1252, tipi senza TYPE=.
        let outlook = "BEGIN:VCARD\r\nVERSION:2.1\r\nN;LANGUAGE=it;CHARSET=Windows-1252:Orlandi;Davide\r\nFN:Davide Orlandi\r\nORG:Studio Orlandi Architetti\r\nTITLE:Architetto\r\nTEL;WORK;VOICE:+39 0444 123456\r\nTEL;CELL;VOICE:+39 333 1234567\r\nADR;WORK;PREF;ENCODING=QUOTED-PRINTABLE;CHARSET=Windows-1252:;;Contr=E0 Porti 12;Vicenza;VI;36100;Italia\r\nEMAIL;PREF;INTERNET:d.orlandi@studiorlandi.it\r\nNOTE;ENCODING=QUOTED-PRINTABLE;CHARSET=Windows-1252:Preferisce essere chiamato=\r\n la mattina.\r\nEND:VCARD\r\n";
        let c = &parse_vcard(&decode(outlook.as_bytes()))[0];
        assert_eq!((c.first_name.as_str(), c.last_name.as_str()), ("Davide", "Orlandi"));
        assert_eq!(c.company, "Studio Orlandi Architetti");
        assert_eq!(c.job_title, "Architetto");
        assert_eq!(c.emails, vec!["d.orlandi@studiorlandi.it".to_string()]);
        assert_eq!(c.phones.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(), vec!["lavoro", "cellulare"]);
        assert_eq!(c.address, "Contrà Porti 12\n36100 Vicenza (VI)\nItalia");
        assert_eq!(c.notes, "Preferisce essere chiamato la mattina.");

        // Apple: vCard 3.0, gruppi item1., righe piegate, foto da saltare.
        let apple = "BEGIN:VCARD\nVERSION:3.0\nN:Marchetti;Lucia;;;\nFN:Lucia Marchetti\nORG:Tessuti Marchetti;\nitem1.EMAIL;type=INTERNET;type=pref:l.marchetti@tessuti.exa\n mple\nitem1.X-ABLabel:_$!<Work>!$_\nTEL;type=CELL;type=VOICE;type=pref:+39 347 765 4321\nBDAY:1985-03-09\nPHOTO;ENCODING=b;TYPE=JPEG:/9j/4AAQSkZJRgABAQ\n AAAQABAAD\nEND:VCARD\n";
        let c = &parse_vcard(apple)[0];
        assert_eq!(c.emails, vec!["l.marchetti@tessuti.example".to_string()]);
        assert_eq!(c.company, "Tessuti Marchetti");
        assert_eq!(c.birthday, "1985-03-09");
        assert_eq!(c.phones[0].kind, "cellulare");

        // Google e vCard 4.0: tel:, più indirizzi, escape.
        let google = "BEGIN:VCARD\nVERSION:4.0\nFN:Bassi\\, Rinaldi & C.\nEMAIL;TYPE=work:amministrazione@studiobassi.it\nEMAIL;TYPE=home:bassi@example.it\nTEL;VALUE=uri;TYPE=work:tel:+39-0574-111222\nADR;TYPE=home:;;Via Roma 1;Prato;PO;59100;Italia\nADR;TYPE=work:;;Via Ferrucci 20;Prato;PO;59100;Italia\nBDAY:--0612\nEND:VCARD\n";
        let c = &parse_vcard(google)[0];
        assert_eq!(c.display_name, "Bassi, Rinaldi & C.");
        assert_eq!(c.emails.len(), 2);
        assert_eq!(c.phones[0].number, "+39-0574-111222");
        assert!(c.address.starts_with("Via Ferrucci 20"), "{}", c.address);
        assert_eq!(c.birthday, "--06-12");
    }

    #[test]
    fn csv_from_outlook_in_italian_and_english() {
        // Outlook italiano, separato da punto e virgola, in Windows-1252.
        let it = "Nome;Secondo nome;Cognome;Società;Posizione;Via (uff.);Città (uff.);Provincia (uff.);CAP (uff.);Paese/Area geografica (uff.);Telefono ufficio;Cellulare;Compleanno;Indirizzo posta elettronica;Tipo posta elettronica;Nome visualizzato posta elettronica;Note\r\nElena;;Fabbri;Fabbri Legnami;Amministrazione;Via dei Pini 4;Prato;PO;59100;Italia;0574 222333;338 1112223;14/05/1980;amministrazione@fabbrilegnami.it;SMTP;Elena Fabbri (amministrazione@fabbrilegnami.it);\"Fatture a 60 giorni;\nchiamare prima\"\r\n;;;Valli Arredi;;;;;;;;;0/0/00;;;;\r\n";
        let bytes = encoding_rs::WINDOWS_1252.encode(it).0.into_owned();
        let list = parse_file(&bytes, "contatti.csv");
        assert_eq!(list.len(), 2, "{list:?}");
        let c = &list[0];
        assert_eq!(c.name(), "Elena Fabbri");
        assert_eq!(c.company, "Fabbri Legnami");
        assert_eq!(c.job_title, "Amministrazione");
        assert_eq!(c.emails, vec!["amministrazione@fabbrilegnami.it".to_string()]);
        assert_eq!(c.address, "Via dei Pini 4\n59100 Prato (PO)\nItalia");
        assert_eq!(c.birthday, "1980-05-14");
        assert_eq!(c.notes, "Fatture a 60 giorni;\nchiamare prima");
        assert_eq!(c.phones.len(), 2);
        // Solo l'azienda: resta, con l'azienda come nome.
        assert_eq!(list[1].name(), "Valli Arredi");

        // Outlook inglese, separato da virgola.
        let en = "First Name,Last Name,Company,E-mail Address,E-mail 2 Address,Business Phone,Mobile Phone,Birthday\nMarco,Ventura,Valli Arredi,m.ventura@valliarredi.it,marco@example.it,0574 152 600,,5/14/1980\n";
        let c = &parse_csv(en)[0];
        assert_eq!(c.emails.len(), 2);
        assert_eq!(c.birthday, "1980-05-14");
        assert_eq!(c.phones[0].kind, "lavoro");
    }

    #[test]
    fn csv_from_google_and_thunderbird() {
        let google = "First Name,Middle Name,Last Name,Organization Name,Organization Title,E-mail 1 - Label,E-mail 1 - Value,E-mail 2 - Label,E-mail 2 - Value,Phone 1 - Label,Phone 1 - Value,Phone 2 - Label,Phone 2 - Value,Address 1 - Label,Address 1 - Formatted,Birthday,Notes\nGiorgio,,Valli,Valli Arredi,Titolare,* Work,giorgio@valliarredi.it,,,Mobile,+39 335 000 1111 ::: +39 335 000 2222,Work,0574 152 600,Work,\"Via Ferrucci 20\nPrato\",1960-02-28,\n";
        let c = &parse_csv(google)[0];
        assert_eq!(c.name(), "Giorgio Valli");
        assert_eq!(c.company, "Valli Arredi");
        assert_eq!(c.job_title, "Titolare");
        assert_eq!(c.phones.iter().map(|p| (p.kind.as_str(), p.number.as_str())).collect::<Vec<_>>(), vec![("cellulare", "+39 335 000 1111"), ("cellulare", "+39 335 000 2222"), ("lavoro", "0574 152 600")]);
        assert_eq!(c.address, "Via Ferrucci 20\nPrato");
        assert_eq!(c.birthday, "1960-02-28");

        let tb = "First Name,Last Name,Display Name,Nickname,Primary Email,Secondary Email,Screen Name,Work Phone,Home Phone,Fax Number,Pager Number,Mobile Number,Home Address,Home Address 2,Home City,Home State,Home ZipCode,Home Country,Work Address,Work Address 2,Work City,Work State,Work ZipCode,Work Country,Job Title,Department,Organization,Web Page 1,Web Page 2,Birth Year,Birth Month,Birth Day,Notes\nAnna,Neri,Anna Neri,,anna@neri.example,,,055 111,,,,,,,,,,,Via Larga 3,,Firenze,FI,50123,Italia,Grafica,,Studio Neri,https://neri.example,,1990,7,4,\n";
        let c = &parse_csv(tb)[0];
        assert_eq!(c.display_name, "Anna Neri");
        assert_eq!(c.address, "Via Larga 3\n50123 Firenze (FI)\nItalia");
        assert_eq!(c.birthday, "1990-07-04");
        assert_eq!(c.website, "https://neri.example");
        assert_eq!(c.company, "Studio Neri");
    }

    #[test]
    fn ldif_from_thunderbird() {
        let ldif = "dn: cn=Paolo Rossi,mail=paolo@rossi.example\nobjectclass: top\nobjectclass: person\ngivenName: Paolo\nsn: Rossi\ncn: Paolo Rossi\nmail: paolo@rossi.example\ntelephoneNumber: 02 123\nmobile: 333 444\no: Rossi Srl\ndescription:: Q2xpZW50ZSBkaSBNaWxhbm8=\nbirthyear: 1975\nbirthmonth: 11\nbirthday: 2\n\ndn: cn=Fornitori\nobjectclass: top\nobjectclass: groupOfNames\ncn: Fornitori\n";
        let list = parse_ldif(ldif);
        assert_eq!(list.len(), 1);
        let c = &list[0];
        assert_eq!(c.name(), "Paolo Rossi");
        assert_eq!(c.notes, "Cliente di Milano");
        assert_eq!(c.birthday, "1975-11-02");
        assert_eq!(c.phones.len(), 2);
    }

    #[test]
    fn vcard_roundtrip_and_merge() {
        let c = Contact {
            first_name: "Chiara".into(),
            last_name: "Valli".into(),
            emails: vec!["chiara.valli@valliarredi.it".into()],
            phones: vec![Phone { kind: "lavoro".into(), number: "0574 152 638".into() }],
            company: "Valli Arredi; Ufficio acquisti".into(),
            address: "Via Ferrucci 20\n59100 Prato (PO)".into(),
            notes: "Una nota molto lunga che supera di parecchio i settantacinque caratteri della riga di una vCard, con accenti: è così.".into(),
            ..Default::default()
        };
        let text = to_vcard(std::slice::from_ref(&c));
        assert!(text.lines().all(|l| l.len() <= 75), "{text}");
        let back = &parse_vcard(&text)[0];
        assert_eq!(back.emails, c.emails);
        assert_eq!(back.company, c.company);
        assert_eq!(back.notes, c.notes);
        assert_eq!(back.phones, c.phones);

        let mut a = Contact { first_name: "Chiara".into(), emails: vec!["chiara.valli@valliarredi.it".into()], ..Default::default() };
        assert!(merge(&mut a, &c));
        assert_eq!(a.last_name, "Valli");
        assert_eq!(a.phones.len(), 1);
        assert!(!merge(&mut a, &c));
    }

    #[test]
    fn birthdays() {
        assert_eq!(normalize_birthday("14/05/1980", true), "1980-05-14");
        assert_eq!(normalize_birthday("05/14/1980", false), "1980-05-14");
        assert_eq!(normalize_birthday("19800514", true), "1980-05-14");
        assert_eq!(normalize_birthday("1980-05-14T00:00:00Z", true), "1980-05-14");
        assert_eq!(normalize_birthday("3/4/85", true), "1985-04-03");
        assert_eq!(normalize_birthday("0/0/00", true), "");
        assert_eq!(normalize_birthday("--0514", true), "--05-14");
    }
}
