// Stato dell'interfaccia e azioni dell'utente.
// È la traduzione in Svelte 5 della logica del prototipo (Main.dc.html).
// Le preferenze di aspetto e il completamento del primo avvio restano salvati
// sul computer; i messaggi per ora sono quelli di esempio di mock.ts.

import { seedMessages, demoDraft, ME, type Draft, type FolderKey, type Message } from './mock';
import { clock, cleanSubject } from './format';

export type RibbonMode = 'estesa' | 'compatta';
export type EntryMode = 'file' | 'ingranaggio';
export type ComposeMode = 'finestra' | 'pannello';
export type ThemeChoice = 'sistema' | 'chiaro' | 'scuro';
export type StyleName = 'classico' | 'nuovo';
export type RibbonTab = 'home' | 'visualizza' | 'aiuto' | 'messaggio' | 'inserisci';
export type WinTab = 'messaggio' | 'inserisci';
export type ComposeKind = 'new' | 'reply' | 'replyAll' | 'forward';

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

export const FOLDER_NAMES: Record<FolderKey, string> = {
  inbox: 'Posta in arrivo',
  sent: 'Posta inviata',
  drafts: 'Bozze',
  archive: 'Archivio',
  junk: 'Posta indesiderata',
  deleted: 'Elementi eliminati',
  clienti: 'Clienti',
  fornitori: 'Fornitori',
  amministrazione: 'Amministrazione',
  ordini: 'Posta in arrivo',
};

export const GROUPS = ['Oggi', 'Ieri', 'Questa settimana'] as const;

interface Toast {
  kind: 'sent' | 'draft';
  id: number;
}
interface LastSent {
  msg: Message;
  c: Draft;
  win: boolean;
}
interface Saved {
  prefs: Prefs;
  onboardingDone: boolean;
  themeChoice: ThemeChoice;
  showFolders: boolean;
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

export class MailStore {
  private base: Message[] = seedMessages();

  // Navigazione
  folder = $state<FolderKey>('inbox');
  folderSel = $state('fav-inbox');
  tab = $state<'prio' | 'altre'>('prio');
  sel = $state<string | null>('m1');
  q = $state('');

  // Modifiche locali ai messaggi di esempio
  rd = $state<Record<string, boolean>>({ m1: true });
  fl = $state<Record<string, boolean>>({});
  mv = $state<Record<string, FolderKey | 'gone'>>({});
  sent = $state<Message[]>([]);
  drafts = $state<Message[]>([]);

  // Aspetto
  prefs = $state<Prefs>({ ...PRESETS.classico });
  themeChoice = $state<ThemeChoice>('sistema');
  systemDark = $state(false);
  showFolders = $state(true);
  ribbonTab = $state<RibbonTab>('home');
  winTab = $state<WinTab>('messaggio');

  // Pannelli e finestre
  onboarding = $state(true);
  obChoice = $state<StyleName>('classico');
  backstage = $state(false);
  bsPage = $state<'info' | 'apri' | 'about'>('info');
  settings = $state(false);
  setPage = $state<'aspetto' | 'account' | 'info'>('aspetto');
  acctSel = $state<'imap' | 'pop'>('imap');
  acct2Open = $state(false);
  sync = $state({ imapWindow: '3m', attach: 'apertura', popKeep: '14' });
  offline = $state(false);

  // Scrittura
  c = $state<Draft | null>(null);
  cWin = $state(false);
  winMax = $state(false);
  winMin = $state(false);
  toInput = $state('');
  ccInput = $state('');
  err = $state('');
  toast = $state<Toast | null>(null);
  lastSent = $state<LastSent | null>(null);
  private toastTimer: ReturnType<typeof setTimeout> | undefined;
  private persist: boolean;

  constructor(opts: { persist?: boolean } = {}) {
    this.persist = opts.persist ?? true;
    if (this.persist) {
      const saved = readSaved();
      if (saved) {
        if (saved.prefs) this.prefs = { ...PRESETS.classico, ...saved.prefs };
        if (saved.onboardingDone) this.onboarding = false;
        if (saved.themeChoice) this.themeChoice = saved.themeChoice;
        if (typeof saved.showFolders === 'boolean') this.showFolders = saved.showFolders;
      }
    }
    if (typeof navigator !== 'undefined' && typeof window !== 'undefined') {
      this.offline = navigator.onLine === false;
      window.addEventListener('online', () => (this.offline = false));
      window.addEventListener('offline', () => (this.offline = true));
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
    };
    try {
      globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(data));
    } catch {
      /* archivio non disponibile: le preferenze valgono solo per questa sessione */
    }
  }

