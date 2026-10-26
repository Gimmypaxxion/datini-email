//! Cartelle di ricerca: ricerche salvate che si aprono come cartelle, come in
//! Outlook (Posta da leggere, Posta contrassegnata, da e verso persone...).
//! Si conservano nell'archivio; i messaggi si cercano ogni volta.

use super::*;

const SEARCH_FOLDERS_KEY: &str = "cartelle-ricerca";
const MAX_SEARCH_FOLDERS: usize = 60;
/// Quanti messaggi mostrare al massimo in una cartella di ricerca.
const SEARCH_FOLDER_LIMIT: i64 = 1000;

impl Engine {
    fn stored_search_folders(&self) -> Result<Vec<SearchFolder>> {
        Ok(self.db.meta(SEARCH_FOLDERS_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    fn store_search_folders(&self, list: &[SearchFolder]) -> Result<()> {
        let clean: Vec<SearchFolder> = list.iter().map(|f| SearchFolder { total: 0, unread: 0, ..f.clone() }).collect();
        self.db.set_meta(SEARCH_FOLDERS_KEY, &serde_json::to_string(&clean)?)?;
        self.emit(Event::SearchFolders);
        Ok(())
    }

    /// Le cartelle di ricerca degli account presenti, con i conteggi.
    pub fn search_folders(&self) -> Result<Vec<SearchFolder>> {
        let accounts: Vec<String> = self.db.accounts()?.into_iter().map(|a| a.id).collect();
        let now = now_secs();
        let mut out = Vec::new();
        for mut f in self.stored_search_folders()? {
            if !accounts.contains(&f.account_id) {
                continue;
            }
            let (total, unread) = self.db.search_folder_counts(&f.account_id, &f.criteria, now)?;
            f.total = total;
            f.unread = unread;
            out.push(f);
        }
        Ok(out)
    }

    /// Crea (senza id) o aggiorna una cartella di ricerca, dopo averla controllata.
    pub fn save_search_folder(&self, mut f: SearchFolder) -> Result<SearchFolder> {
        f.name = f.name.trim().chars().take(80).collect();
        if f.name.is_empty() {
            return Err(Error::user("Dai un nome alla cartella di ricerca."));
        }
        self.db.account(&f.account_id).map_err(|_| Error::user("Scegli l'account in cui cercare."))?;
        let c = &mut f.criteria;
        c.text = c.text.trim().chars().take(200).collect();
        c.people = c.people.iter().map(|p| p.trim().chars().take(120).collect::<String>()).filter(|p| !p.is_empty()).take(30).collect();
        c.category = c.category.take().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        c.min_kb = c.min_kb.filter(|k| *k > 0).map(|k| k.min(10_000_000));
        c.days = c.days.filter(|d| *d > 0).map(|d| d.min(36_500));
        match f.kind.as_str() {
            "persone" if c.people.is_empty() => return Err(Error::user("Scrivi almeno una persona: nome o indirizzo.")),
            "parole" if crate::db::fts_query(&c.text).is_none() => return Err(Error::user("Scrivi le parole da cercare.")),
            _ => {}
        }
        let mut all = self.stored_search_folders()?;
        if all.iter().any(|x| x.id != f.id && x.account_id == f.account_id && x.name.eq_ignore_ascii_case(&f.name)) {
            return Err(Error::user(format!("Esiste già una cartella di ricerca «{}».", f.name)));
        }
        if f.id.is_empty() {
            if all.len() >= MAX_SEARCH_FOLDERS {
                return Err(Error::user("Hai già troppe cartelle di ricerca: eliminane qualcuna."));
            }
            f.id = format!("cr-{}", mime::random_id());
            all.push(f.clone());
        } else {
            let Some(slot) = all.iter_mut().find(|x| x.id == f.id) else { return Err(Error::NotFound) };
            *slot = f.clone();
        }
        self.store_search_folders(&all)?;
        let (total, unread) = self.db.search_folder_counts(&f.account_id, &f.criteria, now_secs())?;
        Ok(SearchFolder { total, unread, ..f })
    }

    pub fn delete_search_folder(&self, id: &str) -> Result<()> {
        let mut all = self.stored_search_folders()?;
        let before = all.len();
        all.retain(|f| f.id != id);
        if all.len() != before {
            self.store_search_folders(&all)?;
        }
        Ok(())
    }

    /// I messaggi che rispettano le condizioni, con il nome della loro cartella.
    pub fn search_folder_messages(&self, id: &str) -> Result<Vec<MessageSummary>> {
        let f = self.stored_search_folders()?.into_iter().find(|f| f.id == id).ok_or(Error::NotFound)?;
        let names: HashMap<i64, String> = self.db.folders(&f.account_id)?.into_iter().map(|x| (x.id, x.name)).collect();
        Ok(self
            .db
            .search_folder_messages(&f.account_id, &f.criteria, now_secs(), SEARCH_FOLDER_LIMIT)?
            .into_iter()
            .map(|m| {
                let mut s = m.summary();
                s.folder_name = names.get(&m.folder_id).cloned();
                s
            })
            .collect())
    }
}
