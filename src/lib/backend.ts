// Livello dati: l'interfaccia parla solo con questa interfaccia.
// Nell'app installata le chiamate vanno al motore in Rust (comandi Tauri);
// nel browser e nei test risponde il motore di esempio (mockBackend.ts).

import type {
  Account,
  Category,
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
  AutoReply,
  FolderUsage,
  Rule,
  SavedDraft,
  Signature,
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
  /** Testo del messaggio da citare in una risposta (frammento HTML ripulito). */
  quoteHtml(id: number): Promise<string>;
  signatures(): Promise<Signature[]>;
  saveSignature(s: Signature): Promise<Signature>;
  deleteSignature(id: string): Promise<void>;
  /** Immagine incollata da una pagina web, come dati da incorporare (null se non si può). */
  fetchImage(url: string): Promise<string | null>;
  setFlags(ids: number[], seen: boolean | null, flagged: boolean | null): Promise<void>;
  moveMessages(ids: number[], folderId: number): Promise<void>;
  moveToRole(ids: number[], role: Role): Promise<void>;
  deleteMessages(ids: number[]): Promise<void>;
  categories(): Promise<Category[]>;
  saveCategories(list: Category[]): Promise<Category[]>;
  setCategory(ids: number[], category: string, on: boolean): Promise<void>;
  clearCategories(ids: number[]): Promise<void>;
  /** Messaggi di una conversazione in tutte le cartelle dell'account, dal più vecchio. */
  thread(accountId: string, thread: string): Promise<MessageSummary[]>;
  /** Nuova cartella dentro `parentId`, o al primo livello dell'account. Restituisce l'identificativo. */
  createFolder(accountId: string, parentId: number | null, name: string): Promise<number>;
  renameFolder(id: number, name: string): Promise<void>;
  /** Elimina la cartella e le sottocartelle; i messaggi vanno in Posta eliminata. */
  deleteFolder(id: number): Promise<void>;
  markFolderRead(id: number): Promise<void>;
  /** Elimina definitivamente tutti i messaggi della cartella. Restituisce quanti erano. */
  emptyFolder(id: number): Promise<number>;
  saveDraft(d: Draft): Promise<SavedDraft>;
  closeDraft(d: Draft): Promise<SavedDraft | null>;
  discardDraft(accountId: string, key: string): Promise<void>;
  openDraft(id: number): Promise<Draft>;
  send(d: Draft): Promise<number>;
  /** Invio programmato all'ora indicata (millisecondi). */
  sendLater(d: Draft, at: number): Promise<number>;
  rules(): Promise<Rule[]>;
  saveRules(list: Rule[]): Promise<Rule[]>;
  /** Esegui ora: applica le regole indicate ai messaggi della cartella. Restituisce quanti ne ha toccati. */
  runRules(ids: string[], folderId: number): Promise<number>;
  setAutoReply(accountId: string, reply: AutoReply): Promise<void>;
  /** Pulizia: messaggi e spazio per cartella, dalla più grande. */
  mailboxUsage(accountId: string): Promise<FolderUsage[]>;
  /** Pulizia: messaggi più grandi di `minSize` byte. */
  largeMessages(accountId: string, minSize: number): Promise<MessageSummary[]>;
  /** Pulizia: quanti messaggi della cartella sono più vecchi di `before` (ms); con `archive` li sposta in Archivio. */
  olderMessages(folderId: number, before: number, archive: boolean): Promise<number>;
  /** Avviso sul desktop. Falso se il sistema non lo permette. */
  notify(title: string, body: string): Promise<boolean>;
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
  /** Finestra con il messaggio pronto da stampare. Falso se non si può aprire. */
  openPrintWindow(id: number, remote: boolean): Promise<boolean>;
  /** Finestra di stampa del sistema per la pagina corrente. */
  printPage(): Promise<void>;
  /** Lettura con la voce del sistema, quando la pagina non ha voci proprie. */
  speak(text: string): Promise<void>;
  speaking(): Promise<boolean>;
  stopSpeaking(): Promise<void>;
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
  quoteHtml(id: number) {
    return this.call<string>('quote_html', { id });
  }
  signatures() {
    return this.call<Signature[]>('signatures');
  }
  saveSignature(signature: Signature) {
    return this.call<Signature>('save_signature', { signature });
  }
  deleteSignature(id: string) {
    return this.call<void>('delete_signature', { id });
  }
  fetchImage(url: string) {
    return this.call<string | null>('fetch_image', { url });
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
  categories() {
    return this.call<Category[]>('categories');
  }
  saveCategories(list: Category[]) {
    return this.call<Category[]>('save_categories', { list });
  }
  setCategory(ids: number[], category: string, on: boolean) {
    return this.call<void>('set_category', { ids, category, on });
  }
  clearCategories(ids: number[]) {
    return this.call<void>('clear_categories', { ids });
  }
  thread(accountId: string, thread: string) {
    return this.call<MessageSummary[]>('thread', { accountId, thread });
  }
  createFolder(accountId: string, parentId: number | null, name: string) {
    return this.call<number>('create_folder', { accountId, parentId, name });
  }
  renameFolder(id: number, name: string) {
    return this.call<void>('rename_folder', { id, name });
  }
  deleteFolder(id: number) {
    return this.call<void>('delete_folder', { id });
  }
  markFolderRead(id: number) {
    return this.call<void>('mark_folder_read', { id });
  }
  emptyFolder(id: number) {
    return this.call<number>('empty_folder', { id });
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
  sendLater(draft: Draft, at: number) {
    return this.call<number>('send_later', { draft, at });
  }
  rules() {
    return this.call<Rule[]>('rules');
  }
  saveRules(list: Rule[]) {
    return this.call<Rule[]>('save_rules', { list });
  }
  runRules(ids: string[], folderId: number) {
    return this.call<number>('run_rules', { ids, folderId });
  }
  setAutoReply(accountId: string, reply: AutoReply) {
    return this.call<void>('set_auto_reply', { accountId, reply });
  }
  mailboxUsage(accountId: string) {
    return this.call<FolderUsage[]>('mailbox_usage', { accountId });
  }
  largeMessages(accountId: string, minSize: number) {
    return this.call<MessageSummary[]>('large_messages', { accountId, minSize });
  }
  olderMessages(folderId: number, before: number, archive: boolean) {
    return this.call<number>('older_messages', { folderId, before, archive });
  }
  async notify(title: string, body: string) {
    try {
      const n = await import('@tauri-apps/plugin-notification');
      let ok = await n.isPermissionGranted();
      if (!ok) ok = (await n.requestPermission()) === 'granted';
      if (ok) n.sendNotification({ title, body });
      return ok;
    } catch {
      return false;
    }
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
  async openPrintWindow(id: number, remote: boolean) {
    await this.call<void>('open_print_window', { id, remote });
    return true;
  }
  printPage() {
    return this.call<void>('print_page');
  }
  speak(text: string) {
    return this.call<void>('speak', { text });
  }
  speaking() {
    return this.call<boolean>('speaking');
  }
  stopSpeaking() {
    return this.call<void>('stop_speaking');
  }
  onEvent(cb: (e: EngineEvent) => void) {
    let stop: (() => void) | null = null;
    let cancelled = false;
    import('@tauri-apps/api/event').then(({ listen }) =>
      listen<EngineEvent>('better-look://evento', (ev) => cb(ev.payload)).then((un) => {
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
