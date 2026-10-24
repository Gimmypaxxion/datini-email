// Piccole funzioni di formattazione condivise dall'interfaccia.

import type { Person } from './types';

/** Iniziali per l'avatar: prima lettera delle prime due parole. */
export function initials(name: string): string {
  const words = String(name || '')
    .replace(/[^A-Za-zÀ-ÿ ]/g, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (!words.length) return '?';
  return ((words[0][0] || '') + (words.length > 1 ? words[1][0] : '')).toUpperCase();
}

/**
 * Tinta dell'avatar (1-5) scelta con un hash del nome, così la stessa persona
 * ha sempre lo stesso colore. Il moltiplicatore 37 distribuisce bene i nomi italiani.
 */
export function tintIndex(name: string): number {
  const s = String(name || '');
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 37 + s.charCodeAt(i)) % 9973;
  return (h % 5) + 1;
}

export function tint(name: string): { bg: string; fg: string } {
  const k = tintIndex(name);
  return { bg: `var(--av${k}-bg)`, fg: `var(--av${k}-fg)` };
}

const DAYS = ['dom', 'lun', 'mar', 'mer', 'gio', 'ven', 'sab'];
const pad = (n: number) => (n < 10 ? '0' : '') + n;

/** Ora corrente nei formati usati nell'elenco (08:42) e nell'intestazione (ven 16/10/2026 08:42). */
export function clock(d = new Date()): { hhmm: string; long: string; stamp: number } {
  const hhmm = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  const long = `${DAYS[d.getDay()]} ${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${hhmm}`;
  return { hhmm, long, stamp: d.getTime() };
}

/** "ven 16/10/2026 08:42", come nell'intestazione dei messaggi di Outlook. */
export function longDate(ms: number): string {
  return clock(new Date(ms)).long;
}

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** Inizio della settimana (lunedì). */
function startOfWeek(d: Date): number {
  const day = (d.getDay() + 6) % 7;
  return startOfDay(d) - day * 86_400_000;
}

/** "oggi alle 17:30", "domani alle 08:00", "lun 26/10 alle 08:00". */
export function whenText(ms: number, now = new Date()): string {
  const d = new Date(ms);
  const at = `alle ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  const day = startOfDay(d);
  const today = startOfDay(now);
  if (day === today) return `oggi ${at}`;
  if (day === new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime()) return `domani ${at}`;
  const year = d.getFullYear() === now.getFullYear() ? '' : `/${d.getFullYear()}`;
  return `${DAYS[d.getDay()]} ${pad(d.getDate())}/${pad(d.getMonth() + 1)}${year} ${at}`;
}

/** Ora nell'elenco: 08:42 oggi, "gio 17:20" in settimana, "mer 14/10" quest'anno, poi la data completa. */
export function listTime(ms: number, now = new Date()): string {
  const d = new Date(ms);
  const today = startOfDay(now);
  if (ms >= today) return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  if (ms >= today - 6 * 86_400_000) return `${DAYS[d.getDay()]} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  if (d.getFullYear() === now.getFullYear()) return `${DAYS[d.getDay()]} ${pad(d.getDate())}/${pad(d.getMonth() + 1)}`;
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
}

export const GROUP_ORDER = ['Oggi', 'Ieri', 'Questa settimana', 'Settimana scorsa', 'Questo mese', 'Mese scorso', 'Meno recenti'] as const;
export type GroupLabel = (typeof GROUP_ORDER)[number];

/** Gruppo dell'elenco per data, come in Outlook. */
export function groupLabel(ms: number, now = new Date()): GroupLabel {
  const today = startOfDay(now);
  if (ms >= today) return 'Oggi';
  if (ms >= today - 86_400_000) return 'Ieri';
  const week = startOfWeek(now);
  if (ms >= week) return 'Questa settimana';
  if (ms >= week - 7 * 86_400_000) return 'Settimana scorsa';
  const month = new Date(now.getFullYear(), now.getMonth(), 1).getTime();
  if (ms >= month) return 'Questo mese';
  const prevMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1).getTime();
  if (ms >= prevMonth) return 'Mese scorso';
  return 'Meno recenti';
}

/** Toglie i prefissi Re:, I:, R:, Fw:, Fwd: ripetuti dall'oggetto. */
export function cleanSubject(subject: string): string {
  return subject.replace(/^((re|i|r|fw|fwd|inoltro|risposta)\s*:\s*)+/i, '');
}

export function personLabel(p: Person): string {
  return p.name.trim() || p.email;
}

/** "Davide Orlandi <d.orlandi@studiorlandi.it>" o solo l'indirizzo. */
export function personFull(p: Person): string {
  if (p.name.trim() && p.email) return `${p.name.trim()} <${p.email}>`;
  return p.email || p.name;
}

export function peopleText(list: Person[]): string {
  return list.map(personLabel).join('; ');
}

const EMAIL = /^[^\s@<>()",;:]+@[^\s@<>()",;:]+\.[^\s@<>()",;:]+$/;

export function validEmail(s: string): boolean {
  return EMAIL.test(s.trim());
}

/** Da quello che si scrive nel campo A: "Nome <indirizzo>", "indirizzo" o solo un nome. */
export function parseRecipient(text: string): Person | null {
  const t = text.replace(/[,;]+$/, '').trim();
  if (!t) return null;
  const m = t.match(/^"?([^"<]*?)"?\s*<([^>]+)>$/);
  if (m) return { name: m[1].trim(), email: m[2].trim() };
  if (t.includes('@')) return { name: '', email: t };
  return { name: t, email: '' };
}

export function sameAddress(a: Person, b: Person): boolean {
  return !!a.email && a.email.toLowerCase() === b.email.toLowerCase();
}

/** Dimensione leggibile: 412 KB, 1,8 MB. */
export function fileSize(bytes: number): string {
  if (!bytes) return '';
  if (bytes < 1024) return `${bytes} byte`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1).replace('.', ',')} MB`;
}

export function randomKey(): string {
  const a = new Uint8Array(12);
  globalThis.crypto?.getRandomValues?.(a);
  let s = '';
  for (const b of a) s += (b + 256).toString(16).slice(1);
  return s || Math.random().toString(16).slice(2);
}

/** Divide il testo in parti, riconoscendo i collegamenti web e gli indirizzi email. */
export function linkify(text: string): { text: string; href?: string }[] {
  const re = /(https?:\/\/[^\s<>"]+[^\s<>".,;:!?)\]]|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})/g;
  const out: { text: string; href?: string }[] = [];
  let last = 0;
  for (const m of text.matchAll(re)) {
    const i = m.index ?? 0;
    if (i > last) out.push({ text: text.slice(last, i) });
    const v = m[0];
    out.push({ text: v, href: v.includes('://') ? v : `mailto:${v}` });
    last = i + v.length;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
}