  // ---------- Valori derivati ----------

  get theme(): 'chiaro' | 'scuro' {
    if (this.themeChoice === 'sistema') return this.systemDark ? 'scuro' : 'chiaro';
    return this.themeChoice;
  }

  private eff(m: Message): Message {
    const o = { ...m };
    if (this.rd[m.id] !== undefined) o.unread = !this.rd[m.id];
    if (this.fl[m.id] !== undefined) o.flagged = this.fl[m.id];
    const moved = this.mv[m.id];
    if (moved) o.folder = moved as FolderKey;
    return o;
  }

  all = $derived.by(() => [...this.sent, ...this.drafts, ...this.base].map((m) => this.eff(m)));

  filterList(folder: FolderKey, tab: 'prio' | 'altre', q: string): Message[] {
    const qq = q.trim().toLowerCase();
    return this.all.filter((m) => {
      if (m.folder !== folder) return false;
      if (qq) return `${m.from} ${m.subject} ${m.preview}`.toLowerCase().includes(qq);
      if (folder === 'inbox' && m.tab !== tab) return false;
      return true;
    });
  }

  list = $derived.by(() => this.filterList(this.folder, this.tab, this.q));

  selected = $derived.by(() => {
    if (!this.sel) return null;
    return this.all.find((m) => m.id === this.sel && m.folder === this.folder) ?? null;
  });

  find(id: string): Message | null {
    return this.all.find((m) => m.id === id) ?? null;
  }

  count(folder: FolderKey, unreadOnly: boolean): number {
    return this.all.filter((m) => m.folder === folder && (!unreadOnly || m.unread)).length;
  }

  get paneCompose(): boolean {
    return !!this.c && !this.cWin;
  }
  get winOpen(): boolean {
    return !!this.c && this.cWin && !this.winMin;
  }
  get winMinimized(): boolean {
    return !!this.c && this.cWin && this.winMin;
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
    return this.q.trim() ? 'Risultati della ricerca' : FOLDER_NAMES[this.folder];
  }

  // ---------- Navigazione ----------

  private selectFirst(list: Message[], skipDrafts = false) {
    const first = list.length && !skipDrafts ? list[0].id : null;
    this.sel = first;
    if (first) this.rd[first] = true;
  }

  pickFolder(key: FolderKey, id: string) {
    this.stashPaneDraft();
    this.folder = key;
    this.folderSel = id;
    this.tab = 'prio';
    this.q = '';
    this.err = '';
    this.selectFirst(this.filterList(key, 'prio', ''), key === 'drafts');
  }

  pickTab(tab: 'prio' | 'altre') {
    this.stashPaneDraft();
    this.tab = tab;
    this.err = '';
    this.selectFirst(this.filterList(this.folder, tab, ''));
  }

  search(value: string) {
    this.q = value;
    const l = this.filterList(this.folder, this.tab, value);
    this.sel = l.length ? l[0].id : null;
  }

  clearSearch() {
    this.q = '';
    const l = this.filterList(this.folder, this.tab, '');
    if (!l.some((m) => m.id === this.sel)) this.sel = l.length ? l[0].id : null;
  }

  pickMsg(id: string) {
    const m = this.find(id);
    if (!m) return;
    if (m.draft) {
      this.reopenDraft(m);
      return;
    }
    this.stashPaneDraft();
    this.sel = id;
    this.rd[id] = true;
    this.err = '';
  }

  /** Sposta la selezione al messaggio precedente o successivo dell'elenco (frecce). */
  moveSelection(step: 1 | -1) {
    const l = this.list;
    if (!l.length) return;
    const i = l.findIndex((m) => m.id === this.sel);
    const next = l[Math.min(Math.max(i + step, 0), l.length - 1)];
    if (next && !next.draft) this.pickMsg(next.id);
  }

  moveMsg(id: string | null, dest: FolderKey) {
    if (!id) return;
    const l = this.list;
    const i = l.findIndex((m) => m.id === id);
    this.mv[id] = dest;
    if (this.sel === id) {
      const rest = l.filter((m) => m.id !== id);
      const next = rest.length ? rest[Math.min(Math.max(i, 0), rest.length - 1)] : null;
      this.sel = next ? next.id : null;
      if (next) this.rd[next.id] = true;
    }
  }

  toggleFlag(id: string | null) {
    const m = id ? this.find(id) : null;
    if (m) this.fl[m.id] = !m.flagged;
  }

