// Cartelle di ricerca: i tipi proposti come in Outlook (Nuova cartella di
// ricerca), le condizioni in parole e il confronto usato dal motore di esempio.
// Il motore vero fa lo stesso in SQL (src-tauri/src/db.rs).

import type { IconName } from './icons';
import type { MessageSummary, SearchCriteria, SearchFolder } from './types';

export type SearchFolderKind =
  | 'da-leggere'
  | 'contrassegnata'
  | 'persone'
  | 'importante'
  | 'allegati'
  | 'parole'
  | 'grandi'
  | 'categorizzata'
  | 'recente'
  | 'personalizzata';

/** Che cosa chiede ogni tipo, oltre al nome. */
export type SearchFolderNeed = 'persone' | 'parole' | 'dimensione' | 'categoria' | 'tutto';

export interface SearchFolderPreset {
  kind: SearchFolderKind;
  group: string;
  label: string;
  /** Nome proposto per la cartella. */
  name: string;
  icon: IconName;
  criteria: Partial<SearchCriteria>;
  need?: SearchFolderNeed;
}

export const SEARCH_FOLDER_PRESETS: SearchFolderPreset[] = [
  { kind: 'da-leggere', group: 'Lettura posta', label: 'Posta da leggere', name: 'Posta da leggere', icon: 'mail', criteria: { unread: true } },
  { kind: 'contrassegnata', group: 'Lettura posta', label: 'Posta contrassegnata per completamento', name: 'Posta contrassegnata', icon: 'flag', criteria: { flagged: true } },
  { kind: 'persone', group: 'Lettura posta', label: 'Posta da e verso persone specifiche', name: 'Posta da e verso', icon: 'users', criteria: {}, need: 'persone' },
  { kind: 'importante', group: 'Lettura posta', label: 'Posta importante', name: 'Posta importante', icon: 'exclamation-mark', criteria: { important: true } },
  { kind: 'recente', group: 'Lettura posta', label: 'Posta degli ultimi 7 giorni', name: 'Ultimi 7 giorni', icon: 'calendar', criteria: { days: 7 } },
  { kind: 'categorizzata', group: 'Organizzazione della posta', label: 'Posta categorizzata', name: 'Posta categorizzata', icon: 'tag', criteria: { categorized: true }, need: 'categoria' },
  { kind: 'allegati', group: 'Organizzazione della posta', label: 'Posta con allegati', name: 'Posta con allegati', icon: 'paperclip', criteria: { attachments: true } },
  { kind: 'parole', group: 'Organizzazione della posta', label: 'Posta con parole specifiche', name: 'Posta con parole', icon: 'search', criteria: {}, need: 'parole' },
  { kind: 'grandi', group: 'Organizzazione della posta', label: 'Posta di grandi dimensioni', name: 'Posta di grandi dimensioni', icon: 'database', criteria: { minKb: 1024 }, need: 'dimensione' },
  { kind: 'personalizzata', group: 'Personalizzata', label: 'Crea una cartella di ricerca personalizzata', name: 'Cartella di ricerca', icon: 'adjustments', criteria: {}, need: 'tutto' },
];

/** Dimensioni proposte per Posta di grandi dimensioni, in KB. */
export const SIZE_CHOICES: [number, string][] = [
  [100, 'oltre 100 KB'],
  [500, 'oltre 500 KB'],
  [1024, 'oltre 1 MB'],
  [5120, 'oltre 5 MB'],
  [10240, 'oltre 10 MB'],
];

/** Giorni proposti nella cartella personalizzata. */
export const DAY_CHOICES: [number, string][] = [
  [1, 'Oggi e ieri'],
  [7, 'Ultimi 7 giorni'],
  [30, 'Ultimi 30 giorni'],
  [90, 'Ultimi 3 mesi'],
  [365, 'Ultimo anno'],
];

export function emptyCriteria(): SearchCriteria {
  return { text: '', people: [], unread: false, flagged: false, important: false, attachments: false, categorized: false, category: null, minKb: null, days: null };
}

export function presetOf(kind: string): SearchFolderPreset {
  return SEARCH_FOLDER_PRESETS.find((p) => p.kind === kind) ?? SEARCH_FOLDER_PRESETS[SEARCH_FOLDER_PRESETS.length - 1];
}

/** Persone scritte separate da punto e virgola o virgola, come nei campi A e Cc. */
export function splitPeople(text: string): string[] {
  return text
    .split(/[;,\n]/)
    .map((s) => s.trim())
    .filter(Boolean);
}

/** Il numero accanto alla cartella: i non letti, oppure tutti (in grigio) per la posta contrassegnata. */
export function searchFolderCount(f: SearchFolder): { n: number; muted: boolean } {
  if (f.criteria.flagged && !f.criteria.unread) return { n: f.total, muted: true };
  return { n: f.unread, muted: false };
}

function sizeText(kb: number): string {
  return kb >= 1024 ? `${Math.round((kb / 1024) * 10) / 10} MB`.replace('.', ',') : `${kb} KB`;
}

/** Le condizioni in parole, per il suggerimento sulla cartella e la finestra. */
export function describeCriteria(c: SearchCriteria, categoryName: (id: string) => string | null = () => null): string {
  const out: string[] = [];
  if (c.unread) out.push('da leggere');
  if (c.flagged) out.push('contrassegnati');
  if (c.important) out.push('con priorità alta');
  if (c.attachments) out.push('con allegati');
  if (c.category) out.push(`nella categoria ${categoryName(c.category) ?? c.category}`);
  else if (c.categorized) out.push('con una categoria');
  if (c.people.length) out.push(`da o verso ${c.people.join(', ')}`);
  if (c.text.trim()) out.push(`che contengono “${c.text.trim()}”`);
  if (c.minKb) out.push(`più grandi di ${sizeText(c.minKb)}`);
  if (c.days) out.push(c.days === 1 ? 'di oggi e ieri' : `degli ultimi ${c.days} giorni`);
  if (!out.length) return 'Tutti i messaggi';
  const s = `Messaggi ${out.join(', ')}`;
  return s;
}

function norm(s: string): string {
  return s.toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');
}

/**
 * Il messaggio rispetta le condizioni? (Motore di esempio: il motore vero usa
 * SQL.) `text` è il testo del messaggio; le cartelle escluse le decide chi chiama.
 */
export function matchesCriteria(m: MessageSummary & { text?: string; cc?: { name: string; email: string }[] }, c: SearchCriteria, now = Date.now()): boolean {
  if (m.draft) return false;
  if (c.unread && !m.unread) return false;
  if (c.flagged && !m.flagged) return false;
  if (c.important && m.importance !== 1) return false;
  if (c.attachments && !m.hasAttachments) return false;
  if (c.minKb && m.size < c.minKb * 1024) return false;
  if (c.days && m.date < now - c.days * 86_400_000) return false;
  if (c.category ? !m.categories.includes(c.category) : c.categorized && !m.categories.length) return false;
  if (c.people.length) {
    const who = norm([m.from, ...m.to, ...(m.cc ?? [])].map((p) => `${p.name} ${p.email}`).join(' '));
    if (!c.people.some((p) => who.includes(norm(p.trim())))) return false;
  }
  const words = norm(c.text).split(/\s+/).filter(Boolean);
  if (words.length) {
    const hay = norm(`${m.from.name} ${m.from.email} ${m.to.map((p) => p.name).join(' ')} ${m.subject} ${m.text ?? m.preview}`);
    if (!words.every((w) => hay.includes(w))) return false;
  }
  return true;
}
