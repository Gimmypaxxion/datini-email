// Stato dell'interfaccia e azioni dell'utente.
// I dati arrivano dal motore (backend.ts): quello vero nell'app installata,
// quello di esempio nel browser e nei test. Le preferenze di aspetto restano
// salvate sul computer.

import type { Backend, Tab } from './backend';
import { TauriBackend } from './backend';
import { MockBackend } from './mockBackend';
import { inTauri, isMac } from './platform';
import { DEFAULT_ACCENT, normalizeHex } from './accent';
import { addPlainSignature, hasQuote, htmlToText, newMessageHtml, ownHtml, plainFromHtml, replaceSignature, replyHtml, textToHtml } from './compose-html';
import { checkQuickSteps, emptyQuickStep, runOrder } from './quicksteps';
import { DEFAULT_FONT, composeEditor } from './editor.svelte';
import {
  clock,
  cleanSubject,
  parseRecipient,
  randomKey,
  personLabel,
  sameAddress,
  validEmail,
  whenText,
} from './format';
import { emptyRule } from './rules';
import { contactName } from './contacts';
import { CalendarState } from './calendar.svelte';
import { TasksState } from './tasks.svelte';
import type { TaskView } from './tasks';
import {
  errorKind,
  errorMessage,
  type Account,
  type AutoReply,
  type Category,
  type Contact,
  type InviteAction,
  type Rule,
  type Draft,
  type EngineEvent,
  type Folder,
  type MessageDetail,
  type MessageSummary,
  type NewAccount,
  type OutboxItem,
  type Person,
  type Role,
  type Signature,
  type SyncOptions,
  type QuickStep,
  type MessageTemplate,
  type Importance,
  type ReceiptReply,
  type SearchCriteria,
  type SearchFolder,
} from './types';
import { emptyCriteria, presetOf, splitPeople, type SearchFolderKind } from './searchfolders';

export type RibbonMode = 'estesa' | 'compatta';
export type EntryMode = 'file' | 'ingranaggio';
export type ComposeMode = 'finestra' | 'pannello';
export type ThemeChoice = 'sistema' | 'chiaro' | 'scuro';
export type StyleName = 'classico' | 'nuovo';
export type RibbonTab = 'home' | 'visualizza' | 'aiuto' | 'messaggio' | 'inserisci' | 'opzioni' | 'formato';
export type WinTab = 'messaggio' | 'inserisci' | 'opzioni' | 'formato';
export type ComposeKind = 'new' | 'reply' | 'replyAll' | 'forward';
export type RecipientField = 'to' | 'cc' | 'bcc';
/** Parte del programma mostrata: posta, calendario, contatti, attività (Ctrl+1, 2, 3, 4). */
export type AppModule = 'posta' | 'calendario' | 'contatti' | 'attivita';
export type SettingsPage = 'aspetto' | 'avvio' | 'account' | 'composizione' | 'modelli' | 'categorie' | 'regole' | 'azioni' | 'risposte' | 'pulizia' | 'info';

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
const HIDDEN_HINT_KEY = 'better-look-avviso-sottofondo';
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

/** Domanda di conferma prima di un'azione che non si può annullare. */
/** La finestra Nuova cartella di ricerca, mentre la si compila. */
export interface SearchFolderEdit {
  /** Vuoto per una cartella nuova. */
  id: string;
  accountId: string;
  kind: SearchFolderKind;
  name: string;
  /** L'utente ha scritto il nome: cambiando tipo non lo si tocca. */
  nameTouched: boolean;
  criteria: SearchCriteria;
  /** Persone come le scrive l'utente, separate da punto e virgola. */
  peopleText: string;
  error: string;
  busy: boolean;
}

export interface Confirm {
  title: string;
  text: string;
  /** Etichetta del pulsante che conferma (per esempio "Elimina"). */
  ok: string;
  danger?: boolean;
  /** Casella da spuntare (per esempio "Non chiedere più"): il valore resta qui. */
  check?: { label: string; value: boolean };
  /** Seconda scelta, accanto a quella principale (per esempio "Tutta la serie"). */
  alt?: string;
  resolve: (answer: boolean | 'alt') => void;
}

/** Nome di una cartella che si sta scrivendo nel riquadro (nuova o da rinominare). */
export interface FolderEdit {
  kind: 'new' | 'rename';
  accountId: string;
  /** Cartella che conterrà la nuova (null: primo livello dell'account). */
  parentId: number | null;
  /** Cartella da rinominare. */
  folderId: number | null;
  value: string;
  error: string;
  busy: boolean;
}

/** Tipo dei dati trascinati quando si sposta un messaggio su una cartella. */
export const DRAG_TYPE = 'application/x-better-look-messaggi';

interface Saved {
  prefs: Prefs;
  onboardingDone: boolean;
  themeChoice: ThemeChoice;
  /** Colore di accento scelto ("#RRGGBB"); assente o null per il blu predefinito. */
  accent?: string | null;
  showFolders: boolean;
  collapsed?: Record<string, boolean>;
  font?: { family: string; size: number };
  /** Vista per conversazioni. */
  conversations?: boolean;
  /** Avviso sul desktop all'arrivo di nuovi messaggi. */
  notify?: boolean;
  /** Riquadro di lettura: a destra, in basso o nascosto. */
  readingPane?: ReadingPanePos;
  /** Spazio fra le righe dell'elenco. */
  density?: Density;
  /** Larghezza dell'elenco (riquadro a destra), in pixel. */
  listW?: number;
  /** Altezza dell'elenco (riquadro in basso), come frazione. */
  listH?: number;
  /** Elenco aperto nel modulo Attività. */
  tasksView?: string;
  /** Chiudendo la finestra Better-Look resta attivo in sottofondo. */
  background?: boolean;
  /** Richieste di conferma di lettura: chiedi, sempre, mai. */
  receiptReply?: ReceiptReply;
  /** Per tutti i messaggi inviati, richiedi la conferma di lettura e di recapito. */
  trackRead?: boolean;
  trackDelivery?: boolean;
}

/** Misure predefinite dell'elenco dei messaggi. */
export const LIST_W = 420;
export const LIST_H = 0.46;

export type ReadingPanePos = 'destra' | 'basso' | 'nascosto';
export type Density = 'ampia' | 'media' | 'compatta';

