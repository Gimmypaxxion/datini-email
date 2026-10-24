// Motore di esempio, tutto in memoria. Si comporta come quello vero (stessi
// comandi, stessi eventi) ma senza rete: serve nel browser e nei test.

import type { Backend, Tab } from './backend';
import { mockData, mockEvents, type MockMessage } from './mock';
import { addDays, isMine } from './calendar';
import { textToHtml } from './compose-html';
import { hasActions, ruleMatches } from './rules';
import { contactName } from './contacts';
import type {
  Account,
  AutoReply,
  CalEvent,
  CalImport,
  Category,
  Contact,
  ContactImport,
  FolderUsage,
  InviteAction,
  PartStat,
  Rule,
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
  Signature,
} from './types';

export const UNDO_MS = 7000;

interface Outgoing {
  item: OutboxItem;
  draft: Draft;
  timer: ReturnType<typeof setTimeout>;
}

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

function err(kind: string, message: string) {
  return { kind, message };
}

function summary(m: MockMessage): MessageSummary {
  return {
    id: m.id,
    accountId: m.accountId,
    folderId: m.folderId,
    from: m.from,
    to: m.to,
    subject: m.subject,
    preview: m.preview,
    date: m.date,
    unread: m.unread,
    flagged: m.flagged,
    answered: m.answered,
    hasAttachments: m.hasAttachments,
    focused: m.focused,
    draft: m.draft,
    hasBody: m.hasBody,
    folderName: null,
    thread: m.thread,
    categories: [...m.categories],
    size: m.size,
    threadCount: 1,
  };
}

const EMPTY_CONTACT: Contact = { id: '', firstName: '', lastName: '', displayName: '', emails: [], phones: [], company: '', jobTitle: '', address: '', website: '', birthday: '', notes: '' };

const SAMPLE_CONTACTS: Contact[] = [
  { ...EMPTY_CONTACT, id: 'k1', firstName: 'Davide', lastName: 'Orlandi', emails: ['d.orlandi@studiorlandi.it'], phones: [{ kind: 'lavoro', number: '0444 123 456' }, { kind: 'cellulare', number: '333 123 4567' }], company: 'Studio Orlandi Architetti', jobTitle: 'Architetto', address: 'Contrà Porti 12\n36100 Vicenza (VI)', website: 'https://studiorlandi.example' },
  { ...EMPTY_CONTACT, id: 'k2', firstName: 'Lucia', lastName: 'Marchetti', emails: ['lucia@marchettitessuti.it'], phones: [{ kind: 'cellulare', number: '347 765 4321' }], company: 'Marchetti Tessuti', jobTitle: 'Commerciale', birthday: '1985-03-09' },
  { ...EMPTY_CONTACT, id: 'k3', firstName: 'Marco', lastName: 'Ventura', emails: ['m.ventura@valliarredi.it'], phones: [{ kind: 'lavoro', number: '0574 152 600' }], company: 'Valli Arredi', jobTitle: 'Responsabile commerciale' },
  { ...EMPTY_CONTACT, id: 'k4', firstName: 'Giorgio', lastName: 'Valli', emails: ['giorgio@valliarredi.it'], company: 'Valli Arredi', jobTitle: 'Titolare', notes: 'Preferisce essere chiamato la mattina.' },
  { ...EMPTY_CONTACT, id: 'k5', firstName: 'Elena', lastName: 'Fabbri', emails: ['amministrazione@fabbrilegnami.it'], phones: [{ kind: 'lavoro', number: '0574 222 333' }], company: 'Fabbri Legnami', jobTitle: 'Amministrazione', address: 'Via dei Pini 4\n59100 Prato (PO)' },
  { ...EMPTY_CONTACT, id: 'k6', displayName: 'Trasporti Ferri', emails: ['consegne@trasportiferri.it'], phones: [{ kind: 'lavoro', number: '0574 900 100' }], company: 'Trasporti Ferri' },
];

const REMOTE_IMG = /<img\b[^>]*\bsrc="https?:[^"]*"[^>]*>/gi;

export class MockBackend implements Backend {
  readonly real = false;
  private accountsList: Account[];
  private folderList: Folder[];
  private msgs: MockMessage[];
  private listeners = new Set<(e: EngineEvent) => void>();
  private outgoing = new Map<number, Outgoing>();
  private nextId = 1000;
  private sigs: Signature[] = [
    {
      id: 'firma-chiara',
      name: 'Chiara Valli',
      html: '<div>Chiara Valli</div><div style="color: #595959;">Ufficio acquisti, Valli Arredi</div><div style="color: #595959;">Tel. 0574 152 638</div>',
    },
  ];
  private cats: Category[] = [
    { id: 'c1', name: 'Clienti', color: 'verde' },
    { id: 'c2', name: 'Fornitori', color: 'blu' },
    { id: 'c3', name: 'Amministrazione', color: 'viola' },
    { id: 'c4', name: 'Urgente', color: 'rosso' },
    { id: 'c5', name: 'Fiera di gennaio', color: 'arancione' },
  ];
  private ruleList: Rule[] = [];
  private book: Contact[] = SAMPLE_CONTACTS.map((c) => ({ ...c, emails: [...c.emails], phones: c.phones.map((p) => ({ ...p })) }));
  private cal: CalEvent[];
  /** Messaggi di calendario messi in uscita (inviti, risposte, annullamenti), per i test. */
  itip: { method: string; to: string[]; subject: string; event: CalEvent }[] = [];
  /** Avvisi sul desktop mandati (per i test). */
  notified: { title: string; body: string }[] = [];
  /** Ritardo simulato per la ricerca dei server. */
  addDelay = 0;

