import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';
import { MockBackend, UNDO_MS } from '../src/lib/mockBackend';

async function fresh(stile: 'classico' | 'nuovo' = 'classico') {
  const s = new MailStore({ persist: false, backend: new MockBackend() });
  s.prefs = { ...PRESETS[stile] };
  s.onboarding = false;
  await s.init();
  return s;
}

const ids = (s: MailStore) => s.list.map((m) => m.id);
const bySubject = (s: MailStore, subject: string) => s.list.find((m) => m.subject === subject);

async function total(s: MailStore, folderId: number) {
  return (await s.backend.folders()).find((f) => f.id === folderId)!.total;
}

describe('elenco e cartelle', () => {
  it('parte dalla Posta in arrivo, scheda Prioritaria, primo messaggio selezionato e letto', async () => {
    const s = await fresh();
    expect(s.currentFolder?.role).toBe('inbox');
    expect(s.listTitle).toBe('Posta in arrivo');
    expect(ids(s)).toEqual([1, 2, 3, 4, 5, 6, 8, 7, 9, 10]);
    expect(s.sel).toBe(1);
    expect(s.selected?.unread).toBe(false);
    expect(s.detail?.subject).toBe('Preventivo uffici Bassano, revisione 3');
    expect(s.count(1, true)).toBe(5);
  });

  it('aprire un messaggio lo segna come letto, anche nel motore', async () => {
    const s = await fresh();
    await s.pickMsg(2);
    expect(s.selected?.id).toBe(2);
    expect(s.selected?.unread).toBe(false);
    expect(s.count(1, true)).toBe(4);
    const inBackend = (await s.backend.messages(1, 'prio')).find((m) => m.id === 2);
    expect(inBackend?.unread).toBe(false);
  });

  it('la scheda Altre mostra solo i suoi messaggi', async () => {
    const s = await fresh();
    await s.pickTab('altre');
    expect(ids(s)).toEqual([11, 12, 13]);
    expect(s.sel).toBe(11);
  });

  it('le immagini esterne restano bloccate finché non si scaricano', async () => {
    const s = await fresh();
    await s.pickTab('altre');
    expect(s.detail?.remoteImages).toBe(2);
    expect(s.detail?.html).not.toContain('traccia.arredoecontract');
    await s.showRemoteImages();
    expect(s.detail?.remoteAllowed).toBe(true);
    expect(s.detail?.html).toContain('traccia.arredoecontract');
  });

  it('la ricerca cerca in mittente, oggetto e testo, senza badare agli accenti', async () => {
    const s = await fresh();
    await s.search('webinar');
    expect(ids(s)).toEqual([13]);
    expect(s.listTitle).toBe('Risultati della ricerca');
    await s.search('boucle');
    expect(ids(s)).toEqual([3, 23]);
    await s.clearSearch();
    expect(s.list.length).toBe(10);
  });

  it('eliminare il messaggio selezionato lo sposta e seleziona il successivo', async () => {
    const s = await fresh();
    await s.deleteMsg(1);
    expect(ids(s)).not.toContain(1);
    expect(s.sel).toBe(2);
    const trash = s.roleFolder('valli', 'deleted')!;
    await s.pickFolder(trash.id, `acc-${trash.id}`);
    expect(ids(s)).toEqual([1]);
    expect(s.listTitle).toBe('Posta eliminata');
    // Dalla posta eliminata si cancella del tutto.
    await s.deleteMsg(1);
    expect(await total(s, trash.id)).toBe(0);
  });

  it('archivia e sposta in una cartella', async () => {
    const s = await fresh();
    await s.moveMsg(1, 'archive');
    await s.moveToFolder(2, 7);
    expect(ids(s)).not.toContain(1);
    expect(ids(s)).not.toContain(2);
    expect(await total(s, 4)).toBe(1);
    expect(await total(s, 7)).toBe(1);
  });

  it('le frecce spostano la selezione nell’elenco', async () => {
    const s = await fresh();
    await s.moveSelection(1);
    expect(s.sel).toBe(2);
    await s.moveSelection(-1);
    await s.moveSelection(-1);
    expect(s.sel).toBe(1);
  });

  it('contrassegno e letto/non letto si invertono', async () => {
    const s = await fresh();
    await s.toggleFlag(1);
    expect(bySubject(s, 'Preventivo uffici Bassano, revisione 3')?.flagged).toBe(true);
    await s.toggleRead(1);
    expect(s.selected?.unread).toBe(true);
    await s.setRead(1, true);
    expect(s.selected?.unread).toBe(false);
  });

  it('nella posta inviata l’elenco mostra i destinatari', async () => {
    const s = await fresh();
    await s.pickFolder(3, 'acc-3');
    expect(s.list[0].to[0].name).toBe('Davide Orlandi');
  });
});

