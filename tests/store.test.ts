import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';

function fresh(stile: 'classico' | 'nuovo' = 'classico') {
  const s = new MailStore({ persist: false });
  s.prefs = { ...PRESETS[stile] };
  s.onboarding = false;
  return s;
}

describe('elenco e cartelle', () => {
  it('parte dalla Posta in arrivo, scheda Prioritaria, primo messaggio selezionato e letto', () => {
    const s = fresh();
    expect(s.folder).toBe('inbox');
    expect(s.list.map((m) => m.id)).toEqual(['m1', 'm2', 'm3', 'm4', 'm5', 'm6', 'm7', 'm8', 'm9', 'm10']);
    expect(s.selected?.id).toBe('m1');
    expect(s.selected?.unread).toBe(false);
    expect(s.count('inbox', true)).toBe(5);
  });

  it('aprire un messaggio lo segna come letto', () => {
    const s = fresh();
    s.pickMsg('m2');
    expect(s.selected?.id).toBe('m2');
    expect(s.find('m2')?.unread).toBe(false);
    expect(s.count('inbox', true)).toBe(4);
  });

  it('la scheda Altre mostra solo i suoi messaggi', () => {
    const s = fresh();
    s.pickTab('altre');
    expect(s.list.map((m) => m.id)).toEqual(['a1', 'a2', 'a3']);
    expect(s.sel).toBe('a1');
  });

  it('la ricerca cerca in mittente, oggetto e anteprima in tutta la cartella', () => {
    const s = fresh();
    s.search('webinar');
    expect(s.list.map((m) => m.id)).toEqual(['a3']);
    expect(s.listTitle).toBe('Risultati della ricerca');
    s.clearSearch();
    expect(s.list.length).toBe(10);
  });

  it('eliminare il messaggio selezionato lo sposta e seleziona il successivo', () => {
    const s = fresh();
    s.moveMsg('m1', 'deleted');
    expect(s.list.some((m) => m.id === 'm1')).toBe(false);
    expect(s.sel).toBe('m2');
    s.pickFolder('deleted', 'acc-deleted');
    expect(s.list.map((m) => m.id)).toEqual(['m1']);
  });

  it('le frecce spostano la selezione nell’elenco', () => {
    const s = fresh();
    s.moveSelection(1);
    expect(s.sel).toBe('m2');
    s.moveSelection(-1);
    s.moveSelection(-1);
    expect(s.sel).toBe('m1');
  });

  it('contrassegno e letto/non letto si invertono', () => {
    const s = fresh();
    s.toggleFlag('m1');
    expect(s.find('m1')?.flagged).toBe(true);
    s.toggleRead('m1');
    expect(s.find('m1')?.unread).toBe(true);
    s.setRead('m1', true);
    expect(s.find('m1')?.unread).toBe(false);
  });
});

