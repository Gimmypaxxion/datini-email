// Rubrica: nome da mostrare, compleanno in italiano, gruppi per lettera e
// istruzioni per esportare la rubrica dagli altri programmi.

import type { Contact } from './types';
import { PHONE_KINDS } from './types';

/** Nome di un contatto come lo mostra la rubrica. */
export function contactName(c: Contact): string {
  return c.displayName.trim() || `${c.firstName} ${c.lastName}`.trim() || c.company.trim() || c.emails[0] || '';
}

/** Lettera sotto cui sta il contatto (per cognome, come Outlook). */
export function contactLetter(c: Contact): string {
  const k = (c.lastName.trim() || contactName(c)).normalize('NFD').replace(/[̀-ͯ]/g, '');
  const ch = k.charAt(0).toUpperCase();
  return /[A-Z]/.test(ch) ? ch : '#';
}

const MONTHS = ['gennaio', 'febbraio', 'marzo', 'aprile', 'maggio', 'giugno', 'luglio', 'agosto', 'settembre', 'ottobre', 'novembre', 'dicembre'];

/** "1980-05-14" → "14 maggio 1980"; "--05-14" → "14 maggio". */
export function birthdayText(b: string): string {
  const m = b.match(/^(\d{4}|-)-?(\d{2})-(\d{2})$/);
  if (!m) return b;
  const month = MONTHS[Number(m[2]) - 1];
  if (!month) return b;
  return `${Number(m[3])} ${month}${m[1] !== '-' ? ` ${m[1]}` : ''}`;
}

export function phoneLabel(kind: string): string {
  return PHONE_KINDS.find(([k]) => k === kind)?.[1] ?? 'Telefono';
}

/** Come esportare la rubrica dai programmi più diffusi. */
export const EXPORT_HELP: { app: string; how: string }[] = [
  { app: 'Outlook classico', how: 'File › Apri ed esporta › Importa/Esporta › Esporta in un file › Valori separati da virgola, poi scegli la cartella Contatti.' },
  { app: 'Nuovo Outlook e Outlook.com', how: 'Contatti › Gestisci contatti › Esporta contatti (file CSV).' },
  { app: 'Gmail', how: 'contacts.google.com › Esporta › vCard o CSV Google.' },
  { app: 'Apple Contatti (Mac, iCloud)', how: 'Seleziona i contatti, poi File › Esporta › Esporta vCard.' },
  { app: 'Thunderbird', how: 'Rubrica › Strumenti › Esporta, in vCard, CSV o LDIF.' },
];