describe('scrittura', () => {
  beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] }));
  afterEach(() => vi.useRealTimers());

  it('nella vista classica il nuovo messaggio si apre in finestra, la risposta nel riquadro', async () => {
    const s = await fresh('classico');
    await s.openCompose('new');
    expect(s.winOpen).toBe(true);
    await s.discard();
    await s.openCompose('reply');
    expect(s.paneCompose).toBe(true);
    expect(s.c?.to).toEqual([{ name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' }]);
    expect(s.c?.subject).toBe('Re: Preventivo uffici Bassano, revisione 3');
    expect(s.c?.body).toContain('Da: Davide Orlandi <d.orlandi@studiorlandi.it>');
    expect(s.c?.body).toContain('Oggetto: Preventivo uffici Bassano, revisione 3');
    expect(s.c?.inReplyTo).toBe('1@valliarredi.example');
    expect(s.c?.replyToMessage).toBe(1);
  });

  it('con l’opzione attiva anche risposte e inoltri si aprono in finestra', async () => {
    const s = await fresh('classico');
    s.setPref('replyWin', true);
    await s.openCompose('reply');
    expect(s.winOpen).toBe(true);
  });

  it('nella vista nuova tutto si apre nel riquadro di lettura', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('new');
    expect(s.paneCompose).toBe(true);
    expect(s.activeRibbon).toBe('messaggio');
  });

  it('Rispondi a tutti mette in Cc gli altri destinatari, senza me e senza il mittente', async () => {
    const s = await fresh();
    await s.openCompose('replyAll');
    expect(s.c?.cc.map((p) => p.name)).toEqual(['Marco Ventura']);
  });

  it('Inoltra porta con sé gli allegati e usa il prefisso I:', async () => {
    const s = await fresh();
    await s.openCompose('forward');
    expect(s.c?.subject).toBe('I: Preventivo uffici Bassano, revisione 3');
    expect(s.c?.attachments.map((a) => a.source)).toEqual([
      { messageId: 1, index: 0 },
      { messageId: 1, index: 1 },
    ]);
    expect(s.c?.to).toEqual([]);
  });

  it('i destinatari si aggiungono con punto e virgola e si tolgono con Backspace', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.recipientInput('to', 'Lucia Marchetti <lucia@marchettitessuti.it>;');
    expect(s.c?.to).toEqual([{ name: 'Lucia Marchetti', email: 'lucia@marchettitessuti.it' }]);
    expect(s.toInput).toBe('');
    s.recipientKey('to', { key: 'Backspace', preventDefault() {} } as unknown as KeyboardEvent);
    expect(s.c?.to).toEqual([]);
  });

  it('senza destinatari non invia e mostra l’errore', async () => {
    const s = await fresh();
    await s.openCompose('new');
    expect(await s.send()).toBe(false);
    expect(s.err).toBe('Aggiungi almeno un destinatario.');
    expect(s.c).not.toBeNull();
  });

  it('un indirizzo scritto male blocca l’invio e viene indicato', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.recipientInput('to', 'Mario Rossi');
    expect(await s.send()).toBe(false);
    expect(s.err).toContain('Mario Rossi');
    expect(s.c?.to[0].name).toBe('Mario Rossi');
  });

  it('l’invio va in uscita, con avviso e Annulla che riapre il messaggio', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.recipientInput('to', 'lucia@marchettitessuti.it');
    s.setSubject('Prova');
    expect(await s.send()).toBe(true);
    expect(s.c).toBeNull();
    expect(s.toast?.kind).toBe('sent');
    expect(s.toast?.outboxId).toBeTypeOf('number');
    await vi.advanceTimersByTimeAsync(10);
    expect(s.outbox.length).toBe(1);
    await s.undoSend();
    expect(s.c?.subject).toBe('Prova');
    expect(s.c?.to).toEqual([{ name: '', email: 'lucia@marchettitessuti.it' }]);
    expect(s.winOpen).toBe(true);
    await vi.advanceTimersByTimeAsync(UNDO_MS + 500);
    expect(await total(s, 3)).toBe(3);
  });

  it('dopo qualche secondo il messaggio parte e finisce in Posta inviata', async () => {
    const s = await fresh();
    await s.openCompose('reply');
    expect(await s.send()).toBe(true);
    await vi.advanceTimersByTimeAsync(UNDO_MS + 500);
    expect(s.toast).toBeNull();
    expect(await total(s, 3)).toBe(4);
    expect(s.outbox.length).toBe(0);
    // Il messaggio originale risulta risposto.
    expect(s.list.find((m) => m.id === 1)?.answered).toBe(true);
  });

  it('il messaggio si salva da solo in Bozze mentre si scrive', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.setSubject('Appunti per Bassano');
    expect(s.savedAt).toBe('');
    await vi.advanceTimersByTimeAsync(3000);
    expect(s.savedAt).not.toBe('');
    expect(await total(s, 2)).toBe(3);
  });

  it('chiudere la finestra salva in Bozze, e la bozza si riapre dalla cartella', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.setSubject('Da finire');
    await s.closeWin();
    expect(s.c).toBeNull();
    expect(s.toast?.kind).toBe('draft');
    await s.pickFolder(2, 'acc-2');
    const draft = bySubject(s, 'Da finire');
    expect(draft).toBeTruthy();
    await s.pickMsg(draft!.id);
    expect(s.c?.subject).toBe('Da finire');
    // Una bozza riaperta e salvata non si duplica.
    s.setSubject('Da finire oggi');
    await s.closeWin();
    expect(await total(s, 2)).toBe(3);
  });

  it('una finestra vuota si chiude senza creare bozze', async () => {
    const s = await fresh();
    await s.openCompose('new');
    await s.closeWin();
    expect(await total(s, 2)).toBe(2);
    expect(s.toast).toBeNull();
  });

  it('cambiare messaggio mentre si scrive nel riquadro salva la bozza', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('reply');
    await s.pickMsg(3);
    expect(s.c).toBeNull();
    expect(await total(s, 2)).toBe(3);
  });

  it('gli allegati scelti si aggiungono e si tolgono', async () => {
    const s = await fresh();
    await s.openCompose('new');
    await s.addFiles();
    expect(s.c?.attachments.map((a) => a.name)).toEqual(['Planimetria_aggiornata.pdf']);
    s.removeAttachment(0);
    expect(s.c?.attachments).toEqual([]);
  });
});

