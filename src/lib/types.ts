// Tipi condivisi con il motore in Rust (src-tauri/src/model.rs).
// I nomi dei campi sono in camelCase, come li produce serde.

export type Role = 'inbox' | 'sent' | 'drafts' | 'deleted' | 'junk' | 'archive';
export type AccountKind = 'imap' | 'pop3' | 'local';
export type Security = 'ssl' | 'starttls';

export interface ServerConfig {
  host: string;
  port: number;
  security: Security;
  username: string;
}

export interface SyncOptions {
  keep: '1m' | '3m' | '1a' | 'tutto';
  attachments: 'subito' | 'apertura';
  popKeep: 'mai' | '14' | 'sempre';
}

export type AccountState = 'pronto' | 'sincronizzazione' | 'offline' | 'errore';

export interface AccountStatus {
  state: AccountState;
  message: string | null;
  lastSync: number | null;
}

export interface Account {
  id: string;
  name: string;
  email: string;
  kind: AccountKind;
  incoming: ServerConfig | null;
  outgoing: ServerConfig | null;
  sync: SyncOptions;
  serverSavesSent: boolean;
  sourceFile: string | null;
  /** Firma aggiunta da sola ai nuovi messaggi (identificativo). */
  signatureNew?: string | null;
  /** Firma aggiunta da sola a risposte e inoltri. */
  signatureReply?: string | null;
  /** Risposta automatica (fuori ufficio). */
  autoReply?: AutoReply | null;
  status: AccountStatus;
}

/** Risposta automatica di un account. Date in millisecondi. */
export interface AutoReply {
  enabled: boolean;
  start: number | null;
  end: number | null;
  html: string;
  text: string;
  /** Quando è stata attivata (lo imposta il motore). */
  since?: number;
}

export type PhoneKind = 'cellulare' | 'lavoro' | 'casa' | 'fax' | 'altro';

export const PHONE_KINDS: [PhoneKind, string][] = [
  ['cellulare', 'Cellulare'],
  ['lavoro', 'Lavoro'],
  ['casa', 'Casa'],
  ['fax', 'Fax'],
  ['altro', 'Altro'],
];

export interface Phone {
  kind: PhoneKind;
  number: string;
}

/** Contatto della rubrica. */
export interface Contact {
  id: string;
  firstName: string;
  lastName: string;
  /** Nome da mostrare; se vuoto si compone da nome e cognome. */
  displayName: string;
  emails: string[];
  phones: Phone[];
  company: string;
  jobTitle: string;
  /** Su più righe. */
  address: string;
  website: string;
  /** "AAAA-MM-GG", oppure "--MM-GG" senza anno. */
  birthday: string;
  notes: string;
}

export interface ContactImport {
  added: number;
  updated: number;
  unchanged: number;
}

/** Spazio occupato da una cartella. */
export interface FolderUsage {
  folderId: number;
  count: number;
  /** Byte. */
  size: number;
}

/** Regola sui messaggi in arrivo, come "Regole e avvisi" di Outlook. */
export interface Rule {
  id: string;
  name: string;
  enabled: boolean;
  /** Solo per questo account; null per tutti. */
  accountId: string | null;
  when: RuleConditions;
  then: RuleActions;
}

/** Tutte le condizioni indicate devono valere; dentro una basta una parola. */
export interface RuleConditions {
  from: string[];
  to: string[];
  subject: string[];
  /** Oggetto o testo. */
  words: string[];
  hasAttachments: boolean;
}

export interface RuleActions {
  moveTo: number | null;
  delete: boolean;
  markRead: boolean;
  flag: boolean;
  category: string | null;
  alert: boolean;
  stop: boolean;
}

/** Categoria colorata, come in Outlook. */
export interface Category {
  id: string;
  name: string;
  color: CategoryColor;
}

export type CategoryColor = 'rosso' | 'arancione' | 'giallo' | 'verde' | 'verdeacqua' | 'blu' | 'viola' | 'rosa' | 'marrone' | 'grigio';

