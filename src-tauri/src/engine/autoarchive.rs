//! Archiviazione automatica, come in Outlook: ogni tanto (di solito ogni 14
//! giorni) i messaggi ricevuti da più di qualche mese lasciano le cartelle di
//! posta. Vanno nella cartella Archivio dell'account, oppure in un file di
//! dati sul computer (così liberano spazio sul server), oppure si eliminano.
//! Restano dove sono bozze e messaggi contrassegnati. Posta eliminata e Posta
//! indesiderata, se l'utente lo chiede, si svuotano dei messaggi che ci stanno
//! da più di qualche giorno.

use super::*;

const AUTO_ARCHIVE_KEY: &str = "archiviazione-automatica";
/// Segna il file di dati creato dall'archiviazione automatica.
pub const LOCAL_ARCHIVE_SOURCE: &str = "archiviazione-automatica";
/// Nome del file di dati sul computer.
const LOCAL_ARCHIVE_NAME: &str = "Archivio sul computer";

const ACTIONS: &[&str] = &["archivio", "computer", "elimina"];

fn role_label(role: &str) -> Option<&'static str> {
    Some(match role {
        "inbox" => "Posta in arrivo",
        "sent" => "Posta inviata",
        "drafts" => "Bozze",
        "deleted" => "Posta eliminata",
        "junk" => "Posta indesiderata",
        "archive" => "Archivio",
        _ => return None,
    })
}

/// La data di `months` mesi fa, a quest'ora (secondi).
fn months_ago(now: i64, months: i64) -> i64 {
    use chrono::{Months, TimeZone};
    let Some(t) = chrono::Local.timestamp_opt(now, 0).single() else { return now - months * 30 * 86_400 };
    t.checked_sub_months(Months::new(months.clamp(0, 1200) as u32)).map(|d| d.timestamp()).unwrap_or(now - months * 30 * 86_400)
}

