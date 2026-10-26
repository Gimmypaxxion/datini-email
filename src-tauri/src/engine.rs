//! Il motore: tiene insieme archivio, account e server. Ogni account ha un
//! lavoratore che sincronizza in sottofondo; un altro lavoratore spedisce la
//! posta in uscita. L'interfaccia riceve notifiche quando qualcosa cambia.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Notify;

use crate::autoconfig;
use crate::db::{Db, FolderRow, MsgRow, Op, OutboxRow, Target};
use crate::error::{Error, Result};
use crate::imap;
use crate::mime::{self, AttachmentData};
use crate::model::*;
use crate::pop3;
use crate::rules::{self, Candidate};
use crate::sanitize;
use crate::secrets::Secrets;
use crate::smtp;
use crate::utf7;

mod agenda;
mod conversations;
mod quick;
mod searchfolders;
mod tasks;
pub use agenda::InviteAction;

pub type Notifier = Arc<dyn Fn(Event) + Send + Sync>;

/// Secondi in cui un messaggio inviato si può ancora annullare.
pub const UNDO_SECS: i64 = 7;
const SYNC_EVERY: Duration = Duration::from_secs(120);

struct Worker {
    wake: Arc<Notify>,
    stop: Arc<AtomicBool>,
}

/// Chiave dell'archivio in cui stanno le firme.
const SIGNATURES_KEY: &str = "firme";
const READ_RECEIPTS_KEY: &str = "conferme-lettura";
/// Quante conferme già gestite ricordare (le più vecchie si dimenticano).
const MAX_RECEIPTS_HANDLED: usize = 5000;
/// Chiave dell'archivio in cui stanno le categorie colorate.
const CATEGORIES_KEY: &str = "categorie";
/// Chiave dell'archivio in cui stanno le regole.
const RULES_KEY: &str = "regole";
/// Messaggi mostrati al massimo in un avviso sul desktop.
const ALERT_MAX: usize = 5;
/// Immagini incollate da una pagina web: oltre questa dimensione restano un collegamento.
const MAX_PASTED_IMAGE: u64 = 5 * 1024 * 1024;

