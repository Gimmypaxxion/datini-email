// Stato del calendario (Ctrl+2): vista e giorno mostrati, appuntamenti
// caricati, modulo di modifica, ricerca e promemoria. Le operazioni passano
// dal motore (backend.ts); gli avvisi e le conferme dallo stato principale.

import type { MailStore } from './store.svelte';
import {
  addDays,
  DAY,
  dueReminders,
  emptyEvent,
  expand,
  isMine,
  matchEvent,
  MIN,
  navigate,
  nextHalfHour,
  nextOccurrence,
  rangeTitle,
  sameDay,
  startOfDay,
  timeText,
  untilText,
  viewRange,
  type CalView,
  type Occurrence,
} from './calendar';
import { errorMessage, type CalEvent, type InviteAction, type Person } from './types';

export interface EventEdit {
  ev: CalEvent;
  /** Riunione: si mostrano i partecipanti e si invia l'invito. */
  meeting: boolean;
  /** Si modifica tutta la serie (aperta da una sua occorrenza). */
  series: boolean;
  /** Versione salvata (la serie, se si modifica una sola occorrenza). */
  original: CalEvent | null;
  /** Occorrenza da cui si è aperto il modulo. */
  occ: Occurrence | null;
}

export interface ReminderItem {
  key: string;
  occ: Occurrence;
}

const DISMISSED_KEY = 'better-look-promemoria';
const CHECK_MS = 15_000;

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v)) as T;
}

export class CalendarState {
  view = $state<CalView>('settimana-lavorativa');
  /** Giorno mostrato (la vista lo contiene). */
  anchor = $state(startOfDay(Date.now()));
  events = $state<CalEvent[]>([]);
  loading = $state(false);
  /** Occorrenza selezionata (chiave). */
  sel = $state<string | null>(null);
  edit = $state<EventEdit | null>(null);
  query = $state('');
  found = $state<Occurrence[]>([]);
  reminders = $state<ReminderItem[]>([]);
  /** Prossimi appuntamenti (riquadro a sinistra), aggiornati con i promemoria. */
  upcoming = $state<Occurrence[]>([]);
  /** Ora attuale, aggiornata ogni tanto per la linea rossa e i promemoria. */
  now = $state(Date.now());

  private store: MailStore;
  private loadSeq = 0;
  private searchSeq = 0;
  private timer: ReturnType<typeof setInterval> | undefined;
  private lastCheck = 0;
  private dismissed = new Map<string, number>();
  private snoozed = new Map<string, number>();

  constructor(store: MailStore) {
    this.store = store;
  }

  get range() {
    return viewRange(this.view, this.anchor);
  }

  get title(): string {
    return rangeTitle(this.view, this.anchor);
  }

  get occs(): Occurrence[] {
    const r = this.range;
    return expand(this.events, r.from, r.to);
  }

  get selected(): Occurrence | null {
    if (!this.sel) return null;
    return this.occs.find((o) => o.key === this.sel) ?? this.found.find((o) => o.key === this.sel) ?? null;
  }

  get myEmails(): string[] {
    return this.store.accounts.filter((a) => a.kind !== 'local').map((a) => a.email);
  }

  /** Variabili di colore dell'appuntamento: la prima categoria, oppure l'accento. */
  tone(ev: CalEvent): string {
    for (const id of ev.categories) {
      const c = this.store.categories.find((x) => x.id === id || x.name.toLowerCase() === id.toLowerCase());
      if (c) return `--ev-bg: var(--cat-${c.color}-bg); --ev-fg: var(--cat-${c.color}-fg); --ev-line: var(--cat-${c.color})`;
    }
    return '';
  }

  /** Stato da mostrare: annullato, provvisorio, da rispondere. */
  state(ev: CalEvent): string {
    if (ev.cancelled) return 'cancelled';
    if (ev.response === 'tentative') return 'tentative';
    if (ev.organizer && !this.isMine(ev) && (ev.response === null || ev.response === 'needs-action')) return 'needs-action';
    return '';
  }

