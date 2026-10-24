//! Strumenti per le conversazioni, come in Outlook: Ignora conversazione
//! (i messaggi vanno in Posta eliminata, anche quelli che arriveranno) e
//! Pulisci conversazione (toglie i messaggi già riportati per intero in una
//! risposta successiva).

use super::*;

/// Conversazioni ignorate: (account, chiave della conversazione).
const IGNORED_KEY: &str = "conversazioni-ignorate";

/// Testo ridotto all'essenziale per confrontare un messaggio con le risposte:
/// minuscole, senza segni di citazione, spazi e punteggiatura.
fn normalized(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_start_matches(|c: char| c == '>' || c.is_whitespace());
        for c in line.chars() {
            if c.is_alphanumeric() {
                out.extend(c.to_lowercase());
            }
        }
    }
    out
}

/// Il testo proprio del messaggio, senza la citazione di quelli precedenti.
fn own_text(text: &str) -> String {
    let mut kept = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        // Inizio della citazione: "Il giorno ... ha scritto:", "Da: ...", "-----Messaggio originale-----", "On ... wrote:".
        let low = t.to_lowercase();
        let quote_head = (low.starts_with("il ") && low.ends_with("ha scritto:"))
            || (low.starts_with("on ") && low.ends_with("wrote:"))
            || low.starts_with("da: ") && kept.len() > 1
            || low.starts_with("from: ") && kept.len() > 1
            || low.contains("messaggio originale")
            || low.contains("original message");
        if quote_head {
            break;
        }
        if t.starts_with('>') {
            continue;
        }
        kept.push(line);
    }
    kept.join("\n")
}

fn thread_key(m: &MsgRow) -> Option<String> {
    m.thread.clone().or_else(|| m.message_id.clone())
}

impl Engine {
    fn ignored_threads(&self) -> Result<HashSet<(String, String)>> {
        Ok(self.db.meta(IGNORED_KEY)?.and_then(|s| serde_json::from_str::<Vec<(String, String)>>(&s).ok()).unwrap_or_default().into_iter().collect())
    }

    fn save_ignored(&self, set: &HashSet<(String, String)>) -> Result<()> {
        let mut list: Vec<&(String, String)> = set.iter().collect();
        list.sort();
        self.db.set_meta(IGNORED_KEY, &serde_json::to_string(&list)?)
    }

    /// La conversazione del messaggio è ignorata?
    pub fn thread_ignored(&self, message_id: i64) -> Result<bool> {
        let m = self.db.message(message_id)?;
        Ok(thread_key(&m).is_some_and(|k| self.ignored_threads().map(|s| s.contains(&(m.account_id.clone(), k))).unwrap_or(false)))
    }

    /// Ignora conversazione (o smetti di ignorarla). Ignorando, i messaggi
    /// della conversazione vanno in Posta eliminata e quelli nuovi li
    /// seguiranno; smettendo, tornano in Posta in arrivo. Restituisce quanti
    /// messaggi sono stati spostati.
    pub async fn ignore_thread(&self, message_id: i64, on: bool) -> Result<usize> {
        let m = self.db.message(message_id)?;
        let key = thread_key(&m).ok_or_else(|| Error::user("Questo messaggio non fa parte di una conversazione."))?;
        let acct = self.db.account(&m.account_id)?;
        let mut set = self.ignored_threads()?;
        let item = (m.account_id.clone(), key.clone());
        if on {
            set.insert(item);
            self.save_ignored(&set)?;
            let mut ids: Vec<i64> = self.db.thread_messages(&m.account_id, &key)?.into_iter().filter(|x| !x.draft).map(|x| x.id).collect();
            if !ids.contains(&message_id) && !m.draft {
                ids.push(message_id);
            }
            self.move_to_role(&ids, "deleted").await?;
            Ok(ids.len())
        } else {
            set.remove(&item);
            self.save_ignored(&set)?;
            let trash = self.role_folder(&acct, "deleted")?;
            let inbox = self.role_folder(&acct, "inbox")?;
            let ids: Vec<i64> = self.db.thread_in_folder(&m.account_id, &key, trash.id)?.into_iter().map(|x| x.id).collect();
            if !ids.is_empty() {
                self.move_messages(&ids, inbox.id).await?;
            }
            Ok(ids.len())
        }
    }

