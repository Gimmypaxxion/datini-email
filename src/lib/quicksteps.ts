// Azioni rapide: elenco delle azioni possibili, controlli (gli stessi del
// motore), descrizione in parole e ordine di esecuzione.

import type { Category, QuickAction, QuickActionKind, QuickStep } from './types';
import type { IconName } from './icons';

export interface KindInfo {
  kind: QuickActionKind;
  label: string;
  icon: IconName;
  /** stato: cambia il messaggio; scrivi: apre un messaggio; posto: lo sposta. */
  phase: 'stato' | 'scrivi' | 'posto';
}

export const QUICK_KINDS: KindInfo[] = [
  { kind: 'letto', label: 'Segna come letto', icon: 'mail-opened', phase: 'stato' },
  { kind: 'da-leggere', label: 'Segna come da leggere', icon: 'mail', phase: 'stato' },
  { kind: 'contrassegna', label: 'Contrassegna', icon: 'flag', phase: 'stato' },
  { kind: 'togli-contrassegno', label: 'Togli il contrassegno', icon: 'flag-off', phase: 'stato' },
  { kind: 'categoria', label: 'Assegna la categoria', icon: 'tag', phase: 'stato' },
  { kind: 'togli-categorie', label: 'Togli le categorie', icon: 'eraser', phase: 'stato' },
  { kind: 'rispondi', label: 'Rispondi', icon: 'arrow-back-up', phase: 'scrivi' },
  { kind: 'rispondi-tutti', label: 'Rispondi a tutti', icon: 'arrow-back-up-double', phase: 'scrivi' },
  { kind: 'inoltra', label: 'Inoltra', icon: 'arrow-forward-up', phase: 'scrivi' },
  { kind: 'inoltra-allegato', label: 'Inoltra come allegato', icon: 'paperclip', phase: 'scrivi' },
  { kind: 'sposta', label: 'Sposta nella cartella', icon: 'folder-symlink', phase: 'posto' },
  { kind: 'archivia', label: 'Archivia', icon: 'archive', phase: 'posto' },
  { kind: 'elimina', label: 'Elimina', icon: 'trash', phase: 'posto' },
  { kind: 'ignora', label: 'Ignora la conversazione', icon: 'mail-off', phase: 'posto' },
];

export function kindInfo(k: QuickActionKind): KindInfo {
  return QUICK_KINDS.find((x) => x.kind === k) ?? QUICK_KINDS[0];
}

/** Icone che si possono scegliere per un'azione rapida. */
export const QUICK_ICONS: [IconName, string][] = [
  ['bolt', 'Fulmine'],
  ['circle-check', 'Fatto'],
  ['arrow-back-up', 'Risposta'],
  ['arrow-forward-up', 'Inoltro'],
  ['folder-symlink', 'Cartella'],
  ['archive', 'Archivio'],
  ['flag', 'Contrassegno'],
  ['tag', 'Categoria'],
  ['users', 'Persone'],
  ['star', 'Stella'],
  ['briefcase', 'Lavoro'],
  ['mail-opened', 'Busta aperta'],
];

/** Le azioni rapide di partenza (le stesse del motore). */
export function defaultQuickSteps(): QuickStep[] {
  return [
    { id: 'fatto', name: 'Fatto', icon: 'circle-check', shortcut: 1, actions: [{ kind: 'letto' }, { kind: 'archivia' }] },
    { id: 'rispondi-elimina', name: 'Rispondi ed elimina', icon: 'arrow-back-up', shortcut: 2, actions: [{ kind: 'rispondi' }, { kind: 'elimina' }] },
  ];
}

export function emptyQuickStep(): QuickStep {
  return { id: '', name: '', icon: 'bolt', shortcut: null, actions: [] };
}

/** Controlla l'elenco come il motore: null se va bene, altrimenti il problema. */
export function checkQuickSteps(list: QuickStep[]): string | null {
  if (list.length > 40) return 'Puoi avere al massimo 40 azioni rapide.';
  const seen = new Map<number, string>();
  for (const q of list) {
    const name = q.name.trim();
    if (!name) return 'Dai un nome a ogni azione rapida.';
    const err = checkQuickStep({ ...q, name });
    if (err) return err;
    if (q.shortcut && q.shortcut >= 1 && q.shortcut <= 9) {
      const other = seen.get(q.shortcut);
      if (other) return `La scorciatoia Ctrl+Maiusc+${q.shortcut} è già di «${other}».`;
      seen.set(q.shortcut, name);
    }
  }
  return null;
}

/** Controlli di una singola azione rapida (nome già ripulito). */
export function checkQuickStep(q: QuickStep): string | null {
  if (!q.name.trim()) return 'Dai un nome all’azione rapida.';
  if (!q.actions.length) return `L'azione rapida «${q.name}» non fa ancora niente: aggiungi almeno un'azione.`;
  const phases = q.actions.map((a) => kindInfo(a.kind).phase);
  if (phases.filter((p) => p === 'scrivi').length > 1) return `«${q.name}» può aprire un solo messaggio da scrivere: tieni una sola fra Rispondi, Rispondi a tutti e Inoltra.`;
  if (phases.filter((p) => p === 'posto').length > 1) return `«${q.name}» può spostare il messaggio in un solo posto: tieni una sola fra Sposta, Archivia, Elimina e Ignora.`;
  for (const a of q.actions) {
    if (a.kind === 'sposta' && (a.folder === null || a.folder === undefined)) return `Scegli la cartella in cui «${q.name}» deve spostare i messaggi.`;
    if (a.kind === 'categoria' && !a.category?.trim()) return `Scegli la categoria che «${q.name}» deve assegnare.`;
  }
  return null;
}

/** Ordine di esecuzione: prima lo stato, poi il messaggio da scrivere, infine lo spostamento. */
export function runOrder(actions: QuickAction[]): QuickAction[] {
  const rank = { stato: 0, scrivi: 1, posto: 2 } as const;
  return actions.map((a, i) => ({ a, i })).sort((x, y) => rank[kindInfo(x.a.kind).phase] - rank[kindInfo(y.a.kind).phase] || x.i - y.i).map((x) => x.a);
}

/** "Segna come letto, poi archivia". `folderName` dà il nome di una cartella (null se non c'è più). */
export function describeQuickStep(q: QuickStep, folderName: (id: number) => string | null, categories: Category[]): string {
  if (!q.actions.length) return 'Nessuna azione';
  const parts = q.actions.map((a) => {
    if (a.kind === 'sposta') {
      const f = a.folder === null || a.folder === undefined ? null : folderName(a.folder);
      return f ? `sposta in ${f}` : 'sposta in una cartella che non c’è più';
    }
    if (a.kind === 'categoria') {
      const c = categories.find((x) => x.id === a.category);
      return c ? `categoria ${c.name}` : 'assegna una categoria';
    }
    if ((a.kind === 'inoltra' || a.kind === 'inoltra-allegato') && a.to?.length) {
      return `${kindInfo(a.kind).label.toLowerCase()} a ${a.to.map((p) => p.name || p.email).join(', ')}`;
    }
    return kindInfo(a.kind).label.toLowerCase();
  });
  const text = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')}, poi ${parts[parts.length - 1]}`;
  return text.charAt(0).toUpperCase() + text.slice(1);
}
