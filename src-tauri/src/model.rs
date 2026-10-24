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
    /// Firma aggiunta da sola ai nuovi messaggi (identificativo della firma).
    #[serde(default)]
    pub signature_new: Option<String>,
    /// Firma aggiunta da sola a risposte e inoltri.
    #[serde(default)]
    pub signature_reply: Option<String>,
    /// Risposta automatica (fuori ufficio).
    #[serde(default)]
    pub auto_reply: Option<AutoReply>,
}

/// Risposta automatica di un account, come "Risposte automatiche" di Outlook.
/// Le date sono in millisecondi, come Date in JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutoReply {
    #[serde(default)]
    pub enabled: bool,
    /// Inizio e fine del periodo; senza, vale finché resta attiva.
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
    /// Testo formattato (frammento HTML) e testo semplice.
    #[serde(default)]
    pub html: String,
    #[serde(default)]
    pub text: String,
    /// Quando è stata attivata: ogni mittente riceve una sola risposta per
    /// attivazione. Lo imposta il motore.
    #[serde(default)]
    pub since: i64,
}

impl AutoReply {
    /// Attiva adesso (now in millisecondi)?
    pub fn active_at(&self, now: i64) -> bool {
        self.enabled && self.start.map_or(true, |s| now >= s) && self.end.map_or(true, |e| now < e)
    }
}

/// Condizioni di una regola: tutte quelle indicate devono valere. Dentro una
/// condizione basta una delle parole (come "oppure" in Outlook).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuleConditions {
    /// Nome o indirizzo del mittente.
    #[serde(default)]
    pub from: Vec<String>,
    /// Nome o indirizzo di un destinatario (A o Cc).
    #[serde(default)]
    pub to: Vec<String>,
    #[serde(default)]
    pub subject: Vec<String>,
    /// Oggetto o testo del messaggio.
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub has_attachments: bool,
}

impl RuleConditions {
    pub fn is_empty(&self) -> bool {
        let none = |v: &Vec<String>| v.iter().all(|w| w.trim().is_empty());
        none(&self.from) && none(&self.to) && none(&self.subject) && none(&self.words) && !self.has_attachments
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuleActions {
    /// Cartella in cui spostare il messaggio.
    #[serde(default)]
    pub move_to: Option<i64>,
    #[serde(default)]
    pub delete: bool,
    #[serde(default)]
    pub mark_read: bool,
    #[serde(default)]
    pub flag: bool,
    #[serde(default)]
    pub category: Option<String>,
    /// Avviso sul desktop anche se il messaggio finisce fuori dalla posta in arrivo.
    #[serde(default)]
    pub alert: bool,
    /// Non applicare le regole successive.
    #[serde(default)]
    pub stop: bool,
}

/// Regola sui messaggi in arrivo, come "Regole e avvisi" di Outlook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// Solo per questo account; senza, per tutti.
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub when: RuleConditions,
    #[serde(default)]
    pub then: RuleActions,
}

fn yes() -> bool {
    true
}

/// Firma da mettere in fondo ai messaggi. Le firme valgono per tutti gli
/// account; ogni account sceglie quale usare per nuovi messaggi e risposte.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    #[serde(default)]
    pub id: String,
    pub name: String,
    /// Contenuto in HTML (testo formattato, immagini incorporate).
    pub html: String,
}

/// Categoria colorata, come in Outlook. Il colore è uno dei nomi della
/// palette dell'interfaccia (rosso, arancione, giallo, verde...).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub color: String,
}

/// Colori disponibili per le categorie.
pub const CATEGORY_COLORS: [&str; 10] = ["rosso", "arancione", "giallo", "verde", "verdeacqua", "blu", "viola", "rosa", "marrone", "grigio"];

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
    /// Conversazione: Message-ID del primo messaggio della discussione.
    pub thread: Option<String>,
    /// Categorie colorate assegnate (identificativi).
    pub categories: Vec<String>,
    /// Dimensione in byte.
    pub size: i64,
    /// Messaggi della stessa conversazione nell'account.
    pub thread_count: i64,
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
    /// Invito a una riunione, da accettare o rifiutare.
    pub invite: Option<Invite>,
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
    /// Testo semplice (versione per i programmi che non leggono l'HTML).
    #[serde(default)]
    pub body: String,
    /// Testo formattato scritto nell'editor: frammento HTML. Se manca, il
    /// messaggio si compone dal testo semplice.
    #[serde(default)]
    pub html: Option<String>,
    /// Messaggio in testo normale (formato "Testo normale" di Outlook): si
    /// spedisce solo `body`, senza versione HTML.
    #[serde(default)]
    pub plain: bool,
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

/// Numero di telefono di un contatto. `kind`: cellulare, lavoro, casa, fax, altro.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Phone {
    #[serde(default)]
    pub kind: String,
    pub number: String,
}

/// Contatto della rubrica.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    /// Nome da mostrare; se manca si compone da nome e cognome.
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub phones: Vec<Phone>,
    #[serde(default)]
    pub company: String,
    #[serde(default)]
    pub job_title: String,
    /// Indirizzo su più righe.
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub website: String,
    /// "AAAA-MM-GG", oppure "--MM-GG" senza anno.
    #[serde(default)]
    pub birthday: String,
    #[serde(default)]
    pub notes: String,
}

