//! Calendario nel motore: appuntamenti, riunioni, inviti ricevuti per posta e
//! risposte all'organizzatore. Le ripetizioni si espandono nell'interfaccia;
//! qui si conservano le serie e le occorrenze modificate a parte.

use super::*;
use crate::calendar;

/// Cosa fare di un invito ricevuto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InviteAction {
    Accepted,
    Tentative,
    Declined,
    /// Aggiungi al calendario (eventi pubblicati, senza risposta).
    Add,
    /// Togli dal calendario (evento annullato dall'organizzatore).
    Remove,
}

fn lower(s: &str) -> String {
    s.trim().to_lowercase()
}

impl Engine {
    fn my_addresses(&self) -> Result<Vec<String>> {
        Ok(self.db.accounts()?.into_iter().filter(|a| a.kind != AccountKind::Local).map(|a| lower(&a.email)).collect())
    }

    /// L'evento è mio: l'ho creato io o sono l'organizzatore.
    fn organized_by_me(ev: &CalEvent, mine: &[String]) -> bool {
        ev.organizer.as_ref().map_or(true, |o| mine.contains(&lower(&o.email)))
    }

    pub fn events(&self, from: i64, to: i64) -> Result<Vec<CalEvent>> {
        self.db.events_between(from, to)
    }

    /// Salva l'evento; un'occorrenza modificata a parte esce dalla sua serie.
    fn store_event(&self, ev: &CalEvent) -> Result<()> {
        self.db.save_event(ev)?;
        if let Some(rid) = ev.recurrence_id {
            for mut master in self.db.events_by_uid(&ev.uid)? {
                if master.recurrence_id.is_none() && master.repeat.is_some() && !master.exceptions.contains(&rid) {
                    master.exceptions.push(rid);
                    master.exceptions.sort_unstable();
                    self.db.save_event(&master)?;
                }
            }
        }
        Ok(())
    }

    fn sending_account(&self, preferred: Option<&str>) -> Result<AccountConfig> {
        let accts = self.db.accounts()?;
        let can = |a: &&AccountConfig| a.kind != AccountKind::Local && a.outgoing.is_some();
        preferred
            .and_then(|id| accts.iter().filter(can).find(|a| a.id == id))
            .or_else(|| accts.iter().find(can))
            .cloned()
            .ok_or_else(|| Error::user("Per inviare gli inviti serve un account di posta."))
    }

    /// Messaggio iTIP nella posta in uscita: testo per le persone più la parte
    /// text/calendar per i programmi.
    fn send_itip(&self, acct: &AccountConfig, method: &str, ev: &CalEvent, to: Vec<Person>, subject: String, text: String) -> Result<()> {
        if to.is_empty() {
            return Ok(());
        }
        let ics = calendar::write(Some(method), std::slice::from_ref(ev));
        let d = Draft {
            key: mime::random_id(),
            account_id: acct.id.clone(),
            to,
            cc: vec![],
            bcc: vec![],
            subject,
            body: text,
            html: None,
            plain: false,
            attachments: vec![],
            in_reply_to: None,
            references: vec![],
            reply_to_message: None,
        };
        let built = mime::build_calendar(acct, &d, method, &ics)?;
        self.db.outbox_add(&acct.id, &built.wire, &built.copy, &built.envelope, &d, now_secs())?;
        self.outbox_wake.notify_one();
        Ok(())
    }

    fn others(list: &[Attendee], mine: &[String]) -> Vec<Person> {
        list.iter().filter(|a| !mine.contains(&lower(&a.email))).map(|a| Person::new(a.name.clone(), a.email.clone())).collect()
    }

    fn title_of(ev: &CalEvent) -> String {
        if ev.title.trim().is_empty() {
            "(senza titolo)".into()
        } else {
            ev.title.trim().to_string()
        }
    }