  constructor(now = new Date()) {
    const d = mockData(now);
    this.accountsList = d.accounts;
    this.folderList = d.folders;
    this.msgs = d.messages;
    this.cal = mockEvents(now);
  }

  private emit(e: EngineEvent) {
    for (const l of this.listeners) l(e);
  }

  private changed(accountId: string, folders: number[] = []) {
    this.emit({ type: 'changed', accountId, folders });
  }

  private find(id: number): MockMessage {
    const m = this.msgs.find((x) => x.id === id);
    if (!m) throw err('non-trovato', 'Messaggio non trovato.');
    return m;
  }

  private roleFolder(accountId: string, role: Role): Folder {
    let f = this.folderList.find((x) => x.accountId === accountId && x.role === role);
    if (!f) {
      f = { id: this.nextId++, accountId, path: role, name: role, role, depth: 0, selectable: true, unread: 0, total: 0 };
      this.folderList.push(f);
    }
    return f;
  }

  onEvent(cb: (e: EngineEvent) => void) {
    this.listeners.add(cb);
    return () => this.listeners.delete(cb);
  }

  async accounts() {
    return this.accountsList.map((a) => ({ ...a }));
  }

  async addAccount(input: NewAccount): Promise<Account> {
    if (this.addDelay) await new Promise((r) => setTimeout(r, this.addDelay));
    const email = input.email.trim();
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) throw err('utente', 'Scrivi un indirizzo email completo, per esempio nome@azienda.it.');
    if (!input.password) throw err('utente', 'Scrivi la password della casella.');
    if (input.password === 'sbagliata') throw err('credenziali', 'Il server ha rifiutato nome utente o password.');
    const domain = email.split('@')[1];
    if (domain.endsWith('.sconosciuto') && !input.incoming) {
      throw err('manuale', `Non ho trovato da solo le impostazioni dei server di ${domain}. Inseriscile qui: le trovi nelle istruzioni del tuo provider o nel pannello del tuo hosting.`);
    }
    const kind = input.kind ?? 'imap';
    const a: Account = {
      id: `a${this.nextId++}`,
      name: input.name || email.split('@')[0],
      email,
      kind,
      incoming: input.incoming ?? { host: `${kind === 'imap' ? 'imap' : 'pop'}.${domain}`, port: kind === 'imap' ? 993 : 995, security: 'ssl', username: email },
      outgoing: input.outgoing ?? { host: `smtp.${domain}`, port: 465, security: 'ssl', username: email },
      sync: { keep: '3m', attachments: 'apertura', popKeep: '14' },
      serverSavesSent: false,
      sourceFile: null,
      status: { state: 'pronto', message: null, lastSync: Date.now() },
    };
    this.accountsList.push(a);
    for (const role of ['inbox', 'drafts', 'sent', 'deleted', 'junk', 'archive'] as Role[]) this.roleFolder(a.id, role);
    this.emit({ type: 'accounts' });
    return { ...a };
  }

  async updateAccount(account: Account) {
    const i = this.accountsList.findIndex((a) => a.id === account.id);
    if (i >= 0) this.accountsList[i] = { ...this.accountsList[i], name: account.name, sync: { ...account.sync }, signatureNew: account.signatureNew ?? null, signatureReply: account.signatureReply ?? null };
    this.emit({ type: 'accounts' });
  }

  async setPassword() {}

  async removeAccount(id: string) {
    this.accountsList = this.accountsList.filter((a) => a.id !== id);
    const gone = new Set(this.folderList.filter((f) => f.accountId === id).map((f) => f.id));
    this.folderList = this.folderList.filter((f) => !gone.has(f.id));
    this.msgs = this.msgs.filter((m) => !gone.has(m.folderId));
    this.emit({ type: 'accounts' });
  }

  async folders() {
    return this.folderList.map((f) => {
      const inside = this.msgs.filter((m) => m.folderId === f.id);
      return { ...f, total: inside.length, unread: inside.filter((m) => m.unread).length };
    });
  }

  /** Messaggi della conversazione fuori da Posta eliminata e Indesiderata. */
  private threadOf(m: MockMessage): MockMessage[] {
    const skip = new Set(this.folderList.filter((f) => f.accountId === m.accountId && (f.role === 'deleted' || f.role === 'junk')).map((f) => f.id));
    return this.msgs.filter((x) => x.accountId === m.accountId && x.thread === m.thread && !skip.has(x.folderId) && x.folderId !== -1);
  }

  private withCount(m: MockMessage): MessageSummary {
    return { ...summary(m), threadCount: this.threadOf(m).length };
  }

  async messages(folderId: number, tab: Tab | null, offset = 0, limit = 400) {
    const f = this.folderList.find((x) => x.id === folderId);
    return this.msgs
      .filter((m) => m.folderId === folderId && (f?.role !== 'inbox' || !tab || m.focused === (tab === 'prio')))
      .sort((a, b) => b.date - a.date || b.id - a.id)
      .slice(offset, offset + limit)
      .map((m) => this.withCount(m));
  }

  async thread(accountId: string, thread: string) {
    const first = this.msgs.find((m) => m.accountId === accountId && m.thread === thread);
    if (!first) return [];
    return this.threadOf(first)
      .sort((a, b) => a.date - b.date)
      .map((m) => ({ ...this.withCount(m), folderName: this.folderList.find((f) => f.id === m.folderId)?.name ?? null }));
  }

  async categories() {
    return this.cats.map((c) => ({ ...c }));
  }

  async saveCategories(list: Category[]) {
    const out: Category[] = [];
    for (const c of list) {
      const name = c.name.trim();
      if (!name) throw err('utente', 'Ogni categoria deve avere un nome.');
      if (out.some((o) => o.name.toLowerCase() === name.toLowerCase())) throw err('utente', `Esiste già una categoria "${name}".`);
      out.push({ ...c, name, id: c.id || `c${this.nextId++}` });
    }
    const gone = this.cats.filter((c) => !out.some((o) => o.id === c.id)).map((c) => c.id);
    for (const m of this.msgs) m.categories = m.categories.filter((x) => !gone.includes(x));
    this.cats = out;
    this.emit({ type: 'categories' });
    this.changed('', []);
    return out.map((c) => ({ ...c }));
  }

  async setCategory(ids: number[], category: string, on: boolean) {
    for (const id of ids) {
      const m = this.find(id);
      const has = m.categories.includes(category);
      if (on && !has) m.categories = [...m.categories, category];
      if (!on && has) m.categories = m.categories.filter((c) => c !== category);
      this.changed(m.accountId, [m.folderId]);
    }
  }

  async clearCategories(ids: number[]) {
    for (const id of ids) {
      const m = this.find(id);
      m.categories = [];
      this.changed(m.accountId, [m.folderId]);
    }
  }

  async search(query: string, accountId: string | null) {
    const words = query
      .toLowerCase()
      .normalize('NFD')
      .replace(/\p{Diacritic}/gu, '')
      .split(/\s+/)
      .filter(Boolean);
    const norm = (s: string) => s.toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
    return this.msgs
      .filter((m) => !accountId || m.accountId === accountId)
      .filter((m) => {
        const hay = norm(`${m.from.name} ${m.from.email} ${m.to.map((p) => p.name).join(' ')} ${m.subject} ${m.text}`);
        return words.every((w) => hay.includes(w));
      })
      .sort((a, b) => b.date - a.date)
      .map((m) => ({ ...this.withCount(m), folderName: this.folderList.find((f) => f.id === m.folderId)?.name ?? null }));
  }

  async quoteHtml(id: number) {
    const m = this.find(id);
    return m.html ?? textToHtml(m.text);
  }

  async signatures() {
    return this.sigs.map((s) => ({ ...s }));
  }

  async saveSignature(s: Signature) {
    if (!s.name.trim()) throw err('utente', 'Dai un nome alla firma.');
    const saved = { ...s, name: s.name.trim(), id: s.id || `firma-${this.nextId++}` };
    const i = this.sigs.findIndex((x) => x.id === saved.id);
    if (i >= 0) this.sigs[i] = saved;
    else this.sigs.push(saved);
    this.emit({ type: 'signatures' });
    return { ...saved };
  }

  async deleteSignature(id: string) {
    this.sigs = this.sigs.filter((s) => s.id !== id);
    for (const a of this.accountsList) {
      if (a.signatureNew === id) a.signatureNew = null;
      if (a.signatureReply === id) a.signatureReply = null;
    }
    this.emit({ type: 'signatures' });
    this.emit({ type: 'accounts' });
  }

  async fetchImage() {
    return null;
  }

  async message(id: number, allowRemote: boolean): Promise<MessageDetail> {
    const m = this.find(id);
    let html = m.html;
    let remote = 0;
    if (html && !allowRemote) {
      html = html.replace(REMOTE_IMG, () => {
        remote++;
        return '';
      });
    }
    const { sig: _sig, ...detail } = m;
    if (m.invite) {
      const ev = m.invite.event;
      const local = this.cal.find((e) => e.uid === ev.uid && e.recurrenceId === ev.recurrenceId) ?? null;
      detail.invite = { ...clone(m.invite), local: local ? clone(local) : null, outdated: !!local && local.sequence > ev.sequence, forMe: ev.attendees.some((a) => this.mine().includes(a.email.toLowerCase())) };
    }
    return { ...detail, html: html ? `<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{margin:0;font-family:system-ui,sans-serif;font-size:14.5px;line-height:1.5;color:#242424;background:#fff}img{max-width:100%;height:auto}table{max-width:100%}</style></head><body>${html}</body></html>` : null, remoteImages: remote, remoteAllowed: allowRemote };
  }

  async setFlags(ids: number[], seen: boolean | null, flagged: boolean | null) {
    for (const id of ids) {
      const m = this.find(id);
      if (seen !== null) m.unread = !seen;
      if (flagged !== null) m.flagged = flagged;
      this.changed(m.accountId, [m.folderId]);
    }
  }

  async moveMessages(ids: number[], folderId: number) {
    for (const id of ids) {
      const m = this.find(id);
      m.folderId = folderId;
      m.focused = true;
    }
    this.changed('', []);
  }

  async moveToRole(ids: number[], role: Role) {
    for (const id of ids) {
      const m = this.find(id);
      m.folderId = this.roleFolder(m.accountId, role).id;
      m.focused = true;
    }
    this.changed('', []);
  }

  async createFolder(accountId: string, parentId: number | null, name: string) {
    const n = name.trim();
    if (!n) throw err('utente', 'Scrivi il nome della cartella.');
    if (n.includes('/')) throw err('utente', 'Il nome della cartella non può contenere il carattere "/".');
    const parent = parentId === null ? null : this.folderList.find((f) => f.id === parentId);
    const path = parent ? `${parent.path}/${n}` : n;
    if (this.folderList.some((f) => f.accountId === accountId && f.path === path)) throw err('utente', `Esiste già una cartella "${n}" in questa posizione.`);
    const f: Folder = { id: this.nextId++, accountId, path, name: n, role: null, depth: parent ? parent.depth + 1 : 0, selectable: true, unread: 0, total: 0 };
    this.folderList.push(f);
    this.changed(accountId, []);
    return f.id;
  }

  async renameFolder(id: number, name: string) {
    const f = this.folderList.find((x) => x.id === id);
    if (!f) throw err('non-trovato', 'Cartella non trovata.');
    if (f.role) throw err('utente', 'Le cartelle predefinite non si possono rinominare.');
    const n = name.trim();
    if (!n) throw err('utente', 'Scrivi il nome della cartella.');
    const parent = f.path.includes('/') ? f.path.slice(0, f.path.lastIndexOf('/')) : '';
    const path = parent ? `${parent}/${n}` : n;
    if (path !== f.path && this.folderList.some((x) => x.accountId === f.accountId && x.path === path)) throw err('utente', `Esiste già una cartella "${n}" in questa posizione.`);
    for (const k of this.folderList) if (k.accountId === f.accountId && k.path.startsWith(`${f.path}/`)) k.path = path + k.path.slice(f.path.length);
    f.path = path;
    f.name = n;
    this.changed(f.accountId, []);
  }

  async deleteFolder(id: number) {
    const f = this.folderList.find((x) => x.id === id);
    if (!f) return;
    if (f.role) throw err('utente', 'Le cartelle predefinite non si possono eliminare.');
    const gone = this.folderList.filter((x) => x.accountId === f.accountId && (x.id === id || x.path.startsWith(`${f.path}/`)));
    const trash = this.roleFolder(f.accountId, 'deleted');
    const ids = new Set(gone.map((x) => x.id));
    for (const m of this.msgs) if (ids.has(m.folderId)) m.folderId = trash.id;
    this.folderList = this.folderList.filter((x) => !ids.has(x.id));
    this.changed(f.accountId, []);
  }

  async markFolderRead(id: number) {
    for (const m of this.msgs) if (m.folderId === id) m.unread = false;
    this.changed('', [id]);
  }

  async emptyFolder(id: number) {
    const n = this.msgs.filter((m) => m.folderId === id).length;
    this.msgs = this.msgs.filter((m) => m.folderId !== id);
    this.changed('', [id]);
    return n;
  }

  async deleteMessages(ids: number[]) {
    for (const id of ids) {
      const m = this.find(id);
      const trash = this.roleFolder(m.accountId, 'deleted');
      if (m.folderId === trash.id) this.msgs = this.msgs.filter((x) => x.id !== id);
      else m.folderId = trash.id;
    }
    this.changed('', []);
  }

  private draftMessage(d: Draft): MockMessage {
    const acct = this.accountsList.find((a) => a.id === d.accountId) ?? this.accountsList[0];
    const drafts = this.roleFolder(acct.id, 'drafts');
    return {
      id: this.nextId++,
      accountId: acct.id,
      folderId: drafts.id,
      from: { name: acct.name, email: acct.email },
      to: d.to,
      cc: d.cc,
      bcc: d.bcc,
      replyTo: [],
      subject: d.subject,
      preview: d.body.replace(/\s+/g, ' ').trim().slice(0, 180),
      date: Date.now(),
      unread: false,
      flagged: false,
      answered: false,
      hasAttachments: d.attachments.length > 0,
      focused: true,
      draft: true,
      hasBody: true,
      folderName: null,
      thread: `${d.key}@bozza`,
      categories: [],
      size: d.body.length * 2 + 3000,
      threadCount: 1,
      messageId: `${d.key}@bozza`,
      references: d.references,
      html: d.html ?? null,
      text: d.body,
      attachments: d.attachments.map((a, index) => ({ index, name: a.name, size: a.size, mime: a.mime ?? 'application/octet-stream', inline: false })),
      remoteImages: 0,
      remoteAllowed: false,
      sig: `draft:${d.key}`,
    };
  }

  private draftsByKey(key: string) {
    return this.msgs.filter((m) => m.sig === `draft:${key}`);
  }

  private static empty(d: Draft) {
    return !d.to.length && !d.cc.length && !d.bcc.length && !d.subject.trim() && !d.body.trim() && !d.attachments.length;
  }

  async saveDraft(d: Draft): Promise<SavedDraft> {
    const fresh = this.draftMessage(d);
    const old = this.draftsByKey(d.key);
    this.msgs = this.msgs.filter((m) => !old.includes(m));
    this.msgs.push(fresh);
    this.changed(fresh.accountId, [fresh.folderId]);
    return { id: fresh.id, attachments: d.attachments };
  }

  async closeDraft(d: Draft) {
    if (MockBackend.empty(d)) {
      await this.discardDraft(d.accountId, d.key);
      return null;
    }
    return this.saveDraft(d);
  }

  async discardDraft(_accountId: string, key: string) {
    this.msgs = this.msgs.filter((m) => m.sig !== `draft:${key}`);
    this.changed('', []);
  }

  async openDraft(id: number): Promise<Draft> {
    const m = this.find(id);
    const key = m.sig.startsWith('draft:') ? m.sig.slice(6) : `bozza-${m.id}`;
    m.sig = `draft:${key}`;
    return {
      key,
      accountId: m.accountId,
      to: m.to,
      cc: m.cc,
      bcc: m.bcc,
      subject: m.subject,
      body: m.text,
      html: m.html,
      attachments: m.attachments.map((a) => ({ name: a.name, size: a.size, mime: a.mime, source: { messageId: m.id, index: a.index } })),
      inReplyTo: null,
      references: m.references,
      replyToMessage: null,
    };
  }

  async send(d: Draft) {
    return this.enqueue(d, null);
  }

  async sendLater(d: Draft, at: number) {
    if (at <= Date.now() + 30_000) throw err('utente', 'Scegli un’ora futura per l’invio.');
    return this.enqueue(d, at);
  }

  private enqueue(d: Draft, at: number | null) {
    if (!d.to.length && !d.cc.length && !d.bcc.length) throw err('utente', 'Aggiungi almeno un destinatario.');
    const acct = this.accountsList.find((a) => a.id === d.accountId);
    if (!acct || acct.kind === 'local') throw err('utente', 'Da questo account non si può inviare posta.');
    const id = this.nextId++;
    const when = at ?? Date.now() + UNDO_MS;
    const item: OutboxItem = { id, accountId: acct.id, subject: d.subject, to: d.to, state: 'attesa', error: null, sendAfter: when };
    // I timer del browser non arrivano oltre i 24 giorni: basta per un'anteprima.
    const timer = setTimeout(() => this.deliver(id), Math.min(when - Date.now(), 2 ** 31 - 1));
    this.outgoing.set(id, { item, draft: JSON.parse(JSON.stringify(d)) as Draft, timer });
    for (const m of this.draftsByKey(d.key)) m.folderId = -1;
    this.emit({ type: 'outbox', accountId: acct.id, outboxId: id, state: at ? 'programmato' : 'attesa', error: null });
    this.changed(acct.id, []);
    return id;
  }

  async rules() {
    return JSON.parse(JSON.stringify(this.ruleList)) as Rule[];
  }

  async saveRules(list: Rule[]) {
    const out: Rule[] = [];
    for (const r of list) {
      const name = r.name.trim();
      if (!name) throw err('utente', 'Dai un nome a ogni regola.');
      if (!hasActions(r)) throw err('utente', `Scegli cosa deve fare la regola "${name}".`);
      out.push({ ...JSON.parse(JSON.stringify(r)), name, id: r.id || `r${this.nextId++}` });
    }
    this.ruleList = out;
    this.emit({ type: 'rules' });
    return this.rules();
  }

  async runRules(ids: string[], folderId: number) {
    let n = 0;
    for (const m of this.msgs.filter((x) => x.folderId === folderId && !x.draft)) {
      const hits = this.ruleList.filter((r) => ids.includes(r.id) && ruleMatches(r, summary(m), m.text));
      if (!hits.length) continue;
      n++;
      for (const r of hits) {
        const a = r.then;
        if (a.markRead) m.unread = false;
        if (a.flag) m.flagged = true;
        if (a.category && !m.categories.includes(a.category)) m.categories = [...m.categories, a.category];
        if (a.delete) m.folderId = this.roleFolder(m.accountId, 'deleted').id;
        else if (a.moveTo !== null) m.folderId = a.moveTo;
        if (a.stop) break;
      }
    }
    if (n) this.changed('', []);
    return n;
  }

  async setAutoReply(accountId: string, reply: AutoReply) {
    const a = this.accountsList.find((x) => x.id === accountId);
    if (!a) throw err('non-trovato', 'Account non trovato.');
    if (a.kind === 'local') throw err('utente', 'I file di dati non ricevono posta.');
    if (reply.start !== null && reply.end !== null && reply.end <= reply.start) throw err('utente', 'La fine del periodo deve venire dopo l’inizio.');
    if (reply.enabled && !reply.text.trim()) throw err('utente', 'Scrivi il messaggio della risposta automatica.');
    a.autoReply = { ...reply, since: reply.enabled ? Date.now() : (a.autoReply?.since ?? 0) };
    this.emit({ type: 'accounts' });
  }

  async mailboxUsage(accountId: string) {
    const out: FolderUsage[] = this.folderList
      .filter((f) => f.accountId === accountId)
      .map((f) => {
        const ms = this.msgs.filter((m) => m.folderId === f.id);
        return { folderId: f.id, count: ms.length, size: ms.reduce((t, m) => t + m.size, 0) };
      });
    return out.sort((a, b) => b.size - a.size || b.count - a.count);
  }

  async largeMessages(accountId: string, minSize: number) {
    return this.msgs
      .filter((m) => m.accountId === accountId && m.size >= minSize && !m.draft && m.folderId > 0)
      .sort((a, b) => b.size - a.size)
      .map((m) => ({ ...summary(m), folderName: this.folderList.find((f) => f.id === m.folderId)?.name ?? null }));
  }

  async olderMessages(folderId: number, before: number, archive: boolean) {
    const ms = this.msgs.filter((m) => m.folderId === folderId && m.date < before && !m.draft);
    if (archive && ms.length) await this.moveToRole(ms.map((m) => m.id), 'archive');
    return ms.length;
  }

  async addressBook() {
    const key = (c: Contact) => (c.lastName ? `${c.lastName} ${c.firstName}` : contactName(c)).toLowerCase();
    return [...this.book].sort((a, b) => key(a).localeCompare(key(b), 'it')).map((c) => JSON.parse(JSON.stringify(c)) as Contact);
  }

  async saveContact(c: Contact) {
    const emails = c.emails.map((e) => e.trim()).filter(Boolean);
    const bad = emails.find((e) => !/^[^@\s]+@[^@\s]+$/.test(e));
    if (bad) throw err('utente', `L'indirizzo "${bad}" non è valido.`);
    const saved: Contact = { ...JSON.parse(JSON.stringify(c)), emails, phones: c.phones.filter((p) => p.number.trim()), id: c.id || `k${this.nextId++}` };
    if (!contactName(saved).trim()) throw err('utente', 'Scrivi almeno un nome o un indirizzo email.');
    const i = this.book.findIndex((x) => x.id === saved.id);
    if (i >= 0) this.book[i] = saved;
    else this.book.push(saved);
    this.emit({ type: 'contacts' });
    return JSON.parse(JSON.stringify(saved)) as Contact;
  }

  async deleteContacts(ids: string[]) {
    this.book = this.book.filter((c) => !ids.includes(c.id));
    this.emit({ type: 'contacts' });
  }

  async importContacts(): Promise<ContactImport> {
    // Nel browser non si leggono file: si aggiungono due contatti di esempio.
    const extra = [
      { firstName: 'Paolo', lastName: 'Rossi', company: 'Rossi Srl', emails: ['paolo@rossi.example'] },
      { firstName: 'Anna', lastName: 'Neri', company: 'Studio Neri', emails: ['anna@neri.example'] },
    ];
    let added = 0;
    for (const e of extra) {
      if (this.book.some((c) => c.emails.includes(e.emails[0]))) continue;
      this.book.push({ ...EMPTY_CONTACT, ...e, id: `k${this.nextId++}` });
      added++;
    }
    this.emit({ type: 'contacts' });
    return { added, updated: 0, unchanged: extra.length - added };
  }

  async exportContacts(ids: string[] | null) {
    return ids ? ids.length : this.book.length;
  }

  // ---------- Calendario ----------

  private mine(): string[] {
    return this.accountsList.filter((a) => a.kind !== 'local').map((a) => a.email.toLowerCase());
  }

  private storeEvent(ev: CalEvent) {
    const i = this.cal.findIndex((x) => x.id === ev.id);
    if (i >= 0) this.cal[i] = clone(ev);
    else this.cal.push(clone(ev));
    if (ev.recurrenceId !== null) {
      for (const m of this.cal) if (m.uid === ev.uid && m.recurrenceId === null && m.repeat && !m.exceptions.includes(ev.recurrenceId)) m.exceptions = [...m.exceptions, ev.recurrenceId].sort((a, b) => a - b);
    }
  }

  private others(list: { name: string; email: string }[]) {
    const mine = this.mine();
    return list.filter((a) => !mine.includes(a.email.toLowerCase())).map((a) => a.email);
  }

  async events(from: number, to: number) {
    return this.cal
      .filter((e) => {
        if (e.start >= to) return false;
        if (!e.repeat) return Math.max(e.end, e.start + 1) > from;
        if (e.repeat.count) return true;
        return e.repeat.until === null || e.repeat.until + (e.end - e.start) > from;
      })
      .map(clone);
  }

  async saveEvent(input: CalEvent, send: boolean) {
    const ev = clone(input);
    ev.title = ev.title.trim();
    if (ev.end < ev.start) throw err('utente', "La fine deve venire dopo l'inizio.");
    if (ev.allDay && ev.end <= ev.start) ev.end = addDays(ev.start, 1);
    if (ev.repeat) {
      ev.repeat.interval = Math.max(1, ev.repeat.interval || 1);
      if (ev.repeat.freq === 'weekly' && !ev.repeat.days.length) ev.repeat.days = [new Date(ev.start).getDay()];
      if (ev.repeat.until !== null && ev.repeat.until < ev.start) throw err('utente', 'La ripetizione deve finire dopo il primo appuntamento.');
    }
    const seen = new Set<string>();
    ev.attendees = ev.attendees.filter((a) => a.email.trim() && !seen.has(a.email.toLowerCase()) && seen.add(a.email.toLowerCase()));
    const old = ev.id ? this.cal.find((x) => x.id === ev.id) : undefined;
    if (!ev.id) ev.id = `ev${this.nextId++}`;
    if (!ev.uid) ev.uid = `${ev.id}@better-look`;
    let sending = false;
    if (ev.attendees.length && isMine(ev, this.mine())) {
      const acct = this.accountsList.find((a) => a.id === ev.accountId && a.kind !== 'local') ?? this.accountsList.find((a) => a.kind !== 'local');
      if (!acct) throw err('utente', 'Per inviare gli inviti serve un account di posta.');
      ev.accountId = acct.id;
      ev.organizer = { name: acct.name, email: acct.email };
      ev.response = null;
      if (send) {
        if (old) ev.sequence = Math.max(old.sequence, ev.sequence) + 1;
        sending = true;
      }
    }
    this.storeEvent(ev);
    if (sending) {
      this.itip.push({ method: 'REQUEST', to: this.others(ev.attendees), subject: ev.title || '(senza titolo)', event: clone(ev) });
      const gone = (old?.attendees ?? []).filter((x) => !ev.attendees.some((y) => y.email.toLowerCase() === x.email.toLowerCase()));
      if (gone.length) this.itip.push({ method: 'CANCEL', to: this.others(gone), subject: `Annullato: ${ev.title}`, event: clone(ev) });
    }
    this.emit({ type: 'calendar' });
    return clone(ev);
  }

  async deleteEvent(id: string, occurrence: number | null, send: boolean) {
    const ev = this.cal.find((x) => x.id === id);
    if (!ev) return;
    if (occurrence !== null && ev.repeat) {
      if (!ev.exceptions.includes(occurrence)) ev.exceptions = [...ev.exceptions, occurrence].sort((a, b) => a - b);
    } else if (ev.recurrenceId === null) {
      this.cal = this.cal.filter((x) => x.uid !== ev.uid);
    } else {
      this.cal = this.cal.filter((x) => x.id !== id);
    }
    if (send && ev.attendees.length) {
      if (isMine(ev, this.mine())) this.itip.push({ method: 'CANCEL', to: this.others(ev.attendees), subject: `Annullato: ${ev.title}`, event: clone(ev) });
      else if (ev.organizer && ev.response !== 'declined') this.itip.push({ method: 'REPLY', to: [ev.organizer.email], subject: `Rifiutato: ${ev.title}`, event: clone(ev) });
    }
    this.emit({ type: 'calendar' });
  }

  async respondInvite(messageId: number, action: InviteAction, send: boolean) {
    const m = this.find(messageId);
    if (!m.invite) throw err('utente', "Nel messaggio non c'è un invito.");
    const inv = clone(m.invite.event);
    const local = this.cal.find((e) => e.uid === inv.uid && e.recurrenceId === inv.recurrenceId);
    const takeOut = () => {
      if (local) this.cal = this.cal.filter((e) => (local.recurrenceId === null ? e.uid !== local.uid : e.id !== local.id));
    };
    if (action === 'remove') {
      takeOut();
      this.emit({ type: 'calendar' });
      return null;
    }
    if (local && local.sequence > inv.sequence) throw err('utente', "Questo invito è superato: nel calendario c'è già una versione più recente.");
    const status: PartStat | null = action === 'add' ? null : action;
    inv.id = local?.id ?? `ev${this.nextId++}`;
    inv.accountId = m.accountId;
    if (local) {
      inv.reminder = local.reminder;
      inv.categories = [...local.categories];
    } else if (inv.reminder === null && !inv.allDay) inv.reminder = 15;
    if (status) {
      inv.response = status;
      const mine = this.mine();
      for (const a of inv.attendees) if (mine.includes(a.email.toLowerCase())) a.status = status;
    }
    if (status === 'declined') takeOut();
    else this.storeEvent(inv);
    if (send && status && inv.organizer && !isMine(inv, this.mine())) {
      const prefix = status === 'accepted' ? 'Accettato' : status === 'tentative' ? 'Provvisorio' : 'Rifiutato';
      this.itip.push({ method: 'REPLY', to: [inv.organizer.email], subject: `${prefix}: ${inv.title}`, event: clone(inv) });
    }
    m.answered = m.answered || (send && !!status);
    this.emit({ type: 'calendar' });
    return status === 'declined' ? null : clone(inv);
  }

  async respondEvent(id: string, action: InviteAction, send: boolean) {
    const ev = this.cal.find((x) => x.id === id);
    if (!ev) throw err('non-trovato', 'Evento non trovato.');
    if (isMine(ev, this.mine())) throw err('utente', "Sei tu l'organizzatore di questo evento.");
    if (action !== 'accepted' && action !== 'tentative' && action !== 'declined') throw err('utente', 'Risposta non valida.');
    ev.response = action;
    const mine = this.mine();
    for (const a of ev.attendees) if (mine.includes(a.email.toLowerCase())) a.status = action;
    if (action === 'declined') this.cal = this.cal.filter((x) => (ev.recurrenceId === null ? x.uid !== ev.uid : x.id !== id));
    if (send && ev.organizer) {
      const prefix = action === 'accepted' ? 'Accettato' : action === 'tentative' ? 'Provvisorio' : 'Rifiutato';
      this.itip.push({ method: 'REPLY', to: [ev.organizer.email], subject: `${prefix}: ${ev.title}`, event: clone(ev) });
    }
    this.emit({ type: 'calendar' });
    return action === 'declined' ? null : clone(ev);
  }

  async importCalendar(): Promise<CalImport> {
    // Nel browser non si leggono file: si aggiungono due eventi di esempio.
    const now = new Date();
    const first = new Date(now.getFullYear(), now.getMonth() + 1, 12).getTime();
    const extra: CalEvent[] = [
      { ...mockEvents(now)[0], id: '', uid: 'fiera-milano@example.test', title: 'Fiera Arredo Milano', location: 'Fiera Milano, Rho', start: first, end: addDays(first, 3), allDay: true, repeat: null, reminder: 1440, categories: ['c5'] },
      { ...mockEvents(now)[0], id: '', uid: 'corso-sicurezza@example.test', title: 'Corso sicurezza sul lavoro', location: 'Confartigianato, Prato', start: new Date(first).setHours(9, 0, 0, 0), end: new Date(first).setHours(13, 0, 0, 0), repeat: null, reminder: 60, categories: [] },
    ];
    let added = 0;
    for (const e of extra) {
      if (this.cal.some((x) => x.uid === e.uid)) continue;
      this.cal.push({ ...e, id: `ev${this.nextId++}` });
      added++;
    }
    this.emit({ type: 'calendar' });
    return { added, updated: 0, unchanged: extra.length - added };
  }

  async exportCalendar() {
    return this.cal.length;
  }

  async notify(title: string, body: string) {
    this.notified.push({ title, body });
    return true;
  }

  private deliver(id: number) {
    const o = this.outgoing.get(id);
    if (!o) return;
    this.outgoing.delete(id);
    const acct = this.accountsList.find((a) => a.id === o.draft.accountId)!;
    const sent = this.draftMessage(o.draft);
    sent.folderId = this.roleFolder(acct.id, 'sent').id;
    sent.draft = false;
    sent.sig = '';
    this.msgs = this.msgs.filter((m) => m.sig !== `draft:${o.draft.key}`);
    this.msgs.push(sent);
    if (o.draft.replyToMessage) {
      const orig = this.msgs.find((m) => m.id === o.draft.replyToMessage);
      if (orig) orig.answered = true;
    }
    this.emit({ type: 'outbox', accountId: acct.id, outboxId: id, state: 'inviato', error: null });
    this.changed(acct.id, [sent.folderId]);
  }

  async cancelSend(id: number) {
    const o = this.outgoing.get(id);
    if (!o) return null;
    clearTimeout(o.timer);
    this.outgoing.delete(id);
    const drafts = this.roleFolder(o.draft.accountId, 'drafts');
    for (const m of this.draftsByKey(o.draft.key)) m.folderId = drafts.id;
    this.emit({ type: 'outbox', accountId: o.item.accountId, outboxId: id, state: 'annullato', error: null });
    return o.draft;
  }

  async outbox() {
    return [...this.outgoing.values()].map((o) => ({ ...o.item }));
  }

  async outboxTake(id: number) {
    return this.cancelSend(id);
  }

  async syncNow() {
    for (const a of this.accountsList) this.emit({ type: 'status', accountId: a.id, status: { state: 'pronto', message: null, lastSync: Date.now() } });
  }

  async contacts(prefix: string): Promise<Person[]> {
    const p = prefix.toLowerCase();
    const seen = new Map<string, Person>();
    // Prima la rubrica, poi chi ha scritto o ricevuto posta.
    for (const c of this.book) {
      const name = contactName(c);
      for (const e of c.emails) {
        if ((e.toLowerCase().includes(p) || name.toLowerCase().includes(p)) && !seen.has(e.toLowerCase())) seen.set(e.toLowerCase(), { name, email: e });
      }
    }
    for (const m of this.msgs) {
      for (const x of [m.from, ...m.to, ...m.cc]) {
        if (x.email && (x.email.toLowerCase().includes(p) || x.name.toLowerCase().includes(p)) && !seen.has(x.email.toLowerCase())) {
          seen.set(x.email.toLowerCase(), x);
        }
      }
    }
    return [...seen.values()].slice(0, 8);
  }

  async openAttachment() {}
  async saveAttachment() {
    return false;
  }
  async exportEml() {
    return false;
  }
  async exportMbox() {
    return null;
  }
  /** Messaggi e allegati illeggibili da segnalare alla prossima importazione (per le prove). */
  importSkipped = 0;
  importLostAttachments = 0;

  async importArchive(kind: 'pst' | 'mbox') {
    // Versione di esempio: un file di dati con alcuni messaggi della posta in arrivo.
    const name = kind === 'pst' ? 'Archivio 2019' : 'Esportazione';
    const a: Account = {
      id: `a${this.nextId++}`,
      name,
      email: name,
      kind: 'local',
      incoming: null,
      outgoing: null,
      sync: { keep: 'tutto', attachments: 'subito', popKeep: 'mai' },
      serverSavesSent: false,
      sourceFile: `/esempio/${name}.${kind}`,
      status: { state: 'pronto', message: null, lastSync: null },
    };
    this.accountsList.push(a);
    this.emit({ type: 'accounts' });
    const source = this.msgs.filter((m) => this.folderList.find((f) => f.id === m.folderId)?.role === 'inbox').slice(0, 3);
    this.emit({ type: 'import', name, done: 0, total: source.length });
    const f = this.roleFolder(a.id, 'inbox');
    for (const m of source) this.msgs.push({ ...JSON.parse(JSON.stringify(m)), id: this.nextId++, folderId: f.id, accountId: a.id });
    this.emit({ type: 'import', name, done: source.length, total: source.length });
    this.changed(a.id, [f.id]);
    return { ...a, skipped: this.importSkipped, lostAttachments: this.importLostAttachments };
  }

  async pickFiles(): Promise<DraftAttachment[]> {
    return [{ name: 'Planimetria_aggiornata.pdf', size: 248_000, mime: 'application/pdf', path: '/esempio/Planimetria_aggiornata.pdf' }];
  }

  async openLink() {}

  async openComposeWindow() {
    return false;
  }

  async takeComposeDraft() {
    return null;
  }

  async openPrintWindow(id: number, remote: boolean) {
    const url = `${location.pathname}?finestra=stampa&id=${id}&remoto=${remote ? 1 : 0}`;
    return !!globalThis.open?.(url, '_blank');
  }

  async printPage() {
    globalThis.print?.();
  }

  async speak() {}

  async speaking() {
    return false;
  }

  async stopSpeaking() {}
}