impl Contact {
    /// Nome con cui il contatto compare negli elenchi.
    pub fn name(&self) -> String {
        let d = self.display_name.trim();
        if !d.is_empty() {
            return d.to_string();
        }
        let full = format!("{} {}", self.first_name.trim(), self.last_name.trim()).trim().to_string();
        if !full.is_empty() {
            return full;
        }
        if !self.company.trim().is_empty() {
            return self.company.trim().to_string();
        }
        self.emails.first().cloned().unwrap_or_default()
    }

    /// Ordine alfabetico per cognome, come la rubrica di Outlook.
    pub fn sort_key(&self) -> String {
        let k = if self.last_name.trim().is_empty() { self.name() } else { format!("{} {}", self.last_name.trim(), self.first_name.trim()) };
        k.to_lowercase()
    }

    pub fn is_empty(&self) -> bool {
        self.name().trim().is_empty()
    }
}

/// Esito di un'importazione di contatti.
#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContactImport {
    /// Contatti nuovi.
    pub added: u32,
    /// Contatti già presenti, completati con i dati nuovi.
    pub updated: u32,
    /// Già presenti e uguali.
    pub unchanged: u32,
}

// ---------------- Calendario ----------------

/// Risposta di un partecipante a un invito.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PartStat {
    #[default]
    NeedsAction,
    Accepted,
    Tentative,
    Declined,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

fn one() -> u32 {
    1
}

/// Ricorrenza di un appuntamento, nelle forme che propone Outlook.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Repeat {
    pub freq: Freq,
    #[serde(default = "one")]
    pub interval: u32,
    /// Giorni della settimana (0 domenica … 6 sabato): per la ricorrenza
    /// settimanale, oppure il giorno di "il secondo martedì del mese".
    #[serde(default)]
    pub days: Vec<u8>,
    /// Mensile o annuale per giorno della settimana: 1-4, oppure -1 per l'ultimo.
    #[serde(default)]
    pub week: Option<i8>,
    /// Ultimo giorno della serie (ms, compreso).
    #[serde(default)]
    pub until: Option<i64>,
    /// Numero di occorrenze.
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Attendee {
    #[serde(default)]
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub status: PartStat,
    #[serde(default)]
    pub optional: bool,
}

/// Appuntamento, riunione o evento di tutto il giorno. Orari in millisecondi;
/// per tutto il giorno, mezzanotte locale del primo giorno e del giorno dopo l'ultimo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CalEvent {
    pub id: String,
    /// Identificativo iCalendar, uguale per tutti i calendari dei partecipanti.
    pub uid: String,
    pub title: String,
    pub location: String,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub repeat: Option<Repeat>,
    /// Inizi delle occorrenze tolte dalla serie.
    pub exceptions: Vec<i64>,
    /// Occorrenza di una serie modificata a parte: il suo inizio originale.
    pub recurrence_id: Option<i64>,
    /// Promemoria: minuti prima dell'inizio.
    pub reminder: Option<i32>,
    pub notes: String,
    pub categories: Vec<String>,
    pub organizer: Option<Person>,
    pub attendees: Vec<Attendee>,
    /// La mia risposta a un invito ricevuto (None se non ho risposto o se l'evento è mio).
    pub response: Option<PartStat>,
    /// Account da cui partono gli inviti, o che ha ricevuto l'invito.
    pub account_id: Option<String>,
    pub sequence: i32,
    /// Annullato dall'organizzatore.
    pub cancelled: bool,
}

/// Invito contenuto in un messaggio (parte text/calendar o allegato .ics).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Invite {
    /// request, cancel, reply, publish, counter…
    pub method: String,
    pub event: CalEvent,
    /// Lo stesso evento, se è già nel calendario.
    pub local: Option<CalEvent>,
    /// Nel calendario c'è una versione più recente.
    pub outdated: bool,
    /// Per una risposta: chi ha risposto e come.
    pub replier: Option<Attendee>,
    /// Sono tra i partecipanti.
    pub for_me: bool,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalImport {
    pub added: u32,
    pub updated: u32,
    pub unchanged: u32,
}

/// Spazio occupato da una cartella (Pulizia cassetta postale).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderUsage {
    pub folder_id: i64,
    pub count: i64,
    /// Byte.
    pub size: i64,
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
    /// Avanzamento dell'importazione di un archivio (total 0 se non si conosce).
    #[serde(rename_all = "camelCase")]
    Import { name: String, done: u32, total: u32 },
    /// Le firme sono cambiate (anche le altre finestre devono rileggerle).
    Signatures,
    /// Le categorie sono cambiate.
    Categories,
    /// Messaggi appena arrivati, per l'avviso sul desktop (al massimo 5).
    #[serde(rename_all = "camelCase")]
    NewMail { account_id: String, count: u32, messages: Vec<MessageSummary> },
    /// Le regole sono cambiate.
    Rules,
    /// La rubrica è cambiata.
    Contacts,
    /// Il calendario è cambiato.
    Calendar,
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