    /// Salva un appuntamento. Se è una riunione organizzata da me e `send` è
    /// vero, manda l'invito (o l'aggiornamento) ai partecipanti.
    pub fn save_event(&self, mut ev: CalEvent, send: bool) -> Result<CalEvent> {
        ev.title = ev.title.trim().to_string();
        ev.location = ev.location.trim().to_string();
        if ev.end < ev.start {
            return Err(Error::user("La fine deve venire dopo l'inizio."));
        }
        if ev.all_day && ev.end <= ev.start {
            ev.end = calendar::add_days_local(ev.start, 1);
        }
        let start = ev.start;
        if let Some(r) = &mut ev.repeat {
            r.interval = r.interval.max(1);
            if r.freq == Freq::Weekly && r.days.is_empty() {
                r.days.push(calendar::local_weekday(start));
            }
            if r.until.is_some_and(|u| u < start) {
                return Err(Error::user("La ripetizione deve finire dopo il primo appuntamento."));
            }
        }
        ev.attendees.retain(|a| !a.email.trim().is_empty());
        let mut seen = HashSet::new();
        ev.attendees.retain(|a| seen.insert(lower(&a.email)));
        let old = if ev.id.is_empty() { None } else { self.db.event(&ev.id)? };
        if ev.id.is_empty() {
            ev.id = mime::random_id();
        }
        if ev.uid.is_empty() {
            ev.uid = format!("{}@better-look", mime::random_id());
        }
        let mine = self.my_addresses()?;
        let mut acct = None;
        if !ev.attendees.is_empty() && Self::organized_by_me(&ev, &mine) {
            let a = self.sending_account(ev.account_id.as_deref())?;
            ev.account_id = Some(a.id.clone());
            ev.organizer = Some(Person::new(a.name.clone(), a.email.clone()));
            ev.response = None;
            if send {
                if let Some(o) = &old {
                    ev.sequence = o.sequence.max(ev.sequence) + 1;
                }
                acct = Some(a);
            }
        }
        self.store_event(&ev)?;
        if let Some(a) = acct {
            self.send_itip(&a, "REQUEST", &ev, Self::others(&ev.attendees, &mine), Self::title_of(&ev), calendar::summary_text(&ev))?;
            // Chi è stato tolto dalla riunione riceve l'annullamento.
            if let Some(o) = &old {
                let gone: Vec<Attendee> = o.attendees.iter().filter(|x| !ev.attendees.iter().any(|y| y.email.eq_ignore_ascii_case(&x.email))).cloned().collect();
                if !gone.is_empty() {
                    let cancel = CalEvent { attendees: gone.clone(), ..ev.clone() };
                    self.send_itip(&a, "CANCEL", &cancel, Self::others(&gone, &mine), format!("Annullato: {}", Self::title_of(&ev)), format!("Non sei più tra i partecipanti.\n\n{}", calendar::summary_text(&cancel)))?;
                }
            }
        }
        self.emit(Event::Calendar);
        Ok(ev)
    }

    /// Risposta all'organizzatore di un invito.
    fn reply(&self, ev: &CalEvent, status: PartStat) -> Result<()> {
        let Some(org) = ev.organizer.clone() else { return Ok(()) };
        let (prefix, verb) = match status {
            PartStat::Accepted => ("Accettato", "ha accettato l'invito"),
            PartStat::Tentative => ("Provvisorio", "potrebbe partecipare: risposta provvisoria"),
            PartStat::Declined => ("Rifiutato", "ha rifiutato l'invito"),
            PartStat::NeedsAction => return Ok(()),
        };
        let acct = self.sending_account(ev.account_id.as_deref())?;
        let mine = self.my_addresses()?;
        let me = ev
            .attendees
            .iter()
            .find(|a| a.email.eq_ignore_ascii_case(&acct.email))
            .or_else(|| ev.attendees.iter().find(|a| mine.contains(&lower(&a.email))))
            .cloned()
            .unwrap_or_else(|| Attendee { name: acct.name.clone(), email: acct.email.clone(), ..Default::default() });
        let me = Attendee { status, name: if me.name.trim().is_empty() { acct.name.clone() } else { me.name.clone() }, ..me };
        let who = if me.name.trim().is_empty() { me.email.clone() } else { me.name.clone() };
        let answer = CalEvent { attendees: vec![me], reminder: None, ..ev.clone() };
        self.send_itip(&acct, "REPLY", &answer, vec![org], format!("{prefix}: {}", Self::title_of(ev)), format!("{who} {verb}.\n\n{}", calendar::summary_text(ev)))
    }

