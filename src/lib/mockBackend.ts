// Motore di esempio, tutto in memoria. Si comporta come quello vero (stessi
// comandi, stessi eventi) ma senza rete: serve nel browser e nei test.

import type { Backend, Tab } from './backend';
import { mockData, type MockMessage } from './mock';
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

export const UNDO_MS = 7000;

interface Outgoing {
  item: OutboxItem;
  draft: Draft;
  timer: ReturnType<typeof setTimeout>;
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
  };
}

const REMOTE_IMG = /<img\b[^>]*\bsrc="https?:[^"]*"[^>]*>/gi;

export class MockBackend implements Backend {
  readonly real = false;
  private accountsList: Account[];
  private folderList: Folder[];
  private msgs: MockMessage[];
  private listeners = new Set<(e: EngineEvent) => void>();
  private outgoing = new Map<number, Outgoing>();
  private nextId = 1000;
  /** Ritardo simulato per la ricerca dei server. */
  addDelay = 0;

  constructor(now = new Date()) {
    const d = mockData(now);
    this.accountsList = d.accounts;
    this.folderList = d.folders;
    this.msgs = d.messages;
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
    if (i >= 0) this.accountsList[i] = { ...this.accountsList[i], name: account.name, sync: { ...account.sync } };
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

  async messages(folderId: number, tab: Tab | null, offset = 0, limit = 400) {
    const f = this.folderList.find((x) => x.id === folderId);
    return this.msgs
      .filter((m) => m.folderId === folderId && (f?.role !== 'inbox' || !tab || m.focused === (tab === 'prio')))
      .sort((a, b) => b.date - a.date || b.id - a.id)
      .slice(offset, offset + limit)
      .map(summary);
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
      .map((m) => ({ ...summary(m), folderName: this.folderList.find((f) => f.id === m.folderId)?.name ?? null }));
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
      messageId: `${d.key}@bozza`,
      references: d.references,
      html: null,
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
      attachments: m.attachments.map((a) => ({ name: a.name, size: a.size, mime: a.mime, source: { messageId: m.id, index: a.index } })),
      inReplyTo: null,
      references: m.references,
      replyToMessage: null,
    };
  }

  async send(d: Draft) {
    if (!d.to.length && !d.cc.length && !d.bcc.length) throw err('utente', 'Aggiungi almeno un destinatario.');
    const acct = this.accountsList.find((a) => a.id === d.accountId);
    if (!acct || acct.kind === 'local') throw err('utente', 'Da questo account non si può inviare posta.');
    const id = this.nextId++;
    const item: OutboxItem = { id, accountId: acct.id, subject: d.subject, to: d.to, state: 'attesa', error: null, sendAfter: Date.now() + UNDO_MS };
    const timer = setTimeout(() => this.deliver(id), UNDO_MS);
    this.outgoing.set(id, { item, draft: JSON.parse(JSON.stringify(d)) as Draft, timer });
    for (const m of this.draftsByKey(d.key)) m.folderId = -1;
    this.emit({ type: 'outbox', accountId: acct.id, outboxId: id, state: 'attesa', error: null });
    this.changed(acct.id, []);
    return id;
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
}