  toggleRead(id: string | null) {
    const m = id ? this.find(id) : null;
    if (m) this.rd[m.id] = m.unread;
  }

  setRead(id: string | null, read: boolean) {
    if (id && this.find(id)) this.rd[id] = read;
  }

  toggleFolders() {
    this.showFolders = !this.showFolders;
    this.save();
  }

  // ---------- Scrittura ----------

  private msgFromDraft(c: Draft, folder: 'sent' | 'drafts'): Message {
    const k = clock();
    const first = c.to[0] || 'Nessun destinatario';
    const paras = String(c.body || '')
      .split(/\n\s*\n/)
      .map((t) => t.trim())
      .filter(Boolean);
    return {
      id: `${folder[0]}${k.stamp}-${++toastSeq}`,
      folder,
      tab: 'prio',
      group: 'Oggi',
      from: first,
      email: '',
      time: k.hhmm,
      dateLong: k.long,
      hhmm: k.hhmm,
      subject: c.subj || '(nessun oggetto)',
      preview: (paras[0] || '').replace(/\n/g, ' ').slice(0, 120),
      body: paras,
      sig: '',
      atts: c.atts.slice(),
      unread: false,
      flagged: false,
      to: c.to.join('; '),
      cc: c.cc.join('; '),
      cat: '',
      sent: folder === 'sent',
      draft: folder === 'drafts',
    };
  }

  private closeCompose() {
    this.c = null;
    this.cWin = false;
    this.winMin = false;
    this.winMax = false;
  }

  private isEmptyDraft(c: Draft) {
    return !c.to.length && !c.subj && !String(c.body).trim();
  }

  /** Chiude il messaggio in scrittura salvandolo in Bozze, se contiene qualcosa. */
  private stashDraft() {
    const c = this.c;
    if (!c) return;
    this.closeCompose();
    if (this.isEmptyDraft(c)) return;
    this.drafts = [this.msgFromDraft(c, 'drafts'), ...this.drafts];
  }

  private stashPaneDraft() {
    if (this.c && !this.cWin) this.stashDraft();
  }

  private flash(kind: Toast['kind'], ms = 6000) {
    clearTimeout(this.toastTimer);
    const id = ++toastSeq;
    this.toast = { kind, id };
    this.toastTimer = setTimeout(() => {
      if (this.toast?.id === id) this.toast = null;
    }, ms);
  }

  private startCompose(c: Draft, inWindow: boolean) {
    this.c = c;
    this.cWin = inWindow;
    this.winMin = false;
    this.winMax = false;
    this.winTab = 'messaggio';
    this.toInput = '';
    this.ccInput = '';
    this.err = '';
    if (!inWindow) this.ribbonTab = 'messaggio';
  }

  private reopenDraft(m: Message) {
    this.stashDraft();
    this.mv[m.id] = 'gone';
    this.drafts = this.drafts.filter((d) => d.id !== m.id);
    this.sel = null;
    this.startCompose(
      {
        to: m.to ? m.to.split('; ').filter(Boolean) : [],
        cc: m.cc ? m.cc.split('; ').filter(Boolean) : [],
        subj: m.subject === '(nessun oggetto)' ? '' : m.subject,
        body: m.body.join('\n\n'),
        atts: m.atts.slice(),
        saved: m.hhmm || '',
      },
      this.prefs.compose === 'finestra',
    );
  }

  openCompose(kind: ComposeKind) {
    const P = this.prefs;
    let m: Message | null = null;
    if (kind !== 'new') {
      m = this.selected;
      if (!m) return;
    }
    const c: Draft = { to: [], cc: [], subj: '', body: '', atts: [], saved: '' };
    if (m) {
      const clean = cleanSubject(m.subject);
      c.body =
        `\n\n\nDa: ${m.from}${m.email ? ` <${m.email}>` : ''}\nInviato: ${m.dateLong}\nOggetto: ${m.subject}\n\n` +
        m.body.join('\n\n');
      if (kind === 'reply' || kind === 'replyAll') {
        c.to = [m.from];
        c.subj = `Re: ${clean}`;
      }
      if (kind === 'replyAll') {
        c.cc = `${m.to}; ${m.cc}`
          .split('; ')
          .map((n) => n.trim())
          .filter((n) => n && n !== ME.name && n !== m!.from);
      }
      if (kind === 'forward') {
        c.subj = `I: ${clean}`;
        c.atts = m.atts.slice();
      }
    }
    const inWindow = kind === 'new' ? P.compose === 'finestra' : P.compose === 'finestra' && P.replyWin;
    this.stashDraft();
    this.startCompose(c, inWindow);
  }