    /// Elimina l'evento, o solo l'occorrenza della serie che inizia a
    /// `occurrence`. Con `send` avvisa i partecipanti (riunione mia) oppure
    /// l'organizzatore (invito accettato).
    pub fn delete_event(&self, id: &str, occurrence: Option<i64>, send: bool) -> Result<()> {
        let Some(mut ev) = self.db.event(id)? else { return Ok(()) };
        let mine = self.my_addresses()?;
        let mut notice = ev.clone();
        match occurrence {
            Some(at) if ev.repeat.is_some() => {
                if !ev.exceptions.contains(&at) {
                    ev.exceptions.push(at);
                    ev.exceptions.sort_unstable();
                }
                self.db.save_event(&ev)?;
                notice = CalEvent { recurrence_id: Some(at), start: at, end: at + (ev.end - ev.start), repeat: None, exceptions: vec![], ..ev.clone() };
            }
            _ if ev.recurrence_id.is_none() => {
                for e in self.db.events_by_uid(&ev.uid)? {
                    self.db.delete_event(&e.id)?;
                }
            }
            // Occorrenza spostata: la serie la tiene già fuori.
            _ => self.db.delete_event(&ev.id)?,
        }
        if send && !ev.attendees.is_empty() {
            if Self::organized_by_me(&ev, &mine) {
                let a = self.sending_account(ev.account_id.as_deref())?;
                notice.sequence += 1;
                let to = Self::others(&notice.attendees, &mine);
                self.send_itip(&a, "CANCEL", &notice, to, format!("Annullato: {}", Self::title_of(&ev)), format!("L'evento è stato annullato.\n\n{}", calendar::summary_text(&notice)))?;
            } else if ev.response != Some(PartStat::Declined) {
                self.reply(&notice, PartStat::Declined)?;
            }
        }
        self.emit(Event::Calendar);
        Ok(())
    }

    /// Invito contenuto nel messaggio, con lo stato nel calendario. Una risposta
    /// a una riunione mia aggiorna subito l'elenco dei partecipanti.
    pub(super) fn invite_info(&self, parsed: &mime::Parsed) -> Option<Invite> {
        let text = parsed.calendar()?;
        let cal = calendar::parse(&text);
        let method = cal.method.clone().unwrap_or_else(|| "PUBLISH".into()).to_ascii_lowercase();
        let ev = cal.events.into_iter().next()?;
        let existing = self.db.events_by_uid(&ev.uid).unwrap_or_default();
        let mut local = existing.iter().find(|e| e.recurrence_id == ev.recurrence_id).cloned();
        if let (None, Some(rid)) = (&local, ev.recurrence_id) {
            local = existing.iter().find(|e| e.recurrence_id.is_none() && !e.exceptions.contains(&rid)).cloned();
        }
        let mine = self.my_addresses().unwrap_or_default();
        let replier = if method == "reply" { ev.attendees.first().cloned() } else { None };
        if let (Some(r), Some(l)) = (&replier, local.as_mut()) {
            let mut changed = false;
            if Self::organized_by_me(l, &mine) {
                if let Some(a) = l.attendees.iter_mut().find(|a| a.email.eq_ignore_ascii_case(&r.email)) {
                    changed = a.status != r.status;
                    a.status = r.status;
                }
            }
            if changed && self.db.save_event(l).is_ok() {
                self.emit(Event::Calendar);
            }
        }
        let outdated = method != "reply" && local.as_ref().is_some_and(|l| l.sequence > ev.sequence);
        let for_me = ev.attendees.iter().any(|a| mine.contains(&lower(&a.email)));
        Some(Invite { method, event: ev, local, outdated, replier, for_me })
    }

