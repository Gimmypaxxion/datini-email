// Stato dell'interfaccia e azioni dell'utente.
// I dati arrivano dal motore (backend.ts): quello vero nell'app installata,
// quello di esempio nel browser e nei test. Le preferenze di aspetto restano
// salvate sul computer.

import type { Backend, Tab } from './backend';
import { TauriBackend } from './backend';
import { MockBackend } from './mockBackend';
import { inTauri } from './platform';
import {
  clock,
  cleanSubject,
  longDate,
  parseRecipient,
  personFull,
  peopleText,
  randomKey,
  sameAddress,
  validEmail,
} from './format';
import {
  errorKind,
  errorMessage,
  type Account,
  type Draft,
  type EngineEvent,
  type Folder,
  type MessageDetail,
  type MessageSummary,
  type NewAccount,
  type OutboxItem,
  type Person,
  type Role,
  type SyncOptions,
} from './types';

export type RibbonMode = 'estesa' | 'compatta';
export type EntryMode = 'file' | 'ingranaggio';
export type ComposeMode = 'finestra' | 'pannello';
export type ThemeChoice = 'sistema' | 'chiaro' | 'scuro';
export type StyleName = 'classico' | 'nuovo';
export type RibbonTab = 'home' | 'visualizza' | 'aiuto' | 'messaggio' | 'inserisci';
export type WinTab = 'messaggio' | 'inserisci';
export type ComposeKind = 'new' | 'reply' | 'replyAll' | 'forward';
export type RecipientField = 'to' | 'cc' | 'bcc';

export interface Prefs {
  ribbon: RibbonMode;
  entry: EntryMode;
  compose: ComposeMode;
  replyWin: boolean;
}

export const PRESETS: Record<StyleName, Prefs> = {
  classico: { ribbon: 'estesa', entry: 'file', compose: 'finestra', replyWin: false },
  nuovo: { ribbon: 'compatta', entry: 'ingranaggio', compose: 'pannello', replyWin: false },
};

export const ROLE_NAMES: Record<Role, string> = {
  inbox: 'Posta in arrivo',
  sent: 'Posta inviata',
  drafts: 'Bozze',
  archive: 'Archivio',
  junk: 'Posta indesiderata',
  deleted: 'Posta eliminata',
};

/** Ordine delle cartelle nel riquadro, come in Outlook. */
export const ROLE_ORDER: Role[] = ['inbox', 'drafts', 'sent', 'deleted', 'archive', 'junk'];

export function folderLabel(f: Folder | null | undefined): string {
  if (!f) return '';
  return f.role ? ROLE_NAMES[f.role] : f.name;
}

export function sortFolders(list: Folder[]): Folder[] {
  const rank = (f: Folder) => (f.role ? ROLE_ORDER.indexOf(f.role) : ROLE_ORDER.length);
  return [...list].sort((a, b) => rank(a) - rank(b) || a.path.localeCompare(b.path, 'it'));
}

const PAGE = 300;
const AUTOSAVE_MS = 2500;

export interface Toast {
  kind: 'sent' | 'draft' | 'error' | 'info';
  id: number;
  text: string;
  outboxId?: number;
}

interface LastSent {
  key: string;
  win: boolean;
}

interface Saved {
  prefs: Prefs;
  onboardingDone: boolean;
  themeChoice: ThemeChoice;
  showFolders: boolean;
  collapsed?: Record<string, boolean>;
}

const STORAGE_KEY = 'datini.interfaccia.v1';

function readSaved(): Partial<Saved> | null {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Partial<Saved>) : null;
  } catch {
    return null;
  }
}

let toastSeq = 0;

export type AddResult = { ok: true; account: Account } | { ok: false; kind: string; message: string };

export class MailStore {
  backend: Backend;
  /** "principale" o "messaggio" (finestra separata per scrivere). */
  mode: 'principale' | 'messaggio' = 'principale';

  ready = $state(false);
  accounts = $state<Account[]>([]);
  folders = $state<Folder[]>([]);
  outbox = $state<OutboxItem[]>([]);

  // Navigazione
  folderId = $state<number | null>(null);
  folderSel = $state('');
  view = $state<'cartella' | 'uscita'>('cartella');
  tab = $state<Tab>('prio');
  q = $state('');
  list = $state<MessageSummary[]>([]);
  listLoading = $state(false);
  listEnd = $state(true);
  sel = $state<number | null>(null);
  detail = $state<MessageDetail | null>(null);
  detailLoading = $state(false);
  detailError = $state('');
  remoteOk = $state<Record<number, boolean>>({});
  collapsed = $state<Record<string, boolean>>({});

  // Aspetto
  prefs = $state<Prefs>({ ...PRESETS.classico });
  themeChoice = $state<ThemeChoice>('sistema');
  systemDark = $state(false);
  showFolders = $state(true);
  ribbonTab = $state<RibbonTab>('home');
  winTab = $state<WinTab>('messaggio');

  // Pannelli
  onboarding = $state(true);
  obChoice = $state<StyleName>('classico');
  backstage = $state(false);
  bsPage = $state<'info' | 'apri' | 'about'>('info');
  settings = $state(false);
  setPage = $state<'aspetto' | 'account' | 'info'>('aspetto');
  acctSel = $state<string | null>(null);
  wizard = $state<{ open: boolean; first: boolean }>({ open: false, first: false });
  netOffline = $state(false);