describe('account', () => {
  it('aggiunge un account e ne mostra le cartelle', async () => {
    const s = await fresh();
    const r = await s.addAccount({ name: 'Ufficio', email: 'ufficio@valliarredi.it', password: 'segreta' });
    expect(r.ok).toBe(true);
    expect(s.accounts.map((a) => a.email)).toContain('ufficio@valliarredi.it');
    const id = s.accounts.find((a) => a.email === 'ufficio@valliarredi.it')!.id;
    expect(s.foldersOf(id).map((f) => f.role)).toEqual(['inbox', 'drafts', 'sent', 'deleted', 'archive', 'junk']);
  });

  it('una password sbagliata dà un errore chiaro', async () => {
    const s = await fresh();
    const r = await s.addAccount({ name: '', email: 'ufficio@valliarredi.it', password: 'sbagliata' });
    expect(r).toEqual({ ok: false, kind: 'credenziali', message: 'Il server ha rifiutato nome utente o password.' });
  });

  it('se i server non si trovano chiede le impostazioni manuali', async () => {
    const s = await fresh();
    const r = await s.addAccount({ name: '', email: 'info@negozio.sconosciuto', password: 'x' });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.kind).toBe('manuale');
  });

  it('rimuovere un account toglie le sue cartelle', async () => {
    const s = await fresh();
    await s.removeAccount('ordini');
    expect(s.accounts.map((a) => a.id)).toEqual(['valli']);
    expect(s.folders.some((f) => f.accountId === 'ordini')).toBe(false);
  });
});

describe('preferenze', () => {
  it('i preset impostano barra, accesso e scrittura', async () => {
    const s = await fresh('classico');
    expect(s.presetName).toBe('classico');
    s.applyPreset('nuovo');
    expect(s.prefs).toEqual(PRESETS.nuovo);
    expect(s.presetName).toBe('nuovo');
  });

  it('una combinazione mista non corrisponde a nessun preset', async () => {
    const s = await fresh('classico');
    s.setPref('ribbon', 'compatta');
    expect(s.presetName).toBe('');
  });

  it('scrivere nel riquadro disattiva l’opzione delle risposte in finestra', async () => {
    const s = await fresh('classico');
    s.setPref('replyWin', true);
    s.setPref('compose', 'pannello');
    expect(s.prefs.replyWin).toBe(false);
  });

  it('il primo avvio applica la vista scelta', () => {
    const s = new MailStore({ persist: false, backend: new MockBackend() });
    expect(s.onboarding).toBe(true);
    s.obChoice = 'nuovo';
    s.finishOnboarding();
    expect(s.onboarding).toBe(false);
    expect(s.prefs).toEqual(PRESETS.nuovo);
  });

  it('il tema segue il sistema finché non se ne sceglie uno', async () => {
    const s = await fresh();
    s.systemDark = true;
    expect(s.theme).toBe('scuro');
    s.setTheme('chiaro');
    expect(s.theme).toBe('chiaro');
  });
});