    /// Accetta, rifiuta o aggiunge al calendario l'invito di un messaggio.
    /// Restituisce l'evento nel calendario (nessuno se rifiutato o tolto).
    pub async fn respond_invite(&self, message_id: i64, action: InviteAction, send: bool) -> Result<Option<CalEvent>> {
        let row = self.db.message(message_id)?;
        let raw = self.raw(&row).await?;
        let parsed = mime::parse(&raw).ok_or_else(|| Error::user("Il messaggio non è leggibile."))?;
        let text = parsed.calendar().ok_or_else(|| Error::user("Nel messaggio non c'è un invito."))?;
        let mut inv = calendar::parse(&text).events.into_iter().next().ok_or_else(|| Error::user("Nel messaggio non c'è un invito."))?;
        let existing = self.db.events_by_uid(&inv.uid)?;
        let local = existing.iter().find(|e| e.recurrence_id == inv.recurrence_id).cloned();
        let master = existing.iter().find(|e| e.recurrence_id.is_none() && e.repeat.is_some()).cloned();
        let take_out = |me: &Engine| -> Result<()> {
            match (&local, inv.recurrence_id, &master) {
                (Some(l), _, _) => me.delete_event(&l.id, None, false),
                (None, Some(rid), Some(m)) => me.delete_event(&m.id, Some(rid), false),
                _ => Ok(()),
            }
        };
        let status = match action {
            InviteAction::Remove => {
                take_out(self)?;
                self.emit(Event::Calendar);
                return Ok(None);
            }
            InviteAction::Accepted => Some(PartStat::Accepted),
            InviteAction::Tentative => Some(PartStat::Tentative),
            InviteAction::Declined => Some(PartStat::Declined),
            InviteAction::Add => None,
        };
        if local.as_ref().is_some_and(|l| l.sequence > inv.sequence) {
            return Err(Error::user("Questo invito è superato: nel calendario c'è già una versione più recente."));
        }
        let mine = self.my_addresses()?;
        inv.id = local.as_ref().map(|l| l.id.clone()).unwrap_or_else(mime::random_id);
        inv.account_id = Some(row.account_id.clone());
        match &local {
            Some(l) => {
                // Promemoria e categorie restano quelli scelti da me.
                inv.reminder = l.reminder;
                inv.categories = l.categories.clone();
                for x in &l.exceptions {
                    if !inv.exceptions.contains(x) {
                        inv.exceptions.push(*x);
                    }
                }
                inv.exceptions.sort_unstable();
            }
            None => {
                if inv.reminder.is_none() && !inv.all_day {
                    inv.reminder = Some(15);
                }
            }
        }
        if let Some(s) = status {
            inv.response = Some(s);
            for a in inv.attendees.iter_mut() {
                if mine.contains(&lower(&a.email)) {
                    a.status = s;
                }
            }
        }
        if status == Some(PartStat::Declined) {
            take_out(self)?;
        } else {
            self.store_event(&inv)?;
        }
        if send {
            if let Some(s) = status {
                if !Self::organized_by_me(&inv, &mine) {
                    self.reply(&inv, s)?;
                }
            }
        }
        self.emit(Event::Calendar);
        Ok(if status == Some(PartStat::Declined) { None } else { Some(inv) })
    }

    /// Risponde dal calendario a un invito già aggiunto (per esempio per
    /// cambiare idea): aggiorna la mia risposta e, con `send`, avvisa l'organizzatore.
    pub fn respond_event(&self, id: &str, action: InviteAction, send: bool) -> Result<Option<CalEvent>> {
        let status = match action {
            InviteAction::Accepted => PartStat::Accepted,
            InviteAction::Tentative => PartStat::Tentative,
            InviteAction::Declined => PartStat::Declined,
            _ => return Err(Error::user("Risposta non valida.")),
        };
        let Some(mut ev) = self.db.event(id)? else { return Err(Error::NotFound) };
        let mine = self.my_addresses()?;
        if Self::organized_by_me(&ev, &mine) {
            return Err(Error::user("Sei tu l'organizzatore di questo evento."));
        }
        ev.response = Some(status);
        for a in ev.attendees.iter_mut() {
            if mine.contains(&lower(&a.email)) {
                a.status = status;
            }
        }
        if status == PartStat::Declined {
            self.delete_event(id, None, false)?;
        } else {
            self.store_event(&ev)?;
        }
        if send {
            self.reply(&ev, status)?;
        }
        self.emit(Event::Calendar);
        Ok((status != PartStat::Declined).then_some(ev))
    }