  // Scrittura
  c = $state<Draft | null>(null);
  cWin = $state(false);
  winMax = $state(false);
  winMin = $state(false);
  toInput = $state('');
  ccInput = $state('');
  bccInput = $state('');
  showBcc = $state(false);
  err = $state('');
  savedAt = $state('');
  saveError = $state('');
  sending = $state(false);
  toast = $state<Toast | null>(null);
  private lastSent: LastSent | null = null;
  private toastTimer: ReturnType<typeof setTimeout> | undefined;
  private saveTimer: ReturnType<typeof setTimeout> | undefined;
  private refreshTimer: ReturnType<typeof setTimeout> | undefined;
  private searchTimer: ReturnType<typeof setTimeout> | undefined;
  private listToken = 0;
  private detailToken = 0;
  private persist: boolean;
  private unsubscribe: (() => void) | null = null;

  constructor(opts: { persist?: boolean; backend?: Backend } = {}) {
    this.persist = opts.persist ?? true;
    this.backend = opts.backend ?? (inTauri ? new TauriBackend() : new MockBackend());
    if (this.persist) {
      const saved = readSaved();
      if (saved) {
        if (saved.prefs) this.prefs = { ...PRESETS.classico, ...saved.prefs };
        if (saved.onboardingDone) this.onboarding = false;
        if (saved.themeChoice) this.themeChoice = saved.themeChoice;
        if (typeof saved.showFolders === 'boolean') this.showFolders = saved.showFolders;
        if (saved.collapsed) this.collapsed = saved.collapsed;
      }
    }
    if (typeof navigator !== 'undefined' && typeof window !== 'undefined') {
      this.netOffline = navigator.onLine === false;
      window.addEventListener('online', () => {
        this.netOffline = false;
        void this.backend.syncNow(null);
      });
      window.addEventListener('offline', () => (this.netOffline = true));
    }
    const mq = globalThis.matchMedia?.('(prefers-color-scheme: dark)');
    if (mq) {
      this.systemDark = mq.matches;
      mq.addEventListener?.('change', (e) => (this.systemDark = e.matches));
    }
  }

  private save() {
    if (!this.persist) return;
    const data: Saved = {
      prefs: { ...this.prefs },
      onboardingDone: !this.onboarding,
      themeChoice: this.themeChoice,
      showFolders: this.showFolders,
      collapsed: { ...this.collapsed },
    };
    try {
      globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      /* archivio non disponibile: le preferenze valgono solo per questa sessione */
    }
  }

  // ---------- Avvio ----------

  async init() {
    this.unsubscribe?.();
    this.unsubscribe = this.backend.onEvent((e) => this.onEvent(e));
    await Promise.all([this.reloadAccounts(), this.reloadFolders(), this.reloadOutbox()]);
    if (this.mode === 'principale') {
      const inbox = this.defaultInbox();
      if (inbox) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
      if (!this.onboarding && this.backend.real && !this.accounts.length) this.openWizard(true);
    }
    this.ready = true;
  }

  dispose() {
    this.unsubscribe?.();
    clearTimeout(this.saveTimer);
    clearTimeout(this.refreshTimer);
    clearTimeout(this.toastTimer);
  }

  private defaultInbox(): Folder | undefined {
    for (const a of this.accounts) {
      const f = this.folders.find((x) => x.accountId === a.id && x.role === 'inbox');
      if (f) return f;
    }
    return this.folders[0];
  }

  private onEvent(e: EngineEvent) {
    switch (e.type) {
      case 'changed':
        this.scheduleRefresh();
        break;
      case 'status': {
        const a = this.accounts.find((x) => x.id === e.accountId);
        if (a) a.status = e.status;
        break;
      }
      case 'accounts':
        void this.reloadAccounts().then(() => this.reloadFolders());
        break;
      case 'outbox':
        void this.reloadOutbox();
        if (this.mode !== 'principale') break;
        if (e.state === 'attesa') {
          this.flash({ kind: 'sent', text: 'Messaggio inviato', outboxId: e.outboxId }, 7000);
        } else if (e.state === 'annullato' && this.toast?.outboxId === e.outboxId) {
          this.toast = null;
        } else if (e.state === 'errore') {
          this.flash({ kind: 'error', text: `Messaggio non inviato. ${e.error ?? ''}`.trim(), outboxId: e.outboxId }, 12000);
        }
        break;
    }
  }

  private scheduleRefresh() {
    clearTimeout(this.refreshTimer);
    this.refreshTimer = setTimeout(() => void this.refresh(), 200);
  }

  async refresh() {
    await this.reloadFolders();
    await this.loadList(false);
    if (this.sel !== null && this.detail && this.detail.id === this.sel) {
      const s = this.list.find((m) => m.id === this.sel);
      if (s) Object.assign(this.detail, { unread: s.unread, flagged: s.flagged, answered: s.answered });
    }
  }

  async reloadAccounts() {
    this.accounts = await this.backend.accounts();
    if (!this.acctSel || !this.accounts.some((a) => a.id === this.acctSel)) this.acctSel = this.accounts[0]?.id ?? null;
  }

