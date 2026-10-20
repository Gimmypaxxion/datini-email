//! Tipi condivisi tra motore e interfaccia. In JSON i nomi dei campi sono in
//! camelCase, come nel codice TypeScript.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Imap,
    Pop3,
    /// File di dati sul computer (per esempio un PST importato).
    Local,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Security {
    /// Connessione cifrata dall'inizio (porte 993, 995, 465).
    Ssl,
    /// Connessione in chiaro che passa subito a cifrata (porte 143, 110, 587).
    Starttls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub security: Security,
    pub username: String,
}

/// Quanta posta tenere sul computer (account IMAP).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum Keep {
    #[serde(rename = "1m")]
    OneMonth,
    #[serde(rename = "3m")]
    #[default]
    ThreeMonths,
    #[serde(rename = "1a")]
    OneYear,
    #[serde(rename = "tutto")]
    All,
}

impl Keep {
    pub fn days(self) -> Option<i64> {
        match self {
            Keep::OneMonth => Some(31),
            Keep::ThreeMonths => Some(92),
            Keep::OneYear => Some(366),
            Keep::All => None,
        }
    }
}

/// Quando scaricare gli allegati (account IMAP).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AttachPolicy {
    #[serde(rename = "subito")]
    Now,
    #[serde(rename = "apertura")]
    #[default]
    OnOpen,
}

/// Per quanto lasciare una copia sul server (account POP).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum PopKeep {
    #[serde(rename = "mai")]
    Never,
    #[serde(rename = "14")]
    #[default]
    Days14,
    #[serde(rename = "sempre")]
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncOptions {
    #[serde(default)]
    pub keep: Keep,
    #[serde(default)]
    pub attachments: AttachPolicy,
    #[serde(default)]
    pub pop_keep: PopKeep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    pub id: String,
    /// Nome di chi scrive, usato come mittente.
    pub name: String,
    pub email: String,
    pub kind: AccountKind,
    #[serde(default)]
    pub incoming: Option<ServerConfig>,
    #[serde(default)]
    pub outgoing: Option<ServerConfig>,
    #[serde(default)]
    pub sync: SyncOptions,
    /// Il server salva da solo i messaggi inviati (come Gmail): non serve
    /// copiarli nella cartella Posta inviata.
    #[serde(default)]
    pub server_saves_sent: bool,
    /// Per i file di dati: percorso del file da cui sono stati importati.
    #[serde(default)]
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountStatus {
    /// "pronto", "sincronizzazione", "offline", "errore"
    pub state: String,
    pub message: Option<String>,
    pub last_sync: Option<i64>,
}

impl Default for AccountStatus {
    fn default() -> Self {
        AccountStatus { state: "pronto".into(), message: None, last_sync: None }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    #[serde(flatten)]
    pub config: AccountConfig,
    pub status: AccountStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderInfo {
    pub id: i64,
    pub account_id: String,
    pub path: String,
    pub name: String,
    pub role: Option<String>,
    pub depth: u32,
    pub selectable: bool,
    pub unread: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
}

impl Person {
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Person { name: name.into(), email: email.into() }
    }
    pub fn label(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.email
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Recipients {
    #[serde(default)]
    pub to: Vec<Person>,
    #[serde(default)]
    pub cc: Vec<Person>,
    #[serde(default)]
    pub bcc: Vec<Person>,
    #[serde(default)]
    pub reply_to: Vec<Person>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummary {
    pub id: i64,
    pub account_id: String,
    pub folder_id: i64,
    pub from: Person,
    pub to: Vec<Person>,
    pub subject: String,
    pub preview: String,
    /// Millisecondi dal 1970, come Date in JavaScript.
    pub date: i64,
    pub unread: bool,
    pub flagged: bool,
    pub answered: bool,
    pub has_attachments: bool,
    pub focused: bool,
    pub draft: bool,
    pub has_body: bool,
    /// Solo nei risultati di ricerca: la cartella del messaggio.
    pub folder_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    pub index: u32,
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub inline: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    #[serde(flatten)]
    pub summary: MessageSummary,
    pub cc: Vec<Person>,
    pub bcc: Vec<Person>,
    pub reply_to: Vec<Person>,
    pub message_id: Option<String>,
    pub references: Vec<String>,
    /// Documento HTML già ripulito, da mostrare in un iframe isolato.
    pub html: Option<String>,
    pub text: String,
    pub attachments: Vec<AttachmentInfo>,
    /// Immagini esterne bloccate per proteggere la privacy.
    pub remote_images: u32,
    pub remote_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentRef {
    pub message_id: i64,
    pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftAttachment {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub mime: Option<String>,
    /// File sul computer.
    #[serde(default)]
    pub path: Option<String>,
    /// Allegato di un messaggio già in archivio (inoltro, bozze riaperte).
    #[serde(default)]
    pub source: Option<AttachmentRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    /// Identifica la bozza tra un salvataggio e l'altro.
    pub key: String,
    pub account_id: String,
    #[serde(default)]
    pub to: Vec<Person>,
    #[serde(default)]
    pub cc: Vec<Person>,
    #[serde(default)]
    pub bcc: Vec<Person>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub attachments: Vec<DraftAttachment>,
    #[serde(default)]
    pub in_reply_to: Option<String>,
    #[serde(default)]
    pub references: Vec<String>,
    /// Messaggio a cui si risponde: dopo l'invio riceve il segno "risposto".
    #[serde(default)]
    pub reply_to_message: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxItem {
    pub id: i64,
    pub account_id: String,
    pub subject: String,
    pub to: Vec<Person>,
    pub state: String,
    pub error: Option<String>,
    pub send_after: i64,
}

/// Notifiche dal motore all'interfaccia.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    Changed { account_id: String, folders: Vec<i64> },
    #[serde(rename_all = "camelCase")]
    Status { account_id: String, status: AccountStatus },
    #[serde(rename_all = "camelCase")]
    Outbox { account_id: String, outbox_id: i64, state: String, error: Option<String> },
    #[serde(rename_all = "camelCase")]
    Accounts,
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