    /// Importa un file iCalendar: aggiunge gli eventi nuovi e aggiorna quelli
    /// già presenti (stesso UID) se il file ne ha una versione più recente.
    pub fn import_calendar(&self, path: &Path) -> Result<CalImport> {
        let bytes = std::fs::read(path).map_err(|e| Error::user(format!("Impossibile leggere il file: {e}")))?;
        let cal = calendar::parse(&String::from_utf8_lossy(&bytes));
        if cal.events.is_empty() {
            return Err(Error::user("Nel file non ho trovato appuntamenti. Usa un file iCalendar (.ics) esportato da un altro calendario."));
        }
        let mine = self.my_addresses()?;
        let mut res = CalImport::default();
        for mut ev in cal.events {
            if ev.cancelled {
                continue;
            }
            if !Self::organized_by_me(&ev, &mine) {
                ev.response = ev.attendees.iter().find(|a| mine.contains(&lower(&a.email))).map(|a| a.status).filter(|s| *s != PartStat::NeedsAction);
            }
            let local = self.db.events_by_uid(&ev.uid)?.into_iter().find(|e| e.recurrence_id == ev.recurrence_id);
            match local {
                Some(l) => {
                    if ev.sequence < l.sequence {
                        res.unchanged += 1;
                        continue;
                    }
                    ev.id = l.id.clone();
                    ev.account_id = l.account_id.clone();
                    if ev.reminder.is_none() {
                        ev.reminder = l.reminder;
                    }
                    for c in &l.categories {
                        if !ev.categories.contains(c) {
                            ev.categories.push(c.clone());
                        }
                    }
                    for x in &l.exceptions {
                        if !ev.exceptions.contains(x) {
                            ev.exceptions.push(*x);
                        }
                    }
                    ev.exceptions.sort_unstable();
                    if ev.response.is_none() {
                        ev.response = l.response;
                    }
                    if ev == l {
                        res.unchanged += 1;
                        continue;
                    }
                    self.store_event(&ev)?;
                    res.updated += 1;
                }
                None => {
                    ev.id = mime::random_id();
                    self.store_event(&ev)?;
                    res.added += 1;
                }
            }
        }
        self.emit(Event::Calendar);
        Ok(res)
    }

