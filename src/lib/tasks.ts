// Attività: elenchi intelligenti, scadenze e promemoria in parole, ripetizioni.
// Logica pura, usata dal modulo Attività e dal motore di esempio.

import { GIORNI_BREVI, MESI, addDays, addMonths, dateInput, daysBetween, startOfDay, startOfWeek, timeText } from './calendar';
import type { Task, TaskRepeat } from './types';
import type { IconName } from './icons';

/** Elenchi intelligenti; gli elenchi dell'utente si indicano con il loro id. */
export type SmartList = 'giornata' | 'importanti' | 'pianificate' | 'attivita' | 'contrassegnati';
export type TaskView = SmartList | `elenco:${string}`;

export const SMART_LISTS: { id: SmartList; label: string; icon: IconName }[] = [
  { id: 'giornata', label: 'La mia giornata', icon: 'sun' },
  { id: 'importanti', label: 'Importanti', icon: 'star' },
  { id: 'pianificate', label: 'Pianificate', icon: 'calendar-due' },
  { id: 'attivita', label: 'Attività', icon: 'home' },
  { id: 'contrassegnati', label: 'Posta contrassegnata', icon: 'flag' },
];

export const REPEATS: [TaskRepeat, string][] = [
  ['daily', 'Ogni giorno'],
  ['weekdays', 'Nei giorni feriali'],
  ['weekly', 'Ogni settimana'],
  ['monthly', 'Ogni mese'],
  ['yearly', 'Ogni anno'],
];

export function repeatLabel(r: TaskRepeat | null): string {
  return REPEATS.find(([k]) => k === r)?.[1] ?? 'Non si ripete';
}

/** "2026-10-24": la chiave del giorno per La mia giornata. */
export function dayKey(t: number): string {
  return dateInput(t);
}

export function emptyTask(list = '', now = Date.now()): Task {
  return { id: '', list, title: '', notes: '', important: false, myDay: null, due: null, reminder: null, repeat: null, steps: [], done: false, doneAt: null, created: now, message: null };
}

/** L'attività appartiene alla vista (completate comprese). */
export function inView(t: Task, view: TaskView, now = Date.now()): boolean {
  switch (view) {
    case 'giornata':
      return t.myDay === dayKey(now);
    case 'importanti':
      return t.important;
    case 'pianificate':
      return t.due !== null || t.reminder !== null;
    case 'attivita':
      return t.list === '';
    case 'contrassegnati':
      return false;
    default:
      return t.list === view.slice('elenco:'.length);
  }
}

/** Da fare (le più nuove in alto) e completate (le ultime in alto). */
export function tasksIn(all: Task[], view: TaskView, now = Date.now()): { open: Task[]; done: Task[] } {
  const mine = all.filter((t) => inView(t, view, now));
  const open = mine.filter((t) => !t.done);
  if (view === 'pianificate') open.sort((a, b) => (a.due ?? a.reminder ?? 0) - (b.due ?? b.reminder ?? 0) || b.created - a.created);
  else open.sort((a, b) => b.created - a.created);
  const done = mine.filter((t) => t.done).sort((a, b) => (b.doneAt ?? 0) - (a.doneAt ?? 0));
  return { open, done };
}

/** Quante attività da fare ci sono in ogni vista (per i numeri accanto agli elenchi). */
export function openCount(all: Task[], view: TaskView, now = Date.now()): number {
  return all.reduce((n, t) => n + (!t.done && inView(t, view, now) ? 1 : 0), 0);
}

/** Gruppi di Pianificate: In ritardo, Oggi, Domani, Questa settimana, Più avanti. */
export function plannedGroups(open: Task[], now = Date.now()): { label: string; tasks: Task[] }[] {
  const today = startOfDay(now);
  const weekEnd = addDays(startOfWeek(now), 7);
  const groups: { label: string; tasks: Task[] }[] = [
    { label: 'In ritardo', tasks: [] },
    { label: 'Oggi', tasks: [] },
    { label: 'Domani', tasks: [] },
    { label: 'Questa settimana', tasks: [] },
    { label: 'Più avanti', tasks: [] },
  ];
  for (const t of open) {
    const when = t.due ?? startOfDay(t.reminder ?? now);
    const i = when < today ? 0 : when < addDays(today, 1) ? 1 : when < addDays(today, 2) ? 2 : when < weekEnd ? 3 : 4;
    groups[i].tasks.push(t);
  }
  return groups.filter((g) => g.tasks.length);
}

