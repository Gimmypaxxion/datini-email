//! Calendario: lettura e scrittura dei file iCalendar (.ics, RFC 5545) e degli
//! inviti che viaggiano per posta (iTIP, RFC 5546).
//!
//! Gli orari diventano millisecondi UTC. Le zone orarie si leggono dai blocchi
//! VTIMEZONE del file (Outlook usa nomi come "W. Europe Standard Time", Google e
//! Apple quelli IANA); senza definizione si usa l'ora del computer. Le serie
//! ripetute si scrivono con la zona del computer, così l'ora resta la stessa
//! anche dopo il cambio dell'ora legale.

use std::collections::HashMap;

use chrono::{Datelike, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Timelike, Utc, Weekday};

use crate::mime::random_id;
use crate::model::{Attendee, CalEvent, Freq, PartStat, Person, Repeat};

// ---------------- Righe e componenti ----------------

#[derive(Debug, Clone)]
pub struct Prop {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

impl Prop {
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v.as_str())
    }
    pub fn text(&self) -> String {
        unescape(&self.value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Comp {
    pub name: String,
    pub props: Vec<Prop>,
    pub subs: Vec<Comp>,
}

impl Comp {
    fn prop(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.name == name)
    }
    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Prop> + 'a {
        self.props.iter().filter(move |p| p.name == name)
    }
    fn text(&self, name: &str) -> String {
        self.prop(name).map(|p| p.text().trim().to_string()).unwrap_or_default()
    }
}

/// Riunisce le righe spezzate (quelle che iniziano con uno spazio o un tab).
fn unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t')) {
            if let Some(last) = out.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        if !line.trim().is_empty() {
            out.push(line.to_string());
        }
    }
    out
}

/// Divide sul separatore, ma non dentro le virgolette.
fn split_unquoted(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_q = false;
    let mut from = 0;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_q = !in_q;
        } else if c == sep && !in_q {
            out.push(&s[from..i]);
            from = i + c.len_utf8();
        }
    }
    out.push(&s[from..]);
    out
}

fn parse_line(line: &str) -> Option<Prop> {
    let mut in_q = false;
    let mut colon = None;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_q = !in_q,
            ':' if !in_q => {
                colon = Some(i);
                break;
            }
            _ => {}
        }
    }
    let colon = colon?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = split_unquoted(head, ';').into_iter();
    let name = parts.next()?.trim().to_ascii_uppercase();
    if name.is_empty() {
        return None;
    }
    let params = parts
        .filter_map(|p| {
            let (k, v) = p.split_once('=')?;
            Some((k.trim().to_ascii_uppercase(), v.trim().trim_matches('"').to_string()))
        })
        .collect();
    Some(Prop { name, params, value: value.to_string() })
}

/// Componenti di primo livello del testo (di solito un solo VCALENDAR).
pub fn components(text: &str) -> Vec<Comp> {
    fn close(stack: &mut [Comp], top: &mut Vec<Comp>, c: Comp) {
        match stack.last_mut() {
            Some(parent) => parent.subs.push(c),
            None => top.push(c),
        }
    }
    let mut stack: Vec<Comp> = Vec::new();
    let mut top = Vec::new();
    for line in unfold(text) {
        let Some(p) = parse_line(&line) else { continue };
        if p.name == "BEGIN" {
            stack.push(Comp { name: p.value.trim().to_ascii_uppercase(), ..Default::default() });
        } else if p.name == "END" {
            let name = p.value.trim().to_ascii_uppercase();
            if !stack.iter().any(|c| c.name == name) {
                continue;
            }
            // Chiude anche i componenti rimasti aperti per errore.
            while let Some(c) = stack.pop() {
                let done = c.name == name;
                close(&mut stack, &mut top, c);
                if done {
                    break;
                }
            }
        } else if let Some(c) = stack.last_mut() {
            c.props.push(p);
        }
    }
    while let Some(c) = stack.pop() {
        close(&mut stack, &mut top, c);
    }
    top
}

fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut it = v.chars();
    while let Some(c) = it.next() {
        if c == '\\' {
            match it.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(x) => out.push(x),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn escape(v: &str) -> String {
    v.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace("\r\n", "\n").replace('\n', "\\n")
}

/// Divide un elenco di testi sulle virgole non precedute da "\".
fn split_text_list(v: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut esc = false;
    for c in v.chars() {
        if esc {
            cur.push('\\');
            cur.push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == ',' {
            out.push(unescape(&cur).trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    out.push(unescape(&cur).trim().to_string());
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

// ---------------- Orari ----------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum When {
    Date(NaiveDate),
    Utc(NaiveDateTime),
    Floating(NaiveDateTime),
}

fn parse_stamp(value: &str) -> Option<When> {
    let v = value.trim();
    if v.len() == 8 {
        return NaiveDate::parse_from_str(v, "%Y%m%d").ok().map(When::Date);
    }
    let (body, utc) = match v.strip_suffix('Z').or_else(|| v.strip_suffix('z')) {
        Some(b) => (b, true),
        None => (v, false),
    };
    let dt = NaiveDateTime::parse_from_str(body, "%Y%m%dT%H%M%S").or_else(|_| NaiveDateTime::parse_from_str(&format!("{body}00"), "%Y%m%dT%H%M%S")).ok()?;
    Some(if utc { When::Utc(dt) } else { When::Floating(dt) })
}

/// Millisecondi di un'ora locale del computer. Nell'ora che non esiste (il
/// passaggio all'ora legale) si va avanti di un'ora.
pub fn local_ms(dt: NaiveDateTime) -> i64 {
    match Local.from_local_datetime(&dt) {
        LocalResult::Single(t) => t.timestamp_millis(),
        LocalResult::Ambiguous(a, _) => a.timestamp_millis(),
        LocalResult::None => Local
            .from_local_datetime(&(dt + Duration::hours(1)))
            .earliest()
            .map(|t| t.timestamp_millis())
            .unwrap_or_else(|| dt.and_utc().timestamp_millis()),
    }
}

fn local_dt(ms: i64) -> NaiveDateTime {
    Local.timestamp_millis_opt(ms).earliest().map(|t| t.naive_local()).unwrap_or_else(|| Utc.timestamp_millis_opt(ms).earliest().map(|t| t.naive_utc()).unwrap_or_default())
}

fn midnight(d: NaiveDate) -> NaiveDateTime {
    d.and_time(NaiveTime::MIN)
}

/// Mezzanotte locale più vicina (per le durate di tutto il giorno).
fn snap_midnight(ms: i64) -> i64 {
    let dt = local_dt(ms);
    let day = if dt.hour() >= 12 { dt.date().succ_opt().unwrap_or(dt.date()) } else { dt.date() };
    local_ms(midnight(day))
}

/// Giorno della settimana nell'ora del computer (0 domenica … 6 sabato).
pub fn local_weekday(ms: i64) -> u8 {
    local_dt(ms).weekday().num_days_from_sunday() as u8
}

/// Stessa ora locale `n` giorni dopo.
pub fn add_days_local(ms: i64, n: i64) -> i64 {
    let dt = local_dt(ms);
    local_ms(dt + Duration::days(n))
}

/// Durata ISO 8601 ("-PT15M", "P1D", "PT1H30M") in millisecondi.
fn parse_duration(v: &str) -> Option<i64> {
    let v = v.trim();
    let (neg, rest) = match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v.strip_prefix('+').unwrap_or(v)),
    };
    let rest = rest.strip_prefix('P').or_else(|| rest.strip_prefix('p'))?;
    let mut total: i64 = 0;
    let mut num = String::new();
    let mut time = false;
    for c in rest.chars() {
        match c.to_ascii_uppercase() {
            '0'..='9' => num.push(c),
            'T' => time = true,
            u @ ('W' | 'D' | 'H' | 'M' | 'S') => {
                let n: i64 = num.parse().ok()?;
                num.clear();
                total += n * match (u, time) {
                    ('W', _) => 604_800,
                    ('D', _) => 86_400,
                    ('H', _) => 3_600,
                    ('M', true) => 60,
                    ('S', _) => 1,
                    _ => return None,
                };
            }
            _ => return None,
        }
    }
    Some(if neg { -total } else { total } * 1000)
}

// ---------------- Zone orarie ----------------

#[derive(Debug, Clone, Copy)]
struct YearRule {
    month: u32,
    /// Settimana del mese (1-4, -1 l'ultima); 0 per un giorno fisso.
    nth: i8,
    weekday: Weekday,
    day: u32,
    until: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
struct Observance {
    /// Primo cambio, in ora locale prima del cambio.
    start: NaiveDateTime,
    from: i32,
    to: i32,
    rule: Option<YearRule>,
    rdates: Vec<NaiveDateTime>,
}

/// Zona oraria descritta da un blocco VTIMEZONE.
#[derive(Debug, Clone, Default)]
pub struct Zone {
    obs: Vec<Observance>,
}

const WEEKDAYS: [(&str, Weekday); 7] = [("SU", Weekday::Sun), ("MO", Weekday::Mon), ("TU", Weekday::Tue), ("WE", Weekday::Wed), ("TH", Weekday::Thu), ("FR", Weekday::Fri), ("SA", Weekday::Sat)];

fn weekday_code(w: Weekday) -> &'static str {
    WEEKDAYS.iter().find(|(_, x)| *x == w).map(|(c, _)| *c).unwrap_or("MO")
}

/// "2TU" → (2, martedì); "-1SU" → (-1, domenica); "MO" → (0, lunedì).
fn parse_byday(s: &str) -> Option<(i8, Weekday)> {
    let s = s.trim().to_ascii_uppercase();
    if s.len() < 2 {
        return None;
    }
    let (num, code) = s.split_at(s.len() - 2);
    let wd = WEEKDAYS.iter().find(|(c, _)| *c == code)?.1;
    let n = if num.is_empty() || num == "+" { 0 } else { num.trim_start_matches('+').parse::<i8>().ok()? };
    Some((n, wd))
}

/// Il giorno `nth` (1-5, o -1 per l'ultimo) del mese che cade di `wd`.
pub fn nth_weekday(year: i32, month: u32, wd: Weekday, nth: i8) -> Option<NaiveDate> {
    if nth > 0 {
        let first = NaiveDate::from_ymd_opt(year, month, 1)?;
        let shift = (7 + wd.num_days_from_monday() as i64 - first.weekday().num_days_from_monday() as i64) % 7;
        let d = first + Duration::days(shift + 7 * (nth as i64 - 1));
        (d.month() == month).then_some(d)
    } else if nth < 0 {
        let next = if month == 12 { NaiveDate::from_ymd_opt(year + 1, 1, 1)? } else { NaiveDate::from_ymd_opt(year, month + 1, 1)? };
        let last = next.pred_opt()?;
        let shift = (7 + last.weekday().num_days_from_monday() as i64 - wd.num_days_from_monday() as i64) % 7;
        let d = last - Duration::days(shift + 7 * (-(nth as i64) - 1));
        (d.month() == month).then_some(d)
    } else {
        None
    }
}

fn parse_offset(v: &str) -> Option<i32> {
    let v = v.trim();
    let (sign, rest) = match v.chars().next()? {
        '+' => (1, &v[1..]),
        '-' => (-1, &v[1..]),
        _ => (1, v),
    };
    if rest.len() < 4 || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let h: i32 = rest[0..2].parse().ok()?;
    let m: i32 = rest[2..4].parse().ok()?;
    let s: i32 = if rest.len() >= 6 { rest[4..6].parse().ok()? } else { 0 };
    Some(sign * (h * 3600 + m * 60 + s))
}

fn rule_map(v: &str) -> HashMap<String, String> {
    v.split(';').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.trim().to_ascii_uppercase(), v.trim().to_string())).collect()
}

impl Zone {
    fn from_comp(c: &Comp) -> Option<(String, Zone)> {
        let id = c.text("TZID");
        if id.is_empty() {
            return None;
        }
        let mut obs = Vec::new();
        for s in &c.subs {
            if s.name != "STANDARD" && s.name != "DAYLIGHT" {
                continue;
            }
            let Some(When::Floating(start) | When::Utc(start)) = s.prop("DTSTART").and_then(|p| parse_stamp(&p.value)) else { continue };
            let (Some(from), Some(to)) = (s.prop("TZOFFSETFROM").and_then(|p| parse_offset(&p.value)), s.prop("TZOFFSETTO").and_then(|p| parse_offset(&p.value))) else { continue };
            let rule = s.prop("RRULE").and_then(|p| {
                let m = rule_map(&p.value);
                let month: u32 = m.get("BYMONTH")?.split(',').next()?.parse().ok()?;
                let until = m.get("UNTIL").and_then(|u| match parse_stamp(u)? {
                    When::Date(d) => Some(midnight(d)),
                    When::Utc(t) | When::Floating(t) => Some(t),
                });
                if let Some((n, wd)) = m.get("BYDAY").and_then(|b| parse_byday(b.split(',').next()?)) {
                    // "BYDAY=SU;BYMONTHDAY=8,9,…,14" (vecchio stile): seconda domenica.
                    let n = if n == 0 {
                        m.get("BYMONTHDAY").and_then(|d| d.split(',').next()?.parse::<i32>().ok()).map(|d| if d < 0 { -1 } else { ((d - 1) / 7 + 1) as i8 }).unwrap_or(1)
                    } else {
                        n
                    };
                    Some(YearRule { month, nth: if n > 4 { -1 } else { n }, weekday: wd, day: 0, until })
                } else {
                    let day: u32 = m.get("BYMONTHDAY")?.split(',').next()?.parse().ok()?;
                    Some(YearRule { month, nth: 0, weekday: Weekday::Mon, day, until })
                }
            });
            let mut rdates = Vec::new();
            for p in s.all("RDATE") {
                for v in p.value.split(',') {
                    if let Some(When::Floating(t) | When::Utc(t)) = parse_stamp(v) {
                        rdates.push(t);
                    }
                }
            }
            obs.push(Observance { start, from, to, rule, rdates });
        }
        if obs.is_empty() {
            return None;
        }
        Some((id, Zone { obs }))
    }

    /// Cambi d'ora intorno all'anno indicato: (ora locale prima del cambio, offset prima, offset dopo).
    fn transitions(&self, year: i32) -> Vec<(NaiveDateTime, i32, i32)> {
        let mut out = Vec::new();
        for o in &self.obs {
            out.push((o.start, o.from, o.to));
            out.extend(o.rdates.iter().map(|t| (*t, o.from, o.to)));
            if let Some(r) = o.rule {
                for y in [year - 1, year] {
                    let date = if r.nth == 0 { NaiveDate::from_ymd_opt(y, r.month, r.day) } else { nth_weekday(y, r.month, r.weekday, r.nth) };
                    let Some(date) = date else { continue };
                    let t = date.and_time(o.start.time());
                    if t >= o.start && r.until.map_or(true, |u| t <= u + Duration::days(1)) {
                        out.push((t, o.from, o.to));
                    }
                }
            }
        }
        out
    }

    fn fallback(&self) -> i32 {
        self.obs.iter().min_by_key(|o| o.start).map(|o| o.from).unwrap_or(0)
    }

    /// Scostamento da UTC (secondi) per un'ora locale della zona.
    pub fn offset_local(&self, local: NaiveDateTime) -> i32 {
        self.transitions(local.year()).into_iter().filter(|(t, _, _)| *t <= local).max_by_key(|(t, _, _)| *t).map(|(_, _, to)| to).unwrap_or_else(|| self.fallback())
    }

    /// Scostamento da UTC (secondi) per un istante UTC.
    pub fn offset_utc(&self, utc: NaiveDateTime) -> i32 {
        self.transitions(utc.year())
            .into_iter()
            .map(|(t, from, to)| (t - Duration::seconds(from as i64), to))
            .filter(|(t, _)| *t <= utc)
            .max_by_key(|(t, _)| *t)
            .map(|(_, to)| to)
            .unwrap_or_else(|| self.fallback())
    }
}

struct Ctx {
    zones: HashMap<String, Zone>,
}

impl Ctx {
    fn resolve(&self, w: When, tzid: Option<&str>) -> (i64, bool) {
        match w {
            When::Date(d) => (local_ms(midnight(d)), true),
            When::Utc(dt) => (dt.and_utc().timestamp_millis(), false),
            When::Floating(dt) => match tzid.and_then(|t| self.zones.get(t.trim().trim_matches('"'))) {
                Some(z) => ((dt - Duration::seconds(z.offset_local(dt) as i64)).and_utc().timestamp_millis(), false),
                None => (local_ms(dt), false),
            },
        }
    }

    fn time(&self, p: &Prop) -> Option<(i64, bool)> {
        let first = p.value.split(',').next()?;
        let w = parse_stamp(first)?;
        let w = match (w, p.param("VALUE")) {
            (When::Floating(dt) | When::Utc(dt), Some(v)) if v.eq_ignore_ascii_case("DATE") => When::Date(dt.date()),
            (w, _) => w,
        };
        Some(self.resolve(w, p.param("TZID")))
    }
}

// ---------------- Eventi ----------------

/// Contenuto di un file o di un invito.
#[derive(Debug, Clone, Default)]
pub struct Calendar {
    /// Metodo iTIP in maiuscolo (REQUEST, CANCEL, REPLY, PUBLISH…), se c'è.
    pub method: Option<String>,
    pub events: Vec<CalEvent>,
}

pub fn parse(text: &str) -> Calendar {
    let text = text.trim_start_matches('\u{feff}');
    let comps = components(text);
    // Un VEVENT senza VCALENDAR intorno: si accetta lo stesso.
    let lone = Comp { name: "VCALENDAR".into(), props: vec![], subs: comps.clone() };
    let mut cals: Vec<&Comp> = comps.iter().filter(|c| c.name == "VCALENDAR").collect();
    if cals.is_empty() {
        cals.push(&lone);
    }
    let mut out = Calendar::default();
    for cal in cals {
        if out.method.is_none() {
            out.method = cal.prop("METHOD").map(|p| p.value.trim().to_ascii_uppercase()).filter(|m| !m.is_empty());
        }
        let zones = cal.subs.iter().filter(|c| c.name == "VTIMEZONE").filter_map(Zone::from_comp).collect();
        let ctx = Ctx { zones };
        out.events.extend(cal.subs.iter().filter(|c| c.name == "VEVENT").filter_map(|c| event_from(c, &ctx)));
    }
    // Prima le serie, poi le occorrenze modificate.
    out.events.sort_by_key(|e| e.recurrence_id.is_some());
    out
}

fn mailto(v: &str) -> String {
    let v = v.trim();
    let low = v.to_ascii_lowercase();
    if low.starts_with("mailto:") { v[7..].trim().to_string() } else { v.to_string() }
}

fn person_from(p: &Prop) -> Person {
    let email = mailto(&p.value);
    Person::new(p.param("CN").map(|n| n.trim().to_string()).filter(|n| !n.eq_ignore_ascii_case(&email)).unwrap_or_default(), email)
}

fn partstat(v: Option<&str>) -> PartStat {
    match v.map(|s| s.trim().to_ascii_uppercase()).as_deref() {
        Some("ACCEPTED") => PartStat::Accepted,
        Some("TENTATIVE") => PartStat::Tentative,
        Some("DECLINED") => PartStat::Declined,
        _ => PartStat::NeedsAction,
    }
}

fn partstat_code(s: PartStat) -> &'static str {
    match s {
        PartStat::NeedsAction => "NEEDS-ACTION",
        PartStat::Accepted => "ACCEPTED",
        PartStat::Tentative => "TENTATIVE",
        PartStat::Declined => "DECLINED",
    }
}

fn event_from(c: &Comp, ctx: &Ctx) -> Option<CalEvent> {
    let (start, all_day) = ctx.time(c.prop("DTSTART")?)?;
    let end = if let Some(p) = c.prop("DTEND") {
        ctx.time(p).map(|(e, _)| e)
    } else {
        c.prop("DURATION").and_then(|p| parse_duration(&p.value)).map(|d| start + d)
    };
    let end = match end {
        Some(e) if e > start => {
            if all_day {
                snap_midnight(e)
            } else {
                e
            }
        }
        _ if all_day => add_days_local(start, 1),
        _ => start,
    };
    let repeat = c.prop("RRULE").and_then(|p| parse_rrule(&p.value, start, ctx));
    let mut exceptions = Vec::new();
    for p in c.all("EXDATE") {
        for v in p.value.split(',') {
            if let Some(w) = parse_stamp(v) {
                let w = match (w, all_day) {
                    (When::Floating(dt) | When::Utc(dt), true) if p.param("VALUE").is_some_and(|x| x.eq_ignore_ascii_case("DATE")) => When::Date(dt.date()),
                    (w, _) => w,
                };
                exceptions.push(ctx.resolve(w, p.param("TZID")).0);
            }
        }
    }
    exceptions.sort_unstable();
    exceptions.dedup();
    let reminder = c.subs.iter().filter(|s| s.name == "VALARM").find_map(|a| {
        let t = a.prop("TRIGGER")?;
        let before = if t.param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE-TIME")) {
            start - ctx.time(t)?.0
        } else {
            let d = parse_duration(&t.value)?;
            let base = if t.param("RELATED").is_some_and(|r| r.eq_ignore_ascii_case("END")) { end } else { start };
            start - (base + d)
        };
        Some((before / 60_000).max(0) as i32)
    });
    let mut categories = Vec::new();
    for p in c.all("CATEGORIES") {
        categories.extend(split_text_list(&p.value));
    }
    let uid = c.text("UID");
    Some(CalEvent {
        id: String::new(),
        uid: if uid.is_empty() { format!("{}@better-look", random_id()) } else { uid },
        title: c.text("SUMMARY"),
        location: c.text("LOCATION"),
        start,
        end,
        all_day,
        repeat,
        exceptions,
        recurrence_id: c.prop("RECURRENCE-ID").and_then(|p| ctx.time(p)).map(|t| t.0),
        reminder,
        notes: c.prop("DESCRIPTION").map(|p| p.text().trim().replace("\r\n", "\n")).unwrap_or_default(),
        categories,
        organizer: c.prop("ORGANIZER").map(person_from).filter(|p| !p.email.is_empty()),
        attendees: c
            .all("ATTENDEE")
            .map(|p| {
                let who = person_from(p);
                Attendee { name: who.name, email: who.email, status: partstat(p.param("PARTSTAT")), optional: p.param("ROLE").is_some_and(|r| r.eq_ignore_ascii_case("OPT-PARTICIPANT")) }
            })
            .filter(|a| !a.email.is_empty())
            .collect(),
        response: None,
        account_id: None,
        sequence: c.text("SEQUENCE").parse().unwrap_or(0),
        cancelled: c.text("STATUS").eq_ignore_ascii_case("CANCELLED"),
    })
}