impl Engine {
    pub fn auto_archive(&self) -> Result<AutoArchive> {
        Ok(self.db.meta(AUTO_ARCHIVE_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    /// Salva le impostazioni (l'ultima esecuzione resta quella registrata dal motore).
    pub fn set_auto_archive(&self, mut a: AutoArchive) -> Result<AutoArchive> {
        if !ACTIONS.contains(&a.action.as_str()) {
            return Err(Error::user("Scegli dove mettere i messaggi vecchi."));
        }
        if !(1..=120).contains(&a.older_months) {
            return Err(Error::user("Scegli dopo quanti mesi archiviare i messaggi."));
        }
        if !(1..=365).contains(&a.every_days) {
            return Err(Error::user("Scegli ogni quanti giorni eseguire l'archiviazione."));
        }
        if !(1..=3650).contains(&a.trash_days) {
            return Err(Error::user("Scegli dopo quanti giorni svuotare Posta eliminata."));
        }
        a.excluded.sort_unstable();
        a.excluded.dedup();
        let cur = self.auto_archive()?;
        a.last_run = cur.last_run;
        // Appena attivata non parte subito: il primo giro è fra un'ora, così
        // c'è tempo di ripensarci (o si usa "Archivia ora").
        if a.enabled && !cur.enabled {
            a.last_run = Some(now_secs() - a.every_days * 86_400 + 3_600);
        }
        self.db.set_meta(AUTO_ARCHIVE_KEY, &serde_json::to_string(&a)?)?;
        Ok(a)
    }

    /// Ogni ora controlla se è il momento di archiviare.
    pub(super) fn spawn_auto_archive(self: &Arc<Self>) {
        let eng = self.clone();
        tokio::spawn(async move {
            // Prima si lascia il tempo alla sincronizzazione di partire.
            tokio::time::sleep(Duration::from_secs(120)).await;
            loop {
                match eng.run_auto_archive(false).await {
                    Ok(_) => {}
                    Err(e) => log::warn!("archiviazione automatica: {e}"),
                }
                tokio::time::sleep(Duration::from_secs(3_600)).await;
            }
        });
    }

    /// Esegue l'archiviazione se è il momento (o subito, con `now`).
    pub async fn run_auto_archive(&self, now: bool) -> Result<AutoArchiveResult> {
        let s = self.auto_archive()?;
        let t = now_secs();
        if !now && (!s.enabled || s.last_run.is_some_and(|last| last + s.every_days * 86_400 > t)) {
            return Ok(AutoArchiveResult::default());
        }
        if self.archiving.swap(true, Ordering::SeqCst) {
            return Err(Error::user("L'archiviazione è già in corso."));
        }
        let res = self.archive_all(&s, t).await;
        self.archiving.store(false, Ordering::SeqCst);
        let res = res?;
        let mut saved = self.auto_archive()?;
        saved.last_run = Some(t);
        self.db.set_meta(AUTO_ARCHIVE_KEY, &serde_json::to_string(&saved)?)?;
        if res.archived + res.deleted > 0 {
            self.emit(Event::AutoArchive { archived: res.archived, deleted: res.deleted, action: s.action.clone() });
        }
        Ok(res)
    }

    async fn archive_all(&self, s: &AutoArchive, t: i64) -> Result<AutoArchiveResult> {
        let before = months_ago(t, s.older_months);
        let mut res = AutoArchiveResult::default();
        for acct in self.db.accounts()?.into_iter().filter(|a| a.kind != AccountKind::Local) {
            for folder in self.db.folders(&acct.id)? {
                let role = folder.role.as_deref();
                if !folder.selectable || matches!(role, Some("archive" | "drafts" | "deleted" | "junk")) || s.excluded.contains(&folder.id) {
                    continue;
                }
                let ids = self.db.archivable(folder.id, before)?;
                if ids.is_empty() {
                    continue;
                }
                match s.action.as_str() {
                    "elimina" => {
                        let rows = self.db.messages_by_ids(&ids)?;
                        self.remove_permanently(&acct, &folder, &rows)?;
                    }
                    "computer" => {
                        let dest = self.local_archive_folder(&acct, &folder)?;
                        self.move_messages(&ids, dest.id).await?;
                    }
                    _ => {
                        let dest = self.role_folder(&acct, "archive")?;
                        self.move_messages(&ids, dest.id).await?;
                    }
                }
                res.archived += ids.len();
            }
            if s.empty_trash {
                for role in ["deleted", "junk"] {
                    let Some(folder) = self.db.folder_by_role(&acct.id, role)? else { continue };
                    if s.excluded.contains(&folder.id) {
                        continue;
                    }
                    let ids = self.db.placed_before(folder.id, t - s.trash_days * 86_400)?;
                    if ids.is_empty() {
                        continue;
                    }
                    let rows = self.db.messages_by_ids(&ids)?;
                    self.remove_permanently(&acct, &folder, &rows)?;
                    res.deleted += ids.len();
                }
            }
        }
        Ok(res)
    }

    /// La cartella del file di dati sul computer per una cartella dell'account:
    /// "indirizzo/Posta in arrivo", "indirizzo/Clienti/Bassano"... Il file si
    /// crea al primo uso.
    fn local_archive_folder(&self, acct: &AccountConfig, folder: &FolderRow) -> Result<FolderRow> {
        let archive = match self.db.accounts()?.into_iter().find(|a| a.kind == AccountKind::Local && a.source_file.as_deref() == Some(LOCAL_ARCHIVE_SOURCE)) {
            Some(a) => a,
            None => self.create_local_account(LOCAL_ARCHIVE_NAME, Some(LOCAL_ARCHIVE_SOURCE.into()))?,
        };
        let delim = folder.delim.clone().unwrap_or_else(|| "/".into());
        let mut parts: Vec<String> = match folder.role.as_deref().and_then(role_label) {
            Some(label) => vec![label.to_string()],
            None => {
                let mut p: Vec<String> = folder.path.split(delim.as_str()).map(|s| s.trim().replace('/', "-")).filter(|s| !s.is_empty()).collect();
                if p.len() > 1 && p[0].eq_ignore_ascii_case("INBOX") {
                    p.remove(0);
                }
                p
            }
        };
        if parts.is_empty() {
            parts.push(folder.name.clone());
        }
        parts.insert(0, acct.email.replace('/', "-"));
        let mut path = String::new();
        let mut id = 0;
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                path.push('/');
            }
            path.push_str(part);
            id = self.db.upsert_folder(&archive.id, &path, part, None, Some("/"), true)?;
        }
        self.changed(&archive.id, vec![]);
        self.db.folder(id)
    }
}