/** Colori delle categorie, con il nome da mostrare. */
export const CATEGORY_COLORS: [CategoryColor, string][] = [
  ['rosso', 'Rosso'],
  ['arancione', 'Arancione'],
  ['giallo', 'Giallo'],
  ['verde', 'Verde'],
  ['verdeacqua', 'Verde acqua'],
  ['blu', 'Blu'],
  ['viola', 'Viola'],
  ['rosa', 'Rosa'],
  ['marrone', 'Marrone'],
  ['grigio', 'Grigio'],
];

/** Firma: vale per tutti gli account, ognuno sceglie quale usare. */
export interface Signature {
  id: string;
  name: string;
  html: string;
}

export interface Folder {
  id: number;
  accountId: string;
  path: string;
  name: string;
  role: Role | null;
  depth: number;
  selectable: boolean;
  unread: number;
  total: number;
}

export interface Person {
  name: string;
  email: string;
}

export interface MessageSummary {
  id: number;
  accountId: string;
  folderId: number;
  from: Person;
  to: Person[];
  subject: string;
  preview: string;
  /** Millisecondi, come Date.getTime(). */
  date: number;
  unread: boolean;
  flagged: boolean;
  answered: boolean;
  hasAttachments: boolean;
  focused: boolean;
  draft: boolean;
  hasBody: boolean;
  folderName: string | null;
  /** Conversazione: Message-ID del primo messaggio della discussione. */
  thread: string | null;
  /** Categorie colorate assegnate (identificativi). */
  categories: string[];
  /** Dimensione in byte. */
  size: number;
  /** Messaggi della stessa conversazione nell'account (escluse eliminata e indesiderata). */
  threadCount: number;
  /** Priorità: 1 alta, 0 normale, -1 bassa. */
  importance?: number;
}

export interface AttachmentInfo {
  index: number;
  name: string;
  size: number;
  mime: string;
  inline: boolean;
}

export interface MessageDetail extends MessageSummary {
  cc: Person[];
  bcc: Person[];
  replyTo: Person[];
  messageId: string | null;
  references: string[];
  /** Documento HTML già ripulito dal motore. */
  html: string | null;
  text: string;
  attachments: AttachmentInfo[];
  remoteImages: number;
  remoteAllowed: boolean;
  /** Invito a una riunione contenuto nel messaggio. */
  invite?: Invite | null;
  /** Il mittente chiede la conferma di lettura (e non è ancora stata data). */
  receiptTo?: Person | null;
}

export interface AttachmentRef {
  messageId: number;
  index: number;
}

export interface DraftAttachment {
  name: string;
  size: number;
  mime?: string | null;
  path?: string | null;
  source?: AttachmentRef | null;
  /** Un intero messaggio dell'archivio, allegato come .eml (Inoltra come allegato). */
  message?: number | null;
}

export interface Draft {
  key: string;
  accountId: string;
  to: Person[];
  cc: Person[];
  bcc: Person[];
  subject: string;
  /** Testo semplice, per chi non legge l'HTML. */
  body: string;
  /** Testo formattato scritto nell'editor (frammento HTML). */
  html?: string | null;
  /** Formato "Testo normale": si spedisce solo `body`, senza HTML. */
  plain?: boolean;
  attachments: DraftAttachment[];
  inReplyTo: string | null;
  references: string[];
  replyToMessage: number | null;
  /** Priorità: 'high', 'low' o normale (null). */
  importance?: Importance | null;
  /** Richiedi conferma di lettura. */
  readReceipt?: boolean;
  /** Richiedi conferma di recapito. */
  deliveryReceipt?: boolean;
}

export type Importance = 'high' | 'low';

/** Risposta alle richieste di conferma di lettura: chiedi ogni volta, sempre, mai. */
export type ReceiptReply = 'chiedi' | 'sempre' | 'mai';

export interface SavedDraft {
  id: number;
  attachments: DraftAttachment[];
}

