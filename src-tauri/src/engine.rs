//! Il motore: tiene insieme archivio, account e server. Ogni account ha un
//! lavoratore che sincronizza in sottofondo; un altro lavoratore spedisce la
//! posta in uscita. L'interfaccia riceve notifiche quando qualcosa cambia.

use std::collections::HashMap;
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
use crate::sanitize;
use crate::secrets::Secrets;
use crate::smtp;

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
            smtp::send(&cfg, &pw, &item.envelope, &item.raw).await?;
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
            return Err(Error::user("Questo indirizzo è già in Datini."));
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

    pub fn contacts(&self, prefix: &str) -> Result<Vec<Person>> {
        self.db.contacts(prefix, 8)
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
        })
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
    // Bozze e invio
    // ------------------------------------------------------------------

    async fn load_attachments(&self, list: &[DraftAttachment]) -> Result<Vec<AttachmentData>> {
        let mut out = Vec::new();
        for a in list {
            if let Some(src) = &a.source {
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
        if d.to.is_empty() && d.cc.is_empty() && d.bcc.is_empty() {
            return Err(Error::user("Aggiungi almeno un destinatario."));
        }
        let acct = self.db.account(&d.account_id)?;
        if acct.kind == AccountKind::Local || acct.outgoing.is_none() {
            return Err(Error::user("Da questo account non si può inviare posta."));
        }
        let atts = self.load_attachments(&d.attachments).await?;
        let built = mime::build(&acct, d, &atts, false)?;
        let id = self.db.outbox_add(&acct.id, &built.wire, &built.copy, &built.envelope, d, now_secs() + self.undo_secs)?;
        let rows = self.db.local_drafts_by_key(&acct.id, &d.key)?;
        self.db.set_pending(&rows, Some("sending"))?;
        if !rows.is_empty() {
            let drafts = self.db.folder_by_role(&acct.id, "drafts")?.map(|f| f.id);
            self.changed(&acct.id, drafts.into_iter().collect());
        }
        self.emit(Event::Outbox { account_id: acct.id.clone(), outbox_id: id, state: "attesa".into(), error: None });
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
