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
  status: AccountStatus;
}

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
}

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
  | { type: 'signatures' };

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
