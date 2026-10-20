// Livello dati: l'interfaccia parla solo con questa interfaccia.
// Nell'app installata le chiamate vanno al motore in Rust (comandi Tauri);
// nel browser e nei test risponde il motore di esempio (mockBackend.ts).

import type {
  Account,
  Draft,
  DraftAttachment,
  EngineEvent,
  Folder,
  MessageDetail,
  MessageSummary,
  NewAccount,
  OutboxItem,
  Person,
  Role,
  SavedDraft,
} from './types';

export type Tab = 'prio' | 'altre';

export interface Backend {
  /** Vero quando c'è il motore vero (app installata). */
  readonly real: boolean;
  accounts(): Promise<Account[]>;
  addAccount(input: NewAccount): Promise<Account>;
  updateAccount(account: Account): Promise<void>;
  setPassword(id: string, password: string): Promise<void>;
  removeAccount(id: string): Promise<void>;
  folders(): Promise<Folder[]>;
  messages(folderId: number, tab: Tab | null, offset?: number, limit?: number): Promise<MessageSummary[]>;
  search(query: string, accountId: string | null): Promise<MessageSummary[]>;
  message(id: number, allowRemote: boolean): Promise<MessageDetail>;
  setFlags(ids: number[], seen: boolean | null, flagged: boolean | null): Promise<void>;
  moveMessages(ids: number[], folderId: number): Promise<void>;
  moveToRole(ids: number[], role: Role): Promise<void>;
  deleteMessages(ids: number[]): Promise<void>;
  saveDraft(d: Draft): Promise<SavedDraft>;
  closeDraft(d: Draft): Promise<SavedDraft | null>;
  discardDraft(accountId: string, key: string): Promise<void>;
  openDraft(id: number): Promise<Draft>;
  send(d: Draft): Promise<number>;
  cancelSend(outboxId: number): Promise<Draft | null>;
  outbox(): Promise<OutboxItem[]>;
  outboxTake(outboxId: number): Promise<Draft | null>;
  syncNow(accountId?: string | null): Promise<void>;
  contacts(prefix: string): Promise<Person[]>;
  openAttachment(id: number, index: number): Promise<void>;
  /** Chiede dove salvare e salva. Falso se l'utente annulla. */
  saveAttachment(id: number, index: number, name: string): Promise<boolean>;
  exportEml(id: number, name: string): Promise<boolean>;
  exportMbox(folderId: number, name: string): Promise<number | null>;
  /** Chiede il file e lo importa come file di dati. */
  /** Importa un archivio, con i messaggi e gli allegati che non si sono potuti leggere. */
  importArchive(kind: 'pst' | 'mbox'): Promise<(Account & { skipped?: number; lostAttachments?: number }) | null>;
  /** Finestra di scelta dei file da allegare. */
  pickFiles(): Promise<DraftAttachment[]>;
  openLink(url: string): Promise<void>;
  /** Apre il messaggio in una finestra del sistema. Falso se non è possibile. */
  openComposeWindow(d: Draft): Promise<boolean>;
  takeComposeDraft(): Promise<Draft | null>;
  onEvent(cb: (e: EngineEvent) => void): () => void;
}