    /// Scrive tutto il calendario in un file iCalendar.
    pub fn export_calendar(&self, path: &Path) -> Result<usize> {
        let list = self.db.all_events()?;
        std::fs::write(path, calendar::write(None, &list)).map_err(|e| Error::user(format!("Impossibile salvare il file: {e}")))?;
        Ok(list.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Memory;

    struct Setup {
        eng: Arc<Engine>,
        inbox: i64,
    }

    fn setup() -> Setup {
        let db = Db::memory().unwrap();
        let srv = |host: &str, port: u16| ServerConfig { host: host.into(), port, security: Security::Ssl, username: "chiara.valli@valliarredi.it".into() };
        let acct = AccountConfig {
            id: "a1".into(),
            name: "Chiara Valli".into(),
            email: "chiara.valli@valliarredi.it".into(),
            kind: AccountKind::Imap,
            incoming: Some(srv("imap.example.test", 993)),
            outgoing: Some(srv("smtp.example.test", 465)),
            sync: SyncOptions::default(),
            server_saves_sent: false,
            source_file: None,
            signature_new: None,
            signature_reply: None,
            auto_reply: None,
        };
        db.save_account(&acct).unwrap();
        let inbox = db.upsert_folder("a1", "INBOX", "Posta in arrivo", Some("inbox"), Some("/"), true).unwrap();
        let eng = Engine::new(db, Arc::new(Memory::default()), std::env::temp_dir(), Arc::new(|_| {}));
        Setup { eng, inbox }
    }

    fn ms(s: &str) -> i64 {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap().and_utc().timestamp_millis()
    }

    /// Messaggio con un invito, come lo manda Outlook: testo più parte text/calendar.
    fn arrive(s: &Setup, uid: u32, from: &str, method: &str, event: &str) -> i64 {
        let ics = format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nMETHOD:{method}\r\nBEGIN:VEVENT\r\n{event}END:VEVENT\r\nEND:VCALENDAR\r\n");
        let raw = format!(
            "From: {from}\r\nTo: Chiara Valli <chiara.valli@valliarredi.it>\r\nSubject: Invito\r\nMessage-ID: <i{uid}@example.test>\r\nDate: Thu, 15 Oct 2026 15:02:00 +0200\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"b\"\r\n\r\n--b\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nRiunione commerciale\r\n--b\r\nContent-Type: text/calendar; charset=utf-8; method={method}\r\n\r\n{ics}--b--\r\n"
        );
        let mut nm = mime::parse(raw.as_bytes()).unwrap().to_new_message(now_secs(), true, raw.len() as i64);
        nm.uid = Some(uid);
        nm.raw = Some(raw.into_bytes());
        s.eng.db.insert_message("a1", s.inbox, &nm).unwrap().unwrap()
    }

    const MEETING: &str = "UID:riunione-1@valliarredi.it\r\nSEQUENCE:0\r\nDTSTART:20261019T083000Z\r\nDTEND:20261019T100000Z\r\nSUMMARY:Riunione commerciale\r\nLOCATION:Sala riunioni\r\nORGANIZER;CN=Marco Ventura:mailto:m.ventura@valliarredi.it\r\nATTENDEE;CN=Chiara Valli;PARTSTAT=NEEDS-ACTION;RSVP=TRUE:mailto:chiara.valli@valliarredi.it\r\nATTENDEE;CN=Giorgio Valli;PARTSTAT=ACCEPTED:mailto:giorgio@valliarredi.it\r\n";

    fn outbox(s: &Setup) -> Vec<(String, Vec<String>, String)> {
        s.eng
            .db
            .outbox_items()
            .unwrap()
            .into_iter()
            .map(|o| {
                let row = s.eng.db.outbox_get(o.id).unwrap().unwrap();
                (o.subject, row.envelope.to, String::from_utf8_lossy(&row.raw).to_string())
            })
            .collect()
    }

    fn ics_of(raw: &str) -> String {
        let p = mime::parse(raw.as_bytes()).unwrap();
        p.calendar().unwrap()
    }

    #[tokio::test]
    async fn accepting_an_invitation_adds_it_and_answers_the_organizer() {
        let s = setup();
        let id = arrive(&s, 1, "Marco Ventura <m.ventura@valliarredi.it>", "REQUEST", MEETING);
        let d = s.eng.detail(id, false).await.unwrap();
        let inv = d.invite.unwrap();
        assert_eq!(inv.method, "request");
        assert!(inv.for_me && inv.local.is_none() && !inv.outdated);
        assert_eq!(inv.event.start, ms("2026-10-19 08:30"));

        let ev = s.eng.respond_invite(id, InviteAction::Accepted, true).await.unwrap().unwrap();
        assert_eq!(ev.response, Some(PartStat::Accepted));
        assert_eq!(ev.reminder, Some(15));
        assert_eq!(ev.attendees[0].status, PartStat::Accepted);
        assert_eq!(s.eng.events(ms("2026-10-19 00:00"), ms("2026-10-20 00:00")).unwrap().len(), 1);

        let sent = outbox(&s);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "Accettato: Riunione commerciale");
        assert_eq!(sent[0].1, vec!["m.ventura@valliarredi.it".to_string()]);
        let reply = calendar::parse(&ics_of(&sent[0].2));
        assert_eq!(reply.method.as_deref(), Some("REPLY"));
        assert_eq!(reply.events[0].attendees, vec![Attendee { name: "Chiara Valli".into(), email: "chiara.valli@valliarredi.it".into(), status: PartStat::Accepted, optional: false }]);
        assert_eq!(reply.events[0].uid, "riunione-1@valliarredi.it");

        // Riaperto, l'invito risulta già nel calendario.
        let inv = s.eng.detail(id, false).await.unwrap().invite.unwrap();
        assert_eq!(inv.local.unwrap().response, Some(PartStat::Accepted));

        // Dal calendario si può cambiare idea: provvisorio, con risposta.
        let t = s.eng.respond_event(&ev.id, InviteAction::Tentative, true).unwrap().unwrap();
        assert_eq!(t.response, Some(PartStat::Tentative));
        assert_eq!(outbox(&s).iter().filter(|m| m.0.starts_with("Provvisorio:")).count(), 1);

        // Rifiutando dopo, l'evento sparisce; senza invio non parte nulla.
        assert!(s.eng.respond_invite(id, InviteAction::Declined, false).await.unwrap().is_none());
        assert!(s.eng.db.events_by_uid("riunione-1@valliarredi.it").unwrap().is_empty());
        assert_eq!(outbox(&s).len(), 2);
    }

    #[tokio::test]
    async fn cancellations_and_outdated_invitations() {
        let s = setup();
        let first = arrive(&s, 1, "Marco Ventura <m.ventura@valliarredi.it>", "REQUEST", MEETING);
        let update = MEETING.replace("SEQUENCE:0", "SEQUENCE:1").replace("Sala riunioni", "Showroom");
        let second = arrive(&s, 2, "Marco Ventura <m.ventura@valliarredi.it>", "REQUEST", &update);
        s.eng.respond_invite(second, InviteAction::Tentative, false).await.unwrap();
        let old = s.eng.detail(first, false).await.unwrap().invite.unwrap();
        assert!(old.outdated);
        assert!(s.eng.respond_invite(first, InviteAction::Accepted, false).await.is_err());
        assert_eq!(s.eng.db.events_by_uid("riunione-1@valliarredi.it").unwrap()[0].location, "Showroom");

        let cancel = arrive(&s, 3, "Marco Ventura <m.ventura@valliarredi.it>", "CANCEL", &update.replace("SEQUENCE:1", "SEQUENCE:2"));
        let inv = s.eng.detail(cancel, false).await.unwrap().invite.unwrap();
        assert_eq!(inv.method, "cancel");
        assert!(inv.local.is_some());
        s.eng.respond_invite(cancel, InviteAction::Remove, false).await.unwrap();
        assert!(s.eng.db.events_by_uid("riunione-1@valliarredi.it").unwrap().is_empty());
    }

    #[tokio::test]
    async fn my_meetings_send_invitations_updates_and_cancellations() {
        let s = setup();
        let at = |a: &str, n: &str| Attendee { name: n.into(), email: a.into(), ..Default::default() };
        let ev = CalEvent {
            title: "Campioni del rovere".into(),
            start: ms("2026-10-22 08:00"),
            end: ms("2026-10-22 09:00"),
            attendees: vec![at("d.orlandi@studiorlandi.it", "Davide Orlandi"), at("lucia@marchettitessuti.it", "Lucia Marchetti")],
            ..Default::default()
        };
        let saved = s.eng.save_event(ev, true).unwrap();
        assert_eq!(saved.organizer, Some(Person::new("Chiara Valli", "chiara.valli@valliarredi.it")));
        assert_eq!(saved.sequence, 0);
        let sent = outbox(&s);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "Campioni del rovere");
        assert_eq!(sent[0].1.len(), 2);
        assert!(sent[0].2.contains("method=REQUEST"), "{}", sent[0].2);
        assert!(mime::parse(sent[0].2.as_bytes()).unwrap().text().contains("Quando: "));

        // Lucia esce dalla riunione: aggiornamento a Davide, annullamento a lei.
        let mut upd = saved.clone();
        upd.attendees.truncate(1);
        upd.location = "Studio Orlandi".into();
        let upd = s.eng.save_event(upd, true).unwrap();
        assert_eq!(upd.sequence, 1);
        let sent = outbox(&s);
        assert_eq!(sent.len(), 3);
        let cancel = sent.iter().find(|m| m.0.starts_with("Annullato:")).unwrap();
        assert_eq!(cancel.1, vec!["lucia@marchettitessuti.it".to_string()]);
        assert_eq!(calendar::parse(&ics_of(&cancel.2)).method.as_deref(), Some("CANCEL"));

        // Davide accetta: la sua risposta aggiorna la riunione quando si apre.
        let reply = arrive(&s, 9, "Davide Orlandi <d.orlandi@studiorlandi.it>", "REPLY", &format!("UID:{}\r\nSEQUENCE:1\r\nDTSTART:20261022T080000Z\r\nSUMMARY:Campioni del rovere\r\nORGANIZER:mailto:chiara.valli@valliarredi.it\r\nATTENDEE;CN=Davide Orlandi;PARTSTAT=ACCEPTED:mailto:d.orlandi@studiorlandi.it\r\n", upd.uid));
        let inv = s.eng.detail(reply, false).await.unwrap().invite.unwrap();
        assert_eq!(inv.replier.unwrap().status, PartStat::Accepted);
        assert_eq!(s.eng.db.event(&upd.id).unwrap().unwrap().attendees[0].status, PartStat::Accepted);

        // Eliminata con avviso: annullamento a Davide.
        s.eng.delete_event(&upd.id, None, true).unwrap();
        assert!(s.eng.db.event(&upd.id).unwrap().is_none());
        let sent = outbox(&s);
        assert_eq!(sent.iter().filter(|m| m.0.starts_with("Annullato:")).count(), 2);
    }