  isMine(ev: CalEvent): boolean {
    return isMine(ev, this.myEmails);
  }

  // ---------- Caricamento e navigazione ----------

  async reload() {
    const r = this.range;
    const seq = ++this.loadSeq;
    this.loading = true;
    try {
      const list = await this.store.backend.events(addDays(r.from, -7), addDays(r.to, 7));
      if (seq !== this.loadSeq) return;
      this.events = list;
      if (this.sel && !this.occs.some((o) => o.key === this.sel)) this.sel = null;
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    } finally {
      if (seq === this.loadSeq) this.loading = false;
    }
    if (this.query.trim()) void this.search(this.query);
    if (this.timer) void this.refreshUpcoming();
  }

  private async refreshUpcoming() {
    const now = Date.now();
    try {
      const list = await this.store.backend.events(now - DAY, now + 8 * DAY);
      this.upcoming = expand(list, now, now + 7 * DAY).filter((o) => o.end > now && !o.ev.cancelled).slice(0, 6);
    } catch {
      /* resta l'elenco di prima */
    }
  }

  setView(v: CalView) {
    this.view = v;
    void this.reload();
  }

  today() {
    this.anchor = startOfDay(Date.now());
    void this.reload();
  }

  move(dir: 1 | -1) {
    this.anchor = navigate(this.view, this.anchor, dir);
    this.sel = null;
    void this.reload();
  }

  goTo(t: number, view: CalView | null = null) {
    this.anchor = startOfDay(t);
    if (view) this.view = view;
    void this.reload();
  }

  // ---------- Ricerca ----------