  setSubject(v: string) {
    if (this.c) {
      this.c.subj = v;
      this.c.saved = clock().hhmm;
    }
  }

  setBody(v: string) {
    if (this.c) {
      this.c.body = v;
      this.c.saved = clock().hhmm;
    }
  }

  addRecipient(key: 'to' | 'cc') {
    const input = key === 'to' ? this.toInput : this.ccInput;
    const v = String(input || '').replace(/[,;]+$/, '').trim();
    if (!v || !this.c) return;
    this.c[key] = [...this.c[key], v];
    if (key === 'to') this.toInput = '';
    else this.ccInput = '';
    this.err = '';
  }

  removeRecipient(key: 'to' | 'cc', index: number) {
    if (!this.c) return;
    this.c[key] = this.c[key].filter((_, j) => j !== index);
  }

  recipientInput(key: 'to' | 'cc', value: string) {
    if (key === 'to') {
      this.toInput = value;
      this.err = '';
    } else {
      this.ccInput = value;
    }
    if (/[,;]$/.test(value)) this.addRecipient(key);
  }

  recipientKey(key: 'to' | 'cc', e: KeyboardEvent) {
    const input = key === 'to' ? this.toInput : this.ccInput;
    if (e.key === 'Enter' || e.key === ',' || e.key === ';') {
      e.preventDefault();
      this.addRecipient(key);
      return;
    }
    if (e.key === 'Backspace' && !input && this.c && this.c[key].length) {
      this.c[key] = this.c[key].slice(0, -1);
    }
  }

  send() {
    if (!this.c) return;
    const to = this.c.to.slice();
    const pending = String(this.toInput || '').replace(/[,;]+$/, '').trim();
    if (pending) to.push(pending);
    if (!to.length) {
      this.err = 'Aggiungi almeno un destinatario.';
      return;
    }
    const c: Draft = { ...this.c, to, cc: this.c.cc.slice(), atts: this.c.atts.slice() };
    const msg = this.msgFromDraft(c, 'sent');
    const win = this.cWin;
    this.sent = [msg, ...this.sent];
    this.closeCompose();
    this.toInput = '';
    this.err = '';
    this.lastSent = { msg, c, win };
    this.flash('sent', 6000);
  }

  undoSend() {
    const ls = this.lastSent;
    if (!ls) return;
    this.sent = this.sent.filter((m) => m.id !== ls.msg.id);
    this.lastSent = null;
    this.toast = null;
    this.startCompose(ls.c, ls.win);
  }

  closeWin() {
    const c = this.c;
    if (!c) return;
    const had = !this.isEmptyDraft(c);
    this.stashDraft();
    if (had) this.flash('draft', 5000);
    else this.toast = null;
  }

  discard() {
    this.closeCompose();
    this.err = '';
  }

  popOut() {
    this.cWin = true;
    this.winMin = false;
    this.winMax = false;
    this.winTab = 'messaggio';
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
   * Porta l'interfaccia in uno degli stati mostrati nel prototipo.
   * Serve per le schermate di prova e per i test; non salva nulla.
   */
  applyDemo(opts: { stile?: StyleName; view?: string; tema?: ThemeChoice; avvio?: boolean; offline?: boolean }) {
    if (opts.offline !== undefined) this.offline = opts.offline;
    if (opts.stile) this.prefs = { ...PRESETS[opts.stile] };
    if (opts.tema) this.themeChoice = opts.tema;
    if (opts.avvio !== undefined) {
      this.onboarding = opts.avvio;
      this.obChoice = opts.stile ?? 'classico';
    }
    const v = opts.view;
    if (v === 'composizione') {
      const win = this.prefs.compose === 'finestra';
      this.startCompose(demoDraft(win), win);
    } else if (v === 'inviato') {
      const win = this.prefs.compose === 'finestra';
      const c = demoDraft(win);
      const msg = this.msgFromDraft(c, 'sent');
      this.sent = [msg];
      this.lastSent = { msg, c, win };
      this.toast = { kind: 'sent', id: ++toastSeq };
    } else if (v === 'file') {
      this.backstage = true;
      this.bsPage = 'info';
    } else if (v === 'impostazioni') {
      this.openSettings('aspetto');
    } else if (v === 'account') {
      this.openSettings('account');
    } else if (v === 'informazioni') {
      this.openSettings('info');
    }
  }
}

export const store = new MailStore();
