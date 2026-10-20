// Scorciatoie da tastiera di Outlook in versione italiana.
// Quelle che toccano un messaggio funzionano solo quando non si sta scrivendo
// in un campo di testo, per non interferire con la digitazione.

import { store } from './store.svelte';

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || target.isContentEditable;
}

export function handleShortcut(e: KeyboardEvent, ui: { focusSearch: () => void }): void {
  if (store.onboarding) return;
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  const typing = isTyping(e.target);
  const overlay = store.settings || store.backstage;

  // La webview ricarica la pagina con Ctrl+R e F5, e apre la ricerca con Ctrl+F:
  // in un programma di posta questi tasti hanno altri significati.
  if ((ctrl && (key === 'r' || key === 'f')) || e.key === 'F5') e.preventDefault();

  if (e.key === 'Escape') {
    if (store.settings) store.settings = false;
    else if (store.backstage) store.backstage = false;
    return;
  }
  if (overlay) return;

  if (ctrl && !e.shiftKey && key === 'n') {
    e.preventDefault();
    store.openCompose('new');
    return;
  }
  if (ctrl && key === 'e') {
    e.preventDefault();
    ui.focusSearch();
    return;
  }
  if (typing) return;

  if (ctrl && key === 'r') {
    store.openCompose(e.shiftKey ? 'replyAll' : 'reply');
  } else if (ctrl && key === 'f') {
    store.openCompose('forward');
  } else if (ctrl && key === 'q') {
    e.preventDefault();
    store.setRead(store.sel, true);
  } else if (ctrl && key === 'u') {
    e.preventDefault();
    store.setRead(store.sel, false);
  } else if (!ctrl && e.key === 'Delete') {
    store.moveMsg(store.sel, 'deleted');
  } else if (!ctrl && e.key === 'Backspace') {
    e.preventDefault();
    store.moveMsg(store.sel, 'archive');
  } else if (!ctrl && e.key === 'Insert') {
    store.toggleFlag(store.sel);
  } else if (!ctrl && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
    e.preventDefault();
    store.moveSelection(e.key === 'ArrowDown' ? 1 : -1);
  }
}