  async reloadFolders() {
    this.folders = await this.backend.folders();
    if (this.folderId !== null && !this.folders.some((f) => f.id === this.folderId)) {
      const inbox = this.defaultInbox();
      if (inbox) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
      else {
        this.folderId = null;
        this.list = [];
        this.sel = null;
        this.detail = null;
      }
    } else if (this.folderId === null && this.mode === 'principale') {
      const inbox = this.defaultInbox();
      if (inbox) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
    }
  }

  async reloadOutbox() {
    this.outbox = await this.backend.outbox();
  }

  // ---------- Valori derivati ----------

  get theme(): 'chiaro' | 'scuro' {
    if (this.themeChoice === 'sistema') return this.systemDark ? 'scuro' : 'chiaro';
    return this.themeChoice;
  }

  get currentFolder(): Folder | null {
    return this.folders.find((f) => f.id === this.folderId) ?? null;
  }

  get currentAccount(): Account | null {
    const f = this.currentFolder;
    return (f && this.accounts.find((a) => a.id === f.accountId)) || this.accounts[0] || null;
  }

  get isInbox(): boolean {
    return this.currentFolder?.role === 'inbox';
  }

  get selected(): MessageSummary | null {
    return this.list.find((m) => m.id === this.sel) ?? null;
  }

  get offline(): boolean {
    if (this.netOffline) return true;
    const remote = this.accounts.filter((a) => a.kind !== 'local');
    return remote.length > 0 && remote.every((a) => a.status.state === 'offline');
  }

  get syncing(): boolean {
    return this.accounts.some((a) => a.status.state === 'sincronizzazione');
  }

  get paneCompose(): boolean {
    return !!this.c && !this.cWin && this.mode === 'principale';
  }
  get winOpen(): boolean {
    return !!this.c && this.cWin && !this.winMin && this.mode === 'principale';
  }
  get winMinimized(): boolean {
    return !!this.c && this.cWin && this.winMin && this.mode === 'principale';
  }

  get ribbonTabs(): RibbonTab[] {
    return this.paneCompose ? ['home', 'visualizza', 'aiuto', 'messaggio', 'inserisci'] : ['home', 'visualizza', 'aiuto'];
  }
  get activeRibbon(): RibbonTab {
    return this.ribbonTabs.includes(this.ribbonTab) ? this.ribbonTab : 'home';
  }

  get presetName(): StyleName | '' {
    const p = this.prefs;
    for (const name of ['classico', 'nuovo'] as StyleName[]) {
      const pp = PRESETS[name];
      if (pp.ribbon === p.ribbon && pp.entry === p.entry && pp.compose === p.compose) return name;
    }
    return '';
  }

  get listTitle(): string {
    if (this.q.trim()) return 'Risultati della ricerca';
    if (this.view === 'uscita') return 'Posta in uscita';
    return folderLabel(this.currentFolder);
  }

  foldersOf(accountId: string): Folder[] {
    return sortFolders(this.folders.filter((f) => f.accountId === accountId));
  }

  roleFolder(accountId: string | undefined, role: Role): Folder | undefined {
    return this.folders.find((f) => f.accountId === accountId && f.role === role);
  }

  count(folderId: number, unreadOnly: boolean): number {
    const f = this.folders.find((x) => x.id === folderId);
    if (!f) return 0;
    return unreadOnly ? f.unread : f.total;
  }

  canSend(a: Account | null | undefined): boolean {
    return !!a && a.kind !== 'local' && !!a.outgoing;
  }

  // ---------- Navigazione ----------

  async pickFolder(folderId: number, selId: string) {
    await this.stashPaneDraft();
    this.view = 'cartella';
    this.folderId = folderId;
    this.folderSel = selId;
    this.tab = 'prio';
    this.q = '';
    this.err = '';
    this.sel = null;
    this.detail = null;
    this.detailError = '';
    await this.loadList(true);
    if (this.currentFolder?.role !== 'drafts' && this.list.length) await this.pickMsg(this.list[0].id);
  }

  async pickOutbox(selId: string) {
    await this.stashPaneDraft();
    this.view = 'uscita';
    this.folderSel = selId;
    this.q = '';
    this.sel = null;
    this.detail = null;
    this.list = [];
    await this.reloadOutbox();
  }

  async pickTab(tab: Tab) {
    await this.stashPaneDraft();
    this.tab = tab;
    this.err = '';
    this.sel = null;
    this.detail = null;
    await this.loadList(true);
    if (this.list.length) await this.pickMsg(this.list[0].id);
  }

  /** Ricerca mentre si scrive, con una breve attesa tra un tasto e l'altro. */
  search(value: string) {
    this.q = value;
    clearTimeout(this.searchTimer);
    return new Promise<void>((resolve) => {
      this.searchTimer = setTimeout(
        async () => {
          await this.loadList(true);
          if (this.list.length) await this.pickMsg(this.list[0].id, false);
          else {
            this.sel = null;
            this.detail = null;
          }
          resolve();
        },
        this.backend.real ? 180 : 0,
      );
    });
  }

  async clearSearch() {
    clearTimeout(this.searchTimer);
    this.q = '';
    await this.loadList(true);
    if (!this.list.some((m) => m.id === this.sel)) {
      this.sel = null;
      this.detail = null;
      if (this.list.length) await this.pickMsg(this.list[0].id, false);
    }
  }