    /// Messaggi appena arrivati che appartengono a conversazioni ignorate:
    /// vanno subito in Posta eliminata. Restituisce i loro identificativi.
    pub(super) async fn drop_ignored(&self, rows: &[MsgRow]) -> Result<Vec<i64>> {
        let set = self.ignored_threads()?;
        if set.is_empty() {
            return Ok(vec![]);
        }
        let ids: Vec<i64> = rows.iter().filter(|m| thread_key(m).is_some_and(|k| set.contains(&(m.account_id.clone(), k)))).map(|m| m.id).collect();
        if !ids.is_empty() {
            self.move_to_role(&ids, "deleted").await?;
        }
        Ok(ids)
    }

    async fn text_of(&self, m: &MsgRow) -> Option<String> {
        let raw = match self.db.body(m.id).ok()? {
            Some(r) => r,
            None => self.raw(m).await.ok()?,
        };
        mime::parse(&raw).map(|p| p.text())
    }

    /// Pulisci conversazione: nella conversazione del messaggio (oppure in
    /// tutte quelle della cartella, ed eventualmente delle sottocartelle)
    /// i messaggi riportati per intero in una risposta successiva vanno in
    /// Posta eliminata. Restano quelli da leggere, contrassegnati, con
    /// allegati o bozze. Restituisce quanti messaggi sono stati spostati.
    pub async fn clean_up(&self, message_id: Option<i64>, folder_id: Option<i64>, subfolders: bool) -> Result<usize> {
        // Messaggi da valutare, raggruppati per conversazione.
        let mut candidates: Vec<MsgRow> = Vec::new();
        if let Some(id) = message_id {
            let m = self.db.message(id)?;
            match thread_key(&m) {
                Some(k) => candidates = self.db.thread_messages(&m.account_id, &k)?,
                None => return Ok(0),
            }
        } else if let Some(fid) = folder_id {
            let mut folders = vec![fid];
            if subfolders {
                folders.extend(self.db.subfolders(fid)?.into_iter().map(|f| f.id));
            }
            for f in folders {
                candidates.extend(self.db.list_messages(f, None, 100_000, 0)?);
            }
        }
        let mut threads: HashMap<(String, String), Vec<MsgRow>> = HashMap::new();
        for m in candidates {
            if let Some(k) = thread_key(&m) {
                threads.entry((m.account_id.clone(), k)).or_default().push(m);
            }
        }
        let mut redundant = Vec::new();
        for ((account, key), _) in threads.into_iter().filter(|(_, v)| !v.is_empty()) {
            // Tutta la conversazione, anche le risposte in Posta inviata.
            let all = self.db.thread_messages(&account, &key)?;
            if all.len() < 2 {
                continue;
            }
            let scope: HashSet<i64> = if message_id.is_some() {
                all.iter().map(|m| m.id).collect()
            } else {
                let mut fs = vec![folder_id.unwrap_or_default()];
                if subfolders {
                    fs.extend(self.db.subfolders(folder_id.unwrap_or_default())?.into_iter().map(|f| f.id));
                }
                all.iter().filter(|m| fs.contains(&m.folder_id)).map(|m| m.id).collect()
            };
            let mut texts: Vec<(i64, i64, String)> = Vec::new();
            for m in &all {
                if let Some(t) = self.text_of(m).await {
                    texts.push((m.id, m.date, normalized(&t)));
                }
            }
            for m in all.iter().filter(|m| scope.contains(&m.id) && !m.unread && !m.flagged && !m.has_atts && !m.draft) {
                let Some(t) = self.text_of(m).await else { continue };
                let own = normalized(&own_text(&t));
                if own.chars().count() < 20 {
                    continue;
                }
                let covered = texts.iter().any(|(id, date, later)| *id != m.id && *date >= m.date && later.len() > own.len() && later.contains(&own));
                if covered {
                    redundant.push(m.id);
                }
            }
        }
        if !redundant.is_empty() {
            self.move_to_role(&redundant, "deleted").await?;
        }
        Ok(redundant.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::Memory;

    struct Setup {
        eng: Arc<Engine>,
        inbox: i64,
        sent: i64,
        trash: i64,
    }

    fn setup() -> Setup {
        let db = Db::memory().unwrap();
        let acct = AccountConfig {
            id: "a1".into(),
            name: "Chiara Valli".into(),
            email: "chiara.valli@valliarredi.it".into(),
            kind: AccountKind::Local,
            incoming: None,
            outgoing: None,
            sync: SyncOptions::default(),
            server_saves_sent: false,
            source_file: None,
            signature_new: None,
            signature_reply: None,
            auto_reply: None,
        };
        db.save_account(&acct).unwrap();
        let inbox = db.upsert_folder("a1", "INBOX", "Posta in arrivo", Some("inbox"), Some("/"), true).unwrap();
        let sent = db.upsert_folder("a1", "Sent", "Posta inviata", Some("sent"), Some("/"), true).unwrap();
        let trash = db.upsert_folder("a1", "Trash", "Posta eliminata", Some("deleted"), Some("/"), true).unwrap();
        let eng = Engine::new(db, Arc::new(Memory::default()), std::env::temp_dir(), Arc::new(|_| {}));
        Setup { eng, inbox, sent, trash }
    }

    fn add(s: &Setup, folder: i64, n: u32, date: &str, refs: &str, body: &str, read: bool) -> i64 {
        let headers = if refs.is_empty() { String::new() } else { format!("In-Reply-To: <{refs}>\r\nReferences: <{refs}>\r\n") };
        let raw = format!("From: Davide Orlandi <d.orlandi@studiorlandi.it>\r\nTo: Chiara Valli <chiara.valli@valliarredi.it>\r\nSubject: Preventivo\r\nMessage-ID: <c{n}@example.test>\r\n{headers}Date: {date}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n");
        let mut nm = mime::parse(raw.as_bytes()).unwrap().to_new_message(now_secs(), true, raw.len() as i64);
        nm.unread = !read;
        nm.raw = Some(raw.into_bytes());
        s.eng.db.insert_message("a1", folder, &nm).unwrap().unwrap()
    }

    #[tokio::test]
    async fn clean_up_keeps_only_what_is_not_quoted_later() {
        let s = setup();
        let first = add(&s, s.inbox, 1, "Mon, 19 Oct 2026 09:00:00 +0200", "", "Ciao Chiara, ti mando il preventivo per gli uffici di Bassano.", true);
        add(&s, s.sent, 2, "Mon, 19 Oct 2026 10:00:00 +0200", "c1@example.test", "Grazie Davide, va bene.\n\nIl giorno lun 19 ott 2026, Davide Orlandi ha scritto:\n> Ciao Chiara, ti mando il preventivo per gli uffici di Bassano.", true);
        let unread = add(&s, s.inbox, 3, "Mon, 19 Oct 2026 11:00:00 +0200", "c1@example.test", "Aggiungo le cassettiere su ruote.", false);
        let other = add(&s, s.inbox, 4, "Mon, 19 Oct 2026 12:00:00 +0200", "", "Messaggio senza risposte, resta dove si trova.", true);
        assert_eq!(s.eng.clean_up(None, Some(s.inbox), false).await.unwrap(), 1);
        assert_eq!(s.eng.db.message(first).unwrap().folder_id, s.trash);
        assert_eq!(s.eng.db.message(unread).unwrap().folder_id, s.inbox);
        assert_eq!(s.eng.db.message(other).unwrap().folder_id, s.inbox);
        assert_eq!(s.eng.clean_up(Some(unread), None, false).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn ignored_threads_go_and_stay_in_the_trash() {
        let s = setup();
        let first = add(&s, s.inbox, 1, "Mon, 19 Oct 2026 09:00:00 +0200", "", "Prima.", true);
        let second = add(&s, s.inbox, 2, "Mon, 19 Oct 2026 10:00:00 +0200", "c1@example.test", "Seconda.", false);
        assert_eq!(s.eng.ignore_thread(second, true).await.unwrap(), 2);
        assert!(s.eng.thread_ignored(first).unwrap());
        assert_eq!(s.eng.db.message(first).unwrap().folder_id, s.trash);
        // Una nuova risposta nella stessa conversazione va subito nel cestino.
        let third = add(&s, s.inbox, 3, "Mon, 19 Oct 2026 11:00:00 +0200", "c1@example.test", "Terza.", false);
        let row = s.eng.db.message(third).unwrap();
        assert_eq!(s.eng.drop_ignored(&[row]).await.unwrap(), vec![third]);
        assert_eq!(s.eng.db.message(third).unwrap().folder_id, s.trash);
        // Smettendo di ignorarla, i messaggi tornano in arrivo.
        assert_eq!(s.eng.ignore_thread(third, false).await.unwrap(), 3);
        assert_eq!(s.eng.db.message(first).unwrap().folder_id, s.inbox);
        assert!(!s.eng.thread_ignored(first).unwrap());
    }

    #[test]
    fn own_text_drops_quotes() {
        let t = "Va bene.\n\nIl giorno lun 19 ott 2026, Davide ha scritto:\n> testo vecchio";
        assert_eq!(own_text(t).trim(), "Va bene.");
        assert_eq!(normalized("> Ciao,\n>  a  presto!"), "ciaoapresto");
    }
}