describe('scrittura', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('nella vista classica il nuovo messaggio si apre in finestra, la risposta nel riquadro', () => {
    const s = fresh('classico');
    s.openCompose('new');
    expect(s.winOpen).toBe(true);
    s.discard();
    s.openCompose('reply');
    expect(s.paneCompose).toBe(true);
    expect(s.c?.to).toEqual(['Davide Orlandi']);
    expect(s.c?.subj).toBe('Re: Preventivo uffici Bassano, revisione 3');
    expect(s.c?.body).toContain('Da: Davide Orlandi <d.orlandi@studiorlandi.it>');
  });

  it('con l’opzione attiva anche risposte e inoltri si aprono in finestra', () => {
    const s = fresh('classico');
    s.setPref('replyWin', true);
    s.openCompose('reply');
    expect(s.winOpen).toBe(true);
  });

  it('nella vista nuova tutto si apre nel riquadro di lettura', () => {
    const s = fresh('nuovo');
    s.openCompose('new');
    expect(s.paneCompose).toBe(true);
    expect(s.activeRibbon).toBe('messaggio');
  });

  it('Rispondi a tutti mette in Cc gli altri destinatari, senza me e senza il mittente', () => {
    const s = fresh();
    s.openCompose('replyAll');
    expect(s.c?.cc).toEqual(['Marco Ventura']);
  });

  it('Inoltra porta con sé gli allegati e usa il prefisso I:', () => {
    const s = fresh();
    s.openCompose('forward');
    expect(s.c?.subj).toBe('I: Preventivo uffici Bassano, revisione 3');
    expect(s.c?.atts.length).toBe(2);
    expect(s.c?.to).toEqual([]);
  });

  it('i destinatari si aggiungono con punto e virgola e si tolgono con Backspace', () => {
    const s = fresh();
    s.openCompose('new');
    s.recipientInput('to', 'Lucia Marchetti;');
    expect(s.c?.to).toEqual(['Lucia Marchetti']);
    expect(s.toInput).toBe('');
    s.recipientKey('to', { key: 'Backspace', preventDefault() {} } as unknown as KeyboardEvent);
    expect(s.c?.to).toEqual([]);
  });

  it('senza destinatari non invia e mostra l’errore', () => {
    const s = fresh();
    s.openCompose('new');
    s.send();
    expect(s.err).toBe('Aggiungi almeno un destinatario.');
    expect(s.c).not.toBeNull();
  });

  it('l’invio è immediato, con avviso e Annulla che riapre il messaggio', () => {
    const s = fresh();
    s.openCompose('new');
    s.recipientInput('to', 'Lucia Marchetti');
    s.setSubject('Prova');
    s.send();
    expect(s.c).toBeNull();
    expect(s.toast?.kind).toBe('sent');
    expect(s.count('sent', false)).toBe(4);
    s.undoSend();
    expect(s.c?.subj).toBe('Prova');
    expect(s.c?.to).toEqual(['Lucia Marchetti']);
    expect(s.winOpen).toBe(true);
    expect(s.count('sent', false)).toBe(3);
  });

  it('l’avviso di invio sparisce da solo dopo qualche secondo', () => {
    const s = fresh();
    s.openCompose('new');
    s.recipientInput('to', 'Lucia Marchetti;');
    s.send();
    vi.advanceTimersByTime(6100);
    expect(s.toast).toBeNull();
  });

  it('chiudere la finestra salva in Bozze, e la bozza si riapre dalla cartella', () => {
    const s = fresh();
    s.openCompose('new');
    s.setSubject('Da finire');
    s.closeWin();
    expect(s.c).toBeNull();
    expect(s.toast?.kind).toBe('draft');
    s.pickFolder('drafts', 'acc-drafts');
    const draft = s.list.find((m) => m.subject === 'Da finire');
    expect(draft).toBeTruthy();
    s.pickMsg(draft!.id);
    expect(s.c?.subj).toBe('Da finire');
    expect(s.list.some((m) => m.subject === 'Da finire')).toBe(false);
  });

  it('una finestra vuota si chiude senza creare bozze', () => {
    const s = fresh();
    s.openCompose('new');
    s.closeWin();
    expect(s.count('drafts', false)).toBe(2);
    expect(s.toast).toBeNull();
  });

  it('cambiare messaggio mentre si scrive nel riquadro salva la bozza', () => {
    const s = fresh('nuovo');
    s.openCompose('reply');
    s.pickMsg('m3');
    expect(s.c).toBeNull();
    expect(s.count('drafts', false)).toBe(3);
  });
});

describe('preferenze', () => {
  it('i preset impostano barra, accesso e scrittura', () => {
    const s = fresh('classico');
    expect(s.presetName).toBe('classico');
    s.applyPreset('nuovo');
    expect(s.prefs).toEqual(PRESETS.nuovo);
    expect(s.presetName).toBe('nuovo');
  });

  it('una combinazione mista non corrisponde a nessun preset', () => {
    const s = fresh('classico');
    s.setPref('ribbon', 'compatta');
    expect(s.presetName).toBe('');
  });

  it('scrivere nel riquadro disattiva l’opzione delle risposte in finestra', () => {
    const s = fresh('classico');
    s.setPref('replyWin', true);
    s.setPref('compose', 'pannello');
    expect(s.prefs.replyWin).toBe(false);
  });

  it('il primo avvio applica la vista scelta', () => {
    const s = new MailStore({ persist: false });
    expect(s.onboarding).toBe(true);
    s.obChoice = 'nuovo';
    s.finishOnboarding();
    expect(s.onboarding).toBe(false);
    expect(s.prefs).toEqual(PRESETS.nuovo);
  });

  it('il tema segue il sistema finché non se ne sceglie uno', () => {
    const s = fresh();
    s.systemDark = true;
    expect(s.theme).toBe('scuro');
    s.setTheme('chiaro');
    expect(s.theme).toBe('chiaro');
  });
});