  /** Carica l'elenco. Con `reset` torna all'inizio, altrimenti aggiorna le righe già caricate. */
  async loadList(reset: boolean) {
    const token = ++this.listToken;
    if (this.view === 'uscita') {
      await this.reloadOutbox();
      return;
    }
    const q = this.q.trim();
    const limit = reset ? PAGE : Math.max(PAGE, this.list.length);
    this.listLoading = true;
    try {
      let items: MessageSummary[];
      if (q) items = await this.backend.search(q, this.currentAccount?.id ?? null);
      else if (this.folderId === null) items = [];
      else items = await this.backend.messages(this.folderId, this.isInbox ? this.tab : null, 0, limit);
      if (token !== this.listToken) return;
      this.list = items;
      this.listEnd = !!q || items.length < limit;
      if (this.sel !== null && !items.some((m) => m.id === this.sel) && !reset) {
        this.sel = null;
        this.detail = null;
      }
    } catch (e) {
      if (token === this.listToken) this.flash({ kind: 'error', text: errorMessage(e) });
    } finally {
      if (token === this.listToken) this.listLoading = false;
    }
  }

  async loadMore() {
    if (this.listEnd || this.listLoading || this.q.trim() || this.folderId === null) return;
    this.listLoading = true;
    const token = ++this.listToken;
    try {
      const more = await this.backend.messages(this.folderId, this.isInbox ? this.tab : null, this.list.length, PAGE);
      if (token !== this.listToken) return;
      const known = new Set(this.list.map((m) => m.id));
      this.list = [...this.list, ...more.filter((m) => !known.has(m.id))];
      this.listEnd = more.length < PAGE;
    } finally {
      if (token === this.listToken) this.listLoading = false;
    }
  }

  async pickMsg(id: number, markRead = true) {
    const m = this.list.find((x) => x.id === id);
    if (!m) return;
    if (m.draft && this.currentFolder?.role === 'drafts' && !this.q.trim()) {
      await this.reopenDraft(m.id);
      return;
    }
    await this.stashPaneDraft();
    this.sel = id;
    this.err = '';
    if (markRead && m.unread) {
      m.unread = false;
      this.bumpFolder(m.folderId, -1);
      void this.backend.setFlags([id], true, null).catch((e) => this.flash({ kind: 'error', text: errorMessage(e) }));
    }
    await this.loadDetail(id);
  }

  async loadDetail(id: number) {
    const token = ++this.detailToken;
    this.detailLoading = true;
    this.detailError = '';
    if (this.detail?.id !== id) this.detail = null;
    try {
      const d = await this.backend.message(id, !!this.remoteOk[id]);
      if (token !== this.detailToken) return;
      const s = this.list.find((x) => x.id === id);
      this.detail = s ? { ...d, unread: s.unread, flagged: s.flagged } : d;
    } catch (e) {
      if (token === this.detailToken) this.detailError = errorMessage(e);
    } finally {
      if (token === this.detailToken) this.detailLoading = false;
    }
  }

  async showRemoteImages() {
    if (this.sel === null) return;
    this.remoteOk[this.sel] = true;
    await this.loadDetail(this.sel);
  }

  private bumpFolder(folderId: number, delta: number) {
    const f = this.folders.find((x) => x.id === folderId);
    if (f) f.unread = Math.max(0, f.unread + delta);
  }

  /** Sposta la selezione al messaggio precedente o successivo dell'elenco (frecce). */
  async moveSelection(step: 1 | -1) {
    const l = this.list;
    if (!l.length) return;
    const i = l.findIndex((m) => m.id === this.sel);
    const next = l[Math.min(Math.max(i + step, 0), l.length - 1)];
    if (next && next.id !== this.sel && !next.draft) await this.pickMsg(next.id);
  }

  /** Toglie un messaggio dall'elenco e seleziona il successivo, come Outlook. */
  private dropFromList(ids: number[]) {
    const l = this.list;
    const i = l.findIndex((m) => m.id === this.sel);
    const gone = new Set(ids);
    for (const m of l) if (gone.has(m.id) && m.unread) this.bumpFolder(m.folderId, -1);
    this.list = l.filter((m) => !gone.has(m.id));
    if (this.sel !== null && gone.has(this.sel)) {
      const rest = this.list;
      const next = rest.length ? rest[Math.min(Math.max(i, 0), rest.length - 1)] : null;
      this.sel = null;
      this.detail = null;
      if (next && !next.draft) void this.pickMsg(next.id);
    }
  }

  private async act(fn: () => Promise<void>) {
    try {
      await fn();
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
      this.scheduleRefresh();
    }
  }

  async moveMsg(id: number | null, role: Role) {
    if (id === null) return;
    const m = this.list.find((x) => x.id === id);
    if (m && this.currentFolder?.role === role && !this.q.trim()) return;
    this.dropFromList([id]);
    await this.act(() => this.backend.moveToRole([id], role));
  }

  async moveToFolder(id: number | null, folderId: number) {
    if (id === null || folderId === this.folderId) return;
    this.dropFromList([id]);
    await this.act(() => this.backend.moveMessages([id], folderId));
  }

  async deleteMsg(id: number | null) {
    if (id === null) return;
    this.dropFromList([id]);
    await this.act(() => this.backend.deleteMessages([id]));
  }

