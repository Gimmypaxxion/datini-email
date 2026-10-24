// Stato del modulo Attività (Ctrl+4): elenchi, attività, messaggi
// contrassegnati, attività aperta nel riquadro e promemoria. Le operazioni
// passano dal motore (backend.ts); avvisi e conferme dallo stato principale.

import type { MailStore } from './store.svelte';
import { SMART_LISTS, dayKey, dueTaskReminders, emptyTask, openCount, tasksIn, type SmartList, type TaskView } from './tasks';
import { dayText, startOfDay } from './calendar';
import { errorMessage, type MessageSummary, type Task, type TaskList, type TaskRepeat } from './types';

const DISMISSED_KEY = 'better-look-promemoria-attivita';
const CHECK_MS = 15_000;
const MIN = 60_000;

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

export class TasksState {
  lists = $state<TaskList[]>([]);
  items = $state<Task[]>([]);
  /** Messaggi contrassegnati di tutti gli account. */
  flagged = $state<MessageSummary[]>([]);
  view = $state<TaskView>('giornata');
  /** Attività aperta nel riquadro a destra. */
  sel = $state<string | null>(null);
  query = $state('');
  /** Sezione Completate aperta. */
  doneOpen = $state(false);
  loaded = $state(false);
  /** Elenco che si sta creando o rinominando nel riquadro a sinistra. */
  listEdit = $state<{ id: string; name: string; error: string } | null>(null);
  /** Promemoria arrivati e non ancora chiusi. */
  reminders = $state<Task[]>([]);
  now = $state(Date.now());
  /** Cresce per chiedere il fuoco sul campo Aggiungi un'attività (Nuova attività, Ctrl+N). */
  addFocus = $state(0);

  private store: MailStore;
  private timer: ReturnType<typeof setInterval> | undefined;
  private lastCheck = 0;
  private dismissed = new Set<string>();
  private snoozed = new Map<string, number>();

  constructor(store: MailStore) {
    this.store = store;
  }

  // ---------- Letture ----------

  get selected(): Task | null {
    return this.items.find((t) => t.id === this.sel) ?? null;
  }

  get isSmart(): boolean {
    return !this.view.startsWith('elenco:');
  }

  /** L'elenco dell'utente aperto, se la vista è un elenco. */
  get currentList(): TaskList | null {
    return this.view.startsWith('elenco:') ? (this.lists.find((l) => `elenco:${l.id}` === this.view) ?? null) : null;
  }

  get title(): string {
    return this.currentList?.name ?? SMART_LISTS.find((s) => s.id === this.view)?.label ?? 'Attività';
  }

  /** Sottotitolo: la data per La mia giornata. */
  get subtitle(): string {
    return this.view === 'giornata' ? dayText(this.now, false) : '';
  }

  /** Attività della vista, filtrate dalla ricerca. */
  get shown(): { open: Task[]; done: Task[] } {
    const q = this.query.trim().toLowerCase();
    if (q) {
      const hit = (t: Task) => t.title.toLowerCase().includes(q) || t.notes.toLowerCase().includes(q) || t.steps.some((s) => s.title.toLowerCase().includes(q));
      const all = this.items.filter(hit);
      return { open: all.filter((t) => !t.done).sort((a, b) => b.created - a.created), done: all.filter((t) => t.done) };
    }
    return tasksIn(this.items, this.view, this.now);
  }

  get shownFlagged(): MessageSummary[] {
    const q = this.query.trim().toLowerCase();
    if (!q) return this.flagged;
    return this.flagged.filter((m) => m.subject.toLowerCase().includes(q) || (m.from.name || m.from.email).toLowerCase().includes(q));
  }

  count(view: TaskView): number {
    if (view === 'contrassegnati') return this.flagged.length;
    return openCount(this.items, view, this.now);
  }

  listName(id: string): string {
    return id ? (this.lists.find((l) => l.id === id)?.name ?? 'Attività') : 'Attività';
  }

  // ---------- Caricamento ----------

  async reload() {
    try {
      const [lists, items, flagged] = await Promise.all([this.store.backend.taskLists(), this.store.backend.tasks(), this.store.backend.flaggedMessages()]);
      this.lists = lists;
      this.items = items;
      this.flagged = flagged;
      this.now = Date.now();
      if (this.view.startsWith('elenco:') && !this.currentList) this.view = 'attivita';
      if (this.sel && !this.items.some((t) => t.id === this.sel)) this.sel = null;
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    } finally {
      this.loaded = true;
    }
  }

  async reloadFlagged() {
    try {
      this.flagged = await this.store.backend.flaggedMessages();
    } catch {
      /* resta l'elenco di prima */
    }
  }

  /** Nuova attività: nelle viste senza campo (Messaggi contrassegnati) si passa ad Attività. */
  newTask() {
    if (this.view === 'contrassegnati') this.setView('attivita');
    this.store.setModule('attivita');
    this.addFocus++;
  }

  setView(v: TaskView) {
    this.view = v;
    this.sel = null;
    this.query = '';
    this.listEdit = null;
    this.now = Date.now();
    if (v === 'contrassegnati') void this.reloadFlagged();
    this.store.save();
  }