    #[tokio::test]
    async fn series_occurrences_import_and_export() {
        let s = setup();
        let ev = CalEvent {
            title: "Stand-up".into(),
            start: ms("2026-10-19 07:00"),
            end: ms("2026-10-19 07:15"),
            repeat: Some(Repeat { freq: Freq::Weekly, interval: 1, days: vec![], week: None, until: Some(ms("2026-12-18 23:00")), count: None }),
            ..Default::default()
        };
        let saved = s.eng.save_event(ev, false).unwrap();
        assert_eq!(saved.repeat.as_ref().unwrap().days, vec![calendar::local_weekday(saved.start)]);
        assert!(outbox(&s).is_empty());
        // La serie si trova anche nelle settimane dopo l'inizio, non dopo la fine.
        assert_eq!(s.eng.events(ms("2026-11-30 00:00"), ms("2026-12-07 00:00")).unwrap().len(), 1);
        assert!(s.eng.events(ms("2027-01-04 00:00"), ms("2027-01-11 00:00")).unwrap().is_empty());
        s.eng.delete_event(&saved.id, Some(ms("2026-10-26 07:00")), false).unwrap();
        assert_eq!(s.eng.db.event(&saved.id).unwrap().unwrap().exceptions, vec![ms("2026-10-26 07:00")]);
        assert!(s.eng.save_event(CalEvent { end: ms("2026-10-19 06:00"), ..saved.clone() }, false).is_err());

        let dir = std::env::temp_dir().join(format!("bl-cal-{}", mime::random_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("calendario.ics");
        assert_eq!(s.eng.export_calendar(&file).unwrap(), 1);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains(&format!("UID:{}", saved.uid)));
        assert!(text.contains("EXDATE"));

        // Reimportato nello stesso calendario non cambia nulla; in uno nuovo si aggiunge.
        assert_eq!(s.eng.import_calendar(&file).unwrap(), CalImport { added: 0, updated: 0, unchanged: 1 });
        let other = setup();
        assert_eq!(other.eng.import_calendar(&file).unwrap(), CalImport { added: 1, updated: 0, unchanged: 0 });
        let copy = other.eng.db.all_events().unwrap().remove(0);
        assert_eq!((copy.start, copy.end, copy.exceptions.clone(), copy.repeat.clone()), (saved.start, saved.end, vec![ms("2026-10-26 07:00")], s.eng.db.event(&saved.id).unwrap().unwrap().repeat));
        std::fs::write(&file, "non è un calendario").unwrap();
        assert!(other.eng.import_calendar(&file).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