  async search(q: string) {
    this.query = q;
    const seq = ++this.searchSeq;
    if (!q.trim()) {
      this.found = [];
      return;
    }
    try {
      const all = await this.store.backend.events(-8.64e15, 8.64e15);
      if (seq !== this.searchSeq) return;
      const now = Date.now();
      this.found = all
        .filter((e) => matchEvent(e, q))
        .map((e) => nextOccurrence(e, now))
        .filter((o): o is Occurrence => !!o)
        .sort((a, b) => a.start - b.start);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Modifica ----------

  /** Nuovo appuntamento (o riunione) nell'orario indicato, o alla prossima mezz'ora. */
  newEvent(opts: { start?: number; end?: number; allDay?: boolean; meeting?: boolean; title?: string; attendees?: Person[]; notes?: string } = {}) {
    let start = opts.start;
    if (start === undefined) {
      const now = Date.now();
      start = this.store.module === 'calendario' && !sameDay(this.anchor, now) ? this.anchor + 9 * 60 * MIN : nextHalfHour(now);
    }
    const allDay = !!opts.allDay;
    const end = opts.end ?? (allDay ? addDays(startOfDay(start), 1) : start + 30 * MIN);
    const ev = emptyEvent(allDay ? startOfDay(start) : start, end, allDay);
    ev.title = opts.title ?? '';
    ev.notes = opts.notes ?? '';
    const mine = this.myEmails.map((e) => e.toLowerCase());
    ev.attendees = (opts.attendees ?? []).filter((p) => p.email && !mine.includes(p.email.toLowerCase())).map((p) => ({ name: p.name, email: p.email, status: 'needs-action', optional: false }));
    ev.accountId = this.store.composeAccount()?.id ?? null;
    this.edit = { ev, meeting: !!opts.meeting || ev.attendees.length > 0, series: false, original: null, occ: null };
  }

  /** Apre un'occorrenza; per una serie chiede se solo questa o tutta la serie. */
  async open(o: Occurrence) {
    let ev = clone(o.ev);
    let series = false;
    if (o.ev.repeat) {
      const ans = await this.store.choose({
        title: 'Appuntamento ricorrente',
        text: `“${o.ev.title || 'Senza titolo'}” si ripete. Vuoi aprire solo questa occorrenza o tutta la serie?`,
        ok: 'Solo questa',
        alt: 'Tutta la serie',
      });
      if (!ans) return;
      if (ans === 'ok') ev = { ...ev, id: '', recurrenceId: o.start, start: o.start, end: o.end, repeat: null, exceptions: [] };
      else series = true;
    }
    this.sel = o.key;
    this.edit = { ev, meeting: ev.attendees.length > 0, series, original: clone(o.ev), occ: o };
  }

  /** Salva il modulo; con `send` manda l'invito o l'aggiornamento. Restituisce l'errore, o null. */
  async save(send: boolean): Promise<string | null> {
    const e = this.edit;
    if (!e) return null;
    const ev = e.ev;
    if (ev.end < ev.start) return "La fine deve venire dopo l'inizio.";
    const invited = ev.attendees.length > 0 && this.isMine(ev);
    try {
      const saved = await this.store.backend.saveEvent(clone(ev), send && invited);
      this.edit = null;
      if (this.store.module === 'calendario' && !(saved.start >= this.range.from && saved.start < this.range.to)) this.anchor = startOfDay(saved.start);
      await this.reload();
      this.sel = `${saved.id}:${saved.start}`;
      if (send && invited) {
        const n = saved.attendees.length;
        this.store.flash({ kind: 'info', text: e.original ? `Aggiornamento inviato ${n === 1 ? 'al partecipante' : `a ${n} partecipanti`}.` : `Invito inviato ${n === 1 ? 'al partecipante' : `a ${n} partecipanti`}.` }, 5000);
      }
      return null;
    } catch (err) {
      return errorMessage(err);
    }
  }

  /** Elimina l'occorrenza o la serie, chiedendo conferma e se avvisare gli altri. */
  async remove(o: Occurrence) {
    const ev = o.ev;
    let occurrence: number | null = null;
    const name = ev.title || 'Senza titolo';
    if (ev.repeat) {
      const ans = await this.store.choose({ title: 'Eliminare un appuntamento ricorrente?', text: `“${name}” si ripete. Vuoi eliminare solo questa occorrenza o tutta la serie?`, ok: 'Solo questa', alt: 'Tutta la serie', danger: true });
      if (!ans) return;
      if (ans === 'ok') occurrence = o.start;
    }
    let send = false;
    if (ev.attendees.length && this.isMine(ev)) {
      const check = { label: "Invia l'annullamento ai partecipanti", value: true };
      if (!(await this.store.ask({ title: 'Annullare la riunione?', text: `“${name}” sparisce dal calendario.`, ok: 'Annulla la riunione', danger: true, check }))) return;
      send = check.value;
    } else if (ev.organizer && !this.isMine(ev) && (ev.response === 'accepted' || ev.response === 'tentative')) {
      const check = { label: `Comunica a ${ev.organizer.name || ev.organizer.email} che non parteciperai`, value: true };
      if (!(await this.store.ask({ title: `Eliminare “${name}”?`, text: "L'evento sparisce dal tuo calendario.", ok: 'Elimina', danger: true, check }))) return;
      send = check.value;
    } else if (!ev.repeat) {
      if (!(await this.store.ask({ title: `Eliminare “${name}”?`, text: "L'appuntamento sparisce dal calendario.", ok: 'Elimina', danger: true }))) return;
    }
    try {
      await this.store.backend.deleteEvent(ev.id, occurrence, send);
      if (this.sel === o.key) this.sel = null;
      this.edit = null;
      await this.reload();
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  /** Cambia dal calendario la risposta a un invito. */
  async respond(o: Occurrence, action: InviteAction, send = true) {
    try {
      await this.store.backend.respondEvent(o.ev.id, action, send);
      await this.reload();
      const verb = action === 'accepted' ? 'Hai accettato' : action === 'tentative' ? 'Hai risposto provvisorio' : 'Hai rifiutato';
      this.store.flash({ kind: 'info', text: `${verb}${send && o.ev.organizer ? '. Risposta inviata.' : '.'}` }, 4000);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  async importIcs() {
    try {
      const r = await this.store.backend.importCalendar();
      if (!r) return;
      await this.reload();
      const parts = [r.added && `${r.added} ${r.added === 1 ? 'nuovo' : 'nuovi'}`, r.updated && `${r.updated} ${r.updated === 1 ? 'aggiornato' : 'aggiornati'}`, r.unchanged && `${r.unchanged} già presenti`].filter(Boolean);
      this.store.flash({ kind: 'info', text: `Appuntamenti importati: ${parts.join(', ') || 'nessuno'}.` }, 7000);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) }, 9000);
    }
  }

  async exportIcs() {
    try {
      const n = await this.store.backend.exportCalendar();
      if (n !== null) this.store.flash({ kind: 'info', text: n === 1 ? '1 appuntamento esportato in iCalendar.' : `${n} appuntamenti esportati in iCalendar.` }, 5000);
    } catch (e) {
      this.store.flash({ kind: 'error', text: errorMessage(e) });
    }
  }

  // ---------- Promemoria ----------

  private loadDismissed() {
    try {
      const raw = globalThis.localStorage?.getItem(DISMISSED_KEY);
      const data = raw ? (JSON.parse(raw) as Record<string, number>) : {};
      const old = Date.now() - 3 * DAY;
      for (const [k, at] of Object.entries(data)) if (at > old) this.dismissed.set(k, at);
    } catch {
      /* nessun archivio: i promemoria ignorati valgono per questa sessione */
    }
  }

  private saveDismissed() {
    try {
      globalThis.localStorage?.setItem(DISMISSED_KEY, JSON.stringify(Object.fromEntries(this.dismissed)));
    } catch {
      /* come sopra */
    }
  }

  /** Avvia i promemoria: anche quelli scaduti da poco, per eventi non ancora finiti. */
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
    this.now = now;
    const from = this.lastCheck;
    this.lastCheck = now;
    let list: CalEvent[];
    try {
      list = await this.store.backend.events(now - 2 * DAY, now + 15 * DAY);
    } catch {
      return;
    }
    const occs = expand(list, now - DAY, now + 15 * DAY);
    this.upcoming = occs.filter((o) => o.end > now && o.start < now + 7 * DAY && !o.ev.cancelled).slice(0, 6);
    const due = dueReminders(occs, from, now).filter((o) => o.end > now && !this.dismissed.has(o.key));
    for (const [key, until] of this.snoozed) {
      if (until > now) continue;
      this.snoozed.delete(key);
      const o = occs.find((x) => x.key === key);
      if (o && o.end > now) due.push(o);
    }
    const fresh = due.filter((o) => !this.reminders.some((r) => r.key === o.key));
    if (!fresh.length) return;
    this.reminders = [...this.reminders, ...fresh.map((occ) => ({ key: occ.key, occ }))];
    const first = fresh[0];
    const body = fresh.length === 1 ? [untilText(first.start, now), first.ev.allDay ? 'Tutto il giorno' : timeText(first.start), first.ev.location].filter(Boolean).join(' · ') : fresh.map((o) => `${timeText(o.start)} ${o.ev.title}`).join('\n');
    void this.store.backend.notify(fresh.length === 1 ? first.ev.title || 'Promemoria' : `${fresh.length} promemoria`, body);
  }

  dismiss(key: string) {
    this.reminders = this.reminders.filter((r) => r.key !== key);
    this.dismissed.set(key, Date.now());
    this.saveDismissed();
  }

  dismissAll() {
    for (const r of this.reminders) this.dismissed.set(r.key, Date.now());
    this.reminders = [];
    this.saveDismissed();
  }

  snooze(key: string, minutes: number) {
    this.reminders = this.reminders.filter((r) => r.key !== key);
    this.snoozed.set(key, Date.now() + minutes * MIN);
  }

  /** Dal promemoria al calendario, con l'appuntamento selezionato. */
  showReminder(r: ReminderItem) {
    this.store.setModule('calendario');
    this.goTo(r.occ.start, this.view === 'mese' ? 'giorno' : null);
    this.sel = r.key;
  }
}
