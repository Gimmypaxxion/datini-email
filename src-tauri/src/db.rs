//! Archivio locale in SQLite: account, cartelle, messaggi, testo per la
//! ricerca (FTS5), operazioni in attesa di arrivare al server e posta in uscita.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::*;

const SCHEMA: &str = r#"
PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS accounts(
  id TEXT PRIMARY KEY,
  position INTEGER NOT NULL,
  config TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS folders(
  id INTEGER PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  name TEXT NOT NULL,
  role TEXT,
  delim TEXT,
  selectable INTEGER NOT NULL DEFAULT 1,
  uidvalidity INTEGER,
  highest_uid INTEGER NOT NULL DEFAULT 0,
  uidnext INTEGER,
  last_sync INTEGER,
  UNIQUE(account_id, path)
);
CREATE TABLE IF NOT EXISTS messages(
  id INTEGER PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
  uid INTEGER,
  message_id TEXT,
  draft_key TEXT,
  pop_uidl TEXT,
  date INTEGER NOT NULL,
  from_name TEXT NOT NULL DEFAULT '',
  from_email TEXT NOT NULL DEFAULT '',
  recipients TEXT NOT NULL DEFAULT '{}',
  subject TEXT NOT NULL DEFAULT '',
  preview TEXT NOT NULL DEFAULT '',
  unread INTEGER NOT NULL DEFAULT 0,
  flagged INTEGER NOT NULL DEFAULT 0,
  answered INTEGER NOT NULL DEFAULT 0,
  draft INTEGER NOT NULL DEFAULT 0,
  has_atts INTEGER NOT NULL DEFAULT 0,
  focused INTEGER NOT NULL DEFAULT 1,
  size INTEGER NOT NULL DEFAULT 0,
  has_body INTEGER NOT NULL DEFAULT 0,
  refs TEXT NOT NULL DEFAULT '',
  in_reply_to TEXT NOT NULL DEFAULT '',
  pending TEXT,
  dirty INTEGER NOT NULL DEFAULT 0,
  added_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS msg_folder_date ON messages(folder_id, date DESC);
CREATE UNIQUE INDEX IF NOT EXISTS msg_folder_uid ON messages(folder_id, uid) WHERE uid IS NOT NULL;
CREATE INDEX IF NOT EXISTS msg_folder_mid ON messages(folder_id, message_id);
CREATE INDEX IF NOT EXISTS msg_from ON messages(from_email);
CREATE TABLE IF NOT EXISTS bodies(
  message_id INTEGER PRIMARY KEY REFERENCES messages(id) ON DELETE CASCADE,
  raw BLOB NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS search USING fts5(
  subject, people, body,
  content = '', contentless_delete = 1,
  tokenize = 'unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS msg_search_del AFTER DELETE ON messages BEGIN
  DELETE FROM search WHERE rowid = old.id;
END;
CREATE TABLE IF NOT EXISTS ops(
  id INTEGER PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  payload TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS outbox(
  id INTEGER PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  raw BLOB NOT NULL,
  copy BLOB NOT NULL,
  envelope TEXT NOT NULL,
  draft TEXT NOT NULL,
  send_after INTEGER NOT NULL,
  state TEXT NOT NULL DEFAULT 'attesa',
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS pop_seen(
  account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  uidl TEXT NOT NULL,
  downloaded_at INTEGER NOT NULL,
  PRIMARY KEY(account_id, uidl)
);
"#;

/// Bersaglio di un'operazione sul server: per UID, o per Message-ID quando
/// l'UID non è ancora noto (messaggio appena spostato o salvato).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Target {
    pub uid: Option<u32>,
    pub message_id: Option<String>,
}

/// Operazioni fatte sul computer che devono ancora arrivare al server IMAP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    #[serde(rename_all = "camelCase")]
    Flags {
        folder: String,
        targets: Vec<Target>,
        seen: Option<bool>,
        flagged: Option<bool>,
        answered: Option<bool>,
        local_ids: Vec<i64>,
    },
    #[serde(rename_all = "camelCase")]
    Move { folder: String, dest: String, targets: Vec<Target>, local_ids: Vec<i64> },
    #[serde(rename_all = "camelCase")]
    Expunge { folder: String, targets: Vec<Target> },
    #[serde(rename_all = "camelCase")]
    Append { folder: String, local_id: i64, flags: String },
    #[serde(rename_all = "camelCase")]
    CreateFolder { path: String },
    /// Toglie dal server le versioni precedenti di una bozza.
    #[serde(rename_all = "camelCase")]
    DeleteDraftVersions { folder: String, draft_key: String, keep_message_id: Option<String> },
}

#[derive(Debug, Clone)]
pub struct FolderRow {
    pub id: i64,
    pub account_id: String,
    pub path: String,
    pub name: String,
    pub role: Option<String>,
    pub delim: Option<String>,
    pub selectable: bool,
    pub uidvalidity: Option<u32>,
    pub highest_uid: u32,
    pub uidnext: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MsgRow {
    pub id: i64,
    pub account_id: String,
    pub folder_id: i64,
    pub uid: Option<u32>,
    pub message_id: Option<String>,
    pub draft_key: Option<String>,
    pub date: i64,
    pub from: Person,
    pub recipients: Recipients,
    pub subject: String,
    pub preview: String,
    pub unread: bool,
    pub flagged: bool,
    pub answered: bool,
    pub draft: bool,
    pub has_atts: bool,
    pub focused: bool,
    pub size: i64,
    pub has_body: bool,
    pub refs: String,
    pub in_reply_to: String,
    pub pending: Option<String>,
}

impl MsgRow {
    pub fn summary(&self) -> MessageSummary {
        MessageSummary {
            id: self.id,
            account_id: self.account_id.clone(),
            folder_id: self.folder_id,
            from: self.from.clone(),
            to: self.recipients.to.clone(),
            subject: self.subject.clone(),
            preview: self.preview.clone(),
            date: self.date * 1000,
            unread: self.unread,
            flagged: self.flagged,
            answered: self.answered,
            has_attachments: self.has_atts,
            focused: self.focused,
            draft: self.draft,
            has_body: self.has_body,
            folder_name: None,
        }
    }

    pub fn target(&self) -> Target {
        Target { uid: self.uid, message_id: self.message_id.clone() }
    }
}

/// Dati di un messaggio da inserire nell'archivio.
#[derive(Debug, Clone, Default)]
pub struct NewMessage {
    pub uid: Option<u32>,
    pub message_id: Option<String>,
    pub draft_key: Option<String>,
    pub pop_uidl: Option<String>,
    pub date: i64,
    pub from: Person,
    pub recipients: Recipients,
    pub subject: String,
    pub preview: String,
    pub unread: bool,
    pub flagged: bool,
    pub answered: bool,
    pub draft: bool,
    pub has_atts: bool,
    pub focused: bool,
    pub size: i64,
    pub refs: String,
    pub in_reply_to: String,
    pub pending: Option<String>,
    /// Messaggio completo, se già scaricato.
    pub raw: Option<Vec<u8>>,
    /// Testo per la ricerca.
    pub body_text: String,
}

#[derive(Debug, Clone)]
pub struct OutboxRow {
    pub id: i64,
    pub account_id: String,
    pub raw: Vec<u8>,
    pub copy: Vec<u8>,
    pub envelope: Envelope,
    pub draft: Draft,
    pub send_after: i64,
    pub state: String,
    pub attempts: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub from: String,
    pub to: Vec<String>,
}

const MSG_COLS: &str = "id, account_id, folder_id, uid, message_id, draft_key, date, from_name, from_email, \
    recipients, subject, preview, unread, flagged, answered, draft, has_atts, focused, size, has_body, refs, \
    in_reply_to, pending";

fn msg_from_row(r: &Row) -> rusqlite::Result<MsgRow> {
    let recipients: String = r.get(9)?;
    Ok(MsgRow {
        id: r.get(0)?,
        account_id: r.get(1)?,
        folder_id: r.get(2)?,
        uid: r.get::<_, Option<i64>>(3)?.map(|u| u as u32),
        message_id: r.get(4)?,
        draft_key: r.get(5)?,
        date: r.get(6)?,
        from: Person { name: r.get(7)?, email: r.get(8)? },
        recipients: serde_json::from_str(&recipients).unwrap_or_default(),
        subject: r.get(10)?,
        preview: r.get(11)?,
        unread: r.get(12)?,
        flagged: r.get(13)?,
        answered: r.get(14)?,
        draft: r.get(15)?,
        has_atts: r.get(16)?,
        focused: r.get(17)?,
        size: r.get(18)?,
        has_body: r.get(19)?,
        refs: r.get(20)?,
        in_reply_to: r.get(21)?,
        pending: r.get(22)?,
    })
}

fn folder_from_row(r: &Row) -> rusqlite::Result<FolderRow> {
    Ok(FolderRow {
        id: r.get(0)?,
        account_id: r.get(1)?,
        path: r.get(2)?,
        name: r.get(3)?,
        role: r.get(4)?,
        delim: r.get(5)?,
        selectable: r.get(6)?,
        uidvalidity: r.get::<_, Option<i64>>(7)?.map(|v| v as u32),
        highest_uid: r.get::<_, i64>(8)? as u32,
        uidnext: r.get::<_, Option<i64>>(9)?.map(|v| v as u32),
    })
}

const FOLDER_COLS: &str = "id, account_id, path, name, role, delim, selectable, uidvalidity, highest_uid, uidnext";

/// Trasforma ciò che l'utente scrive nella ricerca in una query FTS5:
/// ogni parola deve comparire, anche come inizio di parola più lunga.
pub fn fts_query(input: &str) -> Option<String> {
    let parts: Vec<String> = input
        .split(|c: char| c.is_whitespace() || "\"'()*:^+-,;.<>@".contains(c))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn people_text(from: &Person, rc: &Recipients) -> String {
    let mut s = format!("{} {}", from.name, from.email);
    for p in rc.to.iter().chain(rc.cc.iter()) {
        s.push(' ');
        s.push_str(&p.name);
        s.push(' ');
        s.push_str(&p.email);
    }
    s
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Db> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Self::init(conn)
    }

    pub fn memory() -> Result<Db> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Db> {
        conn.execute_batch(SCHEMA)?;
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Ok(Db { conn: Arc::new(Mutex::new(conn)) })
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    // ---------------- Account ----------------

    pub fn save_account(&self, cfg: &AccountConfig) -> Result<()> {
        let c = self.conn();
        let json = serde_json::to_string(cfg)?;
        let exists: bool = c.query_row("SELECT 1 FROM accounts WHERE id = ?1", [&cfg.id], |_| Ok(true)).optional()?.unwrap_or(false);
        if exists {
            c.execute("UPDATE accounts SET config = ?2 WHERE id = ?1", params![cfg.id, json])?;
        } else {
            let pos: i64 = c.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM accounts", [], |r| r.get(0))?;
            c.execute("INSERT INTO accounts(id, position, config) VALUES (?1, ?2, ?3)", params![cfg.id, pos, json])?;
        }
        Ok(())
    }

    pub fn accounts(&self) -> Result<Vec<AccountConfig>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT config FROM accounts ORDER BY position")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        let mut out = Vec::new();
        for json in rows {
            if let Ok(cfg) = serde_json::from_str::<AccountConfig>(&json?) {
                out.push(cfg);
            }
        }
        Ok(out)
    }

    pub fn account(&self, id: &str) -> Result<AccountConfig> {
        let c = self.conn();
        let json: String = c
            .query_row("SELECT config FROM accounts WHERE id = ?1", [id], |r| r.get(0))
            .optional()?
            .ok_or(Error::NotFound)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn delete_account(&self, id: &str) -> Result<()> {
        let c = self.conn();
        // I messaggi vanno tolti dalla ricerca prima di cancellarli a cascata.
        c.execute("DELETE FROM messages WHERE account_id = ?1", [id])?;
        c.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------------- Cartelle ----------------

    pub fn upsert_folder(&self, account_id: &str, path: &str, name: &str, role: Option<&str>, delim: Option<&str>, selectable: bool) -> Result<i64> {
        let c = self.conn();
        let existing: Option<i64> = c
            .query_row("SELECT id FROM folders WHERE account_id = ?1 AND path = ?2", params![account_id, path], |r| r.get(0))
            .optional()?;
        if let Some(id) = existing {
            c.execute(
                "UPDATE folders SET name = ?2, role = ?3, delim = ?4, selectable = ?5 WHERE id = ?1",
                params![id, name, role, delim, selectable],
            )?;
            return Ok(id);
        }
        c.execute(
            "INSERT INTO folders(account_id, path, name, role, delim, selectable) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![account_id, path, name, role, delim, selectable],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn folders(&self, account_id: &str) -> Result<Vec<FolderRow>> {
        let c = self.conn();
        let mut st = c.prepare(&format!("SELECT {FOLDER_COLS} FROM folders WHERE account_id = ?1 ORDER BY path"))?;
        let rows = st.query_map([account_id], folder_from_row)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn folder(&self, id: i64) -> Result<FolderRow> {
        let c = self.conn();
        c.query_row(&format!("SELECT {FOLDER_COLS} FROM folders WHERE id = ?1"), [id], folder_from_row)
            .optional()?
            .ok_or(Error::NotFound)
    }

    pub fn folder_by_role(&self, account_id: &str, role: &str) -> Result<Option<FolderRow>> {
        let c = self.conn();
        Ok(c.query_row(
            &format!("SELECT {FOLDER_COLS} FROM folders WHERE account_id = ?1 AND role = ?2 ORDER BY id LIMIT 1"),
            params![account_id, role],
            folder_from_row,
        )
        .optional()?)
    }

    pub fn folder_by_path(&self, account_id: &str, path: &str) -> Result<Option<FolderRow>> {
        let c = self.conn();
        Ok(c.query_row(
            &format!("SELECT {FOLDER_COLS} FROM folders WHERE account_id = ?1 AND path = ?2"),
            params![account_id, path],
            folder_from_row,
        )
        .optional()?)
    }

    pub fn delete_folder(&self, id: i64) -> Result<()> {
        let c = self.conn();
        c.execute("DELETE FROM messages WHERE folder_id = ?1", [id])?;
        c.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_folder_state(&self, id: i64, uidvalidity: Option<u32>, highest_uid: u32, uidnext: Option<u32>) -> Result<()> {
        self.conn().execute(
            "UPDATE folders SET uidvalidity = ?2, highest_uid = ?3, uidnext = ?4, last_sync = ?5 WHERE id = ?1",
            params![id, uidvalidity, highest_uid, uidnext, now_secs()],
        )?;
        Ok(())
    }

    /// Cartelle di tutti gli account, con i conteggi per il riquadro.
    pub fn folder_infos(&self) -> Result<Vec<FolderInfo>> {
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT f.id, f.account_id, f.path, f.name, f.role, f.delim, f.selectable,
                    COALESCE(SUM(m.unread), 0), COUNT(m.id)
             FROM folders f LEFT JOIN messages m ON m.folder_id = f.id
             GROUP BY f.id",
        )?;
        let rows = st.query_map([], |r| {
            let path: String = r.get(2)?;
            let delim: Option<String> = r.get(5)?;
            let depth = match &delim {
                Some(d) if !d.is_empty() => path.matches(d.as_str()).count() as u32,
                _ => 0,
            };
            Ok(FolderInfo {
                id: r.get(0)?,
                account_id: r.get(1)?,
                path,
                name: r.get(3)?,
                role: r.get(4)?,
                depth,
                selectable: r.get(6)?,
                unread: r.get(7)?,
                total: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ---------------- Messaggi ----------------

    /// Inserisce un messaggio. Se nella cartella c'è già la copia locale dello
    /// stesso messaggio (stesso Message-ID, ancora senza UID), la collega a
    /// quella del server invece di duplicarla. Restituisce `None` se il
    /// messaggio è una versione vecchia di una bozza aperta qui.
    pub fn insert_message(&self, account_id: &str, folder_id: i64, m: &NewMessage) -> Result<Option<i64>> {
        let c = self.conn();
        if let Some(uid) = m.uid {
            let existing: Option<i64> = c
                .query_row("SELECT id FROM messages WHERE folder_id = ?1 AND uid = ?2", params![folder_id, uid], |r| r.get(0))
                .optional()?;
            if existing.is_some() {
                return Ok(existing);
            }
            if let Some(mid) = &m.message_id {
                let local: Option<i64> = c
                    .query_row(
                        "SELECT id FROM messages WHERE folder_id = ?1 AND uid IS NULL AND message_id = ?2 LIMIT 1",
                        params![folder_id, mid],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(id) = local {
                    c.execute(
                        "UPDATE messages SET uid = ?2, pending = NULL, unread = ?3, flagged = ?4, answered = ?5 WHERE id = ?1",
                        params![id, uid, m.unread, m.flagged, m.answered],
                    )?;
                    return Ok(Some(id));
                }
            }
            if let Some(key) = &m.draft_key {
                let newer_local: bool = c
                    .query_row(
                        "SELECT 1 FROM messages WHERE folder_id = ?1 AND uid IS NULL AND draft_key = ?2 LIMIT 1",
                        params![folder_id, key],
                        |_| Ok(true),
                    )
                    .optional()?
                    .unwrap_or(false);
                if newer_local {
                    return Ok(None);
                }
            }
        }
        let recipients = serde_json::to_string(&m.recipients)?;
        c.execute(
            "INSERT INTO messages(account_id, folder_id, uid, message_id, draft_key, pop_uidl, date, from_name, from_email,
               recipients, subject, preview, unread, flagged, answered, draft, has_atts, focused, size, has_body, refs,
               in_reply_to, pending, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
            params![
                account_id,
                folder_id,
                m.uid,
                m.message_id,
                m.draft_key,
                m.pop_uidl,
                m.date,
                m.from.name,
                m.from.email,
                recipients,
                m.subject,
                m.preview,
                m.unread,
                m.flagged,
                m.answered,
                m.draft,
                m.has_atts,
                m.focused,
                m.size,
                m.raw.is_some(),
                m.refs,
                m.in_reply_to,
                m.pending,
                now_secs()
            ],
        )?;
        let id = c.last_insert_rowid();
        if let Some(raw) = &m.raw {
            c.execute("INSERT OR REPLACE INTO bodies(message_id, raw) VALUES (?1, ?2)", params![id, raw])?;
        }
        c.execute(
            "INSERT INTO search(rowid, subject, people, body) VALUES (?1, ?2, ?3, ?4)",
            params![id, m.subject, people_text(&m.from, &m.recipients), m.body_text],
        )?;
        Ok(Some(id))
    }

    /// Salva il messaggio completo scaricato in un secondo momento.
    pub fn set_body(&self, id: i64, raw: &[u8], preview: &str, has_atts: bool, body_text: &str) -> Result<()> {
        let c = self.conn();
        c.execute("INSERT OR REPLACE INTO bodies(message_id, raw) VALUES (?1, ?2)", params![id, raw])?;
        c.execute(
            "UPDATE messages SET has_body = 1, preview = ?2, has_atts = ?3 WHERE id = ?1",
            params![id, preview, has_atts],
        )?;
        let (subject, from_name, from_email, recipients): (String, String, String, String) = c.query_row(
            "SELECT subject, from_name, from_email, recipients FROM messages WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        let rc: Recipients = serde_json::from_str(&recipients).unwrap_or_default();
        c.execute("DELETE FROM search WHERE rowid = ?1", [id])?;
        c.execute(
            "INSERT INTO search(rowid, subject, people, body) VALUES (?1, ?2, ?3, ?4)",
            params![id, subject, people_text(&Person { name: from_name, email: from_email }, &rc), body_text],
        )?;
        Ok(())
    }

    pub fn body(&self, id: i64) -> Result<Option<Vec<u8>>> {
        Ok(self.conn().query_row("SELECT raw FROM bodies WHERE message_id = ?1", [id], |r| r.get(0)).optional()?)
    }

    pub fn message(&self, id: i64) -> Result<MsgRow> {
        let c = self.conn();
        c.query_row(&format!("SELECT {MSG_COLS} FROM messages WHERE id = ?1"), [id], msg_from_row)
            .optional()?
            .ok_or(Error::NotFound)
    }

    pub fn messages_by_ids(&self, ids: &[i64]) -> Result<Vec<MsgRow>> {
        let mut out = Vec::new();
        for id in ids {
            if let Ok(m) = self.message(*id) {
                out.push(m);
            }
        }
        Ok(out)
    }

    pub fn list_messages(&self, folder_id: i64, focused: Option<bool>, limit: i64, offset: i64) -> Result<Vec<MsgRow>> {
        let c = self.conn();
        let (sql, focus) = match focused {
            Some(f) => (
                format!("SELECT {MSG_COLS} FROM messages WHERE folder_id = ?1 AND focused = ?4 AND (pending IS NULL OR pending != 'sending') ORDER BY date DESC, id DESC LIMIT ?2 OFFSET ?3"),
                f,
            ),
            None => (
                format!("SELECT {MSG_COLS} FROM messages WHERE folder_id = ?1 AND ?4 = ?4 AND (pending IS NULL OR pending != 'sending') ORDER BY date DESC, id DESC LIMIT ?2 OFFSET ?3"),
                true,
            ),
        };
        let mut st = c.prepare(&sql)?;
        let rows = st.query_map(params![folder_id, limit, offset, focus], msg_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Cerca nel testo, nell'oggetto e nelle persone. Senza account cerca ovunque.
    pub fn search(&self, query: &str, account_id: Option<&str>, limit: i64) -> Result<Vec<(MsgRow, String)>> {
        let Some(q) = fts_query(query) else { return Ok(Vec::new()) };
        let c = self.conn();
        let cols = MSG_COLS.split(", ").map(|col| format!("m.{col}")).collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT {cols}, f.name FROM search s JOIN messages m ON m.id = s.rowid JOIN folders f ON f.id = m.folder_id
             WHERE search MATCH ?1 AND (?2 IS NULL OR m.account_id = ?2)
             ORDER BY m.date DESC LIMIT ?3"
        );
        let mut st = c.prepare(&sql)?;
        let rows = st.query_map(params![q, account_id, limit], |r| Ok((msg_from_row(r)?, r.get::<_, String>(23)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// UID dei messaggi del server presenti nella cartella.
    pub fn folder_uids(&self, folder_id: i64) -> Result<Vec<(i64, u32)>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT id, uid FROM messages WHERE folder_id = ?1 AND uid IS NOT NULL")?;
        let rows = st.query_map([folder_id], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u32)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_messages(&self, ids: &[i64]) -> Result<()> {
        let c = self.conn();
        for id in ids {
            c.execute("DELETE FROM messages WHERE id = ?1", [id])?;
        }
        Ok(())
    }

    /// Applica i flag letti dal server, tranne dove ci sono modifiche locali
    /// non ancora inviate.
    pub fn apply_server_flags(&self, folder_id: i64, flags: &[(u32, bool, bool, bool)]) -> Result<usize> {
        let c = self.conn();
        let mut changed = 0;
        let mut st = c.prepare(
            "UPDATE messages SET unread = ?3, flagged = ?4, answered = ?5
             WHERE folder_id = ?1 AND uid = ?2 AND dirty = 0 AND (unread != ?3 OR flagged != ?4 OR answered != ?5)",
        )?;
        for (uid, seen, flagged, answered) in flags {
            changed += st.execute(params![folder_id, uid, !seen, flagged, answered])?;
        }
        Ok(changed)
    }

    /// Messaggi della cartella senza testo completo, dal più recente, entro la finestra indicata.
    pub fn missing_bodies(&self, folder_id: i64, since: Option<i64>, max_size: Option<i64>, limit: i64) -> Result<Vec<(i64, u32, i64)>> {
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT id, uid, size FROM messages
             WHERE folder_id = ?1 AND has_body = 0 AND uid IS NOT NULL
               AND (?2 IS NULL OR date >= ?2) AND (?3 IS NULL OR size <= ?3)
             ORDER BY date DESC LIMIT ?4",
        )?;
        let rows = st.query_map(params![folder_id, since, max_size, limit], |r| Ok((r.get(0)?, r.get::<_, i64>(1)? as u32, r.get(2)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn set_local_flags(&self, ids: &[i64], seen: Option<bool>, flagged: Option<bool>, answered: Option<bool>, dirty: bool) -> Result<()> {
        let c = self.conn();
        for id in ids {
            if let Some(s) = seen {
                c.execute("UPDATE messages SET unread = ?2, dirty = dirty OR ?3 WHERE id = ?1", params![id, !s, dirty])?;
            }
            if let Some(f) = flagged {
                c.execute("UPDATE messages SET flagged = ?2, dirty = dirty OR ?3 WHERE id = ?1", params![id, f, dirty])?;
            }
            if let Some(a) = answered {
                c.execute("UPDATE messages SET answered = ?2, dirty = dirty OR ?3 WHERE id = ?1", params![id, a, dirty])?;
            }
        }
        Ok(())
    }

    pub fn clear_dirty(&self, ids: &[i64]) -> Result<()> {
        let c = self.conn();
        for id in ids {
            c.execute("UPDATE messages SET dirty = 0 WHERE id = ?1", [id])?;
        }
        Ok(())
    }

    /// Sposta sul computer. I messaggi del server perdono l'UID: lo
    /// riprenderanno dalla cartella di destinazione alla prossima sincronizzazione.
    pub fn local_move(&self, ids: &[i64], dest: i64, server_backed: bool) -> Result<()> {
        let c = self.conn();
        for id in ids {
            if server_backed {
                c.execute(
                    "UPDATE messages SET folder_id = ?2, uid = NULL, pending = 'move', dirty = 0 WHERE id = ?1",
                    params![id, dest],
                )?;
            } else {
                c.execute("UPDATE messages SET folder_id = ?2 WHERE id = ?1", params![id, dest])?;
            }
        }
        Ok(())
    }

    pub fn set_pending(&self, ids: &[i64], pending: Option<&str>) -> Result<()> {
        let c = self.conn();
        for id in ids {
            c.execute("UPDATE messages SET pending = ?2 WHERE id = ?1", params![id, pending])?;
        }
        Ok(())
    }

    /// Dopo la sincronizzazione di una cartella: le copie locali già arrivate
    /// al server ma non riconosciute (senza Message-ID) sono doppioni.
    pub fn drop_done_placeholders(&self, folder_id: i64) -> Result<usize> {
        Ok(self.conn().execute("DELETE FROM messages WHERE folder_id = ?1 AND uid IS NULL AND pending = 'done'", [folder_id])?)
    }

    pub fn local_drafts_by_key(&self, account_id: &str, key: &str) -> Result<Vec<i64>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT id FROM messages WHERE account_id = ?1 AND draft_key = ?2")?;
        let rows = st.query_map(params![account_id, key], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn update_message_raw(&self, id: i64, m: &NewMessage) -> Result<()> {
        let c = self.conn();
        let recipients = serde_json::to_string(&m.recipients)?;
        c.execute(
            "UPDATE messages SET message_id = ?2, date = ?3, from_name = ?4, from_email = ?5, recipients = ?6, subject = ?7,
               preview = ?8, has_atts = ?9, size = ?10, has_body = 1, refs = ?11, in_reply_to = ?12 WHERE id = ?1",
            params![id, m.message_id, m.date, m.from.name, m.from.email, recipients, m.subject, m.preview, m.has_atts, m.size, m.refs, m.in_reply_to],
        )?;
        if let Some(raw) = &m.raw {
            c.execute("INSERT OR REPLACE INTO bodies(message_id, raw) VALUES (?1, ?2)", params![id, raw])?;
        }
        c.execute("DELETE FROM search WHERE rowid = ?1", [id])?;
        c.execute(
            "INSERT INTO search(rowid, subject, people, body) VALUES (?1, ?2, ?3, ?4)",
            params![id, m.subject, people_text(&m.from, &m.recipients), m.body_text],
        )?;
        Ok(())
    }

    /// Indirizzi già usati, per suggerire i destinatari mentre si scrive.
    pub fn contacts(&self, prefix: &str, limit: i64) -> Result<Vec<Person>> {
        let c = self.conn();
        let like = format!("%{}%", prefix.replace(['%', '_'], ""));
        let mut st = c.prepare(
            "SELECT from_name, from_email, COUNT(*) AS n FROM messages
             WHERE from_email != '' AND (from_email LIKE ?1 OR from_name LIKE ?1)
             GROUP BY lower(from_email) ORDER BY n DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![like, limit], |r| Ok(Person { name: r.get(0)?, email: r.get(1)? }))?;
        let mut out: Vec<Person> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        // Anche i destinatari dei messaggi inviati.
        let mut st = c.prepare(
            "SELECT m.recipients FROM messages m JOIN folders f ON f.id = m.folder_id
             WHERE f.role = 'sent' AND m.recipients LIKE ?1 ORDER BY m.date DESC LIMIT 200",
        )?;
        let low = prefix.to_lowercase();
        for json in st.query_map([&like], |r| r.get::<_, String>(0))? {
            let rc: Recipients = serde_json::from_str(&json?).unwrap_or_default();
            for p in rc.to.into_iter().chain(rc.cc) {
                if (p.email.to_lowercase().contains(&low) || p.name.to_lowercase().contains(&low))
                    && !out.iter().any(|o| o.email.eq_ignore_ascii_case(&p.email))
                {
                    out.push(p);
                }
            }
        }
        out.truncate(limit as usize);
        Ok(out)
    }

    // ---------------- Operazioni in attesa ----------------

    pub fn push_op(&self, account_id: &str, op: &Op) -> Result<i64> {
        let c = self.conn();
        c.execute(
            "INSERT INTO ops(account_id, payload, created_at) VALUES (?1, ?2, ?3)",
            params![account_id, serde_json::to_string(op)?, now_secs()],
        )?;
        Ok(c.last_insert_rowid())
    }

    pub fn ops(&self, account_id: &str) -> Result<Vec<(i64, Op, i64)>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT id, payload, attempts FROM ops WHERE account_id = ?1 ORDER BY id")?;
        let rows = st.query_map([account_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, json, attempts) = row?;
            match serde_json::from_str::<Op>(&json) {
                Ok(op) => out.push((id, op, attempts)),
                Err(_) => {
                    c.execute("DELETE FROM ops WHERE id = ?1", [id])?;
                }
            }
        }
        Ok(out)
    }

    pub fn delete_op(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM ops WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn op_failed(&self, id: i64, err: &str) -> Result<()> {
        self.conn().execute("UPDATE ops SET attempts = attempts + 1, last_error = ?2 WHERE id = ?1", params![id, err])?;
        Ok(())
    }

    pub fn pending_ops_count(&self, account_id: &str) -> Result<i64> {
        Ok(self.conn().query_row("SELECT COUNT(*) FROM ops WHERE account_id = ?1", [account_id], |r| r.get(0))?)
    }

    // ---------------- Posta in uscita ----------------

    pub fn outbox_add(&self, account_id: &str, raw: &[u8], copy: &[u8], env: &Envelope, draft: &Draft, send_after: i64) -> Result<i64> {
        let c = self.conn();
        c.execute(
            "INSERT INTO outbox(account_id, raw, copy, envelope, draft, send_after, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![account_id, raw, copy, serde_json::to_string(env)?, serde_json::to_string(draft)?, send_after, now_secs()],
        )?;
        Ok(c.last_insert_rowid())
    }

    fn outbox_from_row(r: &Row) -> rusqlite::Result<OutboxRow> {
        let env: String = r.get(3)?;
        let draft: String = r.get(4)?;
        Ok(OutboxRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            raw: r.get(2)?,
            copy: r.get(9)?,
            envelope: serde_json::from_str(&env).unwrap_or(Envelope { from: String::new(), to: vec![] }),
            draft: serde_json::from_str(&draft).unwrap_or_else(|_| Draft {
                key: String::new(),
                account_id: String::new(),
                to: vec![],
                cc: vec![],
                bcc: vec![],
                subject: String::new(),
                body: String::new(),
                attachments: vec![],
                in_reply_to: None,
                references: vec![],
                reply_to_message: None,
            }),
            send_after: r.get(5)?,
            state: r.get(6)?,
            attempts: r.get(7)?,
            last_error: r.get(8)?,
        })
    }

    pub fn outbox_due(&self, now: i64) -> Result<Vec<OutboxRow>> {
        let c = self.conn();
        let mut st = c.prepare(
            "SELECT id, account_id, raw, envelope, draft, send_after, state, attempts, last_error, copy FROM outbox
             WHERE send_after <= ?1 AND state IN ('attesa', 'offline') ORDER BY send_after",
        )?;
        let rows = st.query_map([now], Self::outbox_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn outbox_get(&self, id: i64) -> Result<Option<OutboxRow>> {
        let c = self.conn();
        Ok(c.query_row(
            "SELECT id, account_id, raw, envelope, draft, send_after, state, attempts, last_error, copy FROM outbox WHERE id = ?1",
            [id],
            Self::outbox_from_row,
        )
        .optional()?)
    }

    pub fn outbox_items(&self) -> Result<Vec<OutboxItem>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT id, account_id, raw, envelope, draft, send_after, state, attempts, last_error, copy FROM outbox ORDER BY id")?;
        let rows = st.query_map([], Self::outbox_from_row)?;
        let mut out = Vec::new();
        for r in rows {
            let r = r?;
            out.push(OutboxItem {
                id: r.id,
                account_id: r.account_id,
                subject: r.draft.subject.clone(),
                to: r.draft.to.clone(),
                state: r.state,
                error: r.last_error,
                send_after: r.send_after * 1000,
            });
        }
        Ok(out)
    }

    /// Prende in carico l'invio: restituisce falso se nel frattempo è stato annullato.
    pub fn outbox_claim(&self, id: i64) -> Result<bool> {
        let n = self.conn().execute("UPDATE outbox SET state = 'invio' WHERE id = ?1 AND state IN ('attesa', 'offline')", [id])?;
        Ok(n == 1)
    }

    pub fn outbox_set_state(&self, id: i64, state: &str, error: Option<&str>, retry_at: Option<i64>) -> Result<()> {
        let c = self.conn();
        c.execute(
            "UPDATE outbox SET state = ?2, last_error = ?3, attempts = attempts + 1, send_after = COALESCE(?4, send_after) WHERE id = ?1",
            params![id, state, error, retry_at],
        )?;
        Ok(())
    }

    /// Annulla un invio non ancora partito e restituisce la bozza.
    pub fn outbox_cancel(&self, id: i64) -> Result<Option<Draft>> {
        let c = self.conn();
        let row: Option<(String, String)> = c
            .query_row("SELECT draft, state FROM outbox WHERE id = ?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let Some((draft, state)) = row else { return Ok(None) };
        if state == "invio" {
            return Ok(None);
        }
        c.execute("DELETE FROM outbox WHERE id = ?1", [id])?;
        Ok(serde_json::from_str(&draft).ok())
    }

    pub fn outbox_delete(&self, id: i64) -> Result<()> {
        self.conn().execute("DELETE FROM outbox WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------------- POP3 ----------------

    pub fn pop_seen(&self, account_id: &str) -> Result<Vec<(String, i64)>> {
        let c = self.conn();
        let mut st = c.prepare("SELECT uidl, downloaded_at FROM pop_seen WHERE account_id = ?1")?;
        let rows = st.query_map([account_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn pop_mark_seen(&self, account_id: &str, uidl: &str) -> Result<()> {
        self.conn().execute(
            "INSERT OR IGNORE INTO pop_seen(account_id, uidl, downloaded_at) VALUES (?1, ?2, ?3)",
            params![account_id, uidl, now_secs()],
        )?;
        Ok(())
    }

    pub fn pop_forget(&self, account_id: &str, uidl: &str) -> Result<()> {
        self.conn().execute("DELETE FROM pop_seen WHERE account_id = ?1 AND uidl = ?2", params![account_id, uidl])?;
        Ok(())
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn().query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn().execute("INSERT OR REPLACE INTO meta(key, value) VALUES (?1, ?2)", params![key, value])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(db: &Db) -> String {
        let cfg = AccountConfig {
            id: "a1".into(),
            name: "Prova".into(),
            email: "prova@datini.test".into(),
            kind: AccountKind::Imap,
            incoming: None,
            outgoing: None,
            sync: SyncOptions::default(),
            server_saves_sent: false,
            source_file: None,
        };
        db.save_account(&cfg).unwrap();
        cfg.id
    }

    fn msg(uid: Option<u32>, mid: &str, subject: &str) -> NewMessage {
        NewMessage {
            uid,
            message_id: Some(mid.into()),
            date: 1_700_000_000,
            from: Person::new("Lucia Marchetti", "lucia@marchettitessuti.it"),
            subject: subject.into(),
            focused: true,
            body_text: "campioni del velluto a coste".into(),
            ..Default::default()
        }
    }

    #[test]
    fn insert_list_and_search() {
        let db = Db::memory().unwrap();
        let a = account(&db);
        let f = db.upsert_folder(&a, "INBOX", "Posta in arrivo", Some("inbox"), Some("/"), true).unwrap();
        db.insert_message(&a, f, &msg(Some(1), "<a@x>", "Campionario tessuti")).unwrap();
        db.insert_message(&a, f, &msg(Some(2), "<b@x>", "Fattura n. 418")).unwrap();
        assert_eq!(db.list_messages(f, None, 50, 0).unwrap().len(), 2);
        // la ricerca ignora gli accenti e trova anche l'inizio delle parole
        let r = db.search("vellut", Some(&a), 10).unwrap();
        assert_eq!(r.len(), 2);
        let r = db.search("fattura", None, 10).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, "Posta in arrivo");
        // cancellare un messaggio lo toglie anche dalla ricerca
        db.delete_messages(&[r[0].0.id]).unwrap();
        assert!(db.search("fattura", None, 10).unwrap().is_empty());
    }

    #[test]
    fn server_copy_adopts_local_placeholder() {
        let db = Db::memory().unwrap();
        let a = account(&db);
        let f = db.upsert_folder(&a, "Sent", "Posta inviata", Some("sent"), Some("/"), true).unwrap();
        let local = db.insert_message(&a, f, &NewMessage { pending: Some("append".into()), ..msg(None, "<s@x>", "Inviato") }).unwrap().unwrap();
        let server = db.insert_message(&a, f, &msg(Some(7), "<s@x>", "Inviato")).unwrap().unwrap();
        assert_eq!(local, server);
        let row = db.message(local).unwrap();
        assert_eq!(row.uid, Some(7));
        assert_eq!(row.pending, None);
        assert_eq!(db.list_messages(f, None, 50, 0).unwrap().len(), 1);
    }

    #[test]
    fn local_flags_survive_server_refresh_until_sent() {
        let db = Db::memory().unwrap();
        let a = account(&db);
        let f = db.upsert_folder(&a, "INBOX", "Posta in arrivo", Some("inbox"), Some("/"), true).unwrap();
        let id = db.insert_message(&a, f, &NewMessage { unread: true, ..msg(Some(3), "<c@x>", "Ciao") }).unwrap().unwrap();
        db.set_local_flags(&[id], Some(true), None, None, true).unwrap();
        db.apply_server_flags(f, &[(3, false, false, false)]).unwrap();
        assert!(!db.message(id).unwrap().unread);
        db.clear_dirty(&[id]).unwrap();
        db.apply_server_flags(f, &[(3, false, true, false)]).unwrap();
        let m = db.message(id).unwrap();
        assert!(m.unread && m.flagged);
    }

    #[test]
    fn outbox_cancel_returns_draft_unless_sending() {
        let db = Db::memory().unwrap();
        let a = account(&db);
        let draft = Draft {
            key: "k".into(),
            account_id: a.clone(),
            to: vec![Person::new("", "x@y.it")],
            cc: vec![],
            bcc: vec![],
            subject: "Prova".into(),
            body: "ciao".into(),
            attachments: vec![],
            in_reply_to: None,
            references: vec![],
            reply_to_message: None,
        };
        let env = Envelope { from: "prova@datini.test".into(), to: vec!["x@y.it".into()] };
        let id = db.outbox_add(&a, b"raw", b"copy", &env, &draft, 0).unwrap();
        assert_eq!(db.outbox_cancel(id).unwrap().unwrap().subject, "Prova");
        let id = db.outbox_add(&a, b"raw", b"copy", &env, &draft, 0).unwrap();
        assert!(db.outbox_claim(id).unwrap());
        assert!(db.outbox_cancel(id).unwrap().is_none());
    }

    #[test]
    fn fts_query_is_safe() {
        assert_eq!(fts_query("fattura 418").unwrap(), "\"fattura\"* \"418\"*");
        assert_eq!(fts_query("\"OR\" (x)").unwrap(), "\"OR\"* \"x\"*");
        assert!(fts_query("  ").is_none());
    }
}