export interface OutboxItem {
  id: number;
  accountId: string;
  subject: string;
  to: Person[];
  state: 'attesa' | 'invio' | 'offline' | 'errore';
  error: string | null;
  sendAfter: number;
}

export interface NewAccount {
  name: string;
  email: string;
  password: string;
  kind?: AccountKind;
  incoming?: ServerConfig;
  outgoing?: ServerConfig;
}

export type EngineEvent =
  | { type: 'changed'; accountId: string; folders: number[] }
  | { type: 'status'; accountId: string; status: AccountStatus }
  | { type: 'outbox'; accountId: string; outboxId: number; state: string; error: string | null }
  | { type: 'accounts' }
  | { type: 'import'; name: string; done: number; total: number }
  | { type: 'signatures' }
  | { type: 'categories' }
  | { type: 'newMail'; accountId: string; count: number; messages: MessageSummary[] }
  | { type: 'rules' }
  | { type: 'contacts' }
  | { type: 'calendar' }
  | { type: 'quickSteps' }
  | { type: 'templates' }
  | { type: 'tasks' }
  | { type: 'searchFolders' }
  | { type: 'autoArchive'; archived: number; deleted: number; action: AutoArchiveAction };

// ---------- Archiviazione automatica ----------

export type AutoArchiveAction = 'archivio' | 'computer' | 'elimina';

export interface AutoArchive {
  enabled: boolean;
  /** Ogni quanti giorni controllare. */
  everyDays: number;
  /** Messaggi ricevuti da più di tanti mesi. */
  olderMonths: number;
  action: AutoArchiveAction;
  /** Svuota Posta eliminata e Posta indesiderata dopo `trashDays` giorni. */
  emptyTrash: boolean;
  trashDays: number;
  /** Cartelle da non toccare. */
  excluded: number[];
  /** Ultima esecuzione (secondi). */
  lastRun: number | null;
}

export interface AutoArchiveResult {
  archived: number;
  deleted: number;
}

// ---------- Cartelle di ricerca ----------

export interface SearchCriteria {
  /** Parole da cercare nel testo, nell'oggetto e nelle persone. */
  text: string;
  /** Messaggi da o verso queste persone (nome o indirizzo, anche in parte). */
  people: string[];
  unread: boolean;
  flagged: boolean;
  /** Priorità alta. */
  important: boolean;
  attachments: boolean;
  /** Con una categoria qualsiasi, o con quella indicata. */
  categorized: boolean;
  category: string | null;
  /** Almeno questa dimensione, in KB. */
  minKb: number | null;
  /** Arrivati negli ultimi giorni. */
  days: number | null;
}

/** Ricerca salvata che si apre come una cartella. */
export interface SearchFolder {
  id: string;
  accountId: string;
  name: string;
  /** Tipo scelto alla creazione: 'da-leggere', 'persone', 'personalizzata'... */
  kind: string;
  criteria: SearchCriteria;
  /** Messaggi trovati e quanti non letti (calcolati dal motore). */
  total: number;
  unread: number;
}

// ---------- Azioni rapide e modelli ----------

export type QuickActionKind =
  | 'letto'
  | 'da-leggere'
  | 'contrassegna'
  | 'togli-contrassegno'
  | 'categoria'
  | 'togli-categorie'
  | 'rispondi'
  | 'rispondi-tutti'
  | 'inoltra'
  | 'inoltra-allegato'
  | 'sposta'
  | 'archivia'
  | 'elimina'
  | 'ignora';

export interface QuickAction {
  kind: QuickActionKind;
  /** Cartella di destinazione (sposta). */
  folder?: number | null;
  /** Categoria da assegnare (categoria). */
  category?: string | null;
  /** Destinatari già scritti (inoltra). */
  to?: Person[];
  /** Testo da aggiungere in cima alla risposta o all'inoltro. */
  text?: string | null;
}