  async toggleFlag(id: number | null) {
    const m = id === null ? null : this.list.find((x) => x.id === id);
    if (!m) return;
    m.flagged = !m.flagged;
    if (this.detail?.id === m.id) this.detail.flagged = m.flagged;
    await this.act(() => this.backend.setFlags([m.id], null, m.flagged));
  }

  async toggleRead(id: number | null) {
    const m = id === null ? null : this.list.find((x) => x.id === id);
    if (!m) return;
    await this.setRead(m.id, m.unread);
  }

  async setRead(id: number | null, read: boolean) {
    const m = id === null ? null : this.list.find((x) => x.id === id);
    if (!m || m.unread === !read) return;
    m.unread = !read;
    this.bumpFolder(m.folderId, read ? -1 : 1);
    if (this.detail?.id === m.id) this.detail.unread = m.unread;
    await this.act(() => this.backend.setFlags([m.id], read, null));
  }

  toggleFolders() {
    this.showFolders = !this.showFolders;
    this.save();
  }

  toggleSection(accountId: string) {
    this.collapsed[accountId] = !this.collapsed[accountId];
    this.save();
  }

  syncNow() {
    void this.backend.syncNow(null);
  }

  // ---------- Scrittura ----------

  private composeAccount(prefer?: string): Account | null {
    const p = this.accounts.find((a) => a.id === prefer);
    if (this.canSend(p)) return p!;
    const cur = this.currentAccount;
    if (this.canSend(cur)) return cur;
    return this.accounts.find((a) => this.canSend(a)) ?? null;
  }

  private emptyDraft(accountId: string): Draft {
    return { key: randomKey(), accountId, to: [], cc: [], bcc: [], subject: '', body: '', attachments: [], inReplyTo: null, references: [], replyToMessage: null };
  }

  static isEmpty(d: Draft): boolean {
    return !d.to.length && !d.cc.length && !d.bcc.length && !d.subject.trim() && !d.body.trim() && !d.attachments.length;
  }

  private snapshot(): Draft | null {
    return this.c ? ($state.snapshot(this.c) as Draft) : null;
  }

  private closeCompose() {
    clearTimeout(this.saveTimer);
    this.c = null;
    this.cWin = false;
    this.winMin = false;
    this.winMax = false;
    this.savedAt = '';
    this.saveError = '';
    this.toInput = '';
    this.ccInput = '';
    this.bccInput = '';
    this.showBcc = false;
  }