pub struct Engine {
    pub db: Db,
    secrets: Arc<dyn Secrets>,
    notify: Notifier,
    passwords: Mutex<HashMap<String, String>>,
    workers: Mutex<HashMap<String, Worker>>,
    status: Mutex<HashMap<String, AccountStatus>>,
    outbox_wake: Arc<Notify>,
    pub data_dir: PathBuf,
    pub undo_secs: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAccount {
    pub name: String,
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub kind: Option<AccountKind>,
    #[serde(default)]
    pub incoming: Option<ServerConfig>,
    #[serde(default)]
    pub outgoing: Option<ServerConfig>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDraft {
    pub id: i64,
    pub attachments: Vec<DraftAttachment>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn guess_mime(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" => "image/heic",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "htm" | "html" => "text/html",
        "xml" => "application/xml",
        "p7m" => "application/pkcs7-mime",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "eml" => "message/rfc822",
        "ics" => "text/calendar",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
    .to_string()
}

/// Risultato delle regole su un gruppo di messaggi.
#[derive(Default)]
struct RulesOutcome {
    /// Messaggi su cui almeno una regola ha agito.
    matched: usize,
    /// Spostati in un'altra cartella o eliminati.
    gone: HashSet<i64>,
    /// Eliminati: niente risposta automatica.
    deleted: HashSet<i64>,
    /// Segnati come letti.
    read: HashSet<i64>,
    /// Da segnalare con l'avviso anche fuori dalla posta in arrivo.
    alert: HashSet<i64>,
}

fn is_server_backed(m: &MsgRow) -> bool {
    m.uid.is_some() || (m.message_id.is_some() && matches!(m.pending.as_deref(), Some("move") | Some("done")))
}

impl Engine {
    pub fn new(db: Db, secrets: Arc<dyn Secrets>, data_dir: PathBuf, notify: Notifier) -> Arc<Engine> {
        Arc::new(Engine {
            db,
            secrets,
            notify,
            passwords: Mutex::new(HashMap::new()),
            workers: Mutex::new(HashMap::new()),
            status: Mutex::new(HashMap::new()),
            outbox_wake: Arc::new(Notify::new()),
            data_dir,
            undo_secs: UNDO_SECS,
        })
    }

    fn emit(&self, e: Event) {
        (self.notify)(e);
    }

    fn changed(&self, account_id: &str, folders: Vec<i64>) {
        self.emit(Event::Changed { account_id: account_id.to_string(), folders });
    }

    /// Segnala all'interfaccia che le cartelle di un account sono cambiate.
    pub fn notify_changed(&self, account_id: &str) {
        self.changed(account_id, Vec::new());
    }

    pub fn import_progress(&self, name: &str, done: u32, total: u32) {
        self.emit(Event::Import { name: name.to_string(), done, total });
    }

    /// Avvia i lavoratori. Va chiamata dentro il runtime tokio.
    pub fn start(self: &Arc<Self>) {
        if let Ok(accounts) = self.db.accounts() {
            for a in accounts {
                self.recover_drafts(&a);
                if a.kind != AccountKind::Local {
                    self.spawn_worker(&a.id);
                }
            }
        }
        // Messaggi rimasti "in invio" per una chiusura improvvisa: si riprovano.
        let _ = self.db.conn().execute("UPDATE outbox SET state = 'attesa' WHERE state = 'invio'", []);
        self.spawn_outbox();
    }

    /// Bozze salvate solo sul computer (programma chiuso mentre si scriveva):
    /// vanno ricopiate sul server.
    fn recover_drafts(&self, a: &AccountConfig) {
        if a.kind != AccountKind::Imap {
            return;
        }
        let Ok(Some(drafts)) = self.db.folder_by_role(&a.id, "drafts") else { return };
        let rows: Vec<(i64, Option<String>, Option<String>)> = {
            let c = self.db.conn();
            let mut st = match c.prepare("SELECT id, draft_key, message_id FROM messages WHERE folder_id = ?1 AND uid IS NULL AND pending IN ('draft', 'sending')") {
                Ok(s) => s,
                Err(_) => return,
            };
            st.query_map([drafts.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map(|it| it.flatten().collect()).unwrap_or_default()
        };
        let pending_ops = self.db.ops(&a.id).unwrap_or_default();
        for (id, key, mid) in rows {
            let queued = pending_ops.iter().any(|(_, op, _)| matches!(op, Op::Append { local_id, .. } if *local_id == id));
            if queued {
                continue;
            }
            let _ = self.db.set_pending(&[id], Some("draft"));
            let _ = self.db.push_op(&a.id, &Op::Append { folder: drafts.path.clone(), local_id: id, flags: "(\\Draft \\Seen)".into() });
            if let Some(key) = key {
                let _ = self.db.push_op(&a.id, &Op::DeleteDraftVersions { folder: drafts.path.clone(), draft_key: key, keep_message_id: mid });
            }
        }
    }

    // ------------------------------------------------------------------
    // Stato e password
    // ------------------------------------------------------------------

    fn set_status(&self, id: &str, state: &str, message: Option<String>) {
        let mut map = lock(&self.status);
        let st = map.entry(id.to_string()).or_default();
        let mut next = st.clone();
        next.state = state.into();
        next.message = message;
        if state == "pronto" {
            next.last_sync = Some(now_secs() * 1000);
        }
        if *st != next {
            *st = next.clone();
            drop(map);
            self.emit(Event::Status { account_id: id.into(), status: next });
        }
    }

    pub fn status(&self, id: &str) -> AccountStatus {
        lock(&self.status).get(id).cloned().unwrap_or_default()
    }

    fn password(&self, id: &str) -> Result<String> {
        if let Some(p) = lock(&self.passwords).get(id) {
            return Ok(p.clone());
        }
        match self.secrets.get(id)? {
            Some(p) => {
                lock(&self.passwords).insert(id.into(), p.clone());
                Ok(p)
            }
            None => Err(Error::user("Manca la password di questo account: inseriscila in Impostazioni, Account.")),
        }
    }

    // ------------------------------------------------------------------
    // Lavoratori
    // ------------------------------------------------------------------

    fn spawn_worker(self: &Arc<Self>, id: &str) {
        let wake = Arc::new(Notify::new());
        let stop = Arc::new(AtomicBool::new(false));
        lock(&self.workers).insert(id.to_string(), Worker { wake: wake.clone(), stop: stop.clone() });
        let eng = self.clone();
        let id = id.to_string();
        tokio::spawn(async move {
            let mut cycle: u64 = 0;
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let Ok(acct) = eng.db.account(&id) else { break };
                eng.set_status(&id, "sincronizzazione", None);
                let res = match acct.kind {
                    AccountKind::Imap => eng.imap_cycle(&acct, cycle % 10 == 0).await,
                    AccountKind::Pop3 => eng.pop_cycle(&acct).await,
                    AccountKind::Local => Ok(()),
                };
                let delay = match res {
                    Ok(()) => {
                        eng.set_status(&id, "pronto", None);
                        eng.process_arrivals(&acct).await;
                        SYNC_EVERY
                    }
                    Err(e) if e.is_network() => {
                        eng.set_status(&id, "offline", Some(e.to_string()));
                        Duration::from_secs(45)
                    }
                    Err(e) => {
                        log::warn!("sincronizzazione {id}: {e}");
                        eng.set_status(&id, "errore", Some(e.to_string()));
                        Duration::from_secs(300)
                    }
                };
                cycle += 1;
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                tokio::select! {
                    _ = wake.notified() => {}
                    _ = tokio::time::sleep(delay) => {}
                }
            }
        });
    }

    fn wake(&self, id: &str) {
        if let Some(w) = lock(&self.workers).get(id) {
            w.wake.notify_one();
        }
    }

    /// "Invia e ricevi": sincronizza subito e riprova la posta in uscita.
    pub fn sync_now(&self, id: Option<&str>) {
        let workers = lock(&self.workers);
        for (wid, w) in workers.iter() {
            if id.is_none() || id == Some(wid.as_str()) {
                w.wake.notify_one();
            }
        }
        drop(workers);
        let _ = self.db.conn().execute("UPDATE outbox SET send_after = ?1 WHERE state = 'offline'", [now_secs()]);
        self.outbox_wake.notify_one();
    }

    async fn imap_cycle(&self, acct: &AccountConfig, thorough: bool) -> Result<()> {
        let cfg = acct.incoming.as_ref().ok_or_else(|| Error::user("Mancano i dati del server IMAP."))?;
        let pw = self.password(&acct.id)?;
        let mut s = imap::login(cfg, &pw).await?;
        let caps = imap::caps(&mut s).await?;
        self.run_ops(&mut s, acct, &caps).await?;

        let mut listed = imap::list(&mut s).await?;
        if imap::ensure_special(&mut s, &listed).await? {
            listed = imap::list(&mut s).await?;
        }
        let before: Vec<String> = self.db.folders(&acct.id)?.into_iter().map(|f| f.path).collect();
        imap::store_folders(&self.db, &acct.id, &listed)?;
        let mut folders = self.db.folders(&acct.id)?;
        if folders.iter().map(|f| f.path.clone()).collect::<Vec<_>>() != before {
            self.changed(&acct.id, vec![]);
        }
        let rank = |f: &FolderRow| match f.role.as_deref() {
            Some("inbox") => 0,
            Some("sent") => 1,
            Some("drafts") => 2,
            Some("deleted") => 3,
            Some("archive") => 4,
            Some("junk") => 5,
            _ => 6,
        };
        folders.sort_by_key(|f| (rank(f), f.path.clone()));
        for f in folders.iter().filter(|f| f.selectable) {
            match imap::sync_folder(&mut s, &self.db, acct, f, thorough).await {
                Ok(r) => {
                    if r.changed {
                        self.changed(&acct.id, vec![f.id]);
                    }
                }
                Err(e) if e.is_network() => return Err(e),
                Err(e) => log::warn!("cartella {}: {e}", f.path),
            }
            if self.db.pending_ops_count(&acct.id)? > 0 {
                self.run_ops(&mut s, acct, &caps).await?;
            }
        }
        let _ = s.logout().await;
        Ok(())
    }

    async fn run_ops(&self, s: &mut imap::Session, acct: &AccountConfig, caps: &imap::Caps) -> Result<()> {
        let ops = self.db.ops(&acct.id)?;
        if ops.is_empty() {
            return Ok(());
        }
        for (id, op, attempts) in ops {
            match imap::exec_op(s, &self.db, caps, &op).await {
                Ok(()) => self.db.delete_op(id)?,
                Err(e) if e.is_network() => return Err(e),
                Err(e) => {
                    log::warn!("operazione {op:?}: {e}");
                    if attempts >= 2 {
                        self.db.delete_op(id)?;
                        if let Op::Move { local_ids, .. } = &op {
                            self.db.set_pending(local_ids, Some("done"))?;
                        }
                        if let Op::Flags { local_ids, .. } = &op {
                            self.db.clear_dirty(local_ids)?;
                        }
                    } else {
                        self.db.op_failed(id, &e.to_string())?;
                    }
                }
            }
        }
        self.changed(&acct.id, vec![]);
        Ok(())
    }

    async fn pop_cycle(&self, acct: &AccountConfig) -> Result<()> {
        let pw = self.password(&acct.id)?;
        let id = acct.id.clone();
        let inbox = self.db.folder_by_role(&acct.id, "inbox")?.map(|f| f.id);
        let progress = || self.changed(&id, inbox.into_iter().collect());
        if pop3::sync(&self.db, acct, &pw, &progress).await? {
            progress();
        }
        Ok(())
    }

    fn spawn_outbox(self: &Arc<Self>) {
        let eng = self.clone();
        let wake = self.outbox_wake.clone();
        tokio::spawn(async move {
            loop {
                let due = eng.db.outbox_due(now_secs()).unwrap_or_default();
                for item in due {
                    eng.send_item(item).await;
                }
                tokio::select! {
                    _ = wake.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(1000)) => {}
                }
            }
        });
    }

    async fn send_item(&self, item: OutboxRow) {
        if !self.db.outbox_claim(item.id).unwrap_or(false) {
            return;
        }
        let emit = |state: &str, error: Option<String>| {
            self.emit(Event::Outbox { account_id: item.account_id.clone(), outbox_id: item.id, state: state.into(), error });
        };
        let result = async {
            let acct = self.db.account(&item.account_id)?;
            let cfg = acct.outgoing.clone().ok_or_else(|| Error::user("Mancano i dati del server di posta in uscita."))?;
            let pw = self.password(&acct.id)?;
            emit("invio", None);
            smtp::send(&cfg, &pw, &item.envelope, &item.raw, item.draft.delivery_receipt).await?;
            Ok::<AccountConfig, Error>(acct)
        }
        .await;
        match result {
            Ok(acct) => {
                let _ = self.db.outbox_delete(item.id);
                if let Err(e) = self.after_sent(&acct, &item) {
                    log::warn!("dopo l'invio: {e}");
                }
                emit("inviato", None);
            }
            Err(e) if e.is_network() => {
                let _ = self.db.outbox_set_state(item.id, "offline", Some(&e.to_string()), Some(now_secs() + 60));
                emit("offline", Some(e.to_string()));
            }
            Err(e) => {
                let _ = self.db.outbox_set_state(item.id, "errore", Some(&e.to_string()), None);
                emit("errore", Some(e.to_string()));
            }
        }
    }

    fn after_sent(&self, acct: &AccountConfig, item: &OutboxRow) -> Result<()> {
        // Le conferme di lettura non finiscono in Posta inviata (come in Outlook).
        if item.draft.mdn {
            return Ok(());
        }
        let sent = self.role_folder(acct, "sent")?;
        let imap = acct.kind == AccountKind::Imap;
        let mut nm = mime::parse(&item.copy).map(|p| p.to_new_message(now_secs(), true, item.copy.len() as i64)).unwrap_or_default();
        nm.unread = false;
        nm.focused = true;
        nm.raw = Some(item.copy.clone());
        let append = imap && !acct.server_saves_sent;
        nm.pending = if append { Some("append".into()) } else if imap { Some("done".into()) } else { None };
        if let Some(id) = self.db.insert_message(&acct.id, sent.id, &nm)? {
            if append {
                self.db.push_op(&acct.id, &Op::Append { folder: sent.path.clone(), local_id: id, flags: "(\\Seen)".into() })?;
            }
        }
        // La bozza da cui è nato il messaggio non serve più.
        self.remove_draft_rows(acct, &item.draft.key, true)?;
        if let Some(orig) = item.draft.reply_to_message {
            let _ = self.set_flags(&[orig], None, None, Some(true));
        }
        self.wake(&acct.id);
        self.changed(&acct.id, vec![sent.id]);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Account
    // ------------------------------------------------------------------

    pub fn accounts(&self) -> Result<Vec<AccountInfo>> {
        Ok(self.db.accounts()?.into_iter().map(|c| AccountInfo { status: self.status(&c.id), config: c }).collect())
    }

    async fn test_incoming(kind: AccountKind, cfg: &ServerConfig, pw: &str) -> Result<()> {
        match kind {
            AccountKind::Imap => {
                let mut s = imap::login(cfg, pw).await?;
                let _ = s.logout().await;
                Ok(())
            }
            AccountKind::Pop3 => pop3::Pop3::connect(cfg, pw).await?.quit().await,
            AccountKind::Local => Ok(()),
        }
    }

    /// Trova i server, verifica la password e aggiunge l'account.
    pub async fn add_account(self: &Arc<Self>, input: NewAccount) -> Result<AccountInfo> {
        let email = input.email.trim().to_string();
        let Some((_, domain)) = email.split_once('@').filter(|(l, d)| !l.is_empty() && d.contains('.')) else {
            return Err(Error::user("Scrivi un indirizzo email completo, per esempio nome@azienda.it."));
        };
        let domain = domain.to_ascii_lowercase();
        if let Some(msg) = autoconfig::unsupported(&domain) {
            return Err(Error::user(msg));
        }
        if input.password.is_empty() {
            return Err(Error::user("Scrivi la password della casella."));
        }
        if self.db.accounts()?.iter().any(|a| a.email.eq_ignore_ascii_case(&email) && a.kind != AccountKind::Local) {
            return Err(Error::user("Questo indirizzo è già in Better-Look."));
        }

        let manual = input.incoming.is_some() && input.outgoing.is_some();
        let candidates: Vec<autoconfig::Candidate> = if manual {
            vec![autoconfig::Candidate {
                kind: input.kind.unwrap_or(AccountKind::Imap),
                incoming: input.incoming.clone().unwrap(),
                outgoing: input.outgoing.clone().unwrap(),
                source: "manuale",
            }]
        } else {
            let want = input.kind.unwrap_or(AccountKind::Imap);
            let mut c = autoconfig::candidates(&email).await;
            c.retain(|x| x.kind == want);
            c
        };

        let mut last_err: Option<Error> = None;
        let mut found: Option<autoconfig::Candidate> = None;
        let mut incoming_ok: Option<ServerConfig> = None;
        for cand in candidates {
            if incoming_ok.as_ref() != Some(&cand.incoming) {
                match Self::test_incoming(cand.kind, &cand.incoming, &input.password).await {
                    Ok(()) => incoming_ok = Some(cand.incoming.clone()),
                    Err(Error::Auth) => return Err(Error::Auth),
                    Err(e) => {
                        if !matches!(last_err, Some(Error::Certificate { .. })) || matches!(e, Error::Certificate { .. }) {
                            last_err = Some(e);
                        }
                        continue;
                    }
                }
            }
            match smtp::test(&cand.outgoing, &input.password).await {
                Ok(()) => {
                    found = Some(cand);
                    break;
                }
                Err(Error::Auth) => {
                    last_err = Some(Error::user(format!(
                        "La posta in arrivo funziona, ma il server di posta in uscita {} ha rifiutato nome utente o password.",
                        cand.outgoing.host
                    )))
                }
                Err(e) => last_err = Some(e),
            }
        }
        let Some(cand) = found else {
            return Err(match last_err {
                Some(e @ (Error::Certificate { .. } | Error::User(_))) if manual || matches!(e, Error::User(_)) => e,
                Some(e) if manual => e,
                Some(Error::Certificate { host, detail }) => Error::Manual(format!(
                    "Il server {host} ha un certificato non valido ({detail}). Controlla le impostazioni del server indicate dal tuo provider e inseriscile qui."
                )),
                _ => Error::Manual(format!(
                    "Non ho trovato da solo le impostazioni dei server di {domain}. Inseriscile qui: le trovi nelle istruzioni del tuo provider o nel pannello del tuo hosting."
                )),
            });
        };

        let id = mime::random_id()[..12].to_string();
        let gmail = cand.incoming.host.ends_with("gmail.com");
        let cfg = AccountConfig {
            id: id.clone(),
            name: if input.name.trim().is_empty() { email.split('@').next().unwrap_or("").to_string() } else { input.name.trim().to_string() },
            email: email.clone(),
            kind: cand.kind,
            incoming: Some(cand.incoming),
            outgoing: Some(cand.outgoing),
            sync: SyncOptions::default(),
            server_saves_sent: gmail,
            source_file: None,
            signature_new: None,
            signature_reply: None,
            auto_reply: None,
        };
        self.secrets.set(&id, &input.password)?;
        lock(&self.passwords).insert(id.clone(), input.password.clone());
        self.db.save_account(&cfg)?;
        if cfg.kind == AccountKind::Pop3 {
            self.create_local_folders(&id)?;
        }
        self.spawn_worker(&id);
        self.emit(Event::Accounts);
        Ok(AccountInfo { status: self.status(&id), config: cfg })
    }

    fn create_local_folders(&self, account_id: &str) -> Result<()> {
        for (path, name, role) in [
            ("INBOX", "Posta in arrivo", "inbox"),
            ("Drafts", "Bozze", "drafts"),
            ("Sent", "Posta inviata", "sent"),
            ("Trash", "Posta eliminata", "deleted"),
            ("Junk", "Posta indesiderata", "junk"),
            ("Archive", "Archivio", "archive"),
        ] {
            self.db.upsert_folder(account_id, path, name, Some(role), Some("/"), true)?;
        }
        Ok(())
    }

    /// File di dati sul computer (per esempio un PST importato).
    pub fn create_local_account(&self, name: &str, source: Option<String>) -> Result<AccountConfig> {
        let id = mime::random_id()[..12].to_string();
        let cfg = AccountConfig {
            id: id.clone(),
            name: name.into(),
            email: name.into(),
            kind: AccountKind::Local,
            incoming: None,
            outgoing: None,
            sync: SyncOptions::default(),
            server_saves_sent: false,
            source_file: source,
            signature_new: None,
            signature_reply: None,
            auto_reply: None,
        };
        self.db.save_account(&cfg)?;
        self.emit(Event::Accounts);
        Ok(cfg)
    }

    /// Nome, opzioni di sincronizzazione e server di un account.
    pub fn update_account(&self, cfg: AccountConfig) -> Result<()> {
        let mut cur = self.db.account(&cfg.id)?;
        let keep_changed = cur.sync.keep != cfg.sync.keep || cur.sync.attachments != cfg.sync.attachments;
        cur.name = cfg.name;
        cur.sync = cfg.sync;
        if cfg.incoming.is_some() {
            cur.incoming = cfg.incoming;
        }
        if cfg.outgoing.is_some() {
            cur.outgoing = cfg.outgoing;
        }
        cur.signature_new = cfg.signature_new;
        cur.signature_reply = cfg.signature_reply;
        self.db.save_account(&cur)?;
        self.emit(Event::Accounts);
        if keep_changed {
            self.wake(&cur.id);
        }
        Ok(())
    }

    pub fn set_password(&self, id: &str, password: &str) -> Result<()> {
        self.secrets.set(id, password)?;
        lock(&self.passwords).insert(id.into(), password.into());
        self.wake(id);
        Ok(())
    }

    pub fn remove_account(&self, id: &str) -> Result<()> {
        if let Some(w) = lock(&self.workers).remove(id) {
            w.stop.store(true, Ordering::Relaxed);
            w.wake.notify_one();
        }
        self.db.delete_account(id)?;
        let _ = self.secrets.delete(id);
        lock(&self.passwords).remove(id);
        lock(&self.status).remove(id);
        self.emit(Event::Accounts);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Cartelle e messaggi
    // ------------------------------------------------------------------

    pub fn folders(&self) -> Result<Vec<FolderInfo>> {
        self.db.folder_infos()
    }

    pub fn messages(&self, folder_id: i64, tab: Option<&str>, offset: i64, limit: i64) -> Result<Vec<MessageSummary>> {
        let f = self.db.folder(folder_id)?;
        let focused = if f.role.as_deref() == Some("inbox") {
            match tab {
                Some("prio") => Some(true),
                Some("altre") => Some(false),
                _ => None,
            }
        } else {
            None
        };
        Ok(self.db.list_messages(folder_id, focused, limit, offset)?.iter().map(|m| m.summary()).collect())
    }

    pub fn search(&self, q: &str, account_id: Option<&str>) -> Result<Vec<MessageSummary>> {
        Ok(self
            .db
            .search(q, account_id, 300)?
            .into_iter()
            .map(|(m, folder)| {
                let mut s = m.summary();
                s.folder_name = Some(folder);
                s
            })
            .collect())
    }

    /// Suggerimenti per i destinatari: prima la rubrica, poi chi ha scritto
    /// o ricevuto posta.
    pub fn contacts(&self, prefix: &str) -> Result<Vec<Person>> {
        let mut out = self.db.contact_people(prefix, 8)?;
        for p in self.db.contacts(prefix, 8)? {
            if !out.iter().any(|o| o.email.eq_ignore_ascii_case(&p.email)) {
                out.push(p);
            }
        }
        out.truncate(8);
        Ok(out)
    }

    // ------------------------------------------------------------------
    // Rubrica
    // ------------------------------------------------------------------

    pub fn address_book(&self) -> Result<Vec<Contact>> {
        self.db.all_contacts()
    }

    /// Salva un contatto (nuovo se non ha identificativo).
    pub fn save_contact(&self, mut c: Contact) -> Result<Contact> {
        c.emails = c.emails.iter().map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect();
        c.phones.retain(|p| !p.number.trim().is_empty());
        if let Some(bad) = c.emails.iter().find(|e| !e.contains('@') || e.contains(' ') || e.starts_with('@') || e.ends_with('@')) {
            return Err(Error::user(format!("L'indirizzo \"{bad}\" non è valido.")));
        }
        if c.is_empty() {
            return Err(Error::user("Scrivi almeno un nome o un indirizzo email."));
        }
        if c.id.is_empty() {
            c.id = mime::random_id()[..12].to_string();
        }
        self.db.save_contact(&c)?;
        self.emit(Event::Contacts);
        Ok(c)
    }

    pub fn delete_contacts(&self, ids: &[String]) -> Result<()> {
        self.db.delete_contacts(ids)?;
        self.emit(Event::Contacts);
        Ok(())
    }

    /// Importa un file vCard, CSV o LDIF. Chi c'è già (stesso indirizzo, o
    /// stesso nome se manca l'indirizzo) viene completato, non duplicato.
    pub fn import_contacts(&self, path: &Path) -> Result<ContactImport> {
        let bytes = std::fs::read(path).map_err(|e| Error::user(format!("Impossibile leggere il file: {e}")))?;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let list = crate::contacts::parse_file(&bytes, &name);
        if list.is_empty() {
            return Err(Error::user("Nel file non ho trovato contatti. Usa un file vCard (.vcf), CSV o LDIF esportato dal programma di posta."));
        }
        let mut book = self.db.all_contacts()?;
        let mut res = ContactImport::default();
        for c in list {
            let found = book.iter().position(|b| c.emails.iter().any(|e| b.emails.iter().any(|x| x.eq_ignore_ascii_case(e)))).or_else(|| {
                if c.emails.is_empty() {
                    book.iter().position(|b| b.name().eq_ignore_ascii_case(&c.name()))
                } else {
                    None
                }
            });
            match found {
                Some(i) => {
                    if crate::contacts::merge(&mut book[i], &c) {
                        self.db.save_contact(&book[i])?;
                        res.updated += 1;
                    } else {
                        res.unchanged += 1;
                    }
                }
                None => {
                    let mut c = c;
                    c.id = mime::random_id()[..12].to_string();
                    self.db.save_contact(&c)?;
                    book.push(c);
                    res.added += 1;
                }
            }
        }
        self.emit(Event::Contacts);
        Ok(res)
    }

    /// Esporta in vCard tutti i contatti, o solo quelli indicati.
    pub fn export_contacts(&self, path: &Path, ids: Option<&[String]>) -> Result<usize> {
        let list: Vec<Contact> = self.db.all_contacts()?.into_iter().filter(|c| ids.map_or(true, |ids| ids.contains(&c.id))).collect();
        std::fs::write(path, crate::contacts::to_vcard(&list)).map_err(|e| Error::user(format!("Impossibile salvare il file: {e}")))?;
        Ok(list.len())
    }

    /// Testo completo di un messaggio, scaricandolo se serve.
    pub async fn raw(&self, m: &MsgRow) -> Result<Vec<u8>> {
        if let Some(raw) = self.db.body(m.id)? {
            return Ok(raw);
        }
        let acct = self.db.account(&m.account_id)?;
        let (AccountKind::Imap, Some(uid)) = (acct.kind, m.uid) else {
            return Err(Error::user("Il messaggio completo non è disponibile sul computer."));
        };
        let folder = self.db.folder(m.folder_id)?;
        let cfg = acct.incoming.clone().ok_or(Error::NotFound)?;
        let pw = self.password(&acct.id)?;
        let mut s = imap::login(&cfg, &pw).await.map_err(|e| {
            if e.is_network() {
                Error::user("Questo messaggio non è ancora stato scaricato sul computer. Sarà leggibile quando torna la connessione.")
            } else {
                e
            }
        })?;
        let raw = imap::fetch_one(&mut s, &cfg.host, &folder.path, uid).await?;
        let _ = s.logout().await;
        let raw = raw.ok_or_else(|| Error::user("Il messaggio non è più sul server."))?;
        imap::store_body(&self.db, m.id, &raw)?;
        self.changed(&m.account_id, vec![m.folder_id]);
        Ok(raw)
    }

    pub async fn detail(&self, id: i64, allow_remote: bool) -> Result<MessageDetail> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        let row = self.db.message(id)?;
        let parsed = mime::parse(&raw).ok_or_else(|| Error::user("Il messaggio non è leggibile."))?;
        let (html, remote) = match parsed.html() {
            Some(h) => {
                let c = sanitize::clean_email_html(&h, &parsed.inline_images(), allow_remote);
                (Some(c.html), c.remote)
            }
            None => (None, 0),
        };
        let rc = parsed.recipients();
        Ok(MessageDetail {
            summary: row.summary(),
            cc: rc.cc,
            bcc: rc.bcc,
            reply_to: rc.reply_to,
            message_id: parsed.msg.message_id().map(|s| s.to_string()),
            references: parsed.references(),
            html,
            text: parsed.text(),
            attachments: parsed.attachments(),
            remote_images: remote,
            remote_allowed: allow_remote,
            invite: self.invite_info(&parsed),
            receipt_to: self.receipt_request(&row, &parsed).unwrap_or(None),
        })
    }

    // ------------------------------------------------------------------
    // Conferme di lettura
    // ------------------------------------------------------------------

    fn receipt_key(row: &MsgRow) -> String {
        row.message_id.clone().filter(|m| !m.is_empty()).unwrap_or_else(|| format!("{}:{}", row.account_id, row.id))
    }

    fn receipts_handled(&self) -> Result<Vec<String>> {
        Ok(self.db.meta(READ_RECEIPTS_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    fn mark_receipt_handled(&self, row: &MsgRow) -> Result<()> {
        let key = Self::receipt_key(row);
        let mut all = self.receipts_handled()?;
        if !all.contains(&key) {
            all.push(key);
            let skip = all.len().saturating_sub(MAX_RECEIPTS_HANDLED);
            all.drain(..skip);
            self.db.set_meta(READ_RECEIPTS_KEY, &serde_json::to_string(&all)?)?;
        }
        Ok(())
    }

    /// Chi aspetta la conferma di lettura di questo messaggio, se va ancora chiesto
    /// all'utente: non per i propri messaggi, le bozze, la posta inviata o indesiderata,
    /// né per quelli a cui si è già risposto (con l'invio o con un no).
    fn receipt_request(&self, row: &MsgRow, p: &mime::Parsed) -> Result<Option<Person>> {
        let Some(to) = p.receipt_to() else { return Ok(None) };
        let acct = self.db.account(&row.account_id)?;
        if acct.kind == AccountKind::Local || acct.outgoing.is_none() {
            return Ok(None);
        }
        if to.email.eq_ignore_ascii_case(&acct.email) || row.from.email.eq_ignore_ascii_case(&acct.email) || row.draft {
            return Ok(None);
        }
        let folder = self.db.folder(row.folder_id)?;
        if matches!(folder.role.as_deref(), Some("drafts" | "sent" | "junk" | "outbox")) {
            return Ok(None);
        }
        if self.receipts_handled()?.contains(&Self::receipt_key(row)) {
            return Ok(None);
        }
        Ok(Some(to))
    }

    /// Invia la conferma di lettura chiesta dal mittente (una volta sola).
    pub async fn send_read_receipt(&self, id: i64, automatic: bool) -> Result<bool> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        let p = mime::parse(&raw).ok_or_else(|| Error::user("Il messaggio non è leggibile."))?;
        let Some(to) = self.receipt_request(&row, &p)? else { return Ok(false) };
        let acct = self.db.account(&row.account_id)?;
        let subject = p.subject();
        let d = Draft {
            key: mime::random_id(),
            account_id: acct.id.clone(),
            to: vec![to],
            subject: if subject.is_empty() { "Letto".into() } else { format!("Letto: {subject}") },
            in_reply_to: p.message_id(),
            mdn: true,
            ..Default::default()
        };
        let orig = mime::ReadOriginal { message_id: p.message_id(), subject, sent: row.date };
        let built = mime::build_mdn(&acct, &d, &orig, automatic)?;
        self.db.outbox_add(&acct.id, &built.wire, &built.copy, &built.envelope, &d, now_secs())?;
        self.mark_receipt_handled(&row)?;
        self.outbox_wake.notify_one();
        Ok(true)
    }

    /// L'utente non vuole inviare la conferma: non la si chiede più.
    pub fn skip_read_receipt(&self, id: i64) -> Result<()> {
        let row = self.db.message(id)?;
        self.mark_receipt_handled(&row)
    }

    /// Testo di un messaggio da citare in una risposta o in un inoltro.
    pub async fn quote_html(&self, id: i64) -> Result<String> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        let parsed = mime::parse(&raw).ok_or_else(|| Error::user("Il messaggio non è leggibile."))?;
        Ok(parsed.quote_fragment())
    }

    // ------------------------------------------------------------------
    // Firme
    // ------------------------------------------------------------------

    pub fn signatures(&self) -> Result<Vec<Signature>> {
        Ok(self.db.meta(SIGNATURES_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    /// Crea o aggiorna una firma. Il contenuto viene ripulito come ogni HTML.
    pub fn save_signature(&self, mut sig: Signature) -> Result<Signature> {
        let name = sig.name.trim();
        if name.is_empty() {
            return Err(Error::user("Dai un nome alla firma."));
        }
        sig.name = name.chars().take(80).collect();
        sig.html = sanitize::clean_fragment(&sig.html, &[]);
        if sig.html.len() > 2_000_000 {
            return Err(Error::user("La firma è troppo grande: usa immagini più piccole."));
        }
        let mut all = self.signatures()?;
        if sig.id.is_empty() {
            sig.id = mime::random_id()[..12].to_string();
            all.push(sig.clone());
        } else if let Some(s) = all.iter_mut().find(|s| s.id == sig.id) {
            *s = sig.clone();
        } else {
            all.push(sig.clone());
        }
        self.db.set_meta(SIGNATURES_KEY, &serde_json::to_string(&all)?)?;
        self.emit(Event::Signatures);
        Ok(sig)
    }

    /// Elimina una firma e la toglie dagli account che la usavano.
    pub fn delete_signature(&self, id: &str) -> Result<()> {
        let all: Vec<Signature> = self.signatures()?.into_iter().filter(|s| s.id != id).collect();
        self.db.set_meta(SIGNATURES_KEY, &serde_json::to_string(&all)?)?;
        for mut a in self.db.accounts()? {
            let mut changed = false;
            if a.signature_new.as_deref() == Some(id) {
                a.signature_new = None;
                changed = true;
            }
            if a.signature_reply.as_deref() == Some(id) {
                a.signature_reply = None;
                changed = true;
            }
            if changed {
                self.db.save_account(&a)?;
            }
        }
        self.emit(Event::Signatures);
        self.emit(Event::Accounts);
        Ok(())
    }

    /// Scarica un'immagine incollata da una pagina web, per incorporarla nel
    /// messaggio (come fa Outlook). La richiesta parte solo da un incolla
    /// dell'utente e va direttamente al sito dell'immagine.
    pub async fn fetch_image(&self, url: &str) -> Result<Option<String>> {
        use base64::Engine as _;
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Ok(None);
        }
        crate::net::install_crypto();
        let client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().map_err(|e| Error::user(e.to_string()))?;
        let Ok(r) = client.get(url).send().await else { return Ok(None) };
        if !r.status().is_success() {
            return Ok(None);
        }
        let mime = r
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.split(';').next().unwrap_or("").trim().to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(mime.as_str(), "image/png" | "image/jpeg" | "image/gif" | "image/webp" | "image/bmp") {
            return Ok(None);
        }
        if r.content_length().unwrap_or(0) > MAX_PASTED_IMAGE {
            return Ok(None);
        }
        let Ok(bytes) = r.bytes().await else { return Ok(None) };
        if bytes.len() as u64 > MAX_PASTED_IMAGE {
            return Ok(None);
        }
        Ok(Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes))))
    }

    pub async fn attachment(&self, id: i64, index: u32) -> Result<(String, String, Vec<u8>)> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        let parsed = mime::parse(&raw).ok_or(Error::NotFound)?;
        parsed.attachment(index).ok_or(Error::NotFound)
    }

    fn group_by_folder(&self, ids: &[i64]) -> Result<Vec<(FolderRow, AccountConfig, Vec<MsgRow>)>> {
        let mut groups: Vec<(FolderRow, AccountConfig, Vec<MsgRow>)> = Vec::new();
        for m in self.db.messages_by_ids(ids)? {
            if let Some(g) = groups.iter_mut().find(|g| g.0.id == m.folder_id) {
                g.2.push(m);
            } else {
                let f = self.db.folder(m.folder_id)?;
                let a = self.db.account(&m.account_id)?;
                groups.push((f, a, vec![m]));
            }
        }
        Ok(groups)
    }

    pub fn set_flags(&self, ids: &[i64], seen: Option<bool>, flagged: Option<bool>, answered: Option<bool>) -> Result<()> {
        for (folder, acct, rows) in self.group_by_folder(ids)? {
            let imap = acct.kind == AccountKind::Imap;
            let server: Vec<&MsgRow> = rows.iter().filter(|m| imap && is_server_backed(m)).collect();
            let all_ids: Vec<i64> = rows.iter().map(|m| m.id).collect();
            self.db.set_local_flags(&all_ids, seen, flagged, answered, imap)?;
            if !server.is_empty() {
                self.db.push_op(
                    &acct.id,
                    &Op::Flags {
                        folder: folder.path.clone(),
                        targets: server.iter().map(|m| m.target()).collect(),
                        seen,
                        flagged,
                        answered,
                        local_ids: server.iter().map(|m| m.id).collect(),
                    },
                )?;
                self.wake(&acct.id);
            } else {
                self.db.clear_dirty(&all_ids)?;
            }
            self.changed(&acct.id, vec![folder.id]);
        }
        Ok(())
    }

    /// Cartella con un certo ruolo; per IMAP la crea sul server se manca.
    pub fn role_folder(&self, acct: &AccountConfig, role: &str) -> Result<FolderRow> {
        if let Some(f) = self.db.folder_by_role(&acct.id, role)? {
            return Ok(f);
        }
        let (name, label) = match role {
            "sent" => ("Sent", "Posta inviata"),
            "drafts" => ("Drafts", "Bozze"),
            "deleted" => ("Trash", "Posta eliminata"),
            "junk" => ("Junk", "Posta indesiderata"),
            "archive" => ("Archive", "Archivio"),
            "inbox" => ("INBOX", "Posta in arrivo"),
            _ => return Err(Error::NotFound),
        };
        let folders = self.db.folders(&acct.id)?;
        let delim = folders.iter().find_map(|f| f.delim.clone()).unwrap_or_else(|| "/".into());
        let others: Vec<&FolderRow> = folders.iter().filter(|f| !f.path.eq_ignore_ascii_case("INBOX")).collect();
        let prefix = !others.is_empty() && others.iter().all(|f| f.path.to_ascii_uppercase().starts_with(&format!("INBOX{delim}")));
        let path = if prefix && role != "inbox" { format!("INBOX{delim}{name}") } else { name.to_string() };
        let id = self.db.upsert_folder(&acct.id, &path, label, Some(role), Some(&delim), true)?;
        if acct.kind == AccountKind::Imap && role != "inbox" {
            self.db.push_op(&acct.id, &Op::CreateFolder { path })?;
        }
        self.changed(&acct.id, vec![]);
        self.db.folder(id)
    }

    pub async fn move_messages(&self, ids: &[i64], dest_id: i64) -> Result<()> {
        let dest = self.db.folder(dest_id)?;
        let dest_acct = self.db.account(&dest.account_id)?;
        for (folder, acct, rows) in self.group_by_folder(ids)? {
            if folder.id == dest.id {
                continue;
            }
            if acct.id != dest_acct.id {
                self.copy_across(&rows, &dest_acct, &dest).await?;
                self.remove_permanently(&acct, &folder, &rows)?;
                continue;
            }
            let all: Vec<i64> = rows.iter().map(|m| m.id).collect();
            if acct.kind == AccountKind::Imap {
                let server: Vec<&MsgRow> = rows.iter().filter(|m| is_server_backed(m)).collect();
                let local_only: Vec<i64> = rows.iter().filter(|m| !is_server_backed(m)).map(|m| m.id).collect();
                if !server.is_empty() {
                    let server_ids: Vec<i64> = server.iter().map(|m| m.id).collect();
                    let targets: Vec<Target> = server.iter().map(|m| m.target()).collect();
                    self.db.local_move(&server_ids, dest.id, true)?;
                    self.db.push_op(&acct.id, &Op::Move { folder: folder.path.clone(), dest: dest.path.clone(), targets, local_ids: server_ids })?;
                }
                self.db.local_move(&local_only, dest.id, false)?;
                self.wake(&acct.id);
            } else {
                self.db.local_move(&all, dest.id, false)?;
            }
            self.changed(&acct.id, vec![folder.id, dest.id]);
        }
        Ok(())
    }

    /// Copia in un altro account (per esempio da un file di dati alla casella).
    async fn copy_across(&self, rows: &[MsgRow], dest_acct: &AccountConfig, dest: &FolderRow) -> Result<()> {
        for m in rows {
            let raw = self.raw(m).await?;
            let mut nm = mime::parse(&raw).map(|p| p.to_new_message(m.date, true, raw.len() as i64)).unwrap_or_default();
            nm.unread = m.unread;
            nm.flagged = m.flagged;
            nm.answered = m.answered;
            nm.focused = if dest.role.as_deref() == Some("inbox") { nm.focused } else { true };
            nm.raw = Some(raw);
            let imap = dest_acct.kind == AccountKind::Imap;
            nm.pending = if imap { Some("append".into()) } else { None };
            if let Some(id) = self.db.insert_message(&dest_acct.id, dest.id, &nm)? {
                if imap {
                    let mut flags = Vec::new();
                    if !m.unread {
                        flags.push("\\Seen");
                    }
                    if m.flagged {
                        flags.push("\\Flagged");
                    }
                    self.db.push_op(&dest_acct.id, &Op::Append { folder: dest.path.clone(), local_id: id, flags: format!("({})", flags.join(" ")) })?;
                }
            }
        }
        self.wake(&dest_acct.id);
        self.changed(&dest_acct.id, vec![dest.id]);
        Ok(())
    }

    fn remove_permanently(&self, acct: &AccountConfig, folder: &FolderRow, rows: &[MsgRow]) -> Result<()> {
        let server: Vec<Target> = rows.iter().filter(|m| acct.kind == AccountKind::Imap && is_server_backed(m)).map(|m| m.target()).collect();
        self.db.delete_messages(&rows.iter().map(|m| m.id).collect::<Vec<_>>())?;
        if !server.is_empty() {
            self.db.push_op(&acct.id, &Op::Expunge { folder: folder.path.clone(), targets: server })?;
            self.wake(&acct.id);
        }
        self.changed(&acct.id, vec![folder.id]);
        Ok(())
    }

    /// Elimina: sposta in Posta eliminata, o cancella del tutto se è già lì.
    pub async fn delete_messages(&self, ids: &[i64]) -> Result<()> {
        for (folder, acct, rows) in self.group_by_folder(ids)? {
            if folder.role.as_deref() == Some("deleted") || acct.kind == AccountKind::Local && self.db.folder_by_role(&acct.id, "deleted")?.is_none() {
                self.remove_permanently(&acct, &folder, &rows)?;
            } else {
                let trash = self.role_folder(&acct, "deleted")?;
                self.move_messages(&rows.iter().map(|m| m.id).collect::<Vec<_>>(), trash.id).await?;
            }
        }
        Ok(())
    }

    pub async fn move_to_role(&self, ids: &[i64], role: &str) -> Result<()> {
        for (_, acct, rows) in self.group_by_folder(ids)? {
            let dest = self.role_folder(&acct, role)?;
            self.move_messages(&rows.iter().map(|m| m.id).collect::<Vec<_>>(), dest.id).await?;
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Categorie colorate
    // ------------------------------------------------------------------

    pub fn categories(&self) -> Result<Vec<Category>> {
        if let Some(list) = self.db.meta(CATEGORIES_KEY)?.and_then(|s| serde_json::from_str(&s).ok()) {
            return Ok(list);
        }
        // Le sei categorie iniziali di Outlook, una per colore.
        Ok([
            ("rosso", "Categoria rossa"),
            ("arancione", "Categoria arancione"),
            ("giallo", "Categoria gialla"),
            ("verde", "Categoria verde"),
            ("blu", "Categoria blu"),
            ("viola", "Categoria viola"),
        ]
        .into_iter()
        .map(|(c, n)| Category { id: c.into(), name: n.into(), color: c.into() })
        .collect())
    }

    /// Salva l'elenco delle categorie. Quelle tolte spariscono dai messaggi.
    pub fn save_categories(&self, list: Vec<Category>) -> Result<Vec<Category>> {
        let mut out: Vec<Category> = Vec::new();
        for mut c in list {
            c.name = c.name.trim().chars().take(60).collect();
            if c.name.is_empty() {
                return Err(Error::user("Ogni categoria deve avere un nome."));
            }
            if out.iter().any(|o| o.name.to_lowercase() == c.name.to_lowercase()) {
                return Err(Error::user(format!("Esiste già una categoria \"{}\".", c.name)));
            }
            if !CATEGORY_COLORS.contains(&c.color.as_str()) {
                c.color = "grigio".into();
            }
            if c.id.is_empty() {
                c.id = mime::random_id()[..10].to_string();
            }
            out.push(c);
        }
        for old in self.categories()? {
            if !out.iter().any(|c| c.id == old.id) {
                self.db.forget_category(&old.id)?;
            }
        }
        self.db.set_meta(CATEGORIES_KEY, &serde_json::to_string(&out)?)?;
        self.emit(Event::Categories);
        self.changed("", vec![]);
        Ok(out)
    }

    /// Assegna o toglie una categoria ai messaggi.
    pub fn set_category(&self, ids: &[i64], category: &str, on: bool) -> Result<()> {
        let rows = self.db.messages_by_ids(ids)?;
        let keys: Vec<String> = rows.iter().map(|m| m.category_key()).collect();
        self.db.set_category(&keys, category, on)?;
        for m in &rows {
            self.changed(&m.account_id, vec![m.folder_id]);
        }
        Ok(())
    }

    pub fn clear_categories(&self, ids: &[i64]) -> Result<()> {
        let rows = self.db.messages_by_ids(ids)?;
        let keys: Vec<String> = rows.iter().map(|m| m.category_key()).collect();
        self.db.clear_categories(&keys)?;
        for m in &rows {
            self.changed(&m.account_id, vec![m.folder_id]);
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // Regole, risposte automatiche e avvisi
    // ------------------------------------------------------------------

    pub fn rules(&self) -> Result<Vec<Rule>> {
        Ok(self.db.meta(RULES_KEY)?.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
    }

    /// Salva l'elenco delle regole, nell'ordine in cui si applicano.
    pub fn save_rules(&self, list: Vec<Rule>) -> Result<Vec<Rule>> {
        let clean = |v: Vec<String>| v.into_iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect::<Vec<_>>();
        let mut out = Vec::new();
        for mut r in list {
            r.name = r.name.trim().chars().take(80).collect();
            if r.name.is_empty() {
                return Err(Error::user("Dai un nome a ogni regola."));
            }
            r.when.from = clean(r.when.from);
            r.when.to = clean(r.when.to);
            r.when.subject = clean(r.when.subject);
            r.when.words = clean(r.when.words);
            r.then.category = r.then.category.filter(|c| !c.is_empty());
            let a = &r.then;
            if a.move_to.is_none() && !a.delete && !a.mark_read && !a.flag && a.category.is_none() && !a.alert {
                return Err(Error::user(format!("Scegli cosa deve fare la regola \"{}\".", r.name)));
            }
            if r.id.is_empty() {
                r.id = mime::random_id()[..10].to_string();
            }
            out.push(r);
        }
        self.db.set_meta(RULES_KEY, &serde_json::to_string(&out)?)?;
        self.emit(Event::Rules);
        Ok(out)
    }

    /// Dati del messaggio per le condizioni. Il testo serve solo se una
    /// regola lo guarda; se il messaggio non è ancora scaricato vale l'anteprima.
    fn candidate(&self, m: &MsgRow, with_body: bool) -> Candidate {
        let body = if with_body {
            self.db.body(m.id).ok().flatten().and_then(|raw| mime::parse(&raw).map(|p| p.text())).unwrap_or_else(|| m.preview.clone())
        } else {
            String::new()
        };
        let to: Vec<(String, String)> = m.recipients.to.iter().chain(m.recipients.cc.iter()).map(|p| (p.name.clone(), p.email.clone())).collect();
        Candidate::new(&m.account_id, (&m.from.name, &m.from.email), &to, &m.subject, &body, m.has_atts)
    }

    /// Applica le regole ai messaggi. Restituisce i messaggi toccati e,
    /// separati, quelli usciti dalla cartella, letti e da segnalare.
    async fn apply_rules(&self, rows: &[MsgRow], rules: &[Rule], only: Option<&[String]>) -> Result<RulesOutcome> {
        let with_body = rules.iter().any(|r| !r.when.words.is_empty());
        let mut out = RulesOutcome::default();
        let mut moves: HashMap<i64, Vec<i64>> = HashMap::new();
        let mut deletes = Vec::new();
        let mut read = Vec::new();
        let mut flag = Vec::new();
        let mut cats: HashMap<String, Vec<i64>> = HashMap::new();
        let folders: HashSet<i64> = self.db.folder_infos()?.into_iter().map(|f| f.id).collect();
        for m in rows {
            let p = rules::plan(rules, &self.candidate(m, with_body), only);
            if p.is_empty() {
                continue;
            }
            out.matched += 1;
            if p.mark_read && m.unread {
                read.push(m.id);
            }
            if p.mark_read {
                out.read.insert(m.id);
            }
            if p.flag && !m.flagged {
                flag.push(m.id);
            }
            for c in p.categories {
                cats.entry(c).or_default().push(m.id);
            }
            if p.alert {
                out.alert.insert(m.id);
            }
            if p.delete {
                deletes.push(m.id);
                out.gone.insert(m.id);
                out.deleted.insert(m.id);
            } else if let Some(dest) = p.move_to.filter(|d| *d != m.folder_id && folders.contains(d)) {
                moves.entry(dest).or_default().push(m.id);
                out.gone.insert(m.id);
            }
        }
        if !read.is_empty() {
            self.set_flags(&read, Some(true), None, None)?;
        }
        if !flag.is_empty() {
            self.set_flags(&flag, None, Some(true), None)?;
        }
        for (c, ids) in cats {
            self.set_category(&ids, &c, true)?;
        }
        for (dest, ids) in moves {
            if let Err(e) = self.move_messages(&ids, dest).await {
                log::warn!("regola, spostamento: {e}");
            }
        }
        if !deletes.is_empty() {
            if let Err(e) = self.move_to_role(&deletes, "deleted").await {
                log::warn!("regola, eliminazione: {e}");
            }
        }
        Ok(out)
    }

    /// "Esegui ora": applica alcune regole ai messaggi già presenti in una
    /// cartella. Restituisce quanti messaggi sono stati toccati.
    pub async fn run_rules(&self, rule_ids: &[String], folder_id: i64) -> Result<usize> {
        let rules: Vec<Rule> = self.rules()?.into_iter().map(|mut r| {
            // Qui nessun avviso: i messaggi non sono nuovi.
            r.then.alert = false;
            r
        }).collect();
        let ids = self.db.folder_message_ids(folder_id)?;
        let rows: Vec<MsgRow> = self.db.messages_by_ids(&ids)?.into_iter().filter(|m| !m.draft).collect();
        Ok(self.apply_rules(&rows, &rules, Some(rule_ids)).await?.matched)
    }

    /// Messaggi appena arrivati nella posta in arrivo: regole, risposta
    /// automatica e avviso sul desktop. Ognuno passa di qui una volta sola.
    async fn process_arrivals(&self, acct: &AccountConfig) {
        if let Err(e) = self.process_arrivals_inner(acct).await {
            log::warn!("nuovi messaggi di {}: {e}", acct.email);
        }
    }

    async fn process_arrivals_inner(&self, acct: &AccountConfig) -> Result<()> {
        let Some(inbox) = self.db.folder_by_role(&acct.id, "inbox")? else { return Ok(()) };
        let rows = self.db.unruled(inbox.id)?;
        if rows.is_empty() {
            return Ok(());
        }
        let ids: Vec<i64> = rows.iter().map(|m| m.id).collect();
        self.db.mark_ruled(&ids)?;
        // Conversazioni ignorate: i messaggi nuovi vanno subito in Posta eliminata.
        let dropped = self.drop_ignored(&rows).await?;
        let rows: Vec<MsgRow> = rows.into_iter().filter(|m| !dropped.contains(&m.id)).collect();
        if rows.is_empty() {
            return Ok(());
        }
        let outcome = self.apply_rules(&rows, &self.rules()?, None).await?;

        // La configurazione può essere cambiata mentre si sincronizzava.
        let acct = self.db.account(&acct.id)?;
        if let Some(ar) = acct.auto_reply.as_ref().filter(|a| a.active_at(now_secs() * 1000)) {
            if acct.outgoing.is_some() {
                for m in rows.iter().filter(|m| !outcome.deleted.contains(&m.id)) {
                    if let Err(e) = self.send_auto_reply(&acct, ar, m).await {
                        log::warn!("risposta automatica: {e}");
                    }
                }
            }
        }

        // Avviso: i messaggi rimasti da leggere in Prioritaria, più quelli
        // per cui una regola chiede l'avviso.
        let mut alert: Vec<&MsgRow> = rows
            .iter()
            .filter(|m| outcome.alert.contains(&m.id) || (m.focused && m.unread && !outcome.gone.contains(&m.id) && !outcome.read.contains(&m.id)))
            .collect();
        if !alert.is_empty() {
            alert.sort_by_key(|m| std::cmp::Reverse(m.date));
            self.emit(Event::NewMail {
                account_id: acct.id.clone(),
                count: alert.len() as u32,
                messages: alert.iter().take(ALERT_MAX).map(|m| m.summary()).collect(),
            });
        }
        Ok(())
    }

    /// Risposta automatica a un messaggio, una volta sola per mittente.
    async fn send_auto_reply(&self, acct: &AccountConfig, ar: &AutoReply, m: &MsgRow) -> Result<()> {
        let raw = match self.db.body(m.id)? {
            Some(r) => r,
            None => self.raw(m).await?,
        };
        let Some(p) = mime::parse(&raw) else { return Ok(()) };
        let Some(to) = p.auto_reply_to(&acct.email) else { return Ok(()) };
        if !self.db.autoreply_once(&acct.id, &to.email, ar.since)? {
            return Ok(());
        }
        let subject = p.subject();
        let mid = p.message_id();
        let mut references = p.references();
        if let Some(id) = &mid {
            references.push(id.clone());
        }
        let d = Draft {
            key: mime::random_id(),
            account_id: acct.id.clone(),
            to: vec![to],
            cc: vec![],
            bcc: vec![],
            subject: if subject.is_empty() { "Risposta automatica".into() } else { format!("Risposta automatica: {subject}") },
            body: ar.text.clone(),
            html: Some(ar.html.clone()).filter(|h| !h.trim().is_empty()),
            plain: false,
            attachments: vec![],
            in_reply_to: mid,
            references,
            reply_to_message: None,
            ..Default::default()
        };
        let built = mime::build_auto_reply(acct, &d)?;
        self.db.outbox_add(&acct.id, &built.wire, &built.copy, &built.envelope, &d, now_secs())?;
        self.outbox_wake.notify_one();
        Ok(())
    }

    /// Salva la risposta automatica di un account. Ogni nuova attivazione
    /// riparte da zero: chi aveva già ricevuto la risposta la riceve di nuovo.
    pub fn set_auto_reply(&self, account_id: &str, mut ar: AutoReply) -> Result<()> {
        let mut cfg = self.db.account(account_id)?;
        if cfg.kind == AccountKind::Local {
            return Err(Error::user("I file di dati non ricevono posta."));
        }
        if let (Some(s), Some(e)) = (ar.start, ar.end) {
            if e <= s {
                return Err(Error::user("La fine del periodo deve venire dopo l'inizio."));
            }
        }
        if ar.enabled && ar.text.trim().is_empty() {
            return Err(Error::user("Scrivi il messaggio della risposta automatica."));
        }
        let prev = cfg.auto_reply.clone().unwrap_or_default();
        ar.since = if ar.enabled && (!prev.enabled || prev.start != ar.start || prev.since == 0) { now_secs() * 1000 } else { prev.since };
        cfg.auto_reply = Some(ar);
        self.db.save_account(&cfg)?;
        self.emit(Event::Accounts);
        Ok(())
    }

    // ------------------------------------------------------------------
    // Conversazioni
    // ------------------------------------------------------------------

    /// Messaggi di una conversazione, in tutte le cartelle dell'account.
    pub fn thread(&self, account_id: &str, thread: &str) -> Result<Vec<MessageSummary>> {
        let names: HashMap<i64, String> = self.db.folders(account_id)?.into_iter().map(|f| (f.id, f.name)).collect();
        Ok(self
            .db
            .thread_messages(account_id, thread)?
            .into_iter()
            .map(|m| {
                let mut s = m.summary();
                s.folder_name = names.get(&m.folder_id).cloned();
                s
            })
            .collect())
    }

    // ------------------------------------------------------------------
    // Gestione delle cartelle
    // ------------------------------------------------------------------

    fn clean_folder_name(name: &str, delim: &str) -> Result<String> {
        let n = name.trim();
        if n.is_empty() {
            return Err(Error::user("Scrivi il nome della cartella."));
        }
        if n.chars().count() > 120 {
            return Err(Error::user("Il nome della cartella è troppo lungo."));
        }
        if n.contains(delim) || n.chars().any(|c| c.is_control()) {
            return Err(Error::user(format!("Il nome della cartella non può contenere il carattere \"{delim}\".")));
        }
        Ok(n.to_string())
    }

    fn folder_delim(&self, account_id: &str) -> Result<String> {
        Ok(self.db.folders(account_id)?.iter().find_map(|f| f.delim.clone()).filter(|d| !d.is_empty()).unwrap_or_else(|| "/".into()))
    }

    /// Percorso di una cartella nuova, dentro `parent` o al primo livello
    /// dell'account (dentro INBOX sui server che mettono lì tutto).
    fn child_path(&self, acct: &AccountConfig, parent: Option<&FolderRow>, name: &str) -> Result<String> {
        let delim = self.folder_delim(&acct.id)?;
        let enc = if acct.kind == AccountKind::Imap { utf7::encode(name) } else { name.to_string() };
        Ok(match parent {
            Some(p) => format!("{}{delim}{enc}", p.path),
            None => {
                let folders = self.db.folders(&acct.id)?;
                let others: Vec<&FolderRow> = folders.iter().filter(|f| !f.path.eq_ignore_ascii_case("INBOX")).collect();
                let under_inbox = acct.kind == AccountKind::Imap
                    && !others.is_empty()
                    && others.iter().all(|f| f.path.to_ascii_uppercase().starts_with(&format!("INBOX{delim}")));
                if under_inbox {
                    format!("INBOX{delim}{enc}")
                } else {
                    enc
                }
            }
        })
    }

    /// Crea una cartella, dentro un'altra o al primo livello dell'account.
    pub fn create_folder(&self, account_id: &str, parent_id: Option<i64>, name: &str) -> Result<i64> {
        let acct = self.db.account(account_id)?;
        let delim = self.folder_delim(account_id)?;
        let name = Self::clean_folder_name(name, &delim)?;
        let parent = match parent_id {
            Some(id) => Some(self.db.folder(id)?),
            None => None,
        };
        let path = self.child_path(&acct, parent.as_ref(), &name)?;
        if self.db.folder_by_path(account_id, &path)?.is_some() {
            return Err(Error::user(format!("Esiste già una cartella \"{name}\" in questa posizione.")));
        }
        let id = self.db.upsert_folder(account_id, &path, &name, None, Some(&delim), true)?;
        if acct.kind == AccountKind::Imap {
            self.db.push_op(account_id, &Op::CreateFolder { path })?;
            self.wake(account_id);
        }
        self.changed(account_id, vec![]);
        Ok(id)
    }

    pub fn rename_folder(&self, folder_id: i64, name: &str) -> Result<()> {
        let f = self.db.folder(folder_id)?;
        if f.role.is_some() {
            return Err(Error::user("Le cartelle predefinite non si possono rinominare."));
        }
        let acct = self.db.account(&f.account_id)?;
        let delim = f.delim.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| "/".into());
        let name = Self::clean_folder_name(name, &delim)?;
        let enc = if acct.kind == AccountKind::Imap { utf7::encode(&name) } else { name.clone() };
        let path = match f.path.rsplit_once(delim.as_str()) {
            Some((parent, _)) => format!("{parent}{delim}{enc}"),
            None => enc,
        };
        if path == f.path {
            return Ok(());
        }
        if self.db.folder_by_path(&f.account_id, &path)?.is_some() {
            return Err(Error::user(format!("Esiste già una cartella \"{name}\" in questa posizione.")));
        }
        self.db.rename_folder(f.id, &path, &name)?;
        if acct.kind == AccountKind::Imap {
            self.db.push_op(&acct.id, &Op::RenameFolder { from: f.path.clone(), to: path })?;
            self.wake(&acct.id);
        }
        self.changed(&acct.id, vec![]);
        Ok(())
    }

    /// Elimina una cartella e le sue sottocartelle. I messaggi vanno in Posta
    /// eliminata; se la cartella è già lì dentro, spariscono del tutto.
    pub async fn delete_folder(&self, folder_id: i64) -> Result<()> {
        let f = self.db.folder(folder_id)?;
        if f.role.is_some() {
            return Err(Error::user("Le cartelle predefinite non si possono eliminare."));
        }
        let acct = self.db.account(&f.account_id)?;
        let mut all = self.db.subfolders(f.id)?;
        all.push(f.clone());
        // Dalla più profonda: il server non elimina le cartelle che ne contengono altre.
        all.sort_by_key(|x| std::cmp::Reverse(x.path.len()));
        let trash = self.db.folder_by_role(&acct.id, "deleted")?;
        let inside_trash = trash.as_ref().map(|t| f.path.starts_with(&format!("{}{}", t.path, f.delim.as_deref().unwrap_or("/")))).unwrap_or(false);
        for x in &all {
            let ids = self.db.folder_message_ids(x.id)?;
            if ids.is_empty() {
                continue;
            }
            if inside_trash || (acct.kind == AccountKind::Local && trash.is_none()) {
                let rows = self.db.messages_by_ids(&ids)?;
                self.remove_permanently(&acct, x, &rows)?;
            } else {
                let t = self.role_folder(&acct, "deleted")?;
                self.move_messages(&ids, t.id).await?;
            }
        }
        for x in &all {
            self.db.delete_folder(x.id)?;
            if acct.kind == AccountKind::Imap {
                self.db.push_op(&acct.id, &Op::DeleteFolder { path: x.path.clone() })?;
            }
        }
        if acct.kind == AccountKind::Imap {
            self.wake(&acct.id);
        }
        self.changed(&acct.id, vec![]);
        Ok(())
    }

    pub fn mark_folder_read(&self, folder_id: i64) -> Result<()> {
        let f = self.db.folder(folder_id)?;
        let acct = self.db.account(&f.account_id)?;
        let rows = self.db.unread_in_folder(f.id)?;
        if rows.is_empty() {
            return Ok(());
        }
        let ids: Vec<i64> = rows.iter().map(|m| m.id).collect();
        let imap = acct.kind == AccountKind::Imap;
        self.db.set_local_flags(&ids, Some(true), None, None, imap)?;
        if imap {
            self.db.push_op(&acct.id, &Op::MarkAllRead { folder: f.path.clone(), local_ids: ids })?;
            self.wake(&acct.id);
        }
        self.changed(&acct.id, vec![f.id]);
        Ok(())
    }

    /// Svuota una cartella: i messaggi spariscono del tutto (per Posta
    /// eliminata e Posta indesiderata).
    pub fn empty_folder(&self, folder_id: i64) -> Result<usize> {
        let f = self.db.folder(folder_id)?;
        let acct = self.db.account(&f.account_id)?;
        let ids = self.db.folder_message_ids(f.id)?;
        let rows = self.db.messages_by_ids(&ids)?;
        let n = rows.len();
        self.remove_permanently(&acct, &f, &rows)?;
        Ok(n)
    }

    // ------------------------------------------------------------------
    // Pulizia cassetta postale
    // ------------------------------------------------------------------

    /// Messaggi e spazio di ogni cartella dell'account, dalla più grande.
    pub fn mailbox_usage(&self, account_id: &str) -> Result<Vec<FolderUsage>> {
        let mut v: Vec<FolderUsage> = self.db.folder_sizes(account_id)?.into_iter().map(|(folder_id, count, size)| FolderUsage { folder_id, count, size }).collect();
        v.sort_by(|a, b| b.size.cmp(&a.size).then(b.count.cmp(&a.count)));
        Ok(v)
    }

    /// Messaggi più grandi di `min_size` byte, con il nome della cartella.
    pub fn large_messages(&self, account_id: &str, min_size: i64) -> Result<Vec<MessageSummary>> {
        let names: HashMap<i64, String> = self.db.folders(account_id)?.into_iter().map(|f| (f.id, f.name)).collect();
        Ok(self
            .db
            .large_messages(account_id, min_size, 200)?
            .into_iter()
            .filter(|m| !m.draft)
            .map(|m| {
                let mut s = m.summary();
                s.folder_name = names.get(&m.folder_id).cloned();
                s
            })
            .collect())
    }

    /// Messaggi della cartella più vecchi della data (millisecondi). Con
    /// `archive` li sposta in Archivio; restituisce quanti sono.
    pub async fn older_messages(&self, folder_id: i64, before_ms: i64, archive: bool) -> Result<usize> {
        let ids = self.db.older_than(folder_id, before_ms / 1000)?;
        if archive && !ids.is_empty() {
            self.move_to_role(&ids, "archive").await?;
        }
        Ok(ids.len())
    }

    // ------------------------------------------------------------------
    // Bozze e invio
    // ------------------------------------------------------------------

    async fn load_attachments(&self, list: &[DraftAttachment]) -> Result<Vec<AttachmentData>> {
        let mut out = Vec::new();
        for a in list {
            if let Some(id) = a.message {
                let row = self.db.message(id).map_err(|_| Error::user(format!("Il messaggio \"{}\" non è più disponibile.", a.name)))?;
                let bytes = self.raw(&row).await?;
                let name = if a.name.is_empty() { format!("{}.eml", row.subject.replace(['/', '\\', ':'], "_")) } else { a.name.clone() };
                out.push(AttachmentData { name, mime: "message/rfc822".into(), bytes });
            } else if let Some(src) = &a.source {
                let (name, mime_type, bytes) = self
                    .attachment(src.message_id, src.index)
                    .await
                    .map_err(|_| Error::user(format!("L'allegato \"{}\" non è più disponibile.", a.name)))?;
                out.push(AttachmentData { name: if a.name.is_empty() { name } else { a.name.clone() }, mime: mime_type, bytes });
            } else if let Some(path) = &a.path {
                let bytes = tokio::fs::read(path).await.map_err(|_| Error::user(format!("Impossibile leggere il file \"{}\".", a.name)))?;
                let name = if a.name.is_empty() {
                    Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "allegato".into())
                } else {
                    a.name.clone()
                };
                out.push(AttachmentData { mime: a.mime.clone().unwrap_or_else(|| guess_mime(&name)), name, bytes });
            }
        }
        Ok(out)
    }

    fn draft_is_empty(d: &Draft) -> bool {
        d.to.is_empty() && d.cc.is_empty() && d.bcc.is_empty() && d.subject.trim().is_empty() && d.body.trim().is_empty() && d.attachments.is_empty()
    }

    /// Salva la bozza sul computer. Gli allegati della bozza salvata
    /// sostituiscono quelli indicati (così non dipendono più dai file originali).
    pub async fn save_draft(&self, d: &Draft) -> Result<SavedDraft> {
        let acct = self.db.account(&d.account_id)?;
        let drafts = self.role_folder(&acct, "drafts")?;
        let atts = self.load_attachments(&d.attachments).await?;
        let built = mime::build(&acct, d, &atts, true)?;
        let parsed = mime::parse(&built.copy).ok_or_else(|| Error::user("Impossibile salvare la bozza."))?;
        let mut nm = parsed.to_new_message(now_secs(), true, built.copy.len() as i64);
        nm.draft = true;
        nm.unread = false;
        nm.focused = true;
        nm.draft_key = Some(d.key.clone());
        nm.raw = Some(built.copy.clone());
        let imap = acct.kind == AccountKind::Imap;
        nm.pending = if imap { Some("draft".into()) } else { None };

        let mut local_id = None;
        for id in self.db.local_drafts_by_key(&acct.id, &d.key)? {
            let row = self.db.message(id)?;
            if row.uid.is_none() && local_id.is_none() {
                self.db.update_message_raw(id, &nm)?;
                if row.folder_id != drafts.id {
                    self.db.local_move(&[id], drafts.id, false)?;
                }
                self.db.set_pending(&[id], nm.pending.as_deref())?;
                local_id = Some(id);
            } else if row.uid.is_some() {
                // Versione già sul server: la sostituirà quella nuova.
                let folder = self.db.folder(row.folder_id)?;
                self.db.delete_messages(&[id])?;
                if imap {
                    self.db.push_op(&acct.id, &Op::Expunge { folder: folder.path, targets: vec![row.target()] })?;
                }
            } else {
                self.db.delete_messages(&[id])?;
            }
        }
        let id = match local_id {
            Some(id) => id,
            None => self.db.insert_message(&acct.id, drafts.id, &nm)?.ok_or_else(|| Error::user("Impossibile salvare la bozza."))?,
        };
        let attachments = parsed
            .attachments()
            .into_iter()
            .filter(|a| !a.inline)
            .map(|a| DraftAttachment {
                name: a.name,
                size: a.size,
                mime: Some(a.mime),
                path: None,
                source: Some(AttachmentRef { message_id: id, index: a.index }),
                message: None,
            })
            .collect();
        self.changed(&acct.id, vec![drafts.id]);
        Ok(SavedDraft { id, attachments })
    }

    /// Chiusura del messaggio in scrittura: la bozza va anche sul server.
    pub async fn close_draft(&self, d: &Draft) -> Result<Option<SavedDraft>> {
        if Self::draft_is_empty(d) {
            self.discard_draft(&d.account_id, &d.key)?;
            return Ok(None);
        }
        let saved = self.save_draft(d).await?;
        let acct = self.db.account(&d.account_id)?;
        if acct.kind == AccountKind::Imap {
            let drafts = self.role_folder(&acct, "drafts")?;
            let row = self.db.message(saved.id)?;
            self.db.push_op(&acct.id, &Op::Append { folder: drafts.path.clone(), local_id: saved.id, flags: "(\\Draft \\Seen)".into() })?;
            self.db.push_op(&acct.id, &Op::DeleteDraftVersions { folder: drafts.path, draft_key: d.key.clone(), keep_message_id: row.message_id })?;
            self.wake(&acct.id);
        }
        Ok(Some(saved))
    }

    fn remove_draft_rows(&self, acct: &AccountConfig, key: &str, everywhere: bool) -> Result<()> {
        let imap = acct.kind == AccountKind::Imap;
        let mut folders = Vec::new();
        for id in self.db.local_drafts_by_key(&acct.id, key)? {
            let row = self.db.message(id)?;
            let folder = self.db.folder(row.folder_id)?;
            if imap && row.uid.is_some() {
                self.db.push_op(&acct.id, &Op::Expunge { folder: folder.path.clone(), targets: vec![row.target()] })?;
            }
            // Una copia locale in attesa di arrivare al server: l'operazione
            // di copia non troverà più nulla da copiare.
            self.db.delete_messages(&[id])?;
            folders.push(folder.id);
        }
        if imap && everywhere {
            if let Some(drafts) = self.db.folder_by_role(&acct.id, "drafts")? {
                self.db.push_op(&acct.id, &Op::DeleteDraftVersions { folder: drafts.path, draft_key: key.into(), keep_message_id: None })?;
            }
            self.wake(&acct.id);
        }
        self.changed(&acct.id, folders);
        Ok(())
    }

    pub fn discard_draft(&self, account_id: &str, key: &str) -> Result<()> {
        let acct = self.db.account(account_id)?;
        self.remove_draft_rows(&acct, key, true)
    }

    pub async fn open_draft(&self, id: i64) -> Result<Draft> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        let parsed = mime::parse(&raw).ok_or_else(|| Error::user("La bozza non è leggibile."))?;
        let d = parsed.to_draft(&row.account_id, id);
        if row.draft_key.as_deref() != Some(d.key.as_str()) {
            self.db.conn().execute("UPDATE messages SET draft_key = ?2 WHERE id = ?1", rusqlite::params![id, d.key])?;
        }
        Ok(d)
    }

    /// Mette il messaggio nella posta in uscita. Parte dopo qualche secondo,
    /// per lasciare il tempo di annullare.
    pub async fn send(&self, d: &Draft) -> Result<i64> {
        self.enqueue(d, None).await
    }

    /// Invio programmato: il messaggio aspetta nella posta in uscita fino
    /// all'ora indicata (millisecondi). Parte solo se il programma è aperto;
    /// altrimenti appena si riapre.
    pub async fn send_later(&self, d: &Draft, at_ms: i64) -> Result<i64> {
        if at_ms <= now_secs() * 1000 + 30_000 {
            return Err(Error::user("Scegli un'ora futura per l'invio."));
        }
        self.enqueue(d, Some(at_ms / 1000)).await
    }

    async fn enqueue(&self, d: &Draft, at: Option<i64>) -> Result<i64> {
        if d.to.is_empty() && d.cc.is_empty() && d.bcc.is_empty() {
            return Err(Error::user("Aggiungi almeno un destinatario."));
        }
        let acct = self.db.account(&d.account_id)?;
        if acct.kind == AccountKind::Local || acct.outgoing.is_none() {
            return Err(Error::user("Da questo account non si può inviare posta."));
        }
        let atts = self.load_attachments(&d.attachments).await?;
        let built = mime::build(&acct, d, &atts, false)?;
        let id = self.db.outbox_add(&acct.id, &built.wire, &built.copy, &built.envelope, d, at.unwrap_or(now_secs() + self.undo_secs))?;
        let rows = self.db.local_drafts_by_key(&acct.id, &d.key)?;
        self.db.set_pending(&rows, Some("sending"))?;
        if !rows.is_empty() {
            let drafts = self.db.folder_by_role(&acct.id, "drafts")?.map(|f| f.id);
            self.changed(&acct.id, drafts.into_iter().collect());
        }
        let state = if at.is_some() { "programmato" } else { "attesa" };
        self.emit(Event::Outbox { account_id: acct.id.clone(), outbox_id: id, state: state.into(), error: None });
        Ok(id)
    }

    pub fn cancel_send(&self, outbox_id: i64) -> Result<Option<Draft>> {
        let Some(row) = self.db.outbox_get(outbox_id)? else { return Ok(None) };
        let Some(d) = self.db.outbox_cancel(outbox_id)? else { return Ok(None) };
        let rows = self.db.local_drafts_by_key(&row.account_id, &d.key)?;
        let acct = self.db.account(&row.account_id)?;
        self.db.set_pending(&rows, if acct.kind == AccountKind::Imap { Some("draft") } else { None })?;
        self.emit(Event::Outbox { account_id: row.account_id.clone(), outbox_id, state: "annullato".into(), error: None });
        self.changed(&row.account_id, vec![]);
        Ok(Some(d))
    }

    pub fn outbox(&self) -> Result<Vec<OutboxItem>> {
        self.db.outbox_items()
    }

    /// Riapre un messaggio rimasto nella posta in uscita (per esempio dopo un errore).
    pub fn outbox_take(&self, outbox_id: i64) -> Result<Option<Draft>> {
        let Some(row) = self.db.outbox_get(outbox_id)? else { return Ok(None) };
        if row.state == "invio" {
            return Err(Error::user("Il messaggio è in invio proprio adesso."));
        }
        self.db.outbox_delete(outbox_id)?;
        self.emit(Event::Outbox { account_id: row.account_id.clone(), outbox_id, state: "annullato".into(), error: None });
        Ok(Some(row.draft))
    }

    // ------------------------------------------------------------------
    // Esportazione
    // ------------------------------------------------------------------

    pub async fn export_eml(&self, id: i64, path: &Path) -> Result<()> {
        let row = self.db.message(id)?;
        let raw = self.raw(&row).await?;
        tokio::fs::write(path, raw).await?;
        Ok(())
    }

    /// Esporta una cartella nel formato MBOX (variante mboxrd).
    pub async fn export_mbox(&self, folder_id: i64, path: &Path) -> Result<usize> {
        let rows = self.db.list_messages(folder_id, None, i64::MAX, 0)?;
        let mut out: Vec<u8> = Vec::new();
        let mut n = 0;
        for m in rows.iter().rev() {
            let Ok(raw) = self.raw(m).await else { continue };
            let date = chrono::DateTime::from_timestamp(m.date, 0).unwrap_or_default();
            let sender = if m.from.email.is_empty() { "MAILER-DAEMON".to_string() } else { m.from.email.clone() };
            out.extend_from_slice(format!("From {} {}\n", sender, date.format("%a %b %e %H:%M:%S %Y")).as_bytes());
            let text = raw.split(|b| *b == b'\n');
            for line in text {
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                let trimmed: &[u8] = {
                    let mut i = 0;
                    while i < line.len() && line[i] == b'>' {
                        i += 1;
                    }
                    &line[i..]
                };
                if trimmed.starts_with(b"From ") {
                    out.push(b'>');
                }
                out.extend_from_slice(line);
                out.push(b'\n');
            }
            out.push(b'\n');
            n += 1;
        }
        tokio::fs::write(path, out).await?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::NewMessage;
    use crate::secrets::Memory;

    struct Setup {
        eng: Arc<Engine>,
        events: Arc<Mutex<Vec<Event>>>,
        inbox: i64,
        suppliers: i64,
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
        db.upsert_folder("a1", "Trash", "Posta eliminata", Some("deleted"), Some("/"), true).unwrap();
        let suppliers = db.upsert_folder("a1", "Fornitori", "Fornitori", None, Some("/"), true).unwrap();
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        let eng = Engine::new(db, Arc::new(Memory::default()), std::env::temp_dir(), Arc::new(move |e| ev.lock().unwrap().push(e)));
        Setup { eng, events, inbox, suppliers }
    }

    fn arrive(s: &Setup, uid: u32, from: &str, subject: &str, extra: &str) -> i64 {
        let raw = format!(
            "From: {from}\r\nTo: Chiara Valli <chiara.valli@valliarredi.it>\r\nSubject: {subject}\r\nMessage-ID: <m{uid}@example.test>\r\nDate: Mon, 19 Oct 2026 09:00:00 +0200\r\n{extra}Content-Type: text/plain; charset=utf-8\r\n\r\nTesto del messaggio {uid}.\r\n"
        );
        let mut nm = mime::parse(raw.as_bytes()).unwrap().to_new_message(now_secs(), true, raw.len() as i64);
        nm.uid = Some(uid);
        nm.unread = true;
        nm.arrival = true;
        nm.raw = Some(raw.into_bytes());
        s.eng.db.insert_message("a1", s.inbox, &nm).unwrap().unwrap()
    }

    fn new_mail(s: &Setup) -> Vec<(u32, Vec<String>)> {
        s.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                Event::NewMail { count, messages, .. } => Some((*count, messages.iter().map(|m| m.subject.clone()).collect())),
                _ => None,
            })
            .collect()
    }

    #[tokio::test]
    async fn search_folders_are_saved_searches() {
        let s = setup();
        let a = arrive(&s, 30, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Preventivo urgente", "X-Priority: 1\r\n");
        let b = arrive(&s, 31, "Studio Bassi <amministrazione@studiobassi.it>", "Fattura n. 418", "");
        let c = arrive(&s, 32, "Lucia Marchetti <lucia@tessuti.example>", "Campionario", "");
        s.eng.set_flags(&[b], Some(true), Some(true), None).unwrap();
        s.eng.move_messages(&[c], s.suppliers).await.unwrap();

        let unread = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Posta da leggere".into(), kind: "da-leggere".into(), criteria: SearchCriteria { unread: true, ..Default::default() }, ..Default::default() }).unwrap();
        assert!(unread.id.starts_with("cr-"));
        assert_eq!((unread.total, unread.unread), (2, 2));
        let ids: Vec<i64> = s.eng.search_folder_messages(&unread.id).unwrap().iter().map(|m| m.id).collect();
        assert!(ids.contains(&a) && !ids.contains(&b), "{ids:?}");
        let moved = s.eng.search_folder_messages(&unread.id).unwrap().into_iter().find(|m| m.subject == "Campionario").unwrap();
        assert_eq!(moved.folder_name.as_deref(), Some("Fornitori"), "anche nelle altre cartelle, con il loro nome");

        let people = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Studio Orlandi".into(), kind: "persone".into(), criteria: SearchCriteria { people: vec!["  orlandi ".into()], ..Default::default() }, ..Default::default() }).unwrap();
        assert_eq!(s.eng.search_folder_messages(&people.id).unwrap().iter().map(|m| m.id).collect::<Vec<_>>(), vec![a]);
        let flagged = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Contrassegnata".into(), criteria: SearchCriteria { flagged: true, ..Default::default() }, ..Default::default() }).unwrap();
        assert_eq!((flagged.total, flagged.unread), (1, 0));
        let words = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Fatture".into(), kind: "parole".into(), criteria: SearchCriteria { text: "fattura".into(), ..Default::default() }, ..Default::default() }).unwrap();
        assert_eq!(s.eng.search_folder_messages(&words.id).unwrap().iter().map(|m| m.id).collect::<Vec<_>>(), vec![b]);
        let important = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Importante".into(), criteria: SearchCriteria { important: true, unread: true, ..Default::default() }, ..Default::default() }).unwrap();
        assert_eq!(important.total, 1);
        // Un nome con caratteri speciali non rompe la ricerca.
        let odd = s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Strano".into(), criteria: SearchCriteria { people: vec!["%_\\'".into()], ..Default::default() }, ..Default::default() }).unwrap();
        assert_eq!(odd.total, 0);

        // Controlli.
        assert!(s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "posta da leggere".into(), ..Default::default() }).is_err());
        assert!(s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: "Persone".into(), kind: "persone".into(), ..Default::default() }).is_err());
        assert!(s.eng.save_search_folder(SearchFolder { account_id: "a1".into(), name: " ".into(), ..Default::default() }).is_err());
        assert!(s.eng.save_search_folder(SearchFolder { account_id: "nessuno".into(), name: "X".into(), ..Default::default() }).is_err());

        // Modifica e eliminazione.
        let renamed = s.eng.save_search_folder(SearchFolder { name: "Da leggere".into(), ..unread.clone() }).unwrap();
        assert_eq!(renamed.id, unread.id);
        let list = s.eng.search_folders().unwrap();
        assert_eq!(list.len(), 6);
        assert_eq!(list[0].name, "Da leggere");
        s.eng.delete_search_folder(&unread.id).unwrap();
        assert_eq!(s.eng.search_folders().unwrap().len(), 5);
        assert!(s.eng.search_folder_messages(&unread.id).is_err());
    }

    #[tokio::test]
    async fn priority_and_read_receipts() {
        let s = setup();
        let ask = "Disposition-Notification-To: Davide Orlandi <d.orlandi@studiorlandi.it>\r\nX-Priority: 1 (Highest)\r\n";
        let a = arrive(&s, 20, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Preventivo firmato", ask);
        let b = arrive(&s, 21, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Campioni", ask);
        let own = arrive(&s, 22, "Chiara Valli <chiara.valli@valliarredi.it>", "Promemoria", "Disposition-Notification-To: chiara.valli@valliarredi.it\r\n");
        let plain = arrive(&s, 23, "Studio Bassi <amministrazione@studiobassi.it>", "Fattura", "Importance: low\r\n");

        let d = s.eng.detail(a, false).await.unwrap();
        assert_eq!(d.summary.importance, 1);
        assert_eq!(d.receipt_to.unwrap().email, "d.orlandi@studiorlandi.it");
        assert!(s.eng.detail(own, false).await.unwrap().receipt_to.is_none(), "mai per i propri messaggi");
        let p = s.eng.detail(plain, false).await.unwrap();
        assert!(p.receipt_to.is_none());
        assert_eq!(p.summary.importance, -1);

        // Invio: una sola volta, poi non si chiede più.
        assert!(s.eng.send_read_receipt(a, false).await.unwrap());
        assert!(!s.eng.send_read_receipt(a, false).await.unwrap());
        assert!(s.eng.detail(a, false).await.unwrap().receipt_to.is_none());
        let out = s.eng.db.outbox_due(now_secs() + 1).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].draft.mdn);
        assert_eq!(out[0].envelope.to, vec!["d.orlandi@studiorlandi.it".to_string()]);
        let wire = String::from_utf8_lossy(&out[0].raw).to_string();
        assert!(wire.contains("Subject: Letto: Preventivo firmato") && wire.contains("Original-Message-ID: <m20@example.test>"), "{wire}");

        // Dopo l'invio la conferma non va in Posta inviata.
        let acct = s.eng.db.account("a1").unwrap();
        s.eng.after_sent(&acct, &out[0]).unwrap();
        let sent = s.eng.db.folder_by_role("a1", "sent").unwrap();
        assert!(sent.map_or(true, |f| s.eng.db.list_messages(f.id, None, 10, 0).unwrap().is_empty()));

        // "No": non la si chiede più, e non parte nulla.
        s.eng.skip_read_receipt(b).unwrap();
        assert!(s.eng.detail(b, false).await.unwrap().receipt_to.is_none());
        assert_eq!(s.eng.db.outbox_due(now_secs() + 1).unwrap().len(), 1);
    }

    #[tokio::test]
    async fn arrivals_go_through_rules_auto_reply_and_alerts() {
        let s = setup();
        s.eng
            .save_rules(vec![Rule {
                id: String::new(),
                name: "Fatture dei fornitori".into(),
                enabled: true,
                account_id: None,
                when: RuleConditions { from: vec!["studiobassi.it".into()], ..Default::default() },
                then: RuleActions { move_to: Some(s.suppliers), category: Some("verde".into()), ..Default::default() },
            }])
            .unwrap();
        s.eng.set_auto_reply("a1", AutoReply { enabled: true, html: "<div>Sono in ferie fino al 2 novembre.</div>".into(), text: "Sono in ferie fino al 2 novembre.".into(), ..Default::default() }).unwrap();

        let invoice = arrive(&s, 10, "Studio Bassi <amministrazione@studiobassi.it>", "Fattura n. 418", "");
        let news = arrive(&s, 11, "Arredo e Contract <news@arredoecontract.example>", "Le novità di ottobre", "List-Unsubscribe: <mailto:off@arredoecontract.example>\r\n");
        let person = arrive(&s, 12, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Preventivo uffici", "");
        // Un messaggio già presente prima (non arrivato adesso) non si tocca.
        let old = s.eng.db.insert_message("a1", s.inbox, &NewMessage { uid: Some(1), subject: "Vecchio".into(), date: 1, ..Default::default() }).unwrap().unwrap();

        let acct = s.eng.db.account("a1").unwrap();
        s.eng.process_arrivals(&acct).await;

        // La regola sposta la fattura e le dà la categoria.
        let m = s.eng.db.message(invoice).unwrap();
        assert_eq!(m.folder_id, s.suppliers);
        assert_eq!(m.categories, vec!["verde".to_string()]);
        assert_eq!(s.eng.db.message(person).unwrap().folder_id, s.inbox);
        assert_eq!(s.eng.db.message(old).unwrap().folder_id, s.inbox);

        // Avviso solo per il messaggio personale rimasto in Prioritaria.
        assert_eq!(new_mail(&s), vec![(1, vec!["Preventivo uffici".to_string()])]);

        // Risposta automatica alla fattura e al messaggio personale, non alla newsletter.
        let out = s.eng.db.outbox_due(now_secs() + 1).unwrap();
        let mut to: Vec<String> = out.iter().map(|o| o.envelope.to.join(",")).collect();
        to.sort();
        assert_eq!(to, vec!["amministrazione@studiobassi.it".to_string(), "d.orlandi@studiorlandi.it".to_string()]);
        let wire = String::from_utf8_lossy(&out[0].raw).to_string();
        assert!(wire.contains("Auto-Submitted: auto-replied"), "{wire}");
        assert!(wire.contains("Subject: Risposta automatica:"), "{wire}");
        assert!(s.eng.db.message(news).unwrap().unread);

        // Una seconda volta non succede nulla; lo stesso mittente non riceve una seconda risposta.
        s.eng.process_arrivals(&acct).await;
        arrive(&s, 13, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Re: Preventivo uffici", "");
        s.eng.process_arrivals(&acct).await;
        assert_eq!(s.eng.db.outbox_due(now_secs() + 1).unwrap().len(), 2);
        assert_eq!(new_mail(&s).len(), 2);
    }

    #[tokio::test]
    async fn run_now_applies_rules_to_existing_mail_without_alerts() {
        let s = setup();
        let rules = s
            .eng
            .save_rules(vec![Rule {
                id: String::new(),
                name: "Contrassegna i preventivi".into(),
                enabled: true,
                account_id: Some("a1".into()),
                when: RuleConditions { subject: vec!["preventivo".into()], ..Default::default() },
                then: RuleActions { flag: true, mark_read: true, alert: true, ..Default::default() },
            }])
            .unwrap();
        let a = arrive(&s, 20, "Davide Orlandi <d.orlandi@studiorlandi.it>", "Preventivo uffici", "");
        let b = arrive(&s, 21, "Lucia Marchetti <l.marchetti@tessuti.example>", "Campionario", "");
        s.eng.db.mark_ruled(&[a, b]).unwrap();
        let n = s.eng.run_rules(&[rules[0].id.clone()], s.inbox).await.unwrap();
        assert_eq!(n, 1);
        let m = s.eng.db.message(a).unwrap();
        assert!(m.flagged && !m.unread);
        assert!(s.eng.db.message(b).unwrap().unread);
        assert!(new_mail(&s).is_empty());
    }

    #[tokio::test]
    async fn mailbox_cleanup_finds_big_and_old_mail() {
        let s = setup();
        let day = 86_400;
        let put = |uid: u32, size: i64, age_days: i64| {
            s.eng
                .db
                .insert_message("a1", s.inbox, &NewMessage { uid: Some(uid), subject: format!("M{uid}"), size, date: now_secs() - age_days * day, ..Default::default() })
                .unwrap()
                .unwrap()
        };
        let big = put(1, 9_000_000, 3);
        put(2, 20_000, 400);
        put(3, 30_000, 10);
        let usage = s.eng.mailbox_usage("a1").unwrap();
        assert_eq!(usage[0].folder_id, s.inbox);
        assert_eq!((usage[0].count, usage[0].size), (3, 9_050_000));
        let large = s.eng.large_messages("a1", 5_000_000).unwrap();
        assert_eq!(large.iter().map(|m| m.id).collect::<Vec<_>>(), vec![big]);
        assert_eq!(large[0].folder_name.as_deref(), Some("Posta in arrivo"));
        // Prima si contano, poi si archiviano.
        let before = (now_secs() - 180 * day) * 1000;
        assert_eq!(s.eng.older_messages(s.inbox, before, false).await.unwrap(), 1);
        assert_eq!(s.eng.older_messages(s.inbox, before, true).await.unwrap(), 1);
        assert_eq!(s.eng.older_messages(s.inbox, before, false).await.unwrap(), 0);
        let archive = s.eng.db.folder_by_role("a1", "archive").unwrap().unwrap();
        assert_eq!(s.eng.db.folder_message_ids(archive.id).unwrap().len(), 1);
    }

    #[test]
    fn contacts_import_merge_export_and_suggest() {
        let s = setup();
        let dir = std::env::temp_dir().join(format!("better-look-rubrica-{}", mime::random_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv = dir.join("outlook.csv");
        std::fs::write(&csv, "First Name,Last Name,Company,E-mail Address,Mobile Phone\nDavide,Orlandi,Studio Orlandi,d.orlandi@studiorlandi.it,333 1234567\nLucia,Marchetti,,l.marchetti@tessuti.example,\n").unwrap();
        let r = s.eng.import_contacts(&csv).unwrap();
        assert_eq!((r.added, r.updated, r.unchanged), (2, 0, 0));
        // Lo stesso contatto da un'altra rubrica: si completa, non si duplica.
        let vcf = dir.join("apple.vcf");
        std::fs::write(&vcf, "BEGIN:VCARD\nVERSION:3.0\nN:Orlandi;Davide;;;\nEMAIL:D.Orlandi@studiorlandi.it\nTITLE:Architetto\nEND:VCARD\nBEGIN:VCARD\nVERSION:3.0\nFN:Lucia Marchetti\nEMAIL:l.marchetti@tessuti.example\nEND:VCARD\n").unwrap();
        let r = s.eng.import_contacts(&vcf).unwrap();
        assert_eq!((r.added, r.updated, r.unchanged), (0, 1, 1));
        let book = s.eng.address_book().unwrap();
        assert_eq!(book.iter().map(|c| c.name()).collect::<Vec<_>>(), vec!["Lucia Marchetti".to_string(), "Davide Orlandi".to_string()]);
        let davide = book.iter().find(|c| c.last_name == "Orlandi").unwrap();
        assert_eq!(davide.job_title, "Architetto");
        assert_eq!(davide.emails.len(), 1);
        // Un file senza contatti dà un errore chiaro.
        let bad = dir.join("vuoto.csv");
        std::fs::write(&bad, "x,y\n").unwrap();
        assert!(s.eng.import_contacts(&bad).is_err());
        // Suggerimenti nei destinatari: la rubrica viene prima.
        let sugg = s.eng.contacts("orla").unwrap();
        assert_eq!(sugg[0].email, "d.orlandi@studiorlandi.it");
        assert_eq!(sugg[0].name, "Davide Orlandi");
        // Esportazione in vCard.
        let out = dir.join("rubrica.vcf");
        assert_eq!(s.eng.export_contacts(&out, None).unwrap(), 2);
        assert_eq!(crate::contacts::parse_vcard(&std::fs::read_to_string(&out).unwrap()).len(), 2);
        // Modifica ed eliminazione.
        let mut c = davide.clone();
        c.emails.push("non valido".into());
        assert!(s.eng.save_contact(c).is_err());
        assert!(s.eng.save_contact(Contact::default()).is_err());
        s.eng.delete_contacts(std::slice::from_ref(&davide.id)).unwrap();
        assert_eq!(s.eng.address_book().unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rules_need_a_name_and_an_action() {
        let s = setup();
        let r = |name: &str, then: RuleActions| Rule { id: String::new(), name: name.into(), enabled: true, account_id: None, when: RuleConditions::default(), then };
        assert!(s.eng.save_rules(vec![r(" ", RuleActions { flag: true, ..Default::default() })]).is_err());
        assert!(s.eng.save_rules(vec![r("Niente", RuleActions::default())]).is_err());
        let saved = s.eng.save_rules(vec![r("Tutto letto", RuleActions { mark_read: true, ..Default::default() })]).unwrap();
        assert!(!saved[0].id.is_empty());
        assert_eq!(s.eng.rules().unwrap(), saved);
    }

    #[test]
    fn auto_reply_periods() {
        let s = setup();
        let ar = |start: Option<i64>, end: Option<i64>| AutoReply { enabled: true, start, end, text: "Fuori ufficio".into(), ..Default::default() };
        assert!(s.eng.set_auto_reply("a1", ar(Some(2_000), Some(1_000))).is_err());
        assert!(s.eng.set_auto_reply("a1", AutoReply { enabled: true, ..Default::default() }).is_err());
        s.eng.set_auto_reply("a1", ar(Some(1_000), Some(5_000))).unwrap();
        let saved = s.eng.db.account("a1").unwrap().auto_reply.unwrap();
        assert!(saved.since > 0);
        assert!(saved.active_at(3_000) && !saved.active_at(500) && !saved.active_at(5_000));
        // Salvare di nuovo senza cambiare periodo non ricomincia da capo.
        s.eng.set_auto_reply("a1", ar(Some(1_000), Some(9_000))).unwrap();
        assert_eq!(s.eng.db.account("a1").unwrap().auto_reply.unwrap().since, saved.since);
    }
}