fn parse_rrule(v: &str, start: i64, ctx: &Ctx) -> Option<Repeat> {
    let m = rule_map(v);
    let freq = match m.get("FREQ")?.to_ascii_uppercase().as_str() {
        "DAILY" => Freq::Daily,
        "WEEKLY" => Freq::Weekly,
        "MONTHLY" => Freq::Monthly,
        "YEARLY" => Freq::Yearly,
        _ => return None,
    };
    let interval = m.get("INTERVAL").and_then(|s| s.parse::<u32>().ok()).filter(|n| *n > 0).unwrap_or(1);
    let byday: Vec<(i8, Weekday)> = m.get("BYDAY").map(|s| s.split(',').filter_map(parse_byday).collect()).unwrap_or_default();
    let setpos = m.get("BYSETPOS").and_then(|s| s.split(',').next()?.trim().parse::<i8>().ok());
    let until = m.get("UNTIL").and_then(|u| {
        Some(match parse_stamp(u)? {
            When::Date(d) => local_ms(d.and_hms_opt(23, 59, 59)?),
            w => ctx.resolve(w, None).0,
        })
    });
    let count = m.get("COUNT").and_then(|s| s.parse::<u32>().ok()).filter(|n| *n > 0);
    let wd_start = local_dt(start).weekday();
    let code = |w: Weekday| w.num_days_from_sunday() as u8;
    let mut r = Repeat { freq, interval, days: vec![], week: None, until, count };
    match freq {
        Freq::Daily => {
            if !byday.is_empty() {
                r.freq = Freq::Weekly;
                r.days = byday.iter().map(|(_, d)| code(*d)).collect();
            }
        }
        Freq::Weekly => {
            r.days = if byday.is_empty() { vec![code(wd_start)] } else { byday.iter().map(|(_, d)| code(*d)).collect() };
        }
        Freq::Monthly | Freq::Yearly => {
            if let Some((n, d)) = byday.first() {
                let n = if *n != 0 { *n } else { setpos.unwrap_or(0) };
                if n != 0 {
                    r.week = Some(if (-1..=4).contains(&n) { n } else { -1 });
                    r.days = vec![code(*d)];
                }
            }
        }
    }
    r.days.sort_unstable();
    r.days.dedup();
    Some(r)
}