  /** Chiude il messaggio in scrittura salvandolo in Bozze, se contiene qualcosa. */
  private async stashDraft(): Promise<boolean> {
    const d = this.snapshot();
    if (!d) return false;
    this.closeCompose();
    if (MailStore.isEmpty(d)) {
      await this.backend.discardDraft(d.accountId, d.key).catch(() => {});
      return false;
    }
    try {
      await this.backend.closeDraft(d);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
    return true;
  }

  private async stashPaneDraft() {
    if (this.c && !this.cWin && this.mode === 'principale') await this.stashDraft();
  }

  private flash(t: Omit<Toast, 'id'>, ms = 6000) {
    clearTimeout(this.toastTimer);
    const id = ++toastSeq;
    this.toast = { ...t, id };
    this.toastTimer = setTimeout(() => {
      if (this.toast?.id === id) this.toast = null;
    }, ms);
  }

  dismissToast() {
    this.toast = null;
  }

  /** Comandi già al loro posto ma non ancora pronti. */
  soon(label: string) {
    this.flash({ kind: 'info', text: `${label}: arriverà in una prossima versione.` }, 4000);
  }

  startCompose(c: Draft, inWindow: boolean) {
    this.c = c;
    this.cWin = inWindow;
    this.winMin = false;
    this.winMax = false;
    this.winTab = 'messaggio';
    this.toInput = '';
    this.ccInput = '';
    this.bccInput = '';
    this.showBcc = c.bcc.length > 0;
    this.err = '';
    this.savedAt = '';
    if (!inWindow && this.mode === 'principale') this.ribbonTab = 'messaggio';
  }

  private async present(d: Draft, inWindow: boolean) {
    if (inWindow && this.backend.real && this.mode === 'principale') {
      try {
        if (await this.backend.openComposeWindow(d)) return;
      } catch (e) {
        this.flash({ kind: 'error', text: errorMessage(e) });
      }
    }
    this.startCompose(d, inWindow);
  }

  private async reopenDraft(id: number) {
    try {
      const d = await this.backend.openDraft(id);
      await this.stashDraft();
      this.sel = null;
      this.detail = null;
      await this.present(d, this.prefs.compose === 'finestra');
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Testo citato sotto la risposta, nel formato di Outlook. */
  static quote(m: MessageDetail): string {
    const lines = [`Da: ${personFull(m.from)}`, `Inviato: ${longDate(m.date)}`];
    if (m.to.length) lines.push(`A: ${peopleText(m.to)}`);
    if (m.cc.length) lines.push(`Cc: ${peopleText(m.cc)}`);
    lines.push(`Oggetto: ${m.subject}`);
    return `\n\n\n________________________________\n${lines.join('\n')}\n\n${m.text.trim()}`;
  }

  /** Bozza per Rispondi, Rispondi a tutti o Inoltra, dal messaggio aperto. */
  draftFrom(kind: Exclude<ComposeKind, 'new'>, m: MessageDetail, acct: Account): Draft {
    const d = this.emptyDraft(acct.id);
    const me: Person = { name: acct.name, email: acct.email };
    const clean = cleanSubject(m.subject);
    d.body = MailStore.quote(m);
    if (kind === 'reply' || kind === 'replyAll') {
      d.to = (m.replyTo.length ? m.replyTo : [m.from]).filter((p) => p.email || p.name);
      if (d.to.length && sameAddress(d.to[0], me) && m.to.length) d.to = [...m.to];
      d.subject = `Re: ${clean}`;
      d.replyToMessage = m.id;
    }
    if (kind === 'replyAll') {
      const seen = [...d.to];
      d.cc = [...m.to, ...m.cc].filter((p) => {
        if (sameAddress(p, me) || seen.some((s) => sameAddress(s, p) || (!p.email && s.name === p.name))) return false;
        seen.push(p);
        return true;
      });
    }
    if (kind === 'forward') {
      d.subject = `I: ${clean}`;
      d.attachments = m.attachments.filter((a) => !a.inline).map((a) => ({ name: a.name, size: a.size, mime: a.mime, source: { messageId: m.id, index: a.index } }));
    }
    if (m.messageId) {
      d.inReplyTo = kind === 'forward' ? null : m.messageId;
      d.references = [...m.references, m.messageId];
    }
    return d;
  }

  async openCompose(kind: ComposeKind) {
    const P = this.prefs;
    let m: MessageDetail | null = null;
    if (kind !== 'new') {
      m = this.detail && this.detail.id === this.sel ? this.detail : null;
      if (!m) return;
    }
    const acct = this.composeAccount(m?.accountId);
    if (!acct) {
      this.openWizard(false);
      return;
    }
    const d = m ? this.draftFrom(kind as Exclude<ComposeKind, 'new'>, m, acct) : this.emptyDraft(acct.id);
    const inWindow = kind === 'new' ? P.compose === 'finestra' : P.compose === 'finestra' && P.replyWin;
    await this.stashDraft();
    await this.present(d, inWindow);
  }

  /** Scrivere a un indirizzo cliccato nel testo di un messaggio (mailto:). */
  async composeTo(email: string) {
    const acct = this.composeAccount();
    if (!acct) return;
    const d = this.emptyDraft(acct.id);
    d.to = [{ name: '', email }];
    await this.stashDraft();
    await this.present(d, this.prefs.compose === 'finestra');
  }

  /** Ogni modifica: salvataggio automatico in Bozze dopo una breve pausa. */
  private touch() {
    clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => void this.autosave(), AUTOSAVE_MS);
  }

  async autosave() {
    const d = this.snapshot();
    if (!d || MailStore.isEmpty(d)) return;
    try {
      const r = await this.backend.saveDraft(d);
      if (this.c && this.c.key === d.key) {
        const same = this.c.attachments.length === d.attachments.length && this.c.attachments.every((a, i) => a.name === d.attachments[i].name);
        if (same) this.c.attachments = r.attachments;
        this.savedAt = clock().hhmm;
        this.saveError = '';
      }
    } catch (e) {
      // Il salvataggio automatico non interrompe chi scrive: lo segnala nel piede.
      this.saveError = errorMessage(e);
    }
  }

  setSubject(v: string) {
    if (!this.c) return;
    this.c.subject = v;
    this.touch();
  }

  setBody(v: string) {
    if (!this.c) return;
    this.c.body = v;
    this.touch();
  }

  setAccount(accountId: string) {
    if (!this.c) return;
    this.c.accountId = accountId;
    this.touch();
  }

  private inputOf(key: RecipientField): string {
    return key === 'to' ? this.toInput : key === 'cc' ? this.ccInput : this.bccInput;
  }

  private setInput(key: RecipientField, v: string) {
    if (key === 'to') this.toInput = v;
    else if (key === 'cc') this.ccInput = v;
    else this.bccInput = v;
  }

  addRecipient(key: RecipientField, person?: Person) {
    if (!this.c) return;
    const p = person ?? parseRecipient(this.inputOf(key));
    if (!p) return;
    if (!this.c[key].some((x) => sameAddress(x, p) && p.email)) this.c[key] = [...this.c[key], p];
    this.setInput(key, '');
    this.err = '';
    this.touch();
  }

  removeRecipient(key: RecipientField, index: number) {
    if (!this.c) return;
    this.c[key] = this.c[key].filter((_, j) => j !== index);
    this.touch();
  }

  recipientInput(key: RecipientField, value: string) {
    this.setInput(key, value);
    if (key === 'to') this.err = '';
    if (/[,;]$/.test(value)) this.addRecipient(key);
  }

  recipientKey(key: RecipientField, e: KeyboardEvent) {
    const input = this.inputOf(key);
    if (e.key === 'Enter' || e.key === ',' || e.key === ';') {
      if (!input.trim()) return;
      e.preventDefault();
      this.addRecipient(key);
      return;
    }
    if (e.key === 'Backspace' && !input && this.c && this.c[key].length) {
      this.c[key] = this.c[key].slice(0, -1);
      this.touch();
    }
  }

  /** Destinatari scritti ma non ancora confermati con Invio o punto e virgola. */
  private commitInputs() {
    for (const key of ['to', 'cc', 'bcc'] as RecipientField[]) if (this.inputOf(key).trim()) this.addRecipient(key);
  }

  async addFiles() {
    if (!this.c) return;
    try {
      const files = await this.backend.pickFiles();
      if (files.length && this.c) {
        this.c.attachments = [...this.c.attachments, ...files];
        this.touch();
      }
    } catch (e) {
      this.err = errorMessage(e);
    }
  }

  removeAttachment(index: number) {
    if (!this.c) return;
    this.c.attachments = this.c.attachments.filter((_, i) => i !== index);
    this.touch();
  }

  async send(): Promise<boolean> {
    if (!this.c || this.sending) return false;
    this.commitInputs();
    const d = this.snapshot()!;
    const all = [...d.to, ...d.cc, ...d.bcc];
    if (!all.length) {
      this.err = 'Aggiungi almeno un destinatario.';
      return false;
    }
    const bad = all.find((p) => !validEmail(p.email));
    if (bad) {
      this.err = `L’indirizzo “${bad.email || bad.name}” non è valido. Scrivi l’indirizzo email completo.`;
      return false;
    }
    clearTimeout(this.saveTimer);
    this.sending = true;
    try {
      const win = this.cWin;
      await this.backend.send(d);
      this.lastSent = { key: d.key, win };
      this.closeCompose();
      this.err = '';
      return true;
    } catch (e) {
      this.err = errorMessage(e);
      return false;
    } finally {
      this.sending = false;
    }
  }

  async undoSend() {
    const t = this.toast;
    if (!t?.outboxId) return;
    this.toast = null;
    try {
      const d = await this.backend.cancelSend(t.outboxId);
      if (!d) {
        this.flash({ kind: 'info', text: 'Il messaggio è già partito.' });
        return;
      }
      const win = this.lastSent?.key === d.key ? this.lastSent.win : this.prefs.compose === 'finestra';
      await this.stashDraft();
      await this.present(d, win);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Riapre un messaggio rimasto nella posta in uscita (per esempio dopo un errore). */
  async openOutboxItem(id: number) {
    if (this.toast?.outboxId === id) this.toast = null;
    try {
      const d = await this.backend.outboxTake(id);
      if (!d) return;
      await this.stashDraft();
      await this.present(d, this.prefs.compose === 'finestra');
      await this.reloadOutbox();
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  async closeWin() {
    const d = this.snapshot();
    if (!d) return;
    const had = !MailStore.isEmpty(d);
    await this.stashDraft();
    if (had) this.flash({ kind: 'draft', text: 'Bozza salvata in Bozze' }, 5000);
    else this.toast = null;
  }

  async discard() {
    const d = this.snapshot();
    this.closeCompose();
    this.err = '';
    if (d) await this.backend.discardDraft(d.accountId, d.key).catch(() => {});
  }

  popOut() {
    const d = this.snapshot();
    if (!d) return;
    if (this.backend.real) {
      this.closeCompose();
      void this.present(d, true);
      return;
    }
    this.cWin = true;
    this.winMin = false;
    this.winMax = false;
    this.winTab = 'messaggio';
  }

  async contacts(prefix: string): Promise<Person[]> {
    if (prefix.trim().length < 2) return [];
    try {
      return await this.backend.contacts(prefix.trim());
    } catch {
      return [];
    }
  }

  // ---------- Allegati ed esportazione ----------

  async openAttachment(index: number) {
    if (this.sel === null) return;
    await this.act(() => this.backend.openAttachment(this.sel!, index));
  }

  async saveAttachment(index: number, name: string) {
    if (this.sel === null) return;
    await this.act(async () => {
      if (await this.backend.saveAttachment(this.sel!, index, name)) this.flash({ kind: 'info', text: `Allegato salvato: ${name}` });
    });
  }

  async saveMessageAs() {
    const m = this.detail;
    if (!m) return;
    await this.act(async () => {
      const name = (m.subject || 'Messaggio').replace(/[\\/:*?"<>|]/g, '_').slice(0, 80);
      if (await this.backend.exportEml(m.id, name)) this.flash({ kind: 'info', text: 'Messaggio salvato.' });
    });
  }

  async exportFolder() {
    const f = this.currentFolder;
    if (!f) return;
    await this.act(async () => {
      const n = await this.backend.exportMbox(f.id, folderLabel(f));
      if (n !== null) this.flash({ kind: 'info', text: n === 1 ? 'Esportato 1 messaggio.' : `Esportati ${n} messaggi.` });
    });
  }

  async importArchive(kind: 'pst' | 'mbox') {
    await this.act(async () => {
      const a = await this.backend.importArchive(kind);
      if (!a) return;
      await this.reloadAccounts();
      await this.reloadFolders();
      this.backstage = false;
      const first = this.foldersOf(a.id).find((f) => f.total > 0) ?? this.foldersOf(a.id)[0];
      if (first) await this.pickFolder(first.id, `acc-${first.id}`);
      this.flash({ kind: 'info', text: `File di dati aperto: ${a.name}` });
    });
  }

  async openLink(url: string) {
    if (url.toLowerCase().startsWith('mailto:')) {
      const email = decodeURIComponent(url.slice(7).split('?')[0]);
      if (email) await this.composeTo(email);
      return;
    }
    await this.act(() => this.backend.openLink(url));
  }

  // ---------- Account ----------

  openWizard(first = false) {
    this.wizard = { open: true, first };
    this.settings = false;
    this.backstage = false;
  }

  closeWizard() {
    this.wizard = { open: false, first: false };
  }

  async addAccount(input: NewAccount): Promise<AddResult> {
    try {
      const account = await this.backend.addAccount(input);
      await this.reloadAccounts();
      await this.reloadFolders();
      const inbox = this.roleFolder(account.id, 'inbox');
      if (inbox && (this.accounts.length === 1 || this.folderId === null)) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
      return { ok: true, account };
    } catch (e) {
      return { ok: false, kind: errorKind(e), message: errorMessage(e) };
    }
  }

  async updateSync(accountId: string, patch: Partial<SyncOptions>) {
    const a = this.accounts.find((x) => x.id === accountId);
    if (!a) return;
    a.sync = { ...a.sync, ...patch };
    await this.act(() => this.backend.updateAccount($state.snapshot(a) as Account));
  }

  async renameAccount(accountId: string, name: string) {
    const a = this.accounts.find((x) => x.id === accountId);
    if (!a || !name.trim()) return;
    a.name = name.trim();
    await this.act(() => this.backend.updateAccount($state.snapshot(a) as Account));
  }

  async setPassword(accountId: string, password: string): Promise<boolean> {
    try {
      await this.backend.setPassword(accountId, password);
      this.flash({ kind: 'info', text: 'Password aggiornata.' });
      return true;
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
      return false;
    }
  }

  async removeAccount(accountId: string) {
    await this.act(async () => {
      await this.backend.removeAccount(accountId);
      await this.reloadAccounts();
      await this.reloadFolders();
    });
  }

  // ---------- Preferenze ----------

  setPref<K extends keyof Prefs>(k: K, v: Prefs[K]) {
    this.prefs[k] = v;
    if (k === 'compose' && v === 'pannello') this.prefs.replyWin = false;
    if (k === 'entry' && v === 'ingranaggio') this.backstage = false;
    this.save();
  }

  applyPreset(name: StyleName) {
    this.prefs = { ...PRESETS[name] };
    if (this.prefs.entry !== 'file') this.backstage = false;
    this.save();
  }

  toggleRibbon() {
    this.setPref('ribbon', this.prefs.ribbon === 'estesa' ? 'compatta' : 'estesa');
  }

  setTheme(t: ThemeChoice) {
    this.themeChoice = t;
    this.save();
  }

  toggleTheme() {
    this.setTheme(this.theme === 'scuro' ? 'chiaro' : 'scuro');
  }

  finishOnboarding() {
    this.prefs = { ...PRESETS[this.obChoice] };
    this.onboarding = false;
    this.save();
    if (this.backend.real && !this.accounts.length) this.openWizard(true);
  }

  openSettings(page: 'aspetto' | 'account' | 'info' = 'aspetto') {
    this.settings = true;
    this.setPage = page;
  }

  openBackstage() {
    this.backstage = true;
    this.bsPage = 'info';
  }

  // ---------- Viste dimostrative ----------

  /**
   * Porta l'interfaccia in uno degli stati del prototipo, per le schermate di
   * prova. Va chiamata dopo init(); non salva nulla.
   */
  async applyDemo(opts: { stile?: StyleName; view?: string; tema?: ThemeChoice; avvio?: boolean; offline?: boolean }) {
    if (opts.offline !== undefined) this.netOffline = opts.offline;
    if (opts.stile) this.prefs = { ...PRESETS[opts.stile] };
    if (opts.tema) this.themeChoice = opts.tema;
    if (opts.avvio !== undefined) {
      this.onboarding = opts.avvio;
      this.obChoice = opts.stile ?? 'classico';
    }
    const v = opts.view;
    const acct = this.composeAccount();
    if ((v === 'composizione' || v === 'inviato') && acct) {
      const win = this.prefs.compose === 'finestra';
      const d = this.emptyDraft(acct.id);
      d.to = [{ name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' }];
      d.cc = [{ name: 'Marco Ventura', email: 'm.ventura@valliarredi.it' }];
      if (win) {
        d.subject = 'Campioni del rovere per Bassano';
        d.body = 'Ciao Davide,\n\nil cliente vuole vedere i campioni del rovere prima di firmare il preventivo. Riesci a portarli giovedì in sede? Bastano il naturale e lo spazzolato.\n\nA presto,\nChiara';
      } else if (this.detail) {
        Object.assign(d, this.draftFrom('reply', this.detail, acct));
        d.body = 'Ciao Davide,\n\ngrazie, la revisione va bene. Il cliente vuole però vedere i campioni del rovere prima di firmare: riesci a portarli giovedì in sede?\n\nA presto,\nChiara' + d.body;
      }
      if (v === 'composizione') {
        this.startCompose(d, win);
        this.savedAt = '09:04';
      } else {
        this.lastSent = { key: d.key, win };
        await this.backend.send(d);
      }
    } else if (v === 'file') {
      this.backstage = true;
      this.bsPage = 'info';
    } else if (v === 'impostazioni') {
      this.openSettings('aspetto');
    } else if (v === 'account') {
      this.openSettings('account');
    } else if (v === 'informazioni') {
      this.openSettings('info');
    } else if (v === 'aggiungi') {
      this.openWizard(false);
    } else if (v === 'newsletter') {
      await this.pickTab('altre');
    } else if (v === 'uscita') {
      await this.pickOutbox('uscita');
    }
  }
}

export const store = new MailStore();