function shortDay(t: number, now: number): string {
  const d = new Date(t);
  const year = d.getFullYear() !== new Date(now).getFullYear() ? ` ${d.getFullYear()}` : '';
  return `${GIORNI_BREVI[d.getDay()]} ${d.getDate()} ${MESI[d.getMonth()].slice(0, 3)}${year}`;
}

/** "Oggi", "Domani", "Ieri", "ven 30 ott". */
export function dayWord(t: number, now = Date.now()): string {
  const n = daysBetween(now, t);
  if (n === 0) return 'Oggi';
  if (n === 1) return 'Domani';
  if (n === -1) return 'Ieri';
  return shortDay(t, now);
}

/** Scadenza in parole, con lo stato per il colore. */
export function dueInfo(due: number, now = Date.now()): { text: string; state: 'scaduta' | 'oggi' | 'futura' } {
  const n = daysBetween(now, due);
  if (n < 0) return { text: `Scaduta ${dayWord(due, now).toLowerCase() === 'ieri' ? 'ieri' : `il ${shortDay(due, now)}`}`, state: 'scaduta' };
  if (n === 0) return { text: 'Scade oggi', state: 'oggi' };
  if (n === 1) return { text: 'Scade domani', state: 'futura' };
  return { text: `Scade ${shortDay(due, now)}`, state: 'futura' };
}

export function reminderWord(t: number, now = Date.now()): string {
  return `${dayWord(t, now)}, ${timeText(t)}`;
}

/** Scelte rapide per la scadenza: oggi, domani, la settimana prossima (lunedì). */
export function dueChoices(now = Date.now()): { label: string; at: number; hint: string }[] {
  const today = startOfDay(now);
  const tomorrow = addDays(today, 1);
  const monday = addDays(startOfWeek(now), 7);
  return [
    { label: 'Oggi', at: today, hint: GIORNI_BREVI[new Date(today).getDay()] },
    { label: 'Domani', at: tomorrow, hint: GIORNI_BREVI[new Date(tomorrow).getDay()] },
    { label: 'Settimana prossima', at: monday, hint: shortDay(monday, now) },
  ];
}

/** Scelte rapide per il promemoria: fra qualche ora, domani alle 9, lunedì alle 9. */
export function reminderChoices(now = Date.now()): { label: string; at: number; hint: string }[] {
  const out: { label: string; at: number; hint: string }[] = [];
  const later = new Date(now);
  later.setMinutes(0, 0, 0);
  later.setHours(later.getHours() + 3);
  if (startOfDay(later.getTime()) === startOfDay(now)) out.push({ label: 'Più tardi oggi', at: later.getTime(), hint: timeText(later.getTime()) });
  const tomorrow = addDays(startOfDay(now), 1) + 9 * 3_600_000;
  out.push({ label: 'Domani', at: tomorrow, hint: `${GIORNI_BREVI[new Date(tomorrow).getDay()]}, 09:00` });
  const monday = addDays(startOfWeek(now), 7) + 9 * 3_600_000;
  out.push({ label: 'Settimana prossima', at: monday, hint: `${shortDay(monday, now)}, 09:00` });
  return out;
}

/** Prossima data di un'attività ripetuta, alla stessa ora (come nel motore). */
export function nextTaskDate(t: number, r: TaskRepeat): number {
  switch (r) {
    case 'daily':
      return addDays(t, 1);
    case 'weekdays': {
      let n = addDays(t, 1);
      while (new Date(n).getDay() === 0 || new Date(n).getDay() === 6) n = addDays(n, 1);
      return n;
    }
    case 'weekly':
      return addDays(t, 7);
    case 'monthly':
      return addMonths(t, 1);
    case 'yearly':
      return addMonths(t, 12);
  }
}

/** Promemoria scaduti fra `from` (escluso) e `to` (compreso), delle attività da fare. */
export function dueTaskReminders(all: Task[], from: number, to: number): Task[] {
  return all.filter((t) => !t.done && t.reminder !== null && t.reminder > from && t.reminder <= to);
}

/** "2 di 5" per i passaggi. */
export function stepsText(t: Task): string {
  return `${t.steps.filter((s) => s.done).length} di ${t.steps.length}`;
}
