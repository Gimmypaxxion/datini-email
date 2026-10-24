// Voci dei menu a comparsa usate in più punti: Sposta in, Categorizza e il
// menu del clic destro su un messaggio (barra dei comandi, elenco, lettura).

import type { IconName } from './icons';
import { store, folderLabel } from './store.svelte';
import type { MessageSummary } from './types';

export interface MenuItem {
  label: string;
  icon?: IconName;
  disabled?: boolean;
  /** Rientro per le sottocartelle. */
  depth?: number;
  onclick?: () => void;
  kind?: 'item' | 'sep' | 'head';
  /** Voce con segno di spunta (per esempio una categoria assegnata). */
  checked?: boolean;
  /** Quadratino colorato al posto dell'icona (colore CSS, per le categorie). */
  swatch?: string;
  /** Scorciatoia da mostrare a destra. */
  keys?: string;
}

/** Cartelle in cui spostare il messaggio: prima quelle del suo account, poi le altre. */
export function moveItems(m: MessageSummary): MenuItem[] {
  const folders = store.foldersOf(m.accountId).filter((f) => f.id !== m.folderId && f.selectable);
  const items: MenuItem[] = folders.map((f) => ({
    label: folderLabel(f),
    icon: f.role === 'archive' ? 'archive' : f.role === 'deleted' ? 'trash' : f.role === 'junk' ? 'ban' : f.role === 'inbox' ? 'inbox' : 'folder',
    depth: f.role ? 0 : Math.max(0, f.depth - (/^inbox[./]/i.test(f.path) ? 1 : 0)),
    onclick: () => store.moveToFolder(m.id, f.id),
  }));
  for (const a of store.accounts.filter((x) => x.id !== m.accountId)) {
    const fs = store.foldersOf(a.id).filter((f) => f.selectable && f.role !== 'drafts' && f.role !== 'sent');
    if (!fs.length) continue;
    items.push({ kind: 'head', label: a.kind === 'local' ? a.name : a.email });
    for (const f of fs) items.push({ label: folderLabel(f), icon: 'folder', onclick: () => store.moveToFolder(m.id, f.id) });
  }
  return items;
}

/** Categorizza: le categorie con la spunta su quelle assegnate. */
export function categoryItems(m: MessageSummary): MenuItem[] {
  const items: MenuItem[] = store.categories.map((c) => ({
    label: c.name,
    swatch: `var(--cat-${c.color})`,
    checked: m.categories.includes(c.id),
    onclick: () => store.toggleCategory(m.id, c.id),
  }));
  if (items.length) items.push({ kind: 'sep', label: '' });
  items.push({ label: 'Cancella tutte le categorie', icon: 'eraser', disabled: !m.categories.length, onclick: () => store.clearMsgCategories(m.id) });
  items.push({ label: 'Tutte le categorie…', icon: 'tags', onclick: () => store.openSettings('categorie') });
  return items;
}

/**
 * Menu del clic destro su un messaggio. `sub` apre un secondo menu nello
 * stesso punto (Categorizza, Sposta in).
 */
export function messageItems(m: MessageSummary, sub: (kind: 'categorie' | 'sposta') => void): MenuItem[] {
  const reply = (kind: 'reply' | 'replyAll' | 'forward') => async () => {
    if (store.sel !== m.id) await store.pickMsg(m.id, false);
    await store.openCompose(kind);
  };
  const draft = m.draft && store.currentFolder?.role === 'drafts';
  return [
    { label: 'Rispondi', icon: 'arrow-back-up', keys: 'Ctrl+R', disabled: draft, onclick: reply('reply') },
    { label: 'Rispondi a tutti', icon: 'arrow-back-up-double', keys: 'Ctrl+Maiusc+R', disabled: draft, onclick: reply('replyAll') },
    { label: 'Inoltra', icon: 'arrow-forward-up', keys: 'Ctrl+F', disabled: draft, onclick: reply('forward') },
    { kind: 'sep', label: '' },
    m.unread
      ? { label: 'Segna come letto', icon: 'mail-opened', keys: 'Ctrl+Q', onclick: () => store.setRead(m.id, true) }
      : { label: 'Segna come da leggere', icon: 'mail', keys: 'Ctrl+U', onclick: () => store.setRead(m.id, false) },
    { label: m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna', icon: 'flag', keys: 'Ins', onclick: () => store.toggleFlag(m.id) },
    { label: 'Categorizza…', icon: 'tag', onclick: () => sub('categorie') },
    { kind: 'sep', label: '' },
    { label: 'Sposta in…', icon: 'folder-symlink', keys: 'Ctrl+Maiusc+V', onclick: () => sub('sposta') },
    { label: 'Crea regola…', icon: 'list-check', disabled: draft, onclick: () => store.newRuleFrom(m.id) },
    { label: 'Archivia', icon: 'archive', keys: 'Backspace', onclick: () => store.moveMsg(m.id, 'archive') },
    { label: 'Elimina', icon: 'trash', keys: 'Canc', onclick: () => store.deleteMsg(m.id) },
    { label: 'Stampa', icon: 'printer', keys: 'Ctrl+P', onclick: () => store.printMsg(m.id) },
    { label: 'Segnala come posta indesiderata', icon: 'ban', onclick: () => store.moveMsg(m.id, 'junk') },
  ];
}