/** Azione rapida: più passaggi con un clic. */
export interface QuickStep {
  id: string;
  name: string;
  /** Icona Tabler registrata in icons.ts. */
  icon: string;
  /** Ctrl+Maiusc+1..9. */
  shortcut: number | null;
  actions: QuickAction[];
}

/** Modello di messaggio (I miei modelli). */
export interface MessageTemplate {
  id: string;
  name: string;
  subject: string;
  html: string;
}

// ---------- Attività ----------

export interface TaskList {
  id: string;
  name: string;
}

export interface TaskStep {
  id: string;
  title: string;
  done: boolean;
}

export type TaskRepeat = 'daily' | 'weekdays' | 'weekly' | 'monthly' | 'yearly';

export interface Task {
  id: string;
  /** Elenco dell'utente; vuoto per l'elenco Attività. */
  list: string;
  title: string;
  notes: string;
  important: boolean;
  /** Giorno (AAAA-MM-GG) in cui è stata aggiunta a La mia giornata. */
  myDay: string | null;
  /** Inizio del giorno di scadenza (ms, ora locale). */
  due: number | null;
  /** Istante del promemoria (ms). */
  reminder: number | null;
  repeat: TaskRepeat | null;
  steps: TaskStep[];
  done: boolean;
  doneAt: number | null;
  created: number;
  /** Messaggio da cui è nata. */
  message: number | null;
}

/** Errore restituito dal motore: `kind` serve all'interfaccia per decidere cosa mostrare. */
export interface EngineError {
  kind: string;
  message: string;
}

export function errorMessage(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return String((e as EngineError).message);
  return String(e);
}

export function errorKind(e: unknown): string {
  if (e && typeof e === 'object' && 'kind' in e) return String((e as EngineError).kind);
  return '';
}

// ---------- Calendario ----------

/** Risposta di un partecipante a un invito. */
export type PartStat = 'needs-action' | 'accepted' | 'tentative' | 'declined';

export type RepeatFreq = 'daily' | 'weekly' | 'monthly' | 'yearly';

export interface Repeat {
  freq: RepeatFreq;
  interval: number;
  /** Giorni della settimana (0 domenica … 6 sabato). */
  days: number[];
  /** Mensile o annuale "il secondo martedì": 1-4, oppure -1 per l'ultimo. */
  week: number | null;
  /** Ultimo giorno della serie (ms, compreso). */
  until: number | null;
  count: number | null;
}

export interface Attendee {
  name: string;
  email: string;
  status: PartStat;
  optional: boolean;
}

/** Appuntamento o riunione. Per tutto il giorno: mezzanotte del primo giorno e del giorno dopo l'ultimo. */
export interface CalEvent {
  id: string;
  uid: string;
  title: string;
  location: string;
  start: number;
  end: number;
  allDay: boolean;
  repeat: Repeat | null;
  /** Inizi delle occorrenze tolte dalla serie. */
  exceptions: number[];
  /** Occorrenza di una serie modificata a parte: il suo inizio originale. */
  recurrenceId: number | null;
  /** Promemoria: minuti prima dell'inizio. */
  reminder: number | null;
  notes: string;
  categories: string[];
  organizer: Person | null;
  attendees: Attendee[];
  /** La mia risposta a un invito ricevuto. */
  response: PartStat | null;
  accountId: string | null;
  sequence: number;
  cancelled: boolean;
}

export interface Invite {
  method: 'request' | 'cancel' | 'reply' | 'publish' | 'counter' | string;
  event: CalEvent;
  /** Lo stesso evento già nel calendario. */
  local: CalEvent | null;
  /** Nel calendario c'è una versione più recente. */
  outdated: boolean;
  /** Per le risposte: chi ha risposto e come. */
  replier: Attendee | null;
  /** Sono tra i partecipanti. */
  forMe: boolean;
}

export type InviteAction = 'accepted' | 'tentative' | 'declined' | 'add' | 'remove';

export interface CalImport {
  added: number;
  updated: number;
  unchanged: number;
}
