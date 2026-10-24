// Regole sui messaggi in arrivo: descrizione in italiano per l'elenco e
// controllo delle condizioni (lo stesso del motore in src-tauri/src/rules.rs,
// usato qui dal motore di esempio).

import type { Category, Folder, MessageSummary, Rule, RuleConditions } from './types';

export function emptyRule(): Rule {
  return {
    id: '',
    name: '',
    enabled: true,
    accountId: null,
    when: { from: [], to: [], subject: [], words: [], hasAttachments: false },
    then: { moveTo: null, delete: false, markRead: false, flag: false, category: null, alert: false, stop: true },
  };
}

/** "fatture; amministrazione, ordini" → ["fatture", "amministrazione", "ordini"] */
export function splitWords(text: string): string[] {
  return text
    .split(/[;,\n]/)
    .map((w) => w.trim())
    .filter(Boolean);
}

export function joinWords(list: string[]): string {
  return list.join('; ');
}

export function hasConditions(w: RuleConditions): boolean {
  return !!(w.from.length || w.to.length || w.subject.length || w.words.length || w.hasAttachments);
}

export function hasActions(r: Rule): boolean {
  const a = r.then;
  return a.moveTo !== null || a.delete || a.markRead || a.flag || !!a.category || a.alert;
}

const quoted = (list: string[]) => list.map((w) => `“${w}”`).join(' o ');

/** "Se il mittente contiene “bassi” e l'oggetto contiene “fattura”" */
export function describeConditions(w: RuleConditions): string {
  const parts: string[] = [];
  if (w.from.length) parts.push(`il mittente contiene ${quoted(w.from)}`);
  if (w.to.length) parts.push(`i destinatari contengono ${quoted(w.to)}`);
  if (w.subject.length) parts.push(`l’oggetto contiene ${quoted(w.subject)}`);
  if (w.words.length) parts.push(`oggetto o testo contengono ${quoted(w.words)}`);
  if (w.hasAttachments) parts.push('ci sono allegati');
  if (!parts.length) return 'Per tutti i messaggi in arrivo';
  const text = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')} e ${parts[parts.length - 1]}`;
  return `Se ${text}`;
}

export function describeActions(r: Rule, folders: Folder[], categories: Category[]): string {
  const a = r.then;
  const parts: string[] = [];
  if (a.delete) parts.push('elimina');
  else if (a.moveTo !== null) parts.push(`sposta in “${folders.find((f) => f.id === a.moveTo)?.name ?? 'cartella non più presente'}”`);
  if (a.markRead) parts.push('segna come letto');
  if (a.flag) parts.push('contrassegna');
  if (a.category) parts.push(`categoria “${categories.find((c) => c.id === a.category)?.name ?? a.category}”`);
  if (a.alert) parts.push('mostra un avviso');
  if (a.stop) parts.push('poi non applicare altre regole');
  return parts.join(', ');
}

const has = (hay: string, list: string[]) => !list.length || list.some((w) => hay.includes(w.toLowerCase()));

/** Le condizioni valgono per il messaggio? (testo: anteprima o testo completo) */
export function ruleMatches(r: Rule, m: MessageSummary, text = m.preview): boolean {
  if (!r.enabled || (r.accountId && r.accountId !== m.accountId)) return false;
  const w = r.when;
  if (w.hasAttachments && !m.hasAttachments) return false;
  if (!has(`${m.from.name} ${m.from.email}`.toLowerCase(), w.from)) return false;
  if (w.to.length && !m.to.some((p) => has(`${p.name} ${p.email}`.toLowerCase(), w.to))) return false;
  if (!has(m.subject.toLowerCase(), w.subject)) return false;
  return has(`${m.subject}\n${text}`.toLowerCase(), w.words);
}