  // ---------- Attività ----------

  /** Nuova attività nella vista aperta: in La mia giornata ci entra, in Importanti è importante. */
  async add(title: string): Promise<boolean> {
    const t = emptyTask(this.currentList?.id ?? '', Date.now());
    t.title = title.trim();
    if (!t.title) return false;
    if (this.view === 'giornata') t.myDay = dayKey(Date.now());
    if (this.view === 'importanti') t.important = true;
    if (this.view === 'pianificate') t.due = startOfDay(Date.now());
    try {
      const [saved] = await this.store.backend.saveTask(t);
      this.items = [saved, ...this.items];
      return true;
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
      return false;
    }
  }

  /** Salva una modifica: si vede subito, poi arriva la versione del motore. */
  async update(id: string, patch: Partial<Task>) {
    const i = this.items.findIndex((t) => t.id === id);
    if (i < 0) return;
    const before = clone(this.items[i]);
    const next = { ...clone(this.items[i]), ...patch };
    this.items[i] = next;
    try {
      const [saved, follow] = await this.store.backend.saveTask(next);
      const j = this.items.findIndex((t) => t.id === id);
      if (j >= 0) this.items[j] = saved;
      if (follow) {
        this.items = [follow, ...this.items];
        this.store.flash({ kind: 'info', text: `Attività completata. La prossima è già in elenco: ${follow.title}.` }, 4000);
      }
    } catch (e) {
      const j = this.items.findIndex((t) => t.id === id);
      if (j >= 0) this.items[j] = before;
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  toggleDone(t: Task) {
    return this.update(t.id, { done: !t.done, doneAt: t.done ? null : Date.now() });
  }

  toggleImportant(t: Task) {
    return this.update(t.id, { important: !t.important });
  }

  toggleMyDay(t: Task) {
    const today = dayKey(Date.now());
    return this.update(t.id, { myDay: t.myDay === today ? null : today });
  }

  setDue(t: Task, due: number | null) {
    return this.update(t.id, { due: due === null ? null : startOfDay(due), repeat: due === null ? null : t.repeat });
  }

  setReminder(t: Task, at: number | null) {
    return this.update(t.id, { reminder: at });
  }

  /** Ripetizione: serve una scadenza, se manca è oggi. */
  setRepeat(t: Task, r: TaskRepeat | null) {
    return this.update(t.id, { repeat: r, due: r && t.due === null ? startOfDay(Date.now()) : t.due });
  }

  moveTo(t: Task, list: string) {
    return this.update(t.id, { list });
  }

  addStep(t: Task, title: string) {
    const s = title.trim();
    if (!s) return;
    return this.update(t.id, { steps: [...t.steps, { id: '', title: s, done: false }] });
  }

  toggleStep(t: Task, stepId: string) {
    return this.update(t.id, { steps: t.steps.map((s) => (s.id === stepId ? { ...s, done: !s.done } : s)) });
  }

  renameStep(t: Task, stepId: string, title: string) {
    if (!title.trim()) return this.removeStep(t, stepId);
    return this.update(t.id, { steps: t.steps.map((s) => (s.id === stepId ? { ...s, title: title.trim() } : s)) });
  }

  removeStep(t: Task, stepId: string) {
    return this.update(t.id, { steps: t.steps.filter((s) => s.id !== stepId) });
  }

  async remove(t: Task) {
    const yes = await this.store.ask({ title: 'Eliminare l’attività?', text: `«${t.title}» verrà eliminata definitivamente.`, ok: 'Elimina', danger: true });
    if (!yes) return;
    try {
      await this.store.backend.deleteTask(t.id);
      this.items = this.items.filter((x) => x.id !== t.id);
      if (this.sel === t.id) this.sel = null;
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Aggiungi alle attività: un messaggio diventa una cosa da fare. */
  async fromMessage(m: MessageSummary) {
    const t = emptyTask('', Date.now());
    t.title = m.subject.trim() || 'Rispondere al messaggio';
    t.notes = `Dal messaggio di ${m.from.name || m.from.email}.`;
    t.message = m.id;
    t.myDay = dayKey(Date.now());
    try {
      const [saved] = await this.store.backend.saveTask(t);
      this.items = [saved, ...this.items];
      this.store.flash({ kind: 'info', text: `Aggiunta a La mia giornata: ${saved.title}.` }, 4000);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Messaggi contrassegnati ----------

  /** Completato: si toglie il contrassegno dal messaggio. */
  async unflag(m: MessageSummary) {
    this.flagged = this.flagged.filter((x) => x.id !== m.id);
    try {
      await this.store.backend.setFlags([m.id], null, false);
      this.store.markFlag(m.id, false);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
      void this.reloadFlagged();
    }
  }

  // ---------- Elenchi ----------

  startNewList() {
    this.listEdit = { id: '', name: '', error: '' };
  }

  startRenameList(l: TaskList) {
    this.listEdit = { id: l.id, name: l.name, error: '' };
  }

  async commitList() {
    const e = this.listEdit;
    if (!e) return;
    const name = e.name.trim();
    if (!name) {
      this.listEdit = null;
      return;
    }
    if (this.lists.some((l) => l.id !== e.id && l.name.trim().toLowerCase() === name.toLowerCase())) {
      e.error = 'Esiste già un elenco con questo nome.';
      return;
    }
    try {
      const saved = await this.store.backend.saveTaskList({ id: e.id, name });
      this.listEdit = null;
      if (e.id) this.lists = this.lists.map((l) => (l.id === saved.id ? saved : l));
      else {
        this.lists = [...this.lists, saved];
        this.setView(`elenco:${saved.id}`);
      }
    } catch (err) {
      e.error = errorMessage(err);
    }
  }

  async deleteList(l: TaskList) {
    const n = this.items.filter((t) => t.list === l.id).length;
    const yes = await this.store.ask({
      title: `Eliminare l’elenco «${l.name}»?`,
      text: n ? `Verranno eliminate anche ${n === 1 ? 'l’attività che contiene' : `le ${n} attività che contiene`}.` : 'L’elenco è vuoto.',
      ok: 'Elimina elenco',
      danger: true,
    });
    if (!yes) return;
    try {
      await this.store.backend.deleteTaskList(l.id);
      this.lists = this.lists.filter((x) => x.id !== l.id);
      this.items = this.items.filter((t) => t.list !== l.id);
      if (this.view === `elenco:${l.id}`) this.setView('attivita');
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Promemoria ----------

  private loadDismissed() {
    try {
      const raw = globalThis.localStorage?.getItem(DISMISSED_KEY);
      if (raw) this.dismissed = new Set(JSON.parse(raw) as string[]);
    } catch {
      /* senza archivio si ricomincia */
    }
  }

  private saveDismissed() {
    try {
      // Si tengono solo gli ultimi, per non crescere all'infinito.
      globalThis.localStorage?.setItem(DISMISSED_KEY, JSON.stringify([...this.dismissed].slice(-300)));
    } catch {
      /* come sopra */
    }
  }

  private key(t: Task) {
    return `${t.id}@${t.reminder}`;
  }

  startReminders(persist = true) {
    if (this.timer) return;
    if (persist) this.loadDismissed();
    this.lastCheck = Date.now() - 6 * 60 * MIN;
    void this.checkReminders();
    this.timer = setInterval(() => void this.checkReminders(), CHECK_MS);
  }

  stopReminders() {
    clearInterval(this.timer);
    this.timer = undefined;
  }

  async checkReminders(now = Date.now()) {
    const from = this.lastCheck;
    this.lastCheck = now;
    // Al cambio di giorno La mia giornata riparte vuota.
    if (startOfDay(now) !== startOfDay(this.now)) this.now = now;
    let all: Task[];
    try {
      all = await this.store.backend.tasks();
    } catch {
      return;
    }
    if (this.store.module !== 'attivita') this.items = all;
    const due = dueTaskReminders(all, from, now).filter((t) => !this.dismissed.has(this.key(t)));
    for (const [id, until] of this.snoozed) {
      if (until > now) continue;
      this.snoozed.delete(id);
      const t = all.find((x) => x.id === id && !x.done);
      if (t) due.push(t);
    }
    const fresh = due.filter((t) => !this.reminders.some((r) => r.id === t.id));
    if (!fresh.length) return;
    this.reminders = [...this.reminders, ...fresh];
    const body = fresh.length === 1 ? 'Promemoria dell’attività' : fresh.map((t) => t.title).join('\n');
    void this.store.backend.notify(fresh.length === 1 ? fresh[0].title : `${fresh.length} attività da ricordare`, body);
  }

  dismiss(t: Task) {
    this.reminders = this.reminders.filter((r) => r.id !== t.id);
    this.dismissed.add(this.key(t));
    this.saveDismissed();
  }

  dismissAll() {
    for (const t of this.reminders) this.dismissed.add(this.key(t));
    this.reminders = [];
    this.saveDismissed();
  }

  snooze(t: Task, minutes: number) {
    this.reminders = this.reminders.filter((r) => r.id !== t.id);
    this.snoozed.set(t.id, Date.now() + minutes * MIN);
  }

  /** Completata dal promemoria. */
  async completeFromReminder(t: Task) {
    this.dismiss(t);
    if (!this.items.some((x) => x.id === t.id)) this.items = [t, ...this.items];
    await this.update(t.id, { done: true, doneAt: Date.now() });
  }

  showReminder(t: Task) {
    this.dismiss(t);
    this.store.setModule('attivita');
    if (!this.items.some((x) => x.id === t.id)) this.items = [t, ...this.items];
    this.view = t.list ? `elenco:${t.list}` : 'attivita';
    this.sel = t.id;
  }

  /** Le viste intelligenti, per il riquadro a sinistra. */
  get smart(): { id: SmartList; label: string; icon: (typeof SMART_LISTS)[number]['icon'] }[] {
    return SMART_LISTS;
  }
}
