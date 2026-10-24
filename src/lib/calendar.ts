// Calendario: date, ripetizioni, disposizione degli appuntamenti nella griglia
// e testi in italiano. Tutto in ora locale: le serie si espandono qui, così
// un appuntamento delle 9:00 resta alle 9:00 anche dopo il cambio dell'ora.

import type { Attendee, CalEvent, PartStat, Repeat } from './types';

export const DAY = 86_400_000;
export const MIN = 60_000;

export type CalView = 'giorno' | 'settimana-lavorativa' | 'settimana' | 'mese';

export const VIEW_NAMES: Record<CalView, string> = {
  giorno: 'Giorno',
  'settimana-lavorativa': 'Settimana lavorativa',
  settimana: 'Settimana',
  mese: 'Mese',
};

export const GIORNI = ['domenica', 'lunedì', 'martedì', 'mercoledì', 'giovedì', 'venerdì', 'sabato'];
export const GIORNI_BREVI = ['dom', 'lun', 'mar', 'mer', 'gio', 'ven', 'sab'];
export const MESI = ['gennaio', 'febbraio', 'marzo', 'aprile', 'maggio', 'giugno', 'luglio', 'agosto', 'settembre', 'ottobre', 'novembre', 'dicembre'];

// ---------- Date ----------

export function startOfDay(t: number): number {
  const d = new Date(t);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** Stessa ora locale `n` giorni dopo. */
export function addDays(t: number, n: number): number {
  const d = new Date(t);
  d.setDate(d.getDate() + n);
  return d.getTime();
}

export function daysInMonth(year: number, month: number): number {
  return new Date(year, month + 1, 0).getDate();
}

/** Stesso giorno `n` mesi dopo (l'ultimo del mese se quel giorno non c'è). */
export function addMonths(t: number, n: number): number {
  const d = new Date(t);
  const day = d.getDate();
  d.setDate(1);
  d.setMonth(d.getMonth() + n);
  d.setDate(Math.min(day, daysInMonth(d.getFullYear(), d.getMonth())));
  return d.getTime();
}

/** Lunedì della settimana (in Italia la settimana inizia di lunedì). */
export function startOfWeek(t: number): number {
  const d = new Date(startOfDay(t));
  return addDays(d.getTime(), -((d.getDay() + 6) % 7));
}

export function sameDay(a: number, b: number): boolean {
  return startOfDay(a) === startOfDay(b);
}

/** Giorni di calendario fra due istanti (anche con il cambio dell'ora). */
export function daysBetween(a: number, b: number): number {
  return Math.round((startOfDay(b) - startOfDay(a)) / DAY);
}

/** Il giorno `nth` (1-5, -1 per l'ultimo) del mese che cade nel giorno della settimana indicato. */
export function nthWeekday(year: number, month: number, weekday: number, nth: number): Date | null {
  if (nth > 0) {
    const first = new Date(year, month, 1).getDay();
    const day = 1 + ((7 + weekday - first) % 7) + 7 * (nth - 1);
    return day <= daysInMonth(year, month) ? new Date(year, month, day) : null;
  }
  const lastDay = daysInMonth(year, month);
  const last = new Date(year, month, lastDay).getDay();
  return new Date(year, month, lastDay - ((7 + last - weekday) % 7));
}

/** Settimana del mese del giorno: 1-4, oppure -1 se è l'ultima (quinta). */
export function weekOfMonth(t: number): number {
  const d = new Date(t);
  const n = Math.ceil(d.getDate() / 7);
  return n >= 5 ? -1 : n;
}

export function isLastWeekOfMonth(t: number): boolean {
  const d = new Date(t);
  return d.getDate() + 7 > daysInMonth(d.getFullYear(), d.getMonth());
}

// ---------- Viste ----------

export interface ViewRange {
  from: number;
  to: number;
  /** Mezzanotte di ogni giorno mostrato. */
  days: number[];
}

export function viewRange(view: CalView, anchor: number): ViewRange {
  let from: number;
  let count: number;
  if (view === 'giorno') {
    from = startOfDay(anchor);
    count = 1;
  } else if (view === 'mese') {
    const d = new Date(anchor);
    const first = new Date(d.getFullYear(), d.getMonth(), 1).getTime();
    from = startOfWeek(first);
    const last = new Date(d.getFullYear(), d.getMonth() + 1, 0).getTime();
    count = (daysBetween(from, startOfWeek(last)) / 7 + 1) * 7;
  } else {
    from = startOfWeek(anchor);
    count = view === 'settimana' ? 7 : 5;
  }
  const days = Array.from({ length: count }, (_, i) => addDays(from, i));
  return { from, to: addDays(from, count), days };
}

export function navigate(view: CalView, anchor: number, dir: 1 | -1): number {
  if (view === 'giorno') return addDays(startOfDay(anchor), dir);
  if (view === 'mese') return addMonths(startOfDay(anchor), dir);
  return addDays(startOfDay(anchor), 7 * dir);
}

export function dayText(t: number, withYear = true): string {
  const d = new Date(t);
  return `${GIORNI[d.getDay()]} ${d.getDate()} ${MESI[d.getMonth()]}${withYear ? ` ${d.getFullYear()}` : ''}`;
}

/** Titolo della vista: "lunedì 19 ottobre 2026", "19-23 ottobre 2026", "ottobre 2026". */
export function rangeTitle(view: CalView, anchor: number): string {
  if (view === 'giorno') return dayText(anchor);
  if (view === 'mese') {
    const d = new Date(anchor);
    return `${MESI[d.getMonth()]} ${d.getFullYear()}`;
  }
  const r = viewRange(view, anchor);
  const a = new Date(r.days[0]);
  const b = new Date(r.days[r.days.length - 1]);
  if (a.getFullYear() !== b.getFullYear()) return `${a.getDate()} ${MESI[a.getMonth()]} ${a.getFullYear()} - ${b.getDate()} ${MESI[b.getMonth()]} ${b.getFullYear()}`;
  if (a.getMonth() !== b.getMonth()) return `${a.getDate()} ${MESI[a.getMonth()]} - ${b.getDate()} ${MESI[b.getMonth()]} ${b.getFullYear()}`;
  return `${a.getDate()}-${b.getDate()} ${MESI[a.getMonth()]} ${a.getFullYear()}`;
}

/** Giorni del mini calendario: sei settimane a partire dal lunedì. */
export function monthGrid(anchor: number): number[] {
  const d = new Date(anchor);
  const from = startOfWeek(new Date(d.getFullYear(), d.getMonth(), 1).getTime());
  return Array.from({ length: 42 }, (_, i) => addDays(from, i));
}

// ---------- Ripetizioni ----------

export interface Occurrence {
  ev: CalEvent;
  start: number;
  end: number;
  /** Identifica l'occorrenza: id dell'evento e inizio. */
  key: string;
}

function at(base: Date, year: number, month: number, day: number): number {
  return new Date(year, month, day, base.getHours(), base.getMinutes(), base.getSeconds(), base.getMilliseconds()).getTime();
}

/** Inizi delle ripetizioni a partire dalla prima, in ordine, finché `stop` dice basta. */
function* candidates(ev: CalEvent, r: Repeat, skipBefore: number): Generator<number> {
  const s = new Date(ev.start);
  const step = Math.max(1, r.interval || 1);
  const counting = r.count !== null && r.count !== undefined;
  // Senza un numero di ripetizioni si salta direttamente vicino all'intervallo.
  const jump = (unit: number) => (counting ? 0 : Math.max(0, Math.floor(Math.max(0, skipBefore - ev.start) / unit / step) - 1));
  const LIMIT = 200_000;
  if (r.freq === 'daily') {
    for (let k = jump(DAY); k < LIMIT; k++) yield addDays(ev.start, k * step);
  }
  if (r.freq === 'weekly') {
    const days = (r.days.length ? [...r.days] : [s.getDay()]).sort((a, b) => ((a + 6) % 7) - ((b + 6) % 7));
    const week0 = startOfWeek(ev.start);
    for (let k = jump(7 * DAY); k < LIMIT; k++) {
      const monday = addDays(week0, 7 * k * step);
      for (const d of days) {
        const day = new Date(addDays(monday, (d + 6) % 7));
        const t = at(s, day.getFullYear(), day.getMonth(), day.getDate());
        if (t >= ev.start) yield t;
      }
    }
  }
  if (r.freq === 'monthly') {
    for (let k = jump(31 * DAY); k < LIMIT; k++) {
      const m = s.getMonth() + k * step;
      const y = s.getFullYear() + Math.floor(m / 12);
      const mm = ((m % 12) + 12) % 12;
      if (r.week !== null && r.days.length) {
        const d = nthWeekday(y, mm, r.days[0], r.week);
        if (d) {
          const t = at(s, y, mm, d.getDate());
          if (t >= ev.start) yield t;
        }
      } else if (s.getDate() <= daysInMonth(y, mm)) {
        yield at(s, y, mm, s.getDate());
      }
    }
  }
  if (r.freq === 'yearly') {
    for (let k = jump(366 * DAY); k < LIMIT; k++) {
      const y = s.getFullYear() + k * step;
      if (r.week !== null && r.days.length) {
        const d = nthWeekday(y, s.getMonth(), r.days[0], r.week);
        if (d) {
          const t = at(s, y, s.getMonth(), d.getDate());
          if (t >= ev.start) yield t;
        }
      } else if (s.getDate() <= daysInMonth(y, s.getMonth())) {
        yield at(s, y, s.getMonth(), s.getDate());
      }
    }
  }
}

/** Occorrenze dell'evento che toccano l'intervallo [from, to). */
export function occurrences(ev: CalEvent, from: number, to: number): Occurrence[] {
  const span = (start: number) => {
    if (!ev.allDay) return start + Math.max(0, ev.end - ev.start);
    return addDays(start, Math.max(1, daysBetween(ev.start, ev.end)));
  };
  const occ = (start: number): Occurrence => ({ ev, start, end: span(start), key: `${ev.id}:${start}` });
  if (!ev.repeat) {
    const end = Math.max(ev.end, ev.start + 1);
    return ev.start < to && end > from ? [occ(ev.start)] : [];
  }
  const r = ev.repeat;
  const out: Occurrence[] = [];
  const dur = span(ev.start) - ev.start;
  let n = 0;
  let guard = 0;
  for (const t of candidates(ev, r, from - dur)) {
    if (++guard > 20000) break;
    if (r.count !== null && r.count !== undefined && ++n > r.count) break;
    if (r.until !== null && r.until !== undefined && t > r.until) break;
    if (t >= to) break;
    if (t + Math.max(dur, 1) > from && !ev.exceptions.includes(t)) out.push(occ(t));
  }
  return out;
}

/** Tutte le occorrenze dell'intervallo, in ordine di inizio (le più lunghe prima). */
export function expand(events: CalEvent[], from: number, to: number): Occurrence[] {
  const out: Occurrence[] = [];
  for (const ev of events) out.push(...occurrences(ev, from, to));
  return out.sort((a, b) => a.start - b.start || b.end - b.start - (a.end - a.start) || a.ev.title.localeCompare(b.ev.title, 'it'));
}

/** Prossima occorrenza a partire da `now` (o l'ultima, se la serie è finita). */
export function nextOccurrence(ev: CalEvent, now: number): Occurrence | null {
  if (!ev.repeat) return occurrences(ev, -8.64e15, 8.64e15)[0] ?? null;
  const next = occurrences(ev, now, addDays(now, 3700))[0];
  if (next) return next;
  const past = occurrences(ev, ev.start, now);
  return past[past.length - 1] ?? null;
}

// ---------- Ruoli ----------

/** L'evento è mio: creato da me o di cui sono l'organizzatore. */
export function isMine(ev: CalEvent, myEmails: string[]): boolean {
  if (!ev.organizer) return true;
  const o = ev.organizer.email.trim().toLowerCase();
  return myEmails.some((e) => e.trim().toLowerCase() === o);
}

export function isMeeting(ev: CalEvent): boolean {
  return ev.attendees.length > 0;
}

/** Va mostrato nella striscia "tutto il giorno" (anche gli appuntamenti di più giorni). */
export function isAllDayLike(o: { start: number; end: number; ev: CalEvent }): boolean {
  return o.ev.allDay || o.end - o.start >= DAY;
}

// ---------- Disposizione nella griglia ----------

export interface Placed {
  occ: Occurrence;
  /** Minuti dall'inizio del giorno. */
  top: number;
  height: number;
  col: number;
  cols: number;
  /** Continua dal giorno prima o nel giorno dopo. */
  cutStart: boolean;
  cutEnd: boolean;
}

/** Appuntamenti di un giorno affiancati in colonne quando si sovrappongono. */
export function layoutDay(occs: Occurrence[], dayStart: number, minHeight = 20): Placed[] {
  const dayEnd = addDays(dayStart, 1);
  const items = occs
    .filter((o) => !isAllDayLike(o) && o.start < dayEnd && Math.max(o.end, o.start + 1) > dayStart)
    .map((o) => {
      const s = Math.max(o.start, dayStart);
      const e = Math.min(Math.max(o.end, o.start + 1), dayEnd);
      const top = (s - dayStart) / MIN;
      const height = Math.max(minHeight, (e - s) / MIN);
      return { occ: o, top, height, bottom: top + height, col: 0, cols: 1, cutStart: o.start < dayStart, cutEnd: o.end > dayEnd };
    })
    .sort((a, b) => a.top - b.top || b.height - a.height);
  const out: Placed[] = [];
  let group: typeof items = [];
  let groupEnd = -1;
  const flush = () => {
    const cols: number[] = [];
    for (const it of group) {
      let c = cols.findIndex((end) => end <= it.top);
      if (c < 0) {
        c = cols.length;
        cols.push(0);
      }
      cols[c] = it.bottom;
      it.col = c;
    }
    for (const it of group) {
      it.cols = cols.length;
      out.push({ occ: it.occ, top: it.top, height: it.height, col: it.col, cols: it.cols, cutStart: it.cutStart, cutEnd: it.cutEnd });
    }
    group = [];
    groupEnd = -1;
  };
  for (const it of items) {
    if (group.length && it.top >= groupEnd) flush();
    group.push(it);
    groupEnd = Math.max(groupEnd, it.bottom);
  }
  if (group.length) flush();
  return out;
}

export interface Bar {
  occ: Occurrence;
  /** Primo giorno (indice nella vista) e quanti giorni copre. */
  first: number;
  span: number;
  row: number;
  cutStart: boolean;
  cutEnd: boolean;
}

/** Barre "tutto il giorno" della vista, una riga per ognuna senza sovrapposizioni. */
export function allDayBars(occs: Occurrence[], days: number[]): Bar[] {
  if (!days.length) return [];
  const from = days[0];
  const last = days.length - 1;
  const rows: number[] = [];
  const out: Bar[] = [];
  for (const o of occs.filter(isAllDayLike).sort((a, b) => a.start - b.start || b.end - a.end)) {
    const a = daysBetween(from, o.start);
    const endDay = o.ev.allDay || o.end === startOfDay(o.end) ? daysBetween(from, o.end) - 1 : daysBetween(from, o.end);
    const first = Math.max(0, a);
    const lastDay = Math.min(last, endDay);
    if (lastDay < 0 || first > last || lastDay < first) continue;
    let row = rows.findIndex((free) => free <= first);
    if (row < 0) {
      row = rows.length;
      rows.push(0);
    }
    rows[row] = lastDay + 1;
    out.push({ occ: o, first, span: lastDay - first + 1, row, cutStart: a < 0, cutEnd: endDay > last });
  }
  return out;
}

/** Occorrenze di ogni giorno (vista mese): prima quelle di tutto il giorno. */
export function dayItems(occs: Occurrence[], day: number): Occurrence[] {
  const end = addDays(day, 1);
  return occs
    .filter((o) => o.start < end && Math.max(o.end, o.start + 1) > day)
    .sort((a, b) => Number(isAllDayLike(b)) - Number(isAllDayLike(a)) || a.start - b.start);
}

/** Altri appuntamenti nello stesso orario (non di tutto il giorno, non lo stesso evento). */
export function conflicts(target: { start: number; end: number; uid: string }, occs: Occurrence[]): Occurrence[] {
  return occs.filter((o) => !isAllDayLike(o) && o.ev.uid !== target.uid && o.start < target.end && o.end > target.start && !o.ev.cancelled);
}

// ---------- Testi ----------

export function timeText(t: number): string {
  const d = new Date(t);
  return `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
}

/** "lunedì 19 ottobre 2026, 10:30-12:00"; per tutto il giorno solo le date. */
export function whenText(start: number, end: number, allDay: boolean, now = Date.now()): string {
  const year = new Date(start).getFullYear() !== new Date(now).getFullYear() || new Date(end).getFullYear() !== new Date(now).getFullYear();
  if (allDay) {
    const last = startOfDay(Math.max(end, start + 1) - 1);
    if (sameDay(start, last)) return `${dayText(start, year)}, tutto il giorno`;
    return `Da ${dayText(start, year)} a ${dayText(last, year)}`;
  }
  if (sameDay(start, end) || end === addDays(startOfDay(start), 1)) return `${dayText(start, year)}, ${timeText(start)}-${timeText(end)}`;
  return `Da ${dayText(start, year)}, ${timeText(start)} a ${dayText(end, year)}, ${timeText(end)}`;
}

export function durationText(ms: number): string {
  const m = Math.round(ms / MIN);
  if (m < 60) return `${m} minuti`;
  const h = Math.floor(m / 60);
  const r = m % 60;
  const hours = h === 1 ? '1 ora' : `${h} ore`;
  return r ? `${hours} e ${r} minuti` : hours;
}

function article(day: number): string {
  return day === 0 ? 'la domenica' : `il ${GIORNI[day]}`;
}

function listText(parts: string[]): string {
  return parts.length <= 1 ? (parts[0] ?? '') : `${parts.slice(0, -1).join(', ')} e ${parts[parts.length - 1]}`;
}

/** "il secondo martedì", "l'ultimo venerdì", "la seconda domenica". */
export function nthDayText(week: number, day: number): string {
  if (day === 0) {
    const fem: Record<number, string> = { 1: 'la prima', 2: 'la seconda', 3: 'la terza', 4: 'la quarta', [-1]: "l'ultima" };
    return `${fem[week] ?? 'la'} domenica`;
  }
  const masc: Record<number, string> = { 1: 'il primo', 2: 'il secondo', 3: 'il terzo', 4: 'il quarto', [-1]: "l'ultimo" };
  return `${masc[week] ?? 'il'} ${GIORNI[day]}`;
}

/** "18 dicembre 2026", "1° gennaio 2027". */
export function dateText(t: number, withYear = true): string {
  const d = new Date(t);
  return `${d.getDate() === 1 ? '1°' : d.getDate()} ${MESI[d.getMonth()]}${withYear ? ` ${d.getFullYear()}` : ''}`;
}

/** "Ogni settimana il lunedì e il mercoledì, fino al 18 dicembre 2026". */
export function repeatText(r: Repeat, start: number): string {
  const s = new Date(start);
  const n = Math.max(1, r.interval || 1);
  const every = (one: string, many: string) => (n === 1 ? `Ogni ${one}` : `Ogni ${n} ${many}`);
  const byWeek = r.week !== null && r.week !== undefined && r.days.length > 0;
  let text: string;
  if (r.freq === 'daily') text = every('giorno', 'giorni');
  else if (r.freq === 'weekly') {
    const days = (r.days.length ? [...r.days] : [s.getDay()]).sort((a, b) => ((a + 6) % 7) - ((b + 6) % 7));
    if (n === 1 && days.join() === '1,2,3,4,5') text = 'Ogni giorno feriale';
    else text = `${every('settimana', 'settimane')} ${listText(days.map(article))}`;
  } else if (r.freq === 'monthly') {
    text = `${every('mese', 'mesi')} ${byWeek ? nthDayText(r.week!, r.days[0]) : `il giorno ${s.getDate()}`}`;
  } else {
    text = `${every('anno', 'anni')} ${byWeek ? `${nthDayText(r.week!, r.days[0])} di ${MESI[s.getMonth()]}` : `il ${dateText(start, false)}`}`;
  }
  if (r.count) text += r.count === 1 ? ', una volta' : `, per ${r.count} volte`;
  else if (r.until) text += `, fino al ${dateText(r.until)}`;
  return text;
}

export const REMINDERS: [number | null, string][] = [
  [null, 'Nessuno'],
  [0, "All'inizio"],
  [5, '5 minuti prima'],
  [10, '10 minuti prima'],
  [15, '15 minuti prima'],
  [30, '30 minuti prima'],
  [60, '1 ora prima'],
  [120, '2 ore prima'],
  [720, '12 ore prima'],
  [1080, '18 ore prima'],
  [1440, '1 giorno prima'],
  [10080, '1 settimana prima'],
];

export function reminderText(min: number | null): string {
  const known = REMINDERS.find(([m]) => m === min);
  if (known) return known[1];
  if (min === null) return 'Nessuno';
  return min % 60 === 0 ? `${min / 60} ore prima` : `${min} minuti prima`;
}

export function partStatText(s: PartStat | null): string {
  switch (s) {
    case 'accepted':
      return 'Accettato';
    case 'tentative':
      return 'Provvisorio';
    case 'declined':
      return 'Rifiutato';
    default:
      return 'Nessuna risposta';
  }
}

/** "2 accettati, 1 provvisorio, 1 senza risposta". */
export function responsesText(list: Attendee[]): string {
  const n = (s: PartStat) => list.filter((a) => a.status === s).length;
  const parts = [
    [n('accepted'), 'accettato', 'accettati'],
    [n('tentative'), 'provvisorio', 'provvisori'],
    [n('declined'), 'rifiutato', 'rifiutati'],
    [n('needs-action'), 'senza risposta', 'senza risposta'],
  ] as const;
  return parts
    .filter(([c]) => c > 0)
    .map(([c, one, many]) => `${c} ${c === 1 ? one : many}`)
    .join(', ');
}

// ---------- Ripetizione nel modulo ----------

export interface RepeatChoice {
  key: string;
  label: string;
  repeat: Repeat | null;
}

const base = (freq: Repeat['freq'], extra: Partial<Repeat> = {}): Repeat => ({ freq, interval: 1, days: [], week: null, until: null, count: null, ...extra });

/** Ripetizioni proposte per un appuntamento che inizia a `start`, come in Outlook. */
export function repeatChoices(start: number): RepeatChoice[] {
  const d = new Date(start);
  const wd = d.getDay();
  const week = Math.ceil(d.getDate() / 7);
  const list: RepeatChoice[] = [
    { key: 'mai', label: 'Non si ripete', repeat: null },
    { key: 'giorno', label: 'Ogni giorno', repeat: base('daily') },
    { key: 'feriali', label: 'Ogni giorno feriale (lun-ven)', repeat: base('weekly', { days: [1, 2, 3, 4, 5] }) },
    { key: 'settimana', label: '', repeat: base('weekly', { days: [wd] }) },
    { key: 'mese', label: '', repeat: base('monthly') },
  ];
  // Il quinto venerdì non c'è tutti i mesi: in quel caso solo "l'ultimo venerdì".
  if (week <= 4) list.push({ key: 'mese-n', label: '', repeat: base('monthly', { days: [wd], week }) });
  if (isLastWeekOfMonth(start)) list.push({ key: 'mese-ultimo', label: '', repeat: base('monthly', { days: [wd], week: -1 }) });
  list.push({ key: 'anno', label: '', repeat: base('yearly') });
  for (const c of list) if (c.repeat && !c.label) c.label = repeatText(c.repeat, start);
  return list;
}

/** Quale scelta corrisponde alla ripetizione (senza fine e numero), o "altra". */
export function repeatKey(r: Repeat | null, start: number): string {
  if (!r) return 'mai';
  const same = (a: Repeat, b: Repeat) => a.freq === b.freq && (a.interval || 1) === (b.interval || 1) && [...a.days].sort().join() === [...b.days].sort().join() && (a.week ?? null) === (b.week ?? null);
  const c = repeatChoices(start).find((x) => x.repeat && same(x.repeat, { ...r, days: r.freq === 'weekly' && !r.days.length ? [new Date(start).getDay()] : r.days }));
  return c?.key ?? 'altra';
}

// ---------- Ricerca e promemoria ----------

export function matchEvent(ev: CalEvent, q: string): boolean {
  const s = q.trim().toLowerCase();
  if (!s) return true;
  return [ev.title, ev.location, ev.notes, ev.organizer?.name ?? '', ev.organizer?.email ?? '', ...ev.attendees.map((a) => `${a.name} ${a.email}`)].some((t) => t.toLowerCase().includes(s));
}

/** Istante del promemoria di un'occorrenza, o null se non c'è. */
export function reminderAt(o: Occurrence): number | null {
  return o.ev.reminder === null || o.ev.reminder === undefined ? null : o.start - o.ev.reminder * MIN;
}

/** Occorrenze il cui promemoria scatta nell'intervallo (from, to]. */
export function dueReminders(occs: Occurrence[], from: number, to: number): Occurrence[] {
  return occs.filter((o) => {
    const t = reminderAt(o);
    return t !== null && t > from && t <= to && !o.ev.cancelled && o.ev.response !== 'declined';
  });
}

/** "Tra 15 minuti", "Adesso", "Iniziato da 5 minuti". */
export function untilText(start: number, now = Date.now()): string {
  const m = Math.round((start - now) / MIN);
  if (Math.abs(m) < 1) return 'Adesso';
  const len = (x: number) => (x < 60 ? `${x} ${x === 1 ? 'minuto' : 'minuti'}` : x < 1440 ? `${Math.round(x / 60)} ${Math.round(x / 60) === 1 ? 'ora' : 'ore'}` : `${Math.round(x / 1440)} ${Math.round(x / 1440) === 1 ? 'giorno' : 'giorni'}`);
  return m > 0 ? `Tra ${len(m)}` : `Iniziato da ${len(-m)}`;
}

// ---------- Campi data e ora ----------

export function dateInput(t: number): string {
  const d = new Date(t);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

export function timeInput(t: number): string {
  return timeText(t);
}

/** Da "2026-10-19" e "10:30" all'istante locale; null se non valido. */
export function fromInputs(date: string, time = '00:00'): number | null {
  const dm = date.match(/^(\d{4})-(\d{2})-(\d{2})$/);
  const tm = time.match(/^(\d{1,2}):(\d{2})$/);
  if (!dm || !tm) return null;
  const d = new Date(+dm[1], +dm[2] - 1, +dm[3], +tm[1], +tm[2]);
  return Number.isNaN(d.getTime()) ? null : d.getTime();
}

/** Prossima mezz'ora piena (per un appuntamento nuovo). */
export function nextHalfHour(now = Date.now()): number {
  const d = new Date(now);
  d.setSeconds(0, 0);
  d.setMinutes(d.getMinutes() < 30 ? 30 : 60);
  return d.getTime();
}

export function emptyEvent(start: number, end: number, allDay = false): CalEvent {
  return {
    id: '',
    uid: '',
    title: '',
    location: '',
    start,
    end,
    allDay,
    repeat: null,
    exceptions: [],
    recurrenceId: null,
    reminder: allDay ? 1080 : 15,
    notes: '',
    categories: [],
    organizer: null,
    attendees: [],
    response: null,
    accountId: null,
    sequence: 0,
    cancelled: false,
  };
}
