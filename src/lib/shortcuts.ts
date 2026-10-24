// Scorciatoie da tastiera di Outlook in versione italiana.
// Quelle che toccano un messaggio funzionano solo quando non si sta scrivendo
// in un campo di testo, per non interferire con la digitazione.

import { store } from './store.svelte';

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target.isContentEditable;
}

export function handleShortcut(e: KeyboardEvent, ui: { focusSearch: () => void }): void {
  if (store.onboarding) return;
  const ctrl = e.ctrlKey || e.metaKey;
  const key = e.key.toLowerCase();
  const typing = isTyping(e.target);
  const overlay = store.settings || store.backstage || store.wizard.open || !!store.confirm || !!store.cal.edit;

  // La webview ricarica la pagina con Ctrl+R e F5, e apre la ricerca con Ctrl+F:
  // in un programma di posta questi tasti hanno altri significati.
  if ((ctrl && (key === 'r' || key === 'f')) || e.key === 'F5') e.preventDefault();

  if (e.key === 'Escape') {
    if (store.settings) store.settings = false;
    else if (store.backstage) store.backstage = false;
    return;
  }
  if (overlay) return;

  // Posta, Calendario, Contatti, come in Outlook.
  if (ctrl && !e.shiftKey && !e.altKey && (e.key === '1' || e.key === '2' || e.key === '3')) {
    e.preventDefault();
    store.setModule(e.key === '1' ? 'posta' : e.key === '2' ? 'calendario' : 'contatti');
    return;
  }
  if (ctrl && e.shiftKey && key === 'c') {
    e.preventDefault();
    store.newContact();
    return;
  }
  // Nuovo appuntamento e nuova riunione, da qualsiasi parte del programma.
  if (ctrl && e.shiftKey && (key === 'a' || key === 'q')) {
    e.preventDefault();
    store.cal.newEvent({ meeting: key === 'q' });
    return;
  }
  if (store.module === 'calendario') {
    const cal = store.cal;
    const views = ['giorno', 'settimana-lavorativa', 'settimana', 'mese'] as const;
    if (ctrl && e.altKey && /^[1-4]$/.test(e.key)) {
      e.preventDefault();
      cal.setView(views[Number(e.key) - 1]);
    } else if (ctrl && !e.shiftKey && key === 'n') {
      e.preventDefault();
      cal.newEvent();
    } else if (ctrl && key === 'e') {
      e.preventDefault();
      ui.focusSearch();
    } else if (!typing && !ctrl && (e.key === 'ArrowLeft' || e.key === 'ArrowRight') && !(e.target instanceof HTMLButtonElement && e.target.closest('.mini-g'))) {
      e.preventDefault();
      cal.move(e.key === 'ArrowLeft' ? -1 : 1);
    } else if (!typing && !ctrl && e.key === 'Delete' && cal.selected) {
      e.preventDefault();
      void cal.remove(cal.selected);
    } else if (!typing && !ctrl && e.key === 'Enter' && cal.selected && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      void cal.open(cal.selected);
    } else if (ctrl && e.key === 'p') {
      e.preventDefault();
    }
    return;
  }
  if (store.module === 'contatti') {
    if (ctrl && !e.shiftKey && key === 'n') {
      e.preventDefault();
      store.newContact();
    } else if (ctrl && key === 'e') {
      e.preventDefault();
      ui.focusSearch();
    } else if (!typing && e.key === 'Delete' && store.contactSel && !store.contactEdit) {
      void store.deleteContacts([store.contactSel]);
    }
    return;
  }

  if (e.key === 'F9') {
    e.preventDefault();
    store.syncNow();
    return;
  }
  if (ctrl && !e.shiftKey && key === 'n') {
    e.preventDefault();
    void store.openCompose('new');
    return;
  }
  if (ctrl && e.shiftKey && key === 'e') {
    // Nuova cartella, dentro quella aperta.
    e.preventDefault();
    const f = store.currentFolder;
    if (f) store.startNewFolder(f.accountId, f.id);
    return;
  }
  if (ctrl && !e.shiftKey && key === 'p') {
    e.preventDefault();
    void store.printMsg();
    return;
  }
  if (ctrl && e.altKey && e.key === ' ') {
    e.preventDefault();
    void store.toggleSpeech();
    return;
  }
  if (ctrl && key === 'e') {
    e.preventDefault();
    ui.focusSearch();
    return;
  }
  if (typing) return;

  if (ctrl && key === 'r') {
    void store.openCompose(e.shiftKey ? 'replyAll' : 'reply');
  } else if (ctrl && key === 'f') {
    void store.openCompose('forward');
  } else if (ctrl && e.shiftKey && key === 'v') {
    e.preventDefault();
    document.querySelector<HTMLButtonElement>('.rib button[aria-label="Sposta in"]')?.click();
  } else if (ctrl && key === 'q') {
    e.preventDefault();
    void store.setRead(store.sel, true);
  } else if (ctrl && key === 'u') {
    e.preventDefault();
    void store.setRead(store.sel, false);
  } else if (!ctrl && e.key === 'Delete') {
    void store.deleteMsg(store.sel);
  } else if (!ctrl && e.key === 'Backspace') {
    e.preventDefault();
    void store.moveMsg(store.sel, 'archive');
  } else if (!ctrl && e.key === 'Insert') {
    void store.toggleFlag(store.sel);
  } else if (!ctrl && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
    e.preventDefault();
    void store.moveSelection(e.key === 'ArrowDown' ? 1 : -1);
  }
}