function baseName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export class TauriBackend implements Backend {
  readonly real = true;

  private async call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }

  accounts() {
    return this.call<Account[]>('accounts');
  }
  addAccount(input: NewAccount) {
    return this.call<Account>('add_account', { input });
  }
  updateAccount(account: Account) {
    const { status: _status, ...config } = account;
    return this.call<void>('update_account', { config });
  }
  setPassword(id: string, password: string) {
    return this.call<void>('set_password', { id, password });
  }
  removeAccount(id: string) {
    return this.call<void>('remove_account', { id });
  }
  folders() {
    return this.call<Folder[]>('folders');
  }
  messages(folderId: number, tab: Tab | null, offset = 0, limit = 400) {
    return this.call<MessageSummary[]>('messages', { folderId, tab, offset, limit });
  }
  search(query: string, accountId: string | null) {
    return this.call<MessageSummary[]>('search', { query, accountId });
  }
  message(id: number, allowRemote: boolean) {
    return this.call<MessageDetail>('message', { id, allowRemote });
  }
  setFlags(ids: number[], seen: boolean | null, flagged: boolean | null) {
    return this.call<void>('set_flags', { ids, seen, flagged });
  }
  moveMessages(ids: number[], folderId: number) {
    return this.call<void>('move_messages', { ids, folderId });
  }
  moveToRole(ids: number[], role: Role) {
    return this.call<void>('move_to_role', { ids, role });
  }
  deleteMessages(ids: number[]) {
    return this.call<void>('delete_messages', { ids });
  }
  saveDraft(draft: Draft) {
    return this.call<SavedDraft>('save_draft', { draft });
  }
  closeDraft(draft: Draft) {
    return this.call<SavedDraft | null>('close_draft', { draft });
  }
  discardDraft(accountId: string, key: string) {
    return this.call<void>('discard_draft', { accountId, key });
  }
  openDraft(id: number) {
    return this.call<Draft>('open_draft', { id });
  }
  send(draft: Draft) {
    return this.call<number>('send', { draft });
  }
  cancelSend(id: number) {
    return this.call<Draft | null>('cancel_send', { id });
  }
  outbox() {
    return this.call<OutboxItem[]>('outbox');
  }
  outboxTake(id: number) {
    return this.call<Draft | null>('outbox_take', { id });
  }
  syncNow(accountId: string | null = null) {
    return this.call<void>('sync_now', { accountId });
  }
  contacts(prefix: string) {
    return this.call<Person[]>('contacts', { prefix });
  }
  openAttachment(id: number, index: number) {
    return this.call<void>('open_attachment', { id, index });
  }
  async saveAttachment(id: number, index: number, name: string) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({ defaultPath: name, title: 'Salva allegato' });
    if (!path) return false;
    await this.call<void>('save_attachment', { id, index, path });
    return true;
  }
  async exportEml(id: number, name: string) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({ defaultPath: `${name}.eml`, title: 'Salva messaggio', filters: [{ name: 'Messaggio di posta', extensions: ['eml'] }] });
    if (!path) return false;
    await this.call<void>('export_eml', { id, path });
    return true;
  }
  async exportMbox(folderId: number, name: string) {
    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({ defaultPath: `${name}.mbox`, title: 'Esporta cartella', filters: [{ name: 'Archivio MBOX', extensions: ['mbox'] }] });
    if (!path) return null;
    return this.call<number>('export_mbox', { folderId, path });
  }
  async importArchive(kind: 'pst' | 'mbox') {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const filters = kind === 'pst' ? [{ name: 'File di dati di Outlook', extensions: ['pst'] }] : [{ name: 'Archivio MBOX', extensions: ['mbox', 'mbx', 'txt'] }];
    const path = await open({ multiple: false, directory: false, filters, title: kind === 'pst' ? 'Apri file di dati di Outlook' : 'Importa archivio MBOX' });
    if (!path || Array.isArray(path)) return null;
    return this.call<Account & { skipped: number; lostAttachments: number }>(kind === 'pst' ? 'import_pst' : 'import_mbox', { path });
  }
  async pickFiles() {
    const { open } = await import('@tauri-apps/plugin-dialog');
    const picked = await open({ multiple: true, directory: false, title: 'Allega file' });
    if (!picked) return [];
    const list = Array.isArray(picked) ? picked : [picked];
    return list.map((path) => ({ name: baseName(path), size: 0, path }));
  }
  openLink(url: string) {
    return this.call<void>('open_link', { url });
  }
  async openComposeWindow(draft: Draft) {
    await this.call<void>('open_compose_window', { draft });
    return true;
  }
  takeComposeDraft() {
    return this.call<Draft | null>('take_compose_draft');
  }
  onEvent(cb: (e: EngineEvent) => void) {
    let stop: (() => void) | null = null;
    let cancelled = false;
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<EngineEvent>('datini://evento', (ev) => cb(ev.payload)).then((un) => {
        if (cancelled) un();
        else stop = un;
      }),
    );
    return () => {
      cancelled = true;
      stop?.();
    };
  }
}