// ---------------- Scrittura ----------------

fn fold(line: &str, out: &mut String) {
    let mut count = 0;
    for c in line.chars() {
        let n = c.len_utf8();
        if count + n > 75 {
            out.push_str("\r\n ");
            count = 1;
        }
        out.push(c);
        count += n;
    }
    out.push_str("\r\n");
}

fn cn(name: &str) -> String {
    let n = name.replace('"', "").trim().to_string();
    if n.is_empty() {
        String::new()
    } else if n.contains([':', ';', ',']) {
        format!(";CN=\"{n}\"")
    } else {
        format!(";CN={n}")
    }
}

fn utc_stamp(ms: i64) -> String {
    Utc.timestamp_millis_opt(ms).earliest().map(|t| t.format("%Y%m%dT%H%M%SZ").to_string()).unwrap_or_default()
}

/// Regole della zona del computer, ricavate dagli scostamenti dell'anno.
#[derive(Debug, Clone)]
pub struct LocalZone {
    pub name: String,
    /// (ora legale?, offset prima, offset dopo, mese, settimana, giorno, ora locale del cambio)
    rules: Vec<(bool, i32, i32, u32, i8, Weekday, NaiveTime)>,
    fixed: i32,
}

impl LocalZone {
    /// Zona del computer.
    pub fn system() -> LocalZone {
        let name = iana_time_zone::get_timezone().unwrap_or_else(|_| "Locale".into());
        let year = Local::now().year();
        Self::from_offsets(name, year, |secs| Local.timestamp_opt(secs, 0).earliest().map(|t| t.offset().fix().local_minus_utc()).unwrap_or(0))
    }