const STORAGE_KEY = 'better-look.interfaccia.v1';

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
  view = $state<'cartella' | 'uscita' | 'ricerca'>('cartella');
  /** Cartelle di ricerca e quella aperta (con la vista 'ricerca'). */
  searchFolders = $state<SearchFolder[]>([]);
  searchFolderId = $state<string | null>(null);
  /** Finestra Nuova cartella di ricerca (o Modifica). */
  sfEdit = $state<SearchFolderEdit | null>(null);
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
  categories = $state<Category[]>([]);
  module = $state<AppModule>('posta');
  /** Calendario: vista, appuntamenti, modulo e promemoria. */
  cal: CalendarState = new CalendarState(this);
  tasks: TasksState = new TasksState(this);
  // Rubrica
  book = $state<Contact[]>([]);
  contactSel = $state<string | null>(null);
  /** Contatto aperto nel modulo di modifica (nuovo se senza id). */
  contactEdit = $state<Contact | null>(null);
  contactQuery = $state('');
  // Regole e avvisi
  rules = $state<Rule[]>([]);
  /** Regola aperta nell'editor delle impostazioni (nuova o da modificare). */
  ruleDraft = $state<Rule | null>(null);
  /** Avviso sul desktop all'arrivo di nuovi messaggi. */
  notifyDesktop = $state(true);
  /** Vista per conversazioni (Visualizza › Conversazioni). */
  conversations = $state(false);
  /** Visualizza › Riquadro di lettura. */
  readingPane = $state<ReadingPanePos>('destra');
  /** Visualizza › Densità. */
  density = $state<Density>('media');
  /** Chiudendo la finestra resta attivo in sottofondo; avvio all'accensione del computer. */
  background = $state(true);
  autoStart = $state(false);
  /** Rilevamento: risposta alle richieste di conferma e richieste per tutti i messaggi inviati. */
  receiptReply = $state<ReceiptReply>('chiedi');
  trackRead = $state(false);
  trackDelivery = $state(false);
  /** Misure dell'elenco, cambiate trascinando il separatore. */
  listW = $state(LIST_W);
  listH = $state(LIST_H);
  /** Riquadro di lettura nascosto: messaggio aperto al posto dell'elenco. */
  fullRead = $state(false);
  /** Conversazioni aperte nell'elenco: chiave → messaggi di tutte le cartelle, dal più recente. */
  threadOpen = $state<Record<string, MessageSummary[]>>({});
  /** Leggi ad alta voce: spento, in lettura o in pausa. */
  speech = $state<'off' | 'playing' | 'paused'>('off');
  private speechPoll: ReturnType<typeof setInterval> | undefined;
  private speechNative = false;

  // Aspetto
  prefs = $state<Prefs>({ ...PRESETS.classico });
  themeChoice = $state<ThemeChoice>('sistema');
  /** Colore di accento ("#RRGGBB"); null per il blu predefinito. */
  accent = $state<string | null>(null);
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
  setPage = $state<SettingsPage>('aspetto');
  acctSel = $state<string | null>(null);
  wizard = $state<{ open: boolean; first: boolean }>({ open: false, first: false });
  netOffline = $state(false);

  // Scrittura
  /** Carattere predefinito dei nuovi messaggi. */
  font = $state({ ...DEFAULT_FONT });
  signatures = $state<Signature[]>([]);
  /** Riquadro "Inserisci collegamento", vicino al pulsante che l'ha aperto. */
  linkAnchor = $state.raw<HTMLElement | null>(null);
  linkOpen = $state(false);
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
  /** Oggetto vuoto: Outlook chiede conferma prima di inviare. */
  askNoSubject = $state(false);
  /** Passaggio a testo normale: la formattazione si perde, si chiede conferma. */
  askPlain = $state(false);
  /** Messaggi selezionati insieme (Ctrl+clic, Maiusc+clic, Ctrl+A); vuoto con la selezione di uno solo. */
  multi = $state<number[]>([]);
  /** Azioni rapide (Home › Azioni rapide) e quella in modifica nelle impostazioni. */
  quickSteps = $state<QuickStep[]>([]);
  quickDraft = $state<QuickStep | null>(null);
  /** Modelli di messaggio (I miei modelli). */
  templates = $state<MessageTemplate[]>([]);
  /** La conversazione del messaggio aperto è ignorata (pulsante Ignora premuto). */
  threadIgnored = $state(false);
  toast = $state<Toast | null>(null);
  confirm = $state<Confirm | null>(null);
  folderEdit = $state<FolderEdit | null>(null);
  /** Importazione di un archivio in corso (total 0 se non si conosce ancora). */
  importing = $state<{ name: string; done: number; total: number } | null>(null);
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
        this.accent = (saved.accent && normalizeHex(saved.accent)) || null;
        if (typeof saved.showFolders === 'boolean') this.showFolders = saved.showFolders;
        if (saved.collapsed) this.collapsed = saved.collapsed;
        if (saved.font?.family && saved.font.size) this.font = { ...saved.font };
        this.conversations = !!saved.conversations;
        if (typeof saved.notify === 'boolean') this.notifyDesktop = saved.notify;
        if (saved.readingPane === 'destra' || saved.readingPane === 'basso' || saved.readingPane === 'nascosto') this.readingPane = saved.readingPane;
        if (saved.density === 'ampia' || saved.density === 'media' || saved.density === 'compatta') this.density = saved.density;
        if (typeof saved.listW === 'number' && saved.listW >= 280 && saved.listW <= 1600) this.listW = Math.round(saved.listW);
        if (typeof saved.listH === 'number' && saved.listH >= 0.2 && saved.listH <= 0.75) this.listH = saved.listH;
        if (typeof saved.background === 'boolean') this.background = saved.background;
        if (saved.receiptReply === 'chiedi' || saved.receiptReply === 'sempre' || saved.receiptReply === 'mai') this.receiptReply = saved.receiptReply;
        this.trackRead = saved.trackRead === true;
        this.trackDelivery = saved.trackDelivery === true;
        if (typeof saved.tasksView === 'string' && /^(giornata|importanti|pianificate|attivita|contrassegnati|elenco:[\w-]+)$/.test(saved.tasksView)) this.tasks.view = saved.tasksView as TaskView;
      }
    }
    if (typeof navigator !== 'undefined' && typeof window !== 'undefined') {
      this.netOffline = navigator.onLine === false;
      window.addEventListener('online', () => {
        this.netOffline = false;
        void this.backend.syncNow(null);
      });
      window.addEventListener('offline', () => (this.netOffline = true));
      // Tema e colore cambiati in un'altra finestra (per esempio quella principale
      // mentre è aperto un messaggio in scrittura).
      if (this.persist) {
        window.addEventListener('storage', (e) => {
          if (e.key !== STORAGE_KEY) return;
          const saved = readSaved();
          if (saved?.themeChoice) this.themeChoice = saved.themeChoice;
          if (saved) this.accent = (saved.accent && normalizeHex(saved.accent)) || null;
        });
      }
    }
    const mq = globalThis.matchMedia?.('(prefers-color-scheme: dark)');
    if (mq) {
      this.systemDark = mq.matches;
      mq.addEventListener?.('change', (e) => (this.systemDark = e.matches));
    }
  }

  save() {
    if (!this.persist) return;
    const data: Saved = {
      prefs: { ...this.prefs },
      onboardingDone: !this.onboarding,
      themeChoice: this.themeChoice,
      accent: this.accent,
      showFolders: this.showFolders,
      collapsed: { ...this.collapsed },
      font: { ...this.font },
      conversations: this.conversations,
      notify: this.notifyDesktop,
      readingPane: this.readingPane,
      density: this.density,
      listW: this.listW,
      listH: this.listH,
      tasksView: this.tasks.view,
      background: this.background,
      receiptReply: this.receiptReply,
      trackRead: this.trackRead,
      trackDelivery: this.trackDelivery,
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
    const stopEvents = this.backend.onEvent((e) => this.onEvent(e));
    const stopCommands = this.mode === 'principale' ? this.backend.onCommand((c) => void this.onCommand(c)) : () => {};
    this.unsubscribe = () => {
      stopEvents();
      stopCommands();
    };
    if (this.mode === 'principale') {
      void this.backend.setBackground(this.background).catch(() => undefined);
      void this.backend.autostart(null).then((on) => (this.autoStart = on), () => undefined);
    }
    await Promise.all([this.reloadAccounts(), this.reloadFolders(), this.reloadOutbox(), this.reloadSignatures(), this.reloadCategories(), this.reloadRules(), this.reloadQuickSteps(), this.reloadTemplates(), this.reloadSearchFolders()]);
    if (this.mode === 'principale') {
      if (this.persist) {
        this.cal.startReminders();
        this.tasks.startReminders();
      }
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
        if (this.module === 'attivita' && this.mode === 'principale') void this.tasks.reloadFlagged();
        break;
      case 'status': {
        const a = this.accounts.find((x) => x.id === e.accountId);
        if (a) a.status = e.status;
        break;
      }
      case 'accounts':
        void this.reloadAccounts().then(() => this.reloadFolders());
        break;
      case 'import':
        // Appena parte si torna alla posta: l'avanzamento è nella barra di stato.
        if (!this.importing) this.backstage = false;
        this.importing = { name: e.name, done: e.done, total: e.total };
        break;
      case 'signatures':
        void this.reloadSignatures();
        break;
      case 'categories':
        void this.reloadCategories();
        break;
      case 'rules':
        void this.reloadRules();
        break;
      case 'quickSteps':
        void this.reloadQuickSteps();
        break;
      case 'searchFolders':
        void this.reloadSearchFolders();
        break;
      case 'templates':
        void this.reloadTemplates();
        break;
      case 'contacts':
        void this.reloadContacts();
        break;
      case 'tasks':
        if (this.mode === 'principale' && this.module === 'attivita') void this.tasks.reload();
        break;
      case 'calendar':
        if (this.mode !== 'principale') break;
        if (this.module === 'calendario') void this.cal.reload();
        // L'invito aperto mostra se è già nel calendario.
        if (this.detail?.invite && this.sel === this.detail.id) void this.refreshInvite(this.detail.id);
        break;
      case 'newMail':
        if (this.mode === 'principale' && this.notifyDesktop) void this.alertNewMail(e.count, e.messages);
        break;
      case 'outbox':
        void this.reloadOutbox();
        if (this.mode !== 'principale') break;
        if (e.state === 'attesa') {
          this.flash({ kind: 'sent', text: 'Messaggio inviato', outboxId: e.outboxId }, 7000);
        } else if (e.state === 'programmato') {
          void this.reloadOutbox().then(() => {
            const it = this.outbox.find((o) => o.id === e.outboxId);
            this.flash({ kind: 'sent', text: it ? `Invio programmato per ${whenText(it.sendAfter)}` : 'Invio programmato', outboxId: e.outboxId }, 9000);
          });
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
    void this.reloadSearchFolders();
    await this.loadList(false);
    for (const key of Object.keys(this.threadOpen)) {
      const first = this.threadOpen[key][0];
      if (first) void this.loadThread(first.accountId, key);
    }
    if (this.sel !== null && this.detail && this.detail.id === this.sel) {
      const s = this.list.find((m) => m.id === this.sel);
      if (s) Object.assign(this.detail, { unread: s.unread, flagged: s.flagged, answered: s.answered });
    }
  }

  // ---------- Moduli e rubrica ----------

  setModule(m: AppModule) {
    if (this.module === m) return;
    this.module = m;
    this.backstage = false;
    if (m === 'contatti') void this.reloadContacts();
    if (m === 'calendario') void this.cal.reload();
    if (m === 'attivita') void this.tasks.reload();
  }

  /** Da un altro modulo al messaggio, nella sua cartella. */
  async revealMessage(m: MessageSummary) {
    this.setModule('posta');
    if (this.folderId !== m.folderId || this.crossFolder) await this.pickFolder(m.folderId, `acc-${m.folderId}`);
    if (this.findMsg(m.id)) await this.pickMsg(m.id);
  }

  // ---------- Inviti ----------

  private async refreshInvite(id: number) {
    try {
      const d = await this.backend.message(id, !!this.remoteOk[id]);
      if (this.detail?.id === id) this.detail.invite = d.invite ?? null;
    } catch {
      /* resta lo stato di prima */
    }
  }

  /** Risponde all'invito del messaggio aperto. Restituisce l'errore da mostrare, o null. */
  async respondInvite(action: InviteAction, send: boolean): Promise<string | null> {
    const d = this.detail;
    if (!d?.invite) return null;
    try {
      await this.backend.respondInvite(d.id, action, send);
      await this.refreshInvite(d.id);
      if (send && (action === 'accepted' || action === 'tentative' || action === 'declined')) {
        const s = this.list.find((m) => m.id === d.id);
        if (s) s.answered = true;
        d.answered = true;
      }
      const org = d.invite.event.organizer;
      const text: Record<InviteAction, string> = {
        accepted: 'Invito accettato e aggiunto al calendario',
        tentative: 'Risposta provvisoria: l’evento è nel calendario',
        declined: 'Invito rifiutato',
        add: 'Aggiunto al calendario',
        remove: 'Tolto dal calendario',
      };
      const sent = send && org && action !== 'add' && action !== 'remove' ? `. Risposta inviata a ${org.name || org.email}.` : '.';
      this.flash({ kind: 'info', text: text[action] + sent }, 5000);
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }

  /** "Rispondi con una riunione": riunione con mittente e destinatari del messaggio. */
  meetingFrom(m: MessageDetail | null = this.detail) {
    if (!m) return;
    const people = [m.from, ...m.to, ...m.cc];
    const seen = new Set<string>();
    const list = people.filter((p) => p.email && !seen.has(p.email.toLowerCase()) && seen.add(p.email.toLowerCase()));
    this.cal.newEvent({ meeting: true, title: cleanSubject(m.subject), attendees: list });
  }

  async reloadContacts() {
    try {
      this.book = await this.backend.addressBook();
      if (this.contactSel && !this.book.some((c) => c.id === this.contactSel)) this.contactSel = null;
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Contatti che corrispondono alla ricerca (nome, azienda, email, telefono). */
  get filteredContacts(): Contact[] {
    const q = this.contactQuery.trim().toLowerCase();
    if (!q) return this.book;
    const digits = q.replace(/\D/g, '');
    return this.book.filter(
      (c) =>
        contactName(c).toLowerCase().includes(q) ||
        c.company.toLowerCase().includes(q) ||
        c.emails.some((e) => e.toLowerCase().includes(q)) ||
        (digits.length >= 3 && c.phones.some((p) => p.number.replace(/\D/g, '').includes(digits))),
    );
  }

  get selectedContact(): Contact | null {
    return this.book.find((c) => c.id === this.contactSel) ?? null;
  }

  newContact(prefill: Partial<Contact> = {}) {
    this.setModule('contatti');
    this.contactEdit = { id: '', firstName: '', lastName: '', displayName: '', emails: [], phones: [], company: '', jobTitle: '', address: '', website: '', birthday: '', notes: '', ...prefill };
  }

  editContact(c: Contact | null = this.selectedContact) {
    if (c) this.contactEdit = JSON.parse(JSON.stringify(c)) as Contact;
  }

  /** Salva il contatto in modifica. Restituisce l'errore da mostrare, o null. */
  async saveContactEdit(): Promise<string | null> {
    const c = this.contactEdit;
    if (!c) return null;
    try {
      const saved = await this.backend.saveContact(c);
      this.contactEdit = null;
      await this.reloadContacts();
      this.contactSel = saved.id;
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }

  async deleteContacts(ids: string[]) {
    if (!ids.length) return;
    const one = this.book.find((c) => c.id === ids[0]);
    const ok = await this.ask({
      title: ids.length === 1 && one ? `Eliminare “${contactName(one)}”?` : `Eliminare ${ids.length} contatti?`,
      text: 'Il contatto sparisce dalla rubrica. I messaggi restano.',
      ok: 'Elimina',
      danger: true,
    });
    if (!ok) return;
    await this.act(async () => {
      await this.backend.deleteContacts(ids);
      this.contactSel = null;
      this.contactEdit = null;
      await this.reloadContacts();
    });
  }

  async importContacts() {
    try {
      const r = await this.backend.importContacts();
      if (!r) return;
      await this.reloadContacts();
      const parts = [r.added && `${r.added} ${r.added === 1 ? 'nuovo' : 'nuovi'}`, r.updated && `${r.updated} ${r.updated === 1 ? 'completato' : 'completati'}`, r.unchanged && `${r.unchanged} già presenti`].filter(Boolean);
      this.flash({ kind: 'info', text: `Contatti importati: ${parts.join(', ') || 'nessuno'}.` }, 7000);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) }, 9000);
    }
  }

  async exportContacts(ids: string[] | null = null) {
    try {
      const n = await this.backend.exportContacts(ids);
      if (n !== null) this.flash({ kind: 'info', text: n === 1 ? '1 contatto esportato in vCard.' : `${n} contatti esportati in vCard.` }, 5000);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Posta scambiata con un indirizzo (dalla scheda del contatto). */
  async showMailFrom(email: string, id: number | null = null) {
    this.setModule('posta');
    await this.search(email);
    if (id !== null && this.list.some((m) => m.id === id)) await this.pickMsg(id);
  }

  /** "Aggiungi ai contatti" da un messaggio: nuovo contatto con nome e indirizzo. */
  async addToContacts(p: Person) {
    await this.reloadContacts();
    const found = this.book.find((c) => c.emails.some((e) => e.toLowerCase() === p.email.toLowerCase()));
    if (found) {
      this.setModule('contatti');
      this.contactSel = found.id;
      return;
    }
    const name = p.name.trim() && p.name.trim() !== p.email ? p.name.trim() : '';
    const sp = name.lastIndexOf(' ');
    this.newContact({ firstName: sp > 0 ? name.slice(0, sp) : name, lastName: sp > 0 ? name.slice(sp + 1) : '', emails: [p.email] });
  }

  // ---------- Azioni rapide ----------

  // Anche nelle finestre del messaggio: le loro impostazioni salvano l'elenco intero.
  async reloadQuickSteps() {
    try {
      this.quickSteps = await this.backend.quickSteps();
    } catch {
      /* senza azioni rapide si lavora lo stesso */
    }
  }

  /** Salva l'elenco delle azioni rapide. Restituisce l'errore da mostrare, o ''. */
  async saveQuickSteps(list: QuickStep[]): Promise<string> {
    const bad = checkQuickSteps(list);
    if (bad) return bad;
    try {
      this.quickSteps = await this.backend.saveQuickSteps(list);
      return '';
    } catch (e) {
      return errorMessage(e);
    }
  }

  /** Nuova azione rapida, nelle impostazioni. */
  newQuickStep() {
    this.quickDraft = emptyQuickStep();
    this.openSettings('azioni');
  }

  manageQuickSteps() {
    this.quickDraft = null;
    this.openSettings('azioni');
  }

  /** Bozza di risposta o inoltro per un'azione rapida (dal messaggio, anche se non è aperto). */
  private async quickDraftFor(kind: 'rispondi' | 'rispondi-tutti' | 'inoltra' | 'inoltra-allegato', id: number): Promise<Draft | null> {
    const s = this.findMsg(id);
    const acct = this.composeAccount(s?.accountId);
    if (!acct) {
      this.openWizard(false);
      return null;
    }
    if (kind === 'inoltra-allegato') return s ? this.attachmentDraft([s], acct) : null;
    const m = this.detail && this.detail.id === id ? this.detail : await this.backend.message(id, false);
    const quote = await this.backend.quoteHtml(id).catch(() => undefined);
    return this.draftFrom(kind === 'rispondi' ? 'reply' : kind === 'rispondi-tutti' ? 'replyAll' : 'forward', m, acct, quote);
  }

  /**
   * Esegue un'azione rapida sui messaggi selezionati (o su quelli indicati):
   * prima cambia lo stato (letto, contrassegno, categorie), poi prepara la
   * risposta o l'inoltro, infine sposta. La risposta si apre alla fine. Con
   * più messaggi l'inoltro li allega tutti; rispondere si fa uno alla volta.
   */
  async runQuickStep(q: QuickStep, target: number | number[] | null = null) {
    const ids = (target === null ? this.selection : Array.isArray(target) ? target : [target]).filter((id) => this.findMsg(id));
    if (!ids.length) return;
    let compose: Draft | null = null;
    let moved = false;
    let skipped = false;
    try {
      for (const a of runOrder(q.actions)) {
        switch (a.kind) {
          case 'letto':
          case 'da-leggere':
            await this.markRead(ids, a.kind === 'letto');
            break;
          case 'contrassegna':
          case 'togli-contrassegno':
            await this.setFlag(ids, a.kind === 'contrassegna');
            break;
          case 'categoria':
            if (a.category) await this.setCategoryOn(ids, a.category, true);
            break;
          case 'togli-categorie':
            await this.clearCategoriesOf(ids);
            break;
          case 'rispondi':
          case 'rispondi-tutti':
          case 'inoltra':
          case 'inoltra-allegato': {
            if (ids.length > 1 && (a.kind === 'rispondi' || a.kind === 'rispondi-tutti')) {
              skipped = true;
              break;
            }
            if (ids.length > 1) {
              const acct = this.composeAccount(this.findMsg(ids[0])?.accountId);
              if (!acct) {
                this.openWizard(false);
                return;
              }
              compose = this.attachmentDraft(this.msgsOf(ids), acct);
            } else compose = await this.quickDraftFor(a.kind, ids[0]);
            if (!compose) return;
            if (a.to?.length && (a.kind === 'inoltra' || a.kind === 'inoltra-allegato')) compose.to = a.to.map((p) => ({ ...p }));
            if (a.text?.trim()) {
              const extra = textToHtml(a.text.trim());
              if (compose.html !== null && compose.html !== undefined) compose.html = extra + compose.html;
              compose.body = `${a.text.trim()}\n${compose.body}`;
            }
            break;
          }
          case 'sposta':
            if (a.folder !== null && a.folder !== undefined) {
              await this.dropOnFolder(ids, a.folder);
              moved = true;
            }
            break;
          case 'archivia':
            await this.moveMany(ids, 'archive');
            moved = true;
            break;
          case 'elimina':
            await this.deleteMany(ids);
            moved = true;
            break;
          case 'ignora':
            for (const id of ids) await this.backend.ignoreThread(id, true);
            this.scheduleRefresh();
            moved = true;
            break;
        }
      }
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
      return;
    }
    if (compose) {
      await this.stashDraft();
      await this.present(compose, this.prefs.compose === 'finestra' && this.prefs.replyWin);
    } else {
      const what = ids.length > 1 ? `${ids.length} messaggi` : 'il messaggio';
      const text = skipped
        ? `Azione rapida «${q.name}» eseguita su ${what}. Per rispondere scegli un messaggio alla volta.`
        : moved
          ? `Azione rapida «${q.name}» eseguita: ${ids.length > 1 ? `${ids.length} messaggi spostati` : 'il messaggio è stato spostato'}.`
          : `Azione rapida «${q.name}» eseguita${ids.length > 1 ? ` su ${what}` : ''}.`;
      this.flash({ kind: 'info', text }, 3500);
    }
  }

  /** Ctrl+Maiusc+1..9. */
  runQuickShortcut(n: number) {
    if (!this.selection.length) return;
    const q = this.quickSteps.find((x) => x.shortcut === n);
    if (q) void this.runQuickStep(q);
    else this.flash({ kind: 'info', text: `Nessuna azione rapida usa Ctrl+Maiusc+${n}. Puoi assegnarla in Azioni rapide.` }, 4000);
  }

  // ---------- Modelli di messaggio ----------

  async reloadSearchFolders() {
    if (this.mode !== 'principale') return;
    try {
      this.searchFolders = await this.backend.searchFolders();
    } catch {
      return;
    }
    if (this.view === 'ricerca' && !this.currentSearchFolder) {
      const inbox = this.defaultInbox();
      if (inbox) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
    }
  }

  searchFoldersOf(accountId: string): SearchFolder[] {
    return this.searchFolders.filter((f) => f.accountId === accountId).sort((a, b) => a.name.localeCompare(b.name, 'it'));
  }

  /**
   * Nuova cartella di ricerca (Ctrl+Maiusc+P), oppure Modifica di una esistente.
   * Con `text` parte dalla ricerca in corso: la ricerca salvata come cartella.
   */
  openSearchFolderDialog(opts: { accountId?: string; edit?: SearchFolder; text?: string } = {}) {
    const mail = this.accounts.filter((a) => a.kind !== 'local');
    const pickAcct = (id?: string) => (id && this.accounts.some((a) => a.id === id) ? id : (this.currentAccount?.kind !== 'local' && this.currentAccount?.id) || mail[0]?.id || this.accounts[0]?.id || '');
    if (!this.accounts.length) {
      this.openWizard(false);
      return;
    }
    if (opts.edit) {
      const f = opts.edit;
      this.sfEdit = { id: f.id, accountId: f.accountId, kind: presetOf(f.kind).kind, name: f.name, nameTouched: true, criteria: { ...emptyCriteria(), ...$state.snapshot(f.criteria) }, peopleText: f.criteria.people.join('; '), error: '', busy: false };
      return;
    }
    const text = opts.text?.trim();
    const kind: SearchFolderKind = text ? 'parole' : 'da-leggere';
    const p = presetOf(kind);
    this.sfEdit = { id: '', accountId: pickAcct(opts.accountId), kind, name: text ? text.slice(0, 80) : p.name, nameTouched: !!text, criteria: { ...emptyCriteria(), ...p.criteria, text: text ?? '' }, peopleText: '', error: '', busy: false };
  }

  /** Nella finestra: un altro tipo di cartella di ricerca. */
  pickSearchFolderKind(kind: SearchFolderKind) {
    const e = this.sfEdit;
    if (!e || e.kind === kind) return;
    const p = presetOf(kind);
    const keepText = kind === 'parole' || kind === 'personalizzata' ? e.criteria.text : '';
    e.kind = kind;
    e.criteria = { ...emptyCriteria(), ...p.criteria, text: keepText };
    if (kind !== 'persone' && kind !== 'personalizzata') e.peopleText = '';
    if (!e.nameTouched) e.name = p.name;
    e.error = '';
  }

  /** Persone o parole scritte nella finestra: il nome proposto le segue finché non lo si cambia. */
  setSearchFolderPeople(text: string) {
    const e = this.sfEdit;
    if (!e) return;
    e.peopleText = text;
    e.error = '';
    if (!e.nameTouched && e.kind === 'persone') e.name = splitPeople(text).join(', ').slice(0, 80) || presetOf('persone').name;
  }

  setSearchFolderText(text: string) {
    const e = this.sfEdit;
    if (!e) return;
    e.criteria.text = text;
    e.error = '';
    if (!e.nameTouched && e.kind === 'parole') e.name = text.trim().slice(0, 80) || presetOf('parole').name;
  }

  async saveSearchFolderDialog() {
    const e = this.sfEdit;
    if (!e || e.busy) return;
    const people = splitPeople(e.peopleText);
    e.busy = true;
    e.error = '';
    try {
      const old = this.searchFolders.find((f) => f.id === e.id);
      const saved = await this.backend.saveSearchFolder({ id: e.id, accountId: e.accountId, name: e.name, kind: e.kind, criteria: { ...$state.snapshot(e.criteria), people }, total: old?.total ?? 0, unread: old?.unread ?? 0 });
      this.sfEdit = null;
      const i = this.searchFolders.findIndex((f) => f.id === saved.id);
      if (i >= 0) this.searchFolders[i] = saved;
      else this.searchFolders = [...this.searchFolders, saved];
      this.collapsed[`ricerca:${saved.accountId}`] = false;
      this.collapsed[saved.accountId] = false;
      this.save();
      this.setModule('posta');
      await this.pickSearchFolder(saved.id, `sf-${saved.id}`);
    } catch (err) {
      if (this.sfEdit === e) {
        e.error = errorMessage(err);
        e.busy = false;
      }
    }
  }

  async deleteSearchFolder(id: string) {
    const f = this.searchFolders.find((x) => x.id === id);
    if (!f) return;
    const ok = await this.ask({ title: `Eliminare la cartella di ricerca “${f.name}”?`, text: 'I messaggi restano dove sono: si elimina solo la ricerca salvata.', ok: 'Elimina' });
    if (!ok) return;
    await this.act(async () => {
      await this.backend.deleteSearchFolder(id);
      this.searchFolders = this.searchFolders.filter((x) => x.id !== id);
      if (this.searchFolderId === id && this.view === 'ricerca') {
        const inbox = this.defaultInbox();
        if (inbox) await this.pickFolder(inbox.id, `fav-${inbox.id}`);
      }
    });
  }

  /** Segna come letti i messaggi della cartella di ricerca. */
  async markSearchFolderRead(id: string) {
    await this.act(async () => {
      const ids = (await this.backend.searchFolderMessages(id)).filter((m) => m.unread).map((m) => m.id);
      if (!ids.length) return;
      for (const m of this.list) if (ids.includes(m.id)) m.unread = false;
      await this.backend.setFlags(ids, true, null);
      this.scheduleRefresh();
    });
  }

  async reloadTemplates() {
    try {
      this.templates = await this.backend.templates();
    } catch {
      /* senza modelli si scrive lo stesso */
    }
  }

  /** Salva un modello. Restituisce l'errore da mostrare, o ''. */
  async saveTemplate(t: MessageTemplate): Promise<string> {
    try {
      const saved = await this.backend.saveTemplate(t);
      await this.reloadTemplates();
      if (!this.templates.some((x) => x.id === saved.id)) this.templates = [...this.templates, saved];
      return '';
    } catch (e) {
      return errorMessage(e);
    }
  }

  async deleteTemplate(id: string) {
    try {
      await this.backend.deleteTemplate(id);
      this.templates = this.templates.filter((t) => t.id !== id);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Inserisce il modello nel messaggio che si sta scrivendo (l'oggetto solo se manca). */
  insertTemplate(t: MessageTemplate) {
    const c = this.c;
    if (!c) return;
    if (t.subject.trim() && !c.subject.trim()) c.subject = t.subject.trim();
    if (c.plain) {
      const text = htmlToText(t.html);
      c.body = c.body.trim() ? `${text}\n${c.body}` : text;
    } else if (composeEditor.attached) {
      composeEditor.insertHtml(t.html);
    } else {
      c.html = t.html + (c.html ?? textToHtml(c.body));
      c.body = htmlToText(c.html);
    }
    this.touch();
  }

  /** Il messaggio che si sta scrivendo diventa un modello (senza firma né messaggio citato). */
  async saveDraftAsTemplate(name: string): Promise<string> {
    const c = this.c;
    if (!c) return '';
    const html = c.plain ? textToHtml(c.body) : ownHtml(c.html ?? textToHtml(c.body));
    if (!htmlToText(html).trim() && !/<img/i.test(html)) return 'Il messaggio è vuoto: scrivi il testo da riusare, poi salvalo come modello.';
    const same = this.templates.find((t) => t.name.trim().toLowerCase() === name.trim().toLowerCase());
    const subject = cleanSubject(c.subject);
    return this.saveTemplate({ id: same?.id ?? '', name, subject, html });
  }

  // ---------- Regole, risposte automatiche e avvisi ----------

  // Anche nelle finestre del messaggio: le loro impostazioni salvano l'elenco intero.
  async reloadRules() {
    try {
      this.rules = await this.backend.rules();
    } catch {
      /* senza regole la posta arriva lo stesso */
    }
  }

  /** Salva l'elenco delle regole. Restituisce l'errore da mostrare, o null. */
  async saveRules(list: Rule[]): Promise<string | null> {
    try {
      this.rules = await this.backend.saveRules(list);
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }

  /** Esegui ora: le regole indicate sui messaggi già presenti nella cartella. Restituisce l'esito da mostrare. */
  async runRulesNow(ids: string[], folderId: number): Promise<string> {
    try {
      const n = await this.backend.runRules(ids, folderId);
      const where = folderLabel(this.folders.find((f) => f.id === folderId));
      return n === 0 ? `Nessun messaggio in ${where} corrisponde alla regola.` : n === 1 ? `Regola applicata a 1 messaggio in ${where}.` : `Regola applicata a ${n} messaggi in ${where}.`;
    } catch (e) {
      return errorMessage(e);
    }
  }

  /** "Crea regola" da un messaggio: mittente già compilato, come in Outlook. */
  newRuleFrom(id: number | null = this.sel) {
    const m = this.list.find((x) => x.id === id) ?? (this.detail?.id === id ? this.detail : null);
    const r = emptyRule();
    if (m) {
      r.name = `Messaggi da ${personLabel(m.from)}`;
      r.when.from = [m.from.email || m.from.name];
      r.accountId = m.accountId;
    }
    this.ruleDraft = r;
    this.backstage = false;
    this.openSettings('regole');
  }

  setNotify(on: boolean) {
    this.notifyDesktop = on;
    this.save();
  }

  private async alertNewMail(count: number, messages: MessageSummary[]) {
    const first = messages[0];
    if (!first) return;
    const title = count === 1 ? personLabel(first.from) : `${count} nuovi messaggi`;
    const body = count === 1 ? first.subject || '(nessun oggetto)' : messages.slice(0, 3).map((m) => `${personLabel(m.from)}: ${m.subject || '(nessun oggetto)'}`).join('\n');
    await this.backend.notify(title, body);
  }

  /** Salva la risposta automatica di un account. Restituisce l'errore da mostrare, o null. */
  async saveAutoReply(accountId: string, reply: AutoReply): Promise<string | null> {
    try {
      await this.backend.setAutoReply(accountId, reply);
      await this.reloadAccounts();
      return null;
    } catch (e) {
      return errorMessage(e);
    }
  }

  /** Account con la risposta automatica attiva adesso (per l'avviso in alto). */
  get autoReplyOn(): Account[] {
    const now = Date.now();
    return this.accounts.filter((a) => {
      const r = a.autoReply;
      return !!r?.enabled && (r.start === null || now >= r.start) && (r.end === null || now < r.end);
    });
  }

  async turnOffAutoReply(accountId: string) {
    const a = this.accounts.find((x) => x.id === accountId);
    if (!a?.autoReply) return;
    const err = await this.saveAutoReply(accountId, { ...a.autoReply, enabled: false });
    this.flash(err ? { kind: 'error', text: err } : { kind: 'info', text: 'Risposte automatiche disattivate.' }, 4000);
  }

  async reloadCategories() {
    try {
      this.categories = await this.backend.categories();
    } catch {
      /* senza categorie si legge e si scrive lo stesso */
    }
  }

  category(id: string): Category | undefined {
    return this.categories.find((c) => c.id === id);
  }

  async reloadSignatures() {
    try {
      this.signatures = await this.backend.signatures();
    } catch {
      /* le firme non sono essenziali per leggere e scrivere */
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
    const id = this.view === 'ricerca' ? this.currentSearchFolder?.accountId : this.currentFolder?.accountId;
    return (id && this.accounts.find((a) => a.id === id)) || this.accounts[0] || null;
  }

  get currentSearchFolder(): SearchFolder | null {
    return this.searchFolders.find((f) => f.id === this.searchFolderId) ?? null;
  }

  /** L'elenco mostra messaggi di più cartelle: risultati della ricerca o cartella di ricerca. */
  get crossFolder(): boolean {
    return !!this.q.trim() || this.view === 'ricerca';
  }

  get isInbox(): boolean {
    return this.currentFolder?.role === 'inbox';
  }

  get selected(): MessageSummary | null {
    return this.sel === null ? null : this.findMsg(this.sel);
  }

  /** Un messaggio dell'elenco o di una conversazione aperta. */
  findMsg(id: number): MessageSummary | null {
    const inList = this.list.find((m) => m.id === id);
    if (inList) return inList;
    for (const items of Object.values(this.threadOpen)) {
      const m = items.find((x) => x.id === id);
      if (m) return m;
    }
    return null;
  }

  /** Contrassegno cambiato altrove (per esempio da Attività): lo aggiorna nell'elenco. */
  markFlag(id: number, flagged: boolean) {
    this.eachCopy(id, (x) => (x.flagged = flagged));
  }

  /** Applica una modifica a tutte le copie del messaggio (elenco, conversazioni, lettura). */
  private eachCopy(id: number, fn: (m: MessageSummary) => void) {
    const l = this.list.find((m) => m.id === id);
    if (l) fn(l);
    for (const items of Object.values(this.threadOpen)) {
      const m = items.find((x) => x.id === id);
      if (m) fn(m);
    }
    if (this.detail?.id === id) fn(this.detail);
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
    if (this.module !== 'posta') return ['home', 'aiuto'];
    return this.paneCompose ? ['home', 'visualizza', 'aiuto', 'messaggio', 'inserisci', 'opzioni', 'formato'] : ['home', 'visualizza', 'aiuto'];
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
    if (this.view === 'ricerca') return this.currentSearchFolder?.name ?? 'Cartella di ricerca';
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
    this.fullRead = false;
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

  async pickSearchFolder(id: string, selId: string) {
    await this.stashPaneDraft();
    this.multi = [];
    this.fullRead = false;
    this.view = 'ricerca';
    this.searchFolderId = id;
    this.folderSel = selId;
    this.q = '';
    this.err = '';
    this.sel = null;
    this.detail = null;
    this.detailError = '';
    await this.loadList(true);
    if (this.list.length) await this.pickMsg(this.list[0].id);
  }

  async pickOutbox(selId: string) {
    await this.stashPaneDraft();
    this.multi = [];
    this.fullRead = false;
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
    this.fullRead = false;
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
      else if (this.view === 'ricerca') items = this.searchFolderId ? await this.backend.searchFolderMessages(this.searchFolderId) : [];
      else if (this.folderId === null) items = [];
      else items = await this.backend.messages(this.folderId, this.isInbox ? this.tab : null, 0, limit);
      if (token !== this.listToken) return;
      // In una cartella di ricerca il messaggio aperto resta finché non se ne
      // sceglie un altro, anche se non rispetta più le condizioni (per esempio
      // appena letto in Posta da leggere), come in Outlook.
      if (this.view === 'ricerca' && !q && !reset && this.sel !== null && !items.some((m) => m.id === this.sel)) {
        const keep = this.list.find((m) => m.id === this.sel);
        if (keep) items = [...items, keep].sort((a, b) => b.date - a.date || b.id - a.id);
      }
      this.list = items;
      this.listEnd = this.crossFolder || items.length < limit;
      if (reset) this.multi = [];
      else if (this.multi.length) {
        const left = this.multi.filter((id) => items.some((m) => m.id === id));
        if (left.length !== this.multi.length) this.multi = left.length > 1 ? left : [];
      }
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
    if (this.listEnd || this.listLoading || this.crossFolder || this.folderId === null) return;
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
    const m = this.findMsg(id);
    if (!m) return;
    if (this.multi.length) this.multi = [];
    if (this.speech !== 'off' && this.sel !== id) this.stopSpeech();
    if (m.draft && this.currentFolder?.role === 'drafts' && !this.crossFolder) {
      await this.reopenDraft(m.id);
      return;
    }
    await this.stashPaneDraft();
    this.sel = id;
    this.err = '';
    if (markRead && m.unread) {
      this.eachCopy(id, (x) => (x.unread = false));
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
      const s = this.findMsg(id);
      this.detail = s ? { ...d, unread: s.unread, flagged: s.flagged } : d;
      this.autoReceipt(this.detail);
      this.threadIgnored = await this.backend.threadIgnored(id).catch(() => false);
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

  // ---------- Sottofondo e avvio ----------

  /** Comandi dall'icona vicino all'orologio, e la finestra appena nascosta. */
  private async onCommand(c: string) {
    if (c === 'nuovo-messaggio') {
      this.setModule('posta');
      await this.openCompose('new');
    } else if (c === 'nascosta') {
      // La prima volta si spiega dove è finito il programma.
      let told = false;
      try {
        told = globalThis.localStorage?.getItem(HIDDEN_HINT_KEY) === '1';
        globalThis.localStorage?.setItem(HIDDEN_HINT_KEY, '1');
      } catch {
        /* senza archivio si avvisa ogni volta */
      }
      if (!told) {
        const where = isMac ? 'nella barra dei menu in alto o nel Dock' : 'vicino all’orologio';
        void this.backend.notify('Better-Look è ancora attivo', `Continua a ricevere la posta e a ricordarti gli appuntamenti. Lo riapri dall’icona ${where}; per chiuderlo davvero scegli Esci.`);
      }
    }
  }

  async setBackground(on: boolean) {
    this.background = on;
    this.save();
    try {
      await this.backend.setBackground(on);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  async setAutoStart(on: boolean) {
    try {
      this.autoStart = await this.backend.autostart(on);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Selezione multipla ----------

  /** Punto di partenza di Maiusc+clic e Maiusc+frecce, e ultimo raggiunto. */
  private anchor: number | null = null;
  private cursor: number | null = null;

  get isMulti(): boolean {
    return this.multi.length > 1;
  }

  /** I messaggi su cui agiscono i comandi: quelli selezionati insieme, o quello aperto. */
  get selection(): number[] {
    return this.isMulti ? this.multi : this.sel !== null ? [this.sel] : [];
  }

  isSelected(id: number): boolean {
    return this.isMulti ? this.multi.includes(id) : this.sel === id;
  }

  /** Clic su un messaggio: con Ctrl (Cmd sul Mac) lo aggiunge o lo toglie, con Maiusc seleziona l'intervallo. */
  async clickMsg(id: number, mods: { ctrl?: boolean; shift?: boolean } = {}) {
    const from = this.anchor ?? this.sel;
    if (mods.shift && from !== null) return this.selectRange(from, id);
    if (mods.ctrl) return this.toggleSelect(id);
    this.anchor = id;
    this.cursor = id;
    await this.pickMsg(id);
  }

  /** Ctrl+clic, o la casella al posto della foto. */
  async toggleSelect(id: number) {
    const base = [...this.selection];
    const i = base.indexOf(id);
    if (i >= 0) base.splice(i, 1);
    else base.push(id);
    this.anchor = id;
    this.cursor = id;
    await this.setMulti(base);
  }

  /** Maiusc+clic: tutti i messaggi dell'elenco fra i due. */
  async selectRange(from: number, to: number) {
    const ids = this.list.map((m) => m.id);
    const a = ids.indexOf(from);
    const b = ids.indexOf(to);
    if (a < 0 || b < 0) return this.toggleSelect(to);
    this.anchor = from;
    this.cursor = to;
    await this.setMulti(ids.slice(Math.min(a, b), Math.max(a, b) + 1));
  }

  /** Ctrl+A nell'elenco. */
  async selectAll() {
    const ids = this.list.map((m) => m.id);
    if (!ids.length) return;
    this.anchor = ids[0];
    this.cursor = ids[ids.length - 1];
    await this.setMulti(ids);
  }

  /** Maiusc+frecce: allarga o restringe la selezione dal messaggio di partenza. */
  async extendSelection(step: 1 | -1) {
    const ids = this.list.map((m) => m.id);
    const from = this.anchor !== null && ids.includes(this.anchor) ? this.anchor : this.sel;
    if (from === null) return this.moveSelection(step);
    const i = ids.indexOf(this.cursor ?? from);
    if (i < 0) return;
    await this.selectRange(from, ids[Math.min(Math.max(i + step, 0), ids.length - 1)]);
  }

  /** Togli la selezione multipla: nessun messaggio aperto. */
  clearMulti() {
    this.multi = [];
    this.sel = null;
    this.detail = null;
  }

  /** Uno solo torna la selezione normale (si apre); nessuno svuota il riquadro. */
  private async setMulti(ids: number[]) {
    const unique = [...new Set(ids)].filter((id) => this.findMsg(id));
    if (unique.length > 1) {
      if (this.speech !== 'off') this.stopSpeech();
      await this.stashPaneDraft();
      this.multi = unique;
      this.sel = null;
      this.detail = null;
      this.fullRead = false;
      return;
    }
    this.multi = [];
    if (unique.length === 1) {
      // Ne resta uno: da lì riparte Maiusc+clic.
      this.anchor = unique[0];
      this.cursor = unique[0];
      await this.pickMsg(unique[0], false);
    } else {
      this.sel = null;
      this.detail = null;
    }
  }

  private msgsOf(ids: number[]): MessageSummary[] {
    return ids.map((id) => this.findMsg(id)).filter((m): m is MessageSummary => !!m);
  }

  /** Elimina più messaggi insieme. */
  async deleteMany(ids: number[] = this.selection) {
    if (!ids.length) return;
    this.dropFromList(ids);
    await this.act(() => this.backend.deleteMessages(ids));
  }

  /** Archivia, posta indesiderata...: più messaggi insieme. */
  async moveMany(ids: number[], role: Role) {
    if (!ids.length || (this.currentFolder?.role === role && !this.crossFolder)) return;
    this.dropFromList(ids);
    await this.act(() => this.backend.moveToRole(ids, role));
  }

  async moveSelected(role: Role) {
    await this.moveMany(this.selection, role);
  }

  /** Letti o da leggere, più messaggi insieme. */
  async markRead(ids: number[], read: boolean) {
    const ms = this.msgsOf(ids).filter((m) => m.unread === read);
    if (!ms.length) return;
    for (const m of ms) {
      this.eachCopy(m.id, (x) => (x.unread = !read));
      this.bumpFolder(m.folderId, read ? -1 : 1);
    }
    await this.act(() => this.backend.setFlags(ms.map((m) => m.id), read, null));
  }

  /** Letto/Non letto: se ce n'è uno da leggere li segna tutti letti, altrimenti tutti da leggere. */
  async toggleReadSelected() {
    const ms = this.msgsOf(this.selection);
    if (ms.length) await this.markRead(ms.map((m) => m.id), ms.some((m) => m.unread));
  }

  async setFlag(ids: number[], flagged: boolean) {
    const ms = this.msgsOf(ids).filter((m) => m.flagged !== flagged);
    if (!ms.length) return;
    for (const m of ms) this.eachCopy(m.id, (x) => (x.flagged = flagged));
    await this.act(() => this.backend.setFlags(ms.map((m) => m.id), null, flagged));
  }

  /** Contrassegna: se ce n'è uno senza contrassegno li contrassegna tutti, altrimenti toglie. */
  async toggleFlagSelected() {
    const ms = this.msgsOf(this.selection);
    if (ms.length) await this.setFlag(ms.map((m) => m.id), ms.some((m) => !m.flagged));
  }

  async setCategoryOn(ids: number[], catId: string, on: boolean) {
    const ms = this.msgsOf(ids).filter((m) => m.categories.includes(catId) !== on);
    if (!ms.length) return;
    for (const m of ms) this.eachCopy(m.id, (x) => (x.categories = on ? [...x.categories, catId] : x.categories.filter((c) => c !== catId)));
    await this.act(() => this.backend.setCategory(ms.map((m) => m.id), catId, on));
  }

  async clearCategoriesOf(ids: number[]) {
    const ms = this.msgsOf(ids).filter((m) => m.categories.length);
    if (!ms.length) return;
    for (const m of ms) this.eachCopy(m.id, (x) => (x.categories = []));
    await this.act(() => this.backend.clearCategories(ms.map((m) => m.id)));
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
    const gone = new Set(ids);
    // Con più messaggi selezionati si riparte dal primo tolto, come Outlook.
    const i = this.multi.length ? l.findIndex((m) => gone.has(m.id)) : l.findIndex((m) => m.id === this.sel);
    for (const m of l) if (gone.has(m.id) && m.unread) this.bumpFolder(m.folderId, -1);
    this.list = l.filter((m) => !gone.has(m.id));
    for (const k of Object.keys(this.threadOpen)) this.threadOpen[k] = this.threadOpen[k].filter((m) => !gone.has(m.id));
    if (this.multi.length) {
      const left = this.multi.filter((id) => !gone.has(id));
      if (left.length !== this.multi.length) {
        this.multi = left.length > 1 ? left : [];
        const next = left.length === 1 ? this.findMsg(left[0]) : this.list.length ? this.list[Math.min(Math.max(i, 0), this.list.length - 1)] : null;
        if (left.length <= 1 && next && !next.draft) void this.pickMsg(next.id, left.length === 1 ? false : true);
      }
    }
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
    if (m && this.currentFolder?.role === role && !this.crossFolder) return;
    this.dropFromList([id]);
    await this.act(() => this.backend.moveToRole([id], role));
  }

  async moveToFolder(id: number | number[] | null, folderId: number) {
    const ids = id === null ? [] : Array.isArray(id) ? id : [id];
    if (!ids.length || folderId === this.folderId) return;
    this.dropFromList(ids);
    await this.act(() => this.backend.moveMessages(ids, folderId));
  }

  async deleteMsg(id: number | null) {
    if (id === null) return;
    this.dropFromList([id]);
    await this.act(() => this.backend.deleteMessages([id]));
  }

  async toggleFlag(id: number | null) {
    const m = id === null ? null : this.findMsg(id);
    if (!m) return;
    const flagged = !m.flagged;
    this.eachCopy(m.id, (x) => (x.flagged = flagged));
    await this.act(() => this.backend.setFlags([m.id], null, flagged));
  }

  async toggleRead(id: number | null) {
    const m = id === null ? null : this.findMsg(id);
    if (!m) return;
    await this.setRead(m.id, m.unread);
  }

  async setRead(id: number | null, read: boolean) {
    const m = id === null ? null : this.findMsg(id);
    if (!m || m.unread === !read) return;
    this.eachCopy(m.id, (x) => (x.unread = !read));
    this.bumpFolder(m.folderId, read ? -1 : 1);
    await this.act(() => this.backend.setFlags([m.id], read, null));
  }

  // ---------- Categorie ----------

  /** Assegna o toglie una categoria (Categorizza). */
  async toggleCategory(id: number | null, catId: string) {
    const m = id === null ? null : this.findMsg(id);
    if (!m) return;
    const on = !m.categories.includes(catId);
    this.eachCopy(m.id, (x) => (x.categories = on ? [...x.categories, catId] : x.categories.filter((c) => c !== catId)));
    await this.act(() => this.backend.setCategory([m.id], catId, on));
  }

  async clearMsgCategories(id: number | null) {
    const m = id === null ? null : this.findMsg(id);
    if (!m || !m.categories.length) return;
    this.eachCopy(m.id, (x) => (x.categories = []));
    await this.act(() => this.backend.clearCategories([m.id]));
  }

  /** Salva l'elenco delle categorie (nomi, colori, nuove, eliminate). */
  async saveCategories(list: Category[]): Promise<string> {
    try {
      this.categories = await this.backend.saveCategories(list);
      return '';
    } catch (e) {
      return errorMessage(e);
    }
  }

  // ---------- Conversazioni ----------

  toggleConversations() {
    this.conversations = !this.conversations;
    this.threadOpen = {};
    this.save();
  }

  private async loadThread(accountId: string, key: string) {
    try {
      const items = await this.backend.thread(accountId, key);
      if (key in this.threadOpen || !items.length) this.threadOpen[key] = items.reverse();
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Apre o chiude una conversazione nell'elenco. */
  async toggleThread(m: MessageSummary) {
    const key = m.thread;
    if (!key) return;
    if (this.threadOpen[key]) {
      delete this.threadOpen[key];
      return;
    }
    this.threadOpen[key] = [];
    await this.loadThread(m.accountId, key);
  }

  /** Ignora conversazione: i messaggi, anche quelli futuri, vanno in Posta eliminata. Di nuovo per smettere. */
  async ignoreConversation(id: number | null = this.sel) {
    if (id === null) return;
    let on: boolean;
    try {
      on = !(await this.backend.threadIgnored(id));
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
      return;
    }
    const yes = await this.ask(
      on
        ? { title: 'Ignora conversazione', text: 'Il messaggio selezionato e tutti quelli della stessa conversazione vanno in Posta eliminata. Anche le risposte che arriveranno in futuro finiranno lì.', ok: 'Ignora conversazione' }
        : { title: 'Smetti di ignorare', text: 'I messaggi di questa conversazione tornano in Posta in arrivo e le risposte future arriveranno di nuovo lì.', ok: 'Smetti di ignorare' },
    );
    if (!yes) return;
    try {
      const n = await this.backend.ignoreThread(id, on);
      this.threadIgnored = on;
      const moved = n === 1 ? '1 messaggio spostato' : `${n} messaggi spostati`;
      this.flash({ kind: 'info', text: on ? `Conversazione ignorata: ${moved} in Posta eliminata.` : `Conversazione ripristinata: ${moved} in Posta in arrivo.` });
      this.scheduleRefresh();
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Pulisci conversazione, cartella o cartella e sottocartelle: via i messaggi già riportati nelle risposte. */
  async cleanUp(scope: 'conversazione' | 'cartella' | 'sottocartelle') {
    const id = this.sel;
    const folder = this.currentFolder;
    if (scope === 'conversazione' ? id === null : !folder) return;
    const where = scope === 'conversazione' ? 'di questa conversazione' : scope === 'cartella' ? `della cartella ${folderLabel(folder)}` : `della cartella ${folderLabel(folder)} e delle sue sottocartelle`;
    const title = scope === 'conversazione' ? 'Pulisci conversazione' : scope === 'cartella' ? 'Pulisci cartella' : 'Pulisci cartella e sottocartelle';
    const yes = await this.ask({ title, text: `I messaggi ${where} già riportati per intero in una risposta successiva vanno in Posta eliminata. Restano quelli da leggere, contrassegnati o con allegati.`, ok: 'Pulisci' });
    if (!yes) return;
    try {
      const n = scope === 'conversazione' ? await this.backend.cleanUp(id, null, false) : await this.backend.cleanUp(null, folder!.id, scope === 'sottocartelle');
      this.flash({ kind: 'info', text: n === 0 ? 'Nessun messaggio da pulire: ogni messaggio contiene qualcosa che non si trova altrove.' : n === 1 ? '1 messaggio superfluo spostato in Posta eliminata.' : `${n} messaggi superflui spostati in Posta eliminata.` });
      if (n) this.scheduleRefresh();
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Conferme ----------

  /** Chiede conferma con una finestra; vero se l'utente conferma. */
  ask(c: Omit<Confirm, 'resolve' | 'alt'>): Promise<boolean> {
    this.confirm?.resolve(false);
    return new Promise((resolve) => {
      this.confirm = { ...c, resolve: (a) => resolve(a === true) };
    });
  }

  /** Domanda con due scelte più Annulla: 'ok', 'alt' oppure null. */
  choose(c: Omit<Confirm, 'resolve'> & { alt: string }): Promise<'ok' | 'alt' | null> {
    this.confirm?.resolve(false);
    return new Promise((resolve) => {
      this.confirm = { ...c, resolve: (a) => resolve(a === true ? 'ok' : a === 'alt' ? 'alt' : null) };
    });
  }

  answer(yes: boolean | 'alt') {
    const c = this.confirm;
    this.confirm = null;
    c?.resolve(yes);
  }

  // ---------- Cartelle ----------

  /** Inizia a scrivere il nome di una cartella nuova, dentro `parentId` o al primo livello. */
  startNewFolder(accountId: string, parentId: number | null) {
    this.collapsed[accountId] = false;
    this.folderEdit = { kind: 'new', accountId, parentId, folderId: null, value: '', error: '', busy: false };
  }

  startRenameFolder(folderId: number) {
    const f = this.folders.find((x) => x.id === folderId);
    if (!f || f.role) return;
    this.folderEdit = { kind: 'rename', accountId: f.accountId, parentId: null, folderId, value: f.name, error: '', busy: false };
  }

  cancelFolderEdit() {
    this.folderEdit = null;
  }

  async commitFolderEdit() {
    const e = this.folderEdit;
    if (!e || e.busy) return;
    const name = e.value.trim();
    if (!name) {
      this.folderEdit = null;
      return;
    }
    e.busy = true;
    try {
      if (e.kind === 'new') await this.backend.createFolder(e.accountId, e.parentId, name);
      else if (e.folderId !== null) await this.backend.renameFolder(e.folderId, name);
      this.folderEdit = null;
      await this.reloadFolders();
    } catch (err) {
      e.error = errorMessage(err);
      e.busy = false;
    }
  }

  async deleteFolder(folderId: number) {
    const f = this.folders.find((x) => x.id === folderId);
    if (!f || f.role) return;
    const inside = this.folders.filter((x) => x.accountId === f.accountId && (x.id === f.id || x.path.startsWith(`${f.path}/`) || x.path.startsWith(`${f.path}.`)));
    const total = inside.reduce((n, x) => n + x.total, 0);
    const what = total === 0 ? 'La cartella è vuota.' : total === 1 ? 'Il messaggio che contiene va in Posta eliminata.' : `I ${total.toLocaleString('it-IT')} messaggi che contiene vanno in Posta eliminata.`;
    const subs = inside.length > 1 ? ' Vengono eliminate anche le sue sottocartelle.' : '';
    const ok = await this.ask({ title: `Eliminare la cartella “${folderLabel(f)}”?`, text: `${what}${subs}`, ok: 'Elimina', danger: true });
    if (!ok) return;
    await this.act(async () => {
      await this.backend.deleteFolder(folderId);
      await this.reloadFolders();
    });
  }

  async markFolderRead(folderId: number) {
    for (const m of this.list) if (m.folderId === folderId) m.unread = false;
    const f = this.folders.find((x) => x.id === folderId);
    if (f) f.unread = 0;
    await this.act(() => this.backend.markFolderRead(folderId));
  }

  async emptyFolder(folderId: number) {
    const f = this.folders.find((x) => x.id === folderId);
    if (!f) return;
    const n = f.total;
    const ok = await this.ask({
      title: `Svuotare “${folderLabel(f)}”?`,
      text: n === 1 ? 'Il messaggio verrà eliminato definitivamente.' : `${n.toLocaleString('it-IT')} messaggi verranno eliminati definitivamente.`,
      ok: 'Svuota',
      danger: true,
    });
    if (!ok) return;
    await this.act(async () => {
      await this.backend.emptyFolder(folderId);
      if (this.folderId === folderId) {
        this.list = [];
        this.sel = null;
        this.detail = null;
      }
      await this.reloadFolders();
    });
  }

  /** Messaggi trascinati su una cartella. */
  async dropOnFolder(ids: number[], folderId: number) {
    const moving = ids.filter((id) => this.list.find((m) => m.id === id)?.folderId !== folderId);
    if (!moving.length) return;
    this.dropFromList(moving);
    await this.act(() => this.backend.moveMessages(moving, folderId));
  }

  setReadingPane(p: ReadingPanePos) {
    this.readingPane = p;
    this.fullRead = false;
    this.save();
  }

  setDensity(d: Density) {
    this.density = d;
    this.save();
  }

  /** Riquadro di lettura nascosto: apre il messaggio selezionato al posto dell'elenco. */
  openFull(id: number | null = this.sel) {
    if (id === null) return;
    if (this.sel !== id) void this.pickMsg(id);
    this.fullRead = true;
  }

  closeFull() {
    this.fullRead = false;
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

  composeAccount(prefer?: string): Account | null {
    const p = this.accounts.find((a) => a.id === prefer);
    if (this.canSend(p)) return p!;
    const cur = this.currentAccount;
    if (this.canSend(cur)) return cur;
    return this.accounts.find((a) => this.canSend(a)) ?? null;
  }

  private emptyDraft(accountId: string): Draft {
    const sig = this.signatureFor(accountId, 'new');
    const html = newMessageHtml(sig?.html);
    const d: Draft = { key: randomKey(), accountId, to: [], cc: [], bcc: [], subject: '', body: htmlToText(html), html, attachments: [], inReplyTo: null, references: [], replyToMessage: null };
    if (this.trackRead) d.readReceipt = true;
    if (this.trackDelivery) d.deliveryReceipt = true;
    return d;
  }

  /** Firma da aggiungere da sola per questo account, se l'utente l'ha scelta. */
  signatureFor(accountId: string, kind: 'new' | 'reply'): Signature | null {
    const a = this.accounts.find((x) => x.id === accountId);
    const id = kind === 'new' ? a?.signatureNew : a?.signatureReply;
    return (id && this.signatures.find((s) => s.id === id)) || null;
  }

  /** Vuoto: niente destinatari, oggetto o allegati, e nel testo solo la firma. */
  static isEmpty(d: Draft): boolean {
    if (d.to.length || d.cc.length || d.bcc.length || d.subject.trim() || d.attachments.length) return false;
    const text = d.html ? htmlToText(replaceSignature(d.html, null)) : d.body;
    return !text.trim();
  }

  private snapshot(): Draft | null {
    // L'editor manda il testo con un attimo di ritardo: prima si prende l'ultimo.
    if (this.c && !this.c.plain && composeEditor.attached) composeEditor.flush();
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
    this.askNoSubject = false;
    this.askPlain = false;
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

  flash(t: Omit<Toast, 'id'>, ms = 6000) {
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

  /** Bozza per Rispondi, Rispondi a tutti o Inoltra, dal messaggio aperto. */
  draftFrom(kind: Exclude<ComposeKind, 'new'>, m: MessageDetail, acct: Account, quote?: string): Draft {
    const d = this.emptyDraft(acct.id);
    const me: Person = { name: acct.name, email: acct.email };
    const clean = cleanSubject(m.subject);
    d.html = replyHtml(this.signatureFor(acct.id, 'reply')?.html, m, quote ?? textToHtml(m.text));
    d.body = htmlToText(d.html);
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
    let quote: string | undefined;
    if (m) quote = await this.backend.quoteHtml(m.id).catch(() => undefined);
    const d = m ? this.draftFrom(kind as Exclude<ComposeKind, 'new'>, m, acct, quote) : this.emptyDraft(acct.id);
    const inWindow = kind === 'new' ? P.compose === 'finestra' : P.compose === 'finestra' && P.replyWin;
    await this.stashDraft();
    await this.present(d, inWindow);
  }

  /** Bozza con i messaggi interi allegati come file .eml (con più messaggi l'oggetto resta da scrivere, come in Outlook). */
  private attachmentDraft(ms: MessageSummary[], acct: Account): Draft {
    const d = this.emptyDraft(acct.id);
    const name = (m: MessageSummary) => (cleanSubject(m.subject) || 'Messaggio').replace(/[\\/:*?"<>|]+/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 80) || 'Messaggio';
    if (ms.length === 1) d.subject = `I: ${cleanSubject(ms[0].subject) || 'Messaggio'}`;
    const used = new Map<string, number>();
    d.attachments = ms.map((m) => {
      const base = name(m);
      const n = (used.get(base) ?? 0) + 1;
      used.set(base, n);
      return { name: n > 1 ? `${base} (${n}).eml` : `${base}.eml`, size: m.size, mime: 'message/rfc822', path: null, source: null, message: m.id };
    });
    return d;
  }

  /** Inoltra come allegato: i messaggi interi viaggiano come file .eml (Ctrl+Alt+F, o Inoltra con più messaggi). */
  async forwardAsAttachment(ids: number | number[] | null = this.selection) {
    const ms = this.msgsOf(ids === null ? [] : Array.isArray(ids) ? ids : [ids]);
    if (!ms.length) return;
    const acct = this.composeAccount(ms[0].accountId);
    if (!acct) {
      this.openWizard(false);
      return;
    }
    const d = this.attachmentDraft(ms, acct);
    await this.stashDraft();
    await this.present(d, this.prefs.compose === 'finestra' && this.prefs.replyWin);
  }

  /** Scrivere a un indirizzo cliccato nel testo di un messaggio (mailto:). */
  async composeTo(email: string, name = '') {
    const acct = this.composeAccount();
    if (!acct) return;
    const d = this.emptyDraft(acct.id);
    d.to = [{ name, email }];
    // Il nuovo messaggio nel riquadro di lettura si vede solo nella posta.
    if (this.prefs.compose !== 'finestra') this.setModule('posta');
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
    this.askNoSubject = false;
    this.touch();
  }

  /** Il testo scritto nell'editor: formattato e semplice. */
  setContent(html: string, text: string) {
    if (!this.c) return;
    if (this.c.html === html && this.c.body === text) return;
    this.c.html = html;
    this.c.body = text;
    this.touch();
  }

  /** Testo semplice (senza editor): diventa anche la versione formattata. */
  setBody(v: string) {
    if (!this.c) return;
    this.c.body = v;
    this.c.html = this.c.plain ? null : textToHtml(v);
    this.touch();
  }

  /** Formato del messaggio, come in Formato testo › HTML / Testo normale. */
  setFormat(kind: 'html' | 'testo', confirmed = false) {
    const c = this.c;
    if (!c) return;
    if (kind === 'html') {
      this.askPlain = false;
      if (!c.plain) return;
      c.plain = false;
      c.html = textToHtml(c.body);
      this.touch();
      return;
    }
    if (c.plain) return;
    if (composeEditor.attached) composeEditor.flush();
    // Se c'è solo testo semplice non serve chiedere: non si perde nulla.
    const formatted = /<(b|strong|i|em|u|s|font|img|table|ul|ol|a|h\d)\b|style="[^"]*(color|background|font-weight|font-style|text-decoration)/i.test(c.html ?? '');
    if (formatted && !confirmed) {
      this.askPlain = true;
      return;
    }
    this.askPlain = false;
    c.body = plainFromHtml(c.html ?? textToHtml(c.body));
    c.html = null;
    c.plain = true;
    this.touch();
  }

  /** Priorità alta o bassa (Messaggio › Tag): un secondo clic torna a normale. */
  setImportance(v: Importance) {
    if (!this.c) return;
    this.c.importance = this.c.importance === v ? null : v;
    this.touch();
  }

  /** Opzioni › Rilevamento: conferma di recapito e di lettura per questo messaggio. */
  toggleReceipt(kind: 'lettura' | 'recapito') {
    if (!this.c) return;
    if (kind === 'lettura') this.c.readReceipt = !this.c.readReceipt;
    else this.c.deliveryReceipt = !this.c.deliveryReceipt;
    this.touch();
  }

  setTracking(patch: { receiptReply?: ReceiptReply; trackRead?: boolean; trackDelivery?: boolean }) {
    if (patch.receiptReply) this.receiptReply = patch.receiptReply;
    if (typeof patch.trackRead === 'boolean') this.trackRead = patch.trackRead;
    if (typeof patch.trackDelivery === 'boolean') this.trackDelivery = patch.trackDelivery;
    this.save();
  }

  /** Conferma di lettura chiesta dal mittente del messaggio aperto: Invia o Non inviare. */
  async answerReceipt(send: boolean) {
    const d = this.detail;
    if (!d?.receiptTo) return;
    const id = d.id;
    d.receiptTo = null;
    try {
      if (send) {
        if (await this.backend.sendReadReceipt(id, false)) this.flash({ kind: 'info', text: 'Conferma di lettura inviata.' }, 3000);
      } else {
        await this.backend.skipReadReceipt(id);
      }
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Con "Invia sempre" la conferma parte da sola appena il messaggio si apre. */
  private autoReceipt(d: MessageDetail) {
    if (!d.receiptTo || this.receiptReply !== 'sempre' || this.mode !== 'principale') return;
    d.receiptTo = null;
    void this.backend.sendReadReceipt(d.id, true).catch(() => {});
  }

  private composeKind(): 'new' | 'reply' {
    return hasQuote(this.c?.html) ? 'reply' : 'new';
  }

  /** Cambiando mittente cambia anche la firma aggiunta da Better-Look, come in Outlook. */
  setAccount(accountId: string) {
    if (!this.c) return;
    const kind = this.composeKind();
    const before = this.signatureFor(this.c.accountId, kind);
    const after = this.signatureFor(accountId, kind);
    this.c.accountId = accountId;
    if (before?.id !== after?.id && !this.c.plain) this.applySignature(after?.html ?? null, false);
    this.touch();
  }

  /** Mette la firma nel testo: nell'editor se c'è, altrimenti nell'HTML salvato. */
  private applySignature(html: string | null, atCursor: boolean) {
    if (!this.c) return;
    if (composeEditor.attached) {
      if (atCursor && html) composeEditor.insertSignature(html);
      else composeEditor.setSignature(html);
      return;
    }
    this.c.html = replaceSignature(this.c.html ?? textToHtml(this.c.body), html);
    this.c.body = htmlToText(this.c.html);
    this.touch();
  }

  /** Pulsante Firma: inserisce la firma scelta (o sostituisce quella già presente). */
  insertSignature(id: string) {
    const s = this.signatures.find((x) => x.id === id);
    if (!s || !this.c) return;
    if (this.c.plain) {
      this.c.body = addPlainSignature(this.c.body, s.html);
      this.touch();
      return;
    }
    this.applySignature(s.html, true);
  }

  openLinkDialog(anchor: HTMLElement | null) {
    this.linkAnchor = anchor;
    this.linkOpen = true;
  }

  closeLinkDialog() {
    this.linkOpen = false;
    this.linkAnchor = null;
  }

  /** Avviso breve in basso (per esempio come usare un comando). */
  hint(text: string) {
    this.flash({ kind: 'info', text }, 5000);
  }

  // ---------- Firme e carattere ----------

  async saveSignature(s: Signature): Promise<Signature | null> {
    try {
      const first = !this.signatures.length;
      const saved = await this.backend.saveSignature(s);
      await this.reloadSignatures();
      // La prima firma diventa subito quella di tutti gli account che ancora
      // non ne hanno una: basta crearla perché compaia nei messaggi.
      if (first) {
        for (const a of this.accounts) {
          if (!this.canSend(a) || (a.signatureNew && a.signatureReply)) continue;
          a.signatureNew ??= saved.id;
          a.signatureReply ??= saved.id;
          await this.backend.updateAccount($state.snapshot(a) as Account);
        }
      }
      return saved;
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) });
      return null;
    }
  }

  async deleteSignature(id: string) {
    await this.act(async () => {
      await this.backend.deleteSignature(id);
      await this.reloadSignatures();
      await this.reloadAccounts();
    });
  }

  async setAccountSignature(accountId: string, kind: 'new' | 'reply', id: string | null) {
    const a = this.accounts.find((x) => x.id === accountId);
    if (!a) return;
    if (kind === 'new') a.signatureNew = id;
    else a.signatureReply = id;
    await this.act(() => this.backend.updateAccount($state.snapshot(a) as Account));
  }

  setFont(font: { family: string; size: number }) {
    this.font = { ...font };
    this.save();
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

  /** Ora scelta con Pianifica invio (millisecondi), in attesa di partire. */
  private sendAt: number | null = null;

  /**
   * Invia il messaggio aperto, subito o all'ora `at` (Invio programmato).
   * `confirmed` arriva dalla domanda sull'oggetto vuoto e tiene l'ora già scelta.
   */
  async send(confirmed = false, at: number | null = null): Promise<boolean> {
    if (!this.c || this.sending) return false;
    if (!confirmed) this.sendAt = at;
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
    if (!d.subject.trim() && !confirmed) {
      this.askNoSubject = true;
      return false;
    }
    this.askNoSubject = false;
    clearTimeout(this.saveTimer);
    this.sending = true;
    try {
      const win = this.cWin;
      if (this.sendAt !== null) await this.backend.sendLater(d, this.sendAt);
      else await this.backend.send(d);
      this.sendAt = null;
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

  // ---------- Stampa e lettura ad alta voce ----------

  /** Stampa il messaggio aperto in una finestra pronta per la stampa (Ctrl+P). */
  async printMsg(id: number | null = this.sel) {
    if (id === null) return;
    if (this.sel !== id) await this.pickMsg(id, false);
    await this.act(async () => {
      if (!(await this.backend.openPrintWindow(id, !!this.remoteOk[id]))) this.flash({ kind: 'error', text: 'Non riesco ad aprire la finestra di stampa.' });
    });
  }

  /** Testo da leggere: oggetto, mittente e testo del messaggio aperto. */
  private speechText(): string {
    const m = this.detail;
    if (!m) return '';
    const who = m.from.name || m.from.email;
    return `${m.subject || 'Messaggio senza oggetto'}. Da ${who}.

${m.text}`.slice(0, 20_000);
  }

  /** Voci della pagina (quelle del sistema, se il motore web le offre). */
  private async webVoices(): Promise<SpeechSynthesisVoice[]> {
    const synth = globalThis.speechSynthesis;
    if (!synth) return [];
    let voices = synth.getVoices();
    if (voices.length) return voices;
    await new Promise<void>((resolve) => {
      const t = setTimeout(resolve, 600);
      synth.addEventListener?.('voiceschanged', () => {
        clearTimeout(t);
        resolve();
      }, { once: true });
    });
    voices = synth.getVoices();
    return voices;
  }

  /** Leggi ad alta voce: avvia o interrompe la lettura del messaggio aperto. */
  async toggleSpeech() {
    if (this.speech !== 'off') {
      this.stopSpeech();
      return;
    }
    const text = this.speechText();
    if (!text.trim()) return;
    const voices = await this.webVoices();
    if (voices.length) {
      const synth = globalThis.speechSynthesis;
      const voice = voices.find((v) => v.lang?.toLowerCase().startsWith('it')) ?? null;
      synth.cancel();
      // Frasi brevi: alcune voci si fermano da sole sui testi lunghi.
      const parts = text.match(/[^.!?\n]+[.!?]*|\n+/g)?.map((p) => p.trim()).filter(Boolean) ?? [text];
      parts.forEach((p, i) => {
        const u = new SpeechSynthesisUtterance(p);
        u.lang = voice?.lang ?? 'it-IT';
        if (voice) u.voice = voice;
        if (i === parts.length - 1) u.onend = () => (this.speech = 'off');
        u.onerror = () => (this.speech = 'off');
        synth.speak(u);
      });
      this.speechNative = false;
      this.speech = 'playing';
      return;
    }
    try {
      await this.backend.speak(text);
      this.speechNative = true;
      this.speech = 'playing';
      clearInterval(this.speechPoll);
      this.speechPoll = setInterval(async () => {
        if (!(await this.backend.speaking().catch(() => false))) {
          clearInterval(this.speechPoll);
          this.speech = 'off';
        }
      }, 1000);
    } catch (e) {
      this.flash({ kind: 'error', text: errorMessage(e) }, 9000);
    }
  }

  /** Pausa e ripresa (solo con le voci della pagina). */
  pauseSpeech() {
    const synth = globalThis.speechSynthesis;
    if (this.speechNative || !synth) return;
    if (this.speech === 'playing') {
      synth.pause();
      this.speech = 'paused';
    } else if (this.speech === 'paused') {
      synth.resume();
      this.speech = 'playing';
    }
  }

  get canPauseSpeech(): boolean {
    return !this.speechNative;
  }

  stopSpeech() {
    clearInterval(this.speechPoll);
    if (this.speechNative) void this.backend.stopSpeaking().catch(() => {});
    else globalThis.speechSynthesis?.cancel();
    this.speech = 'off';
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
      try {
        const a = await this.backend.importArchive(kind);
        if (!a) return;
        await this.reloadAccounts();
        await this.reloadFolders();
        this.backstage = false;
        const first = this.foldersOf(a.id).find((f) => f.role === 'inbox' && f.total > 0) ?? this.foldersOf(a.id).find((f) => f.total > 0) ?? this.foldersOf(a.id)[0];
        if (first) await this.pickFolder(first.id, `acc-${first.id}`);
        const lost = [
          [a.skipped ?? 0, 'messaggio', 'messaggi'],
          [a.lostAttachments ?? 0, 'allegato', 'allegati'],
        ]
          .filter(([n]) => (n as number) > 0)
          .map(([n, one, many]) => (n === 1 ? `un ${one}` : `${(n as number).toLocaleString('it-IT')} ${many}`));
        const total = (a.skipped ?? 0) + (a.lostAttachments ?? 0);
        const note = lost.length === 0 ? '' : ` Non si ${total === 1 ? 'è potuto' : 'sono potuti'} leggere ${lost.join(' e ')}.`;
        this.flash({ kind: 'info', text: `File di dati aperto: ${a.name}.${note}` }, lost.length ? 12000 : 6000);
      } finally {
        this.importing = null;
      }
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

  /** Colore di accento: null (o il blu predefinito) torna ai colori di Outlook. */
  setAccent(hex: string | null) {
    const v = hex ? normalizeHex(hex) : null;
    if (hex && !v) return;
    this.accent = v && v !== DEFAULT_ACCENT ? v : null;
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

  openSettings(page: SettingsPage = 'aspetto') {
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
  async applyDemo(opts: { stile?: StyleName; view?: string; tema?: ThemeChoice; accento?: string; avvio?: boolean; offline?: boolean }) {
    if (opts.offline !== undefined) this.netOffline = opts.offline;
    if (opts.accento) this.accent = normalizeHex(opts.accento);
    if (opts.stile) this.prefs = { ...PRESETS[opts.stile] };
    if (opts.tema) this.themeChoice = opts.tema;
    if (opts.avvio !== undefined) {
      this.onboarding = opts.avvio;
      this.obChoice = opts.stile ?? 'classico';
    }
    const v = opts.view;
    const acct = this.composeAccount();
    if ((v === 'composizione' || v === 'composizione-opzioni' || v === 'inviato') && acct) {
      const win = this.prefs.compose === 'finestra';
      const d = this.emptyDraft(acct.id);
      d.to = [{ name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' }];
      d.cc = [{ name: 'Marco Ventura', email: 'm.ventura@valliarredi.it' }];
      if (win) {
        d.subject = 'Campioni del rovere per Bassano';
        d.html = textToHtml('Ciao Davide,\n\nil cliente vuole vedere i campioni del rovere prima di firmare il preventivo. Riesci a portarli giovedì in sede? Bastano il naturale e lo spazzolato.\n\nA presto,\nChiara') + replaceSignature('', this.signatureFor(acct.id, 'new')?.html ?? null);
      } else if (this.detail) {
        Object.assign(d, this.draftFrom('reply', this.detail, acct, await this.backend.quoteHtml(this.detail.id)));
        d.html = textToHtml('Ciao Davide,\n\ngrazie, la revisione va bene. Il cliente vuole però vedere i campioni del rovere prima di firmare: riesci a portarli giovedì in sede?\n\nA presto,\nChiara') + d.html;
      }
      d.body = htmlToText(d.html ?? '');
      if (v === 'composizione-opzioni') {
        d.importance = 'high';
        d.readReceipt = true;
      }
      if (v !== 'inviato') {
        this.startCompose(d, win);
        this.savedAt = '09:04';
        if (v === 'composizione-opzioni') {
          this.winTab = 'opzioni';
          if (!win) this.ribbonTab = 'opzioni';
        }
      } else {
        this.lastSent = { key: d.key, win };
        await this.backend.send(d);
      }
    } else if (v === 'file') {
      this.backstage = true;
      this.bsPage = 'info';
    } else if (v === 'impostazioni') {
      this.openSettings('aspetto');
    } else if (v?.startsWith('impostazioni-')) {
      this.openSettings(v.slice('impostazioni-'.length) as SettingsPage);
    } else if (v === 'account') {
      this.openSettings('account');
    } else if (v === 'informazioni') {
      this.openSettings('info');
    } else if (v === 'contatti') {
      this.setModule('contatti');
      await this.reloadContacts();
      this.contactSel = this.book[0]?.id ?? null;
    } else if (v === 'calendario' || v === 'calendario-mese' || v === 'calendario-giorno') {
      this.cal.view = v === 'calendario-mese' ? 'mese' : v === 'calendario-giorno' ? 'giorno' : 'settimana-lavorativa';
      this.setModule('calendario');
      await this.cal.reload();
    } else if (v === 'appuntamento' || v === 'riunione') {
      this.setModule('calendario');
      await this.cal.reload();
      const o = this.cal.occs.find((x) => x.ev.id === 'ev3');
      if (v === 'riunione' && o) this.cal.edit = { ev: JSON.parse(JSON.stringify(o.ev)), meeting: true, series: false, original: o.ev, occ: o };
      else this.cal.newEvent({ start: this.cal.range.days[1] + 11 * 3_600_000 });
    } else if (v === 'cartelle-ricerca' || v === 'cartella-ricerca-nuova' || v === 'cartella-ricerca-personalizzata') {
      const base = { id: '', accountId: 'valli', total: 0, unread: 0 };
      await this.backend.saveSearchFolder({ ...base, name: 'Posta da leggere', kind: 'da-leggere', criteria: { ...emptyCriteria(), unread: true } });
      await this.backend.saveSearchFolder({ ...base, name: 'Posta contrassegnata', kind: 'contrassegnata', criteria: { ...emptyCriteria(), flagged: true } });
      const p = await this.backend.saveSearchFolder({ ...base, name: 'Studio Orlandi', kind: 'persone', criteria: { ...emptyCriteria(), people: ['Davide Orlandi', 'Marco Ventura'] } });
      await this.reloadSearchFolders();
      await this.pickSearchFolder(p.id, `sf-${p.id}`);
      if (v === 'cartella-ricerca-nuova') {
        this.openSearchFolderDialog({ accountId: 'valli' });
        this.pickSearchFolderKind('persone');
        this.setSearchFolderPeople('Lucia Marchetti; tessuti');
      } else if (v === 'cartella-ricerca-personalizzata') {
        this.openSearchFolderDialog({ accountId: 'valli' });
        this.pickSearchFolderKind('personalizzata');
        Object.assign(this.sfEdit!.criteria, { unread: true, attachments: true, days: 30 });
        this.sfEdit!.name = 'Allegati da leggere';
        this.sfEdit!.nameTouched = true;
      }
    } else if (v === 'priorita') {
      if (this.list.some((m) => m.id === 4)) await this.pickMsg(4);
    } else if (v === 'invito') {
      if (this.list.some((m) => m.id === 14)) await this.pickMsg(14);
    } else if (v === 'promemoria') {
      this.setModule('calendario');
      await this.cal.reload();
      const o = this.cal.occs.find((x) => x.ev.id === 'ev5');
      if (o) this.cal.reminders = [{ key: o.key, occ: o }];
    } else if (v === 'attivita' || v === 'attivita-dettaglio' || v === 'attivita-pianificate' || v === 'attivita-contrassegnati' || v === 'attivita-elenco') {
      this.tasks.view = v === 'attivita-pianificate' ? 'pianificate' : v === 'attivita-contrassegnati' ? 'contrassegnati' : v === 'attivita-elenco' ? 'elenco:l-bassano' : 'giornata';
      this.setModule('attivita');
      await this.tasks.reload();
      if (v === 'attivita-dettaglio') this.tasks.sel = 't1';
      if (v === 'attivita-elenco') this.tasks.doneOpen = true;
    } else if (v === 'promemoria-attivita') {
      await this.tasks.reload();
      const t = this.tasks.items.find((x) => x.id === 't1');
      if (t) this.tasks.reminders = [t];
    } else if (v === 'azioni-rapide') {
      this.openSettings('azioni');
    } else if (v === 'azione-nuova') {
      this.quickDraft = { id: '', name: 'Al commercialista', icon: 'briefcase', shortcut: 3, actions: [{ kind: 'categoria', category: 'c3' }, { kind: 'inoltra', to: [{ name: '', email: 'studio@commercialista.example' }], text: 'Da registrare, grazie.' }, { kind: 'archivia' }] };
      this.openSettings('azioni');
    } else if (v === 'modelli') {
      this.openSettings('modelli');
    } else if (v === 'selezione') {
      const ids = this.list.slice(1, 4).map((m) => m.id);
      if (ids.length) {
        await this.clickMsg(ids[0]);
        await this.clickMsg(ids[ids.length - 1], { shift: true });
      }
    } else if (v === 'lettura-basso' || v === 'lettura-nascosto') {
      this.readingPane = v === 'lettura-basso' ? 'basso' : 'nascosto';
    } else if (v === 'lettura-piena') {
      this.readingPane = 'nascosto';
      this.fullRead = true;
    } else if (v === 'compatta' || v === 'ampia') {
      this.density = v;
    } else if (v === 'ignora') {
      void this.ignoreConversation();
    } else if (v === 'inoltra-allegato') {
      await this.forwardAsAttachment();
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