    /// Cerca i cambi d'ora dell'anno da una funzione istante UTC → scostamento.
    fn from_offsets(name: String, year: i32, offset: impl Fn(i64) -> i32) -> LocalZone {
        let start = NaiveDate::from_ymd_opt(year, 1, 1).map(midnight).map(|d| d.and_utc().timestamp()).unwrap_or(0);
        let first = offset(start);
        let mut prev = first;
        let mut rules = Vec::new();
        for day in 1..=366 {
            let next = start + day * 86_400;
            let o = offset(next);
            if o == prev {
                continue;
            }
            let (mut lo, mut hi) = (next - 86_400, next);
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if offset(mid) == prev {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let wall = chrono::DateTime::from_timestamp(hi + prev as i64, 0).map(|t| t.naive_utc()).unwrap_or_default();
            let d = wall.date();
            let last = nth_weekday(d.year(), d.month(), d.weekday(), -1);
            let nth = if last == Some(d) { -1 } else { ((d.day() - 1) / 7 + 1) as i8 };
            rules.push((o > prev, prev, o, d.month(), nth, d.weekday(), wall.time()));
            prev = o;
        }
        LocalZone { name, rules, fixed: first }
    }

    fn offset_text(secs: i32) -> String {
        let sign = if secs < 0 { '-' } else { '+' };
        let a = secs.abs();
        format!("{sign}{:02}{:02}", a / 3600, (a % 3600) / 60)
    }

    fn write(&self, out: &mut String) {
        fold("BEGIN:VTIMEZONE", out);
        fold(&format!("TZID:{}", self.name), out);
        if self.rules.is_empty() {
            fold("BEGIN:STANDARD", out);
            fold("DTSTART:19700101T000000", out);
            fold(&format!("TZOFFSETFROM:{}", Self::offset_text(self.fixed)), out);
            fold(&format!("TZOFFSETTO:{}", Self::offset_text(self.fixed)), out);
            fold("END:STANDARD", out);
        }
        for (dst, from, to, month, nth, wd, time) in &self.rules {
            let kind = if *dst { "DAYLIGHT" } else { "STANDARD" };
            let first = nth_weekday(1970, *month, *wd, *nth).map(|d| d.and_time(*time)).unwrap_or_default();
            fold(&format!("BEGIN:{kind}"), out);
            fold(&format!("DTSTART:{}", first.format("%Y%m%dT%H%M%S")), out);
            fold(&format!("TZOFFSETFROM:{}", Self::offset_text(*from)), out);
            fold(&format!("TZOFFSETTO:{}", Self::offset_text(*to)), out);
            fold(&format!("RRULE:FREQ=YEARLY;BYMONTH={month};BYDAY={nth}{}", weekday_code(*wd)), out);
            fold(&format!("END:{kind}"), out);
        }
        fold("END:VTIMEZONE", out);
    }
}

fn time_prop(name: &str, ms: i64, all_day: bool, tz: Option<&LocalZone>) -> String {
    if all_day {
        format!("{name};VALUE=DATE:{}", local_dt(ms).format("%Y%m%d"))
    } else if let Some(z) = tz {
        format!("{name};TZID={}:{}", z.name, local_dt(ms).format("%Y%m%dT%H%M%S"))
    } else {
        format!("{name}:{}", utc_stamp(ms))
    }
}

fn rrule_text(r: &Repeat, start: i64, all_day: bool) -> String {
    let local = local_dt(start);
    let codes = |days: &[u8]| days.iter().filter_map(|d| WEEKDAYS.get(*d as usize).map(|(c, _)| *c)).collect::<Vec<_>>().join(",");
    let mut s = match r.freq {
        Freq::Daily => "FREQ=DAILY".to_string(),
        Freq::Weekly => format!("FREQ=WEEKLY;BYDAY={}", if r.days.is_empty() { weekday_code(local.weekday()).to_string() } else { codes(&r.days) }),
        Freq::Monthly => match (r.week, r.days.first()) {
            (Some(w), Some(d)) => format!("FREQ=MONTHLY;BYDAY={w}{}", codes(&[*d])),
            _ => format!("FREQ=MONTHLY;BYMONTHDAY={}", local.day()),
        },
        Freq::Yearly => match (r.week, r.days.first()) {
            (Some(w), Some(d)) => format!("FREQ=YEARLY;BYMONTH={};BYDAY={w}{}", local.month(), codes(&[*d])),
            _ => format!("FREQ=YEARLY;BYMONTH={};BYMONTHDAY={}", local.month(), local.day()),
        },
    };
    if r.interval > 1 {
        s.push_str(&format!(";INTERVAL={}", r.interval));
    }
    if let Some(c) = r.count {
        s.push_str(&format!(";COUNT={c}"));
    } else if let Some(u) = r.until {
        if all_day {
            s.push_str(&format!(";UNTIL={}", local_dt(u).format("%Y%m%d")));
        } else {
            s.push_str(&format!(";UNTIL={}", utc_stamp(u)));
        }
    }
    s
}

fn write_event(out: &mut String, ev: &CalEvent, method: Option<&str>, tz: Option<&LocalZone>, overridden: &[i64]) {
    let series_tz = if ev.repeat.is_some() { tz } else { None };
    fold("BEGIN:VEVENT", out);
    fold(&format!("UID:{}", ev.uid), out);
    fold(&format!("DTSTAMP:{}", utc_stamp(Utc::now().timestamp_millis())), out);
    fold(&format!("SEQUENCE:{}", ev.sequence.max(0)), out);
    fold(&time_prop("DTSTART", ev.start, ev.all_day, series_tz), out);
    fold(&time_prop("DTEND", ev.end.max(ev.start), ev.all_day, series_tz), out);
    if let Some(rid) = ev.recurrence_id {
        fold(&time_prop("RECURRENCE-ID", rid, ev.all_day, None), out);
    }
    if let Some(r) = &ev.repeat {
        fold(&format!("RRULE:{}", rrule_text(r, ev.start, ev.all_day)), out);
        let ex: Vec<i64> = ev.exceptions.iter().copied().filter(|x| !overridden.contains(x)).collect();
        if !ex.is_empty() {
            let (head, fmt) = if ev.all_day {
                ("EXDATE;VALUE=DATE".to_string(), "%Y%m%d")
            } else if let Some(z) = series_tz {
                (format!("EXDATE;TZID={}", z.name), "%Y%m%dT%H%M%S")
            } else {
                ("EXDATE".to_string(), "")
            };
            let list = ex.iter().map(|x| if fmt.is_empty() { utc_stamp(*x) } else { local_dt(*x).format(fmt).to_string() }).collect::<Vec<_>>().join(",");
            fold(&format!("{head}:{list}"), out);
        }
    }
    fold(&format!("SUMMARY:{}", escape(&ev.title)), out);
    if !ev.location.trim().is_empty() {
        fold(&format!("LOCATION:{}", escape(ev.location.trim())), out);
    }
    if !ev.notes.trim().is_empty() {
        fold(&format!("DESCRIPTION:{}", escape(ev.notes.trim())), out);
    }
    if !ev.categories.is_empty() {
        fold(&format!("CATEGORIES:{}", ev.categories.iter().map(|c| escape(c)).collect::<Vec<_>>().join(",")), out);
    }
    if let Some(o) = &ev.organizer {
        fold(&format!("ORGANIZER{}:mailto:{}", cn(&o.name), o.email), out);
    }
    for a in &ev.attendees {
        let role = if a.optional { "OPT-PARTICIPANT" } else { "REQ-PARTICIPANT" };
        let rsvp = if method == Some("REQUEST") { ";RSVP=TRUE" } else { "" };
        fold(&format!("ATTENDEE{};ROLE={role};PARTSTAT={}{rsvp}:mailto:{}", cn(&a.name), partstat_code(a.status), a.email), out);
    }
    let status = if method == Some("CANCEL") || ev.cancelled { "CANCELLED" } else { "CONFIRMED" };
    fold(&format!("STATUS:{status}"), out);
    fold("TRANSP:OPAQUE", out);
    if let (Some(min), None | Some("REQUEST") | Some("PUBLISH")) = (ev.reminder, method) {
        fold("BEGIN:VALARM", out);
        fold("ACTION:DISPLAY", out);
        fold("DESCRIPTION:Promemoria", out);
        fold(&format!("TRIGGER:-PT{}M", min.max(0)), out);
        fold("END:VALARM", out);
    }
    fold("END:VEVENT", out);
}

/// Testo iCalendar con gli eventi indicati; `method` per gli inviti (REQUEST, REPLY, CANCEL).
pub fn write(method: Option<&str>, events: &[CalEvent]) -> String {
    let mut out = String::new();
    fold("BEGIN:VCALENDAR", &mut out);
    fold("VERSION:2.0", &mut out);
    fold("PRODID:-//Better-Look//Calendario//IT", &mut out);
    fold("CALSCALE:GREGORIAN", &mut out);
    if let Some(m) = method {
        fold(&format!("METHOD:{m}"), &mut out);
    }
    let tz = events.iter().any(|e| e.repeat.is_some() && !e.all_day).then(LocalZone::system);
    if let Some(z) = &tz {
        z.write(&mut out);
    }
    for ev in events {
        // Le occorrenze riscritte a parte non vanno tolte dalla serie.
        let overridden: Vec<i64> = events.iter().filter(|o| o.uid == ev.uid).filter_map(|o| o.recurrence_id).collect();
        write_event(&mut out, ev, method, tz.as_ref(), &overridden);
    }
    fold("END:VCALENDAR", &mut out);
    out
}

// ---------------- Testi per i messaggi ----------------

const GIORNI: [&str; 7] = ["domenica", "lunedì", "martedì", "mercoledì", "giovedì", "venerdì", "sabato"];
const MESI: [&str; 12] = ["gennaio", "febbraio", "marzo", "aprile", "maggio", "giugno", "luglio", "agosto", "settembre", "ottobre", "novembre", "dicembre"];

fn day_text(d: NaiveDate) -> String {
    format!("{} {} {} {}", GIORNI[d.weekday().num_days_from_sunday() as usize], d.day(), MESI[d.month0() as usize], d.year())
}

/// "lunedì 19 ottobre 2026, 10:30-12:00"
pub fn when_text(ev: &CalEvent) -> String {
    let s = local_dt(ev.start);
    let e = local_dt(ev.end.max(ev.start));
    if ev.all_day {
        let last = local_dt(ev.end.max(ev.start + 1) - 1).date();
        if last == s.date() {
            format!("{} (tutto il giorno)", day_text(s.date()))
        } else {
            format!("da {} a {} (tutto il giorno)", day_text(s.date()), day_text(last))
        }
    } else if s.date() == e.date() {
        format!("{}, {}-{}", day_text(s.date()), s.format("%H:%M"), e.format("%H:%M"))
    } else {
        format!("da {} alle {} a {} alle {}", day_text(s.date()), s.format("%H:%M"), day_text(e.date()), e.format("%H:%M"))
    }
}

/// "ogni settimana", "ogni 2 mesi"...
pub fn repeat_text(r: &Repeat) -> String {
    let (one, many) = match r.freq {
        Freq::Daily => ("ogni giorno", "giorni"),
        Freq::Weekly => ("ogni settimana", "settimane"),
        Freq::Monthly => ("ogni mese", "mesi"),
        Freq::Yearly => ("ogni anno", "anni"),
    };
    if r.freq == Freq::Weekly && r.interval == 1 && r.days == [1, 2, 3, 4, 5] {
        return "ogni giorno feriale".into();
    }
    if r.interval > 1 {
        format!("ogni {} {many}", r.interval)
    } else {
        one.into()
    }
}

/// Riepilogo dell'evento nel testo del messaggio di invito.
pub fn summary_text(ev: &CalEvent) -> String {
    let mut lines = vec![ev.title.trim().to_string(), String::new(), format!("Quando: {}", when_text(ev))];
    if let Some(r) = &ev.repeat {
        lines.push(format!("Si ripete: {}", repeat_text(r)));
    }
    if !ev.location.trim().is_empty() {
        lines.push(format!("Dove: {}", ev.location.trim()));
    }
    if let Some(o) = &ev.organizer {
        lines.push(format!("Organizzatore: {}", o.label()));
    }
    if !ev.attendees.is_empty() {
        lines.push(format!("Partecipanti: {}", ev.attendees.iter().map(|a| if a.name.trim().is_empty() { a.email.clone() } else { a.name.clone() }).collect::<Vec<_>>().join(", ")));
    }
    if !ev.notes.trim().is_empty() {
        lines.push(String::new());
        lines.push(ev.notes.trim().to_string());
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROME: &str = "BEGIN:VTIMEZONE\r\nTZID:W. Europe Standard Time\r\nBEGIN:STANDARD\r\nDTSTART:16010101T030000\r\nTZOFFSETFROM:+0200\r\nTZOFFSETTO:+0100\r\nRRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=10\r\nEND:STANDARD\r\nBEGIN:DAYLIGHT\r\nDTSTART:16010101T020000\r\nTZOFFSETFROM:+0100\r\nTZOFFSETTO:+0200\r\nRRULE:FREQ=YEARLY;INTERVAL=1;BYDAY=-1SU;BYMONTH=3\r\nEND:DAYLIGHT\r\nEND:VTIMEZONE\r\n";

    fn ms(s: &str) -> i64 {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc().timestamp_millis()
    }

    fn outlook_invite(extra: &str) -> String {
        format!(
            "BEGIN:VCALENDAR\r\nMETHOD:REQUEST\r\nPRODID:Microsoft Exchange Server 2010\r\nVERSION:2.0\r\n{ROME}BEGIN:VEVENT\r\nORGANIZER;CN=Marco Ventura:mailto:m.ventura@valliarredi.it\r\nATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE;CN=Chiara Valli:mailto:chiara.valli@valliarredi.it\r\nATTENDEE;ROLE=OPT-PARTICIPANT;PARTSTAT=ACCEPTED;CN=\"Valli, Giorgio\":MAILTO:giorgio@valliarredi.it\r\nDESCRIPTION;LANGUAGE=it-IT:Ordine del giorno:\\n- preventivo Bassano\\n- listino 2027\r\nUID:040000008200E00074C5B7101A82E0080000000\r\n SAMPLE\r\nSUMMARY;LANGUAGE=it-IT:Riunione commerciale\r\nDTSTART;TZID=W. Europe Standard Time:20261019T103000\r\nDTEND;TZID=W. Europe Standard Time:20261019T120000\r\nCLASS:PUBLIC\r\nSEQUENCE:2\r\nLOCATION;LANGUAGE=it-IT:Sala riunioni\\, primo piano\r\n{extra}BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER;RELATED=START:-PT15M\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        )
    }

    #[test]
    fn reads_an_outlook_invitation() {
        let cal = parse(&outlook_invite(""));
        assert_eq!(cal.method.as_deref(), Some("REQUEST"));
        let ev = &cal.events[0];
        assert_eq!(ev.uid, "040000008200E00074C5B7101A82E0080000000SAMPLE");
        assert_eq!(ev.title, "Riunione commerciale");
        assert_eq!(ev.location, "Sala riunioni, primo piano");
        assert_eq!(ev.notes, "Ordine del giorno:\n- preventivo Bassano\n- listino 2027");
        // 10:30 a Roma in ottobre (ora legale) = 08:30 UTC.
        assert_eq!(ev.start, ms("2026-10-19 08:30"));
        assert_eq!(ev.end, ms("2026-10-19 10:00"));
        assert!(!ev.all_day);
        assert_eq!(ev.sequence, 2);
        assert_eq!(ev.reminder, Some(15));
        assert_eq!(ev.organizer, Some(Person::new("Marco Ventura", "m.ventura@valliarredi.it")));
        assert_eq!(ev.attendees.len(), 2);
        assert_eq!(ev.attendees[0].status, PartStat::NeedsAction);
        assert_eq!(ev.attendees[1], Attendee { name: "Valli, Giorgio".into(), email: "giorgio@valliarredi.it".into(), status: PartStat::Accepted, optional: true });
    }

    #[test]
    fn time_zones_follow_daylight_saving() {
        let z = Zone::from_comp(&components(ROME)[0]).unwrap().1;
        let local = |s: &str| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        assert_eq!(z.offset_local(local("2026-01-15 10:00")), 3600);
        assert_eq!(z.offset_local(local("2026-07-15 10:00")), 7200);
        assert_eq!(z.offset_local(local("2026-10-25 01:00")), 7200);
        assert_eq!(z.offset_local(local("2026-10-25 04:00")), 3600);
        assert_eq!(z.offset_local(local("2026-03-29 03:30")), 7200);
        assert_eq!(z.offset_utc(local("2026-03-29 00:59")), 3600);
        assert_eq!(z.offset_utc(local("2026-03-29 01:00")), 7200);
        // Le regole ricavate dagli scostamenti tornano a essere le stesse.
        let lz = LocalZone::from_offsets("Europe/Rome".into(), 2026, |secs| z.offset_utc(chrono::DateTime::from_timestamp(secs, 0).unwrap().naive_utc()));
        let mut text = String::new();
        lz.write(&mut text);
        assert!(text.contains("RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU"), "{text}");
        assert!(text.contains("RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU"), "{text}");
        assert!(text.contains("BEGIN:DAYLIGHT\r\nDTSTART:19700329T020000\r\nTZOFFSETFROM:+0100\r\nTZOFFSETTO:+0200"), "{text}");
        let again = Zone::from_comp(&components(&text)[0]).unwrap().1;
        for s in ["2026-02-01 12:00", "2026-06-01 12:00", "2027-11-30 09:00", "2026-10-25 04:00"] {
            assert_eq!(again.offset_local(local(s)), z.offset_local(local(s)), "{s}");
        }
    }

    #[test]
    fn reads_series_exceptions_and_all_day() {
        let text = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:serie@example.test\r\nDTSTART:20261005T070000Z\r\nDURATION:PT45M\r\nRRULE:FREQ=WEEKLY;BYDAY=MO,WE;UNTIL=20261231T225959Z\r\nEXDATE:20261012T070000Z,20261014T070000Z\r\nSUMMARY:Stand-up\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:serie@example.test\r\nRECURRENCE-ID:20261019T070000Z\r\nDTSTART:20261019T080000Z\r\nDTEND:20261019T084500Z\r\nSUMMARY:Stand-up (spostato)\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:ferie@example.test\r\nDTSTART;VALUE=DATE:20261228\r\nDTEND;VALUE=DATE:20261231\r\nSUMMARY:Ferie\r\nCATEGORIES:Personale,Ferie\\, inverno\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:mensile@example.test\r\nDTSTART:20261013T090000\r\nDTEND:20261013T100000\r\nRRULE:FREQ=MONTHLY;BYDAY=TU;BYSETPOS=2;COUNT=6\r\nSUMMARY:Consiglio\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let cal = parse(text);
        assert_eq!(cal.events.len(), 4);
        let s = &cal.events[0];
        assert_eq!(s.end - s.start, 45 * 60_000);
        let r = s.repeat.as_ref().unwrap();
        assert_eq!((r.freq, r.interval, r.days.clone(), r.week), (Freq::Weekly, 1, vec![1, 3], None));
        assert_eq!(r.until, Some(ms("2026-12-31 22:59") + 59_000));
        assert_eq!(s.exceptions, vec![ms("2026-10-12 07:00"), ms("2026-10-14 07:00")]);
        let ferie = cal.events.iter().find(|e| e.uid == "ferie@example.test").unwrap();
        assert!(ferie.all_day);
        assert_eq!(ferie.start, local_ms(midnight(NaiveDate::from_ymd_opt(2026, 12, 28).unwrap())));
        assert_eq!(ferie.end, local_ms(midnight(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap())));
        assert_eq!(ferie.categories, vec!["Personale", "Ferie, inverno"]);
        let m = cal.events.iter().find(|e| e.uid == "mensile@example.test").unwrap().repeat.clone().unwrap();
        assert_eq!((m.freq, m.week, m.days, m.count), (Freq::Monthly, Some(2), vec![2], Some(6)));
        // L'occorrenza spostata viene dopo la serie.
        assert_eq!(cal.events[3].recurrence_id, Some(ms("2026-10-19 07:00")));
    }

    #[test]
    fn writes_and_reads_back() {
        let start = local_ms(NaiveDateTime::parse_from_str("2026-10-20 09:00", "%Y-%m-%d %H:%M").unwrap());
        let ev = CalEvent {
            id: "x".into(),
            uid: "abc@better-look".into(),
            title: "Revisione catalogo; bozza 2, finale".into(),
            location: "Tipografia Gualtieri".into(),
            start,
            end: start + 90 * 60_000,
            repeat: Some(Repeat { freq: Freq::Weekly, interval: 2, days: vec![2, 4], week: None, until: None, count: Some(5) }),
            exceptions: vec![add_days_local(start, 14)],
            reminder: Some(30),
            notes: "Portare le prove colore.\nE il menabò.".into(),
            categories: vec!["Clienti".into()],
            organizer: Some(Person::new("Chiara Valli", "chiara.valli@valliarredi.it")),
            attendees: vec![Attendee { name: "Davide Orlandi".into(), email: "d.orlandi@studiorlandi.it".into(), ..Default::default() }],
            sequence: 1,
            ..Default::default()
        };
        let text = write(Some("REQUEST"), std::slice::from_ref(&ev));
        assert!(text.contains("METHOD:REQUEST\r\n"));
        assert!(text.contains("BEGIN:VTIMEZONE"));
        assert!(text.contains("RRULE:FREQ=WEEKLY;BYDAY=TU,TH;INTERVAL=2;COUNT=5"), "{text}");
        assert!(text.contains("SUMMARY:Revisione catalogo\\; bozza 2\\, finale"));
        assert!(unfold(&text).iter().any(|l| l == "ATTENDEE;CN=Davide Orlandi;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE:mailto:d.orlandi@studiorlandi.it"));
        assert!(text.lines().all(|l| l.len() <= 76), "righe troppo lunghe");
        let back = parse(&text).events.remove(0);
        assert_eq!(back.title, ev.title);
        assert_eq!(back.notes, ev.notes);
        assert_eq!((back.start, back.end), (ev.start, ev.end));
        assert_eq!(back.repeat, ev.repeat);
        assert_eq!(back.exceptions, ev.exceptions);
        assert_eq!(back.reminder, Some(30));
        assert_eq!(back.categories, ev.categories);
        assert_eq!(back.organizer, ev.organizer);
        assert_eq!(back.attendees, ev.attendees);
        // Tutto il giorno: solo date, senza zona.
        let day = CalEvent { uid: "d".into(), title: "Fiera".into(), start: local_ms(midnight(NaiveDate::from_ymd_opt(2027, 1, 18).unwrap())), all_day: true, ..Default::default() };
        let day = CalEvent { end: add_days_local(day.start, 3), ..day };
        let text = write(None, std::slice::from_ref(&day));
        assert!(text.contains("DTSTART;VALUE=DATE:20270118\r\nDTEND;VALUE=DATE:20270121\r\n"), "{text}");
        assert!(!text.contains("VTIMEZONE"));
        let back = parse(&text).events.remove(0);
        assert_eq!((back.start, back.end, back.all_day), (day.start, day.end, true));
    }

    #[test]
    fn folds_long_lines_on_characters() {
        let mut out = String::new();
        fold(&format!("DESCRIPTION:{}", "è".repeat(80)), &mut out);
        for l in out.split("\r\n") {
            assert!(l.len() <= 75);
        }
        assert_eq!(unfold(&out).concat(), format!("DESCRIPTION:{}", "è".repeat(80)));
    }

    #[test]
    fn texts_for_messages() {
        let start = local_ms(NaiveDateTime::parse_from_str("2026-10-19 10:30", "%Y-%m-%d %H:%M").unwrap());
        let ev = CalEvent { title: "Riunione".into(), start, end: start + 90 * 60_000, location: "Sala".into(), ..Default::default() };
        assert_eq!(when_text(&ev), "lunedì 19 ottobre 2026, 10:30-12:00");
        let s = summary_text(&ev);
        assert!(s.contains("Quando: lunedì 19 ottobre 2026, 10:30-12:00\nDove: Sala"), "{s}");
        let r = Repeat { freq: Freq::Weekly, interval: 1, days: vec![1, 2, 3, 4, 5], week: None, until: None, count: None };
        assert_eq!(repeat_text(&r), "ogni giorno feriale");
        assert_eq!(nth_weekday(2026, 10, Weekday::Sun, -1), NaiveDate::from_ymd_opt(2026, 10, 25));
        assert_eq!(nth_weekday(2026, 3, Weekday::Sun, 2), NaiveDate::from_ymd_opt(2026, 3, 8));
    }
}
