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

  it('senza oggetto chiede conferma prima di inviare', async () => {
    const s = await fresh();
    await s.openCompose('new');
    s.recipientInput('to', 'lucia@marchettitessuti.it');
    expect(await s.send()).toBe(false);
    expect(s.askNoSubject).toBe(true);
    expect(await s.send(true)).toBe(true);
    expect(s.c).toBeNull();
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

describe('formato del messaggio', () => {
  it('in testo normale chiede conferma se si perde la formattazione, poi tiene solo il testo', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('reply');
    s.c!.html = s.c!.html!.replace('<div><br></div>', '<div><b>Va bene</b></div>');
    s.setFormat('testo');
    expect(s.askPlain).toBe(true);
    expect(s.c?.plain).toBeFalsy();
    s.setFormat('testo', true);
    expect(s.askPlain).toBe(false);
    expect(s.c?.plain).toBe(true);
    expect(s.c?.html).toBeNull();
    expect(s.c?.body.startsWith('Va bene')).toBe(true);
    expect(s.c?.body).toContain('________________________________\nDa: Davide Orlandi');
    expect(s.c?.body.indexOf('Ufficio acquisti')).toBeLessThan(s.c!.body.indexOf('_____'));
  });

  it('senza formattazione passa subito al testo normale e torna in HTML', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('new');
    // La firma di esempio è grigia: con l'altro account (senza firma) non c'è formattazione.
    s.setAccount('ordini');
    s.setFormat('testo');
    expect(s.askPlain).toBe(false);
    expect(s.c?.plain).toBe(true);
    expect(s.c?.body).toBe('');
    s.setBody('Ciao');
    expect(s.c?.html).toBeNull();
    s.setFormat('html');
    expect(s.c?.plain).toBe(false);
    expect(s.c?.html).toContain('<div>Ciao</div>');
  });

  it('in testo normale la firma scelta va sopra il messaggio citato', async () => {
    const s = await fresh('nuovo');
    const breve = await s.saveSignature({ id: '', name: 'Breve', html: '<div><b>C. Valli</b></div>' });
    await s.openCompose('reply');
    s.setFormat('testo', true);
    s.insertSignature(breve!.id);
    const b = s.c!.body;
    expect(b).toContain('C. Valli');
    expect(b.indexOf('C. Valli')).toBeLessThan(b.indexOf('_____'));
  });
});

describe('archivi', () => {
  it('aprire un PST mostra il file di dati, ne apre la posta e torna senza avanzamento', async () => {
    const s = await fresh();
    const seen: Array<{ done: number; total: number } | null> = [];
    const stop = s.backend.onEvent((e) => {
      if (e.type === 'import') seen.push({ done: e.done, total: e.total });
    });
    s.backstage = true;
    await s.importArchive('pst');
    stop();
    expect(seen).toEqual([{ done: 0, total: 3 }, { done: 3, total: 3 }]);
    const a = s.accounts.find((x) => x.kind === 'local');
    expect(a?.name).toBe('Archivio 2019');
    expect(s.currentFolder?.accountId).toBe(a?.id);
    expect(s.list.length).toBe(3);
    expect(s.backstage).toBe(false);
    expect(s.importing).toBeNull();
    expect(s.toast?.text).toBe('File di dati aperto: Archivio 2019.');
  });

  it('dice quanti messaggi e allegati non si sono potuti leggere', async () => {
    const cases: Array<[number, number, string]> = [
      [4, 0, 'Non si sono potuti leggere 4 messaggi.'],
      [0, 1, 'Non si è potuto leggere un allegato.'],
      [1, 36, 'Non si sono potuti leggere un messaggio e 36 allegati.'],
    ];
    for (const [skipped, lost, text] of cases) {
      const backend = new MockBackend();
      backend.importSkipped = skipped;
      backend.importLostAttachments = lost;
      const s = new MailStore({ persist: false, backend });
      s.onboarding = false;
      await s.init();
      await s.importArchive('pst');
      expect(s.toast?.text).toBe(`File di dati aperto: Archivio 2019. ${text}`);
    }
  });
});

describe('testo formattato e firma', () => {
  it('trova, cambia e toglie il blocco della firma senza toccare la citazione', async () => {
    const { replaceSignature, findBlock, htmlToText, unwrapBody, newMessageHtml } = await import('../src/lib/compose-html');
    const html = `${newMessageHtml('<div>Chiara</div><div><div>Valli</div></div>')}<div id="better-look-citazione"><div>Originale</div></div>`;
    expect(findBlock(html, 'better-look-firma')).not.toBeNull();
    const swapped = replaceSignature(html, '<div>Marco</div>');
    expect(swapped).toContain('<div id="better-look-firma"><div>Marco</div></div>');
    expect(swapped).toContain('<div id="better-look-citazione"><div>Originale</div></div>');
    expect(swapped).not.toContain('Chiara');
    const removed = replaceSignature(html, null);
    expect(removed).not.toContain('better-look-firma');
    expect(removed).toContain('Originale');
    const added = replaceSignature('<div>Testo</div><div id="better-look-citazione">X</div>', '<b>Firma</b>');
    expect(added.indexOf('Firma')).toBeLessThan(added.indexOf('better-look-citazione'));
    // La firma dentro il messaggio citato (di chi ci ha scritto con Better-Look) non si tocca.
    const quoted = '<div>Ciao</div><div id="better-look-citazione"><div>Sotto</div><div id="better-look-firma"><div>Davide</div></div></div>';
    const mine = replaceSignature(quoted, '<div>Chiara</div>');
    expect(mine).toContain('<div id="better-look-firma"><div>Davide</div></div>');
    expect(mine.indexOf('Chiara')).toBeLessThan(mine.indexOf('better-look-citazione'));
    expect(replaceSignature(quoted, null)).toContain('Davide');
    // Le bozze salvate con la versione precedente usavano una classe.
    expect(replaceSignature('<div class="better-look-firma"><div>Vecchia</div></div>', '<div>Nuova</div>')).toContain('Nuova');
    expect(htmlToText('<div>Prezzo &lt; 100&nbsp;€</div><div><br></div><div>Grazie &amp; saluti</div>')).toBe('Prezzo < 100 €\n\nGrazie & saluti');
    expect(htmlToText('<ul><li><div>Uno</div></li><li><div>Due</div></li></ul><ol><li>A</li><li>B</li></ol><div>Fine</div>')).toBe('• Uno\n• Due\n1. A\n2. B\nFine');
    expect(htmlToText('<div><br></div><div><br></div><div>Firma</div>', true)).toBe('\n\nFirma');
    expect(unwrapBody('<div class="better-look-corpo" style="font-size: 11pt;"><div>a</div></div>')).toBe('<div>a</div>');
    expect(unwrapBody('<div class="better-look-corpo"><div>a</div></div><div>b</div>')).toContain('<div>b</div>');
  });

  it('il nuovo messaggio ha la firma dell’account; senza altro testo non diventa una bozza', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('new');
    expect(s.c?.html).toContain('better-look-firma');
    expect(s.c?.body).toContain('Ufficio acquisti, Valli Arredi');
    expect(MailStore.isEmpty($snap(s))).toBe(true);
    await s.discard();
    const drafts = (await s.backend.folders()).find((f) => f.role === 'drafts' && f.accountId === 'valli')!;
    const before = drafts.total;
    await s.openCompose('new');
    await s.closeWin();
    expect(await total(s, drafts.id)).toBe(before);
  });

  it('la risposta ha la firma sopra il messaggio citato', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('reply');
    const h = s.c!.html!;
    expect(h.indexOf('better-look-firma')).toBeLessThan(h.indexOf('better-look-citazione'));
    expect(h).toContain('<b>Oggetto:</b> Preventivo uffici Bassano, revisione 3');
  });

  it('cambiando mittente cambia la firma, come in Outlook', async () => {
    const s = await fresh('nuovo');
    await s.openCompose('new');
    s.setAccount('ordini');
    expect(s.c?.html).not.toContain('better-look-firma');
    s.setAccount('valli');
    expect(s.c?.html).toContain('Ufficio acquisti');
  });

  it('il pulsante Firma mette la firma scelta al posto di quella presente', async () => {
    const s = await fresh('nuovo');
    const nuova = await s.saveSignature({ id: '', name: 'Breve', html: '<div>C. Valli</div>' });
    expect(nuova?.id).toBeTruthy();
    await s.openCompose('new');
    s.insertSignature(nuova!.id);
    expect(s.c?.html).toContain('<div id="better-look-firma"><div>C. Valli</div></div>');
    expect(s.c?.html).not.toContain('Ufficio acquisti');
  });

  it('la prima firma creata diventa quella di tutti gli account', async () => {
    const s = await fresh('nuovo');
    await s.deleteSignature('firma-chiara');
    expect(s.signatures).toHaveLength(0);
    expect(s.accounts.find((a) => a.id === 'valli')?.signatureNew ?? null).toBeNull();
    const f = await s.saveSignature({ id: '', name: 'Ufficio', html: '<div>Valli Arredi</div>' });
    await s.reloadAccounts();
    for (const a of s.accounts) {
      expect(a.signatureNew).toBe(f!.id);
      expect(a.signatureReply).toBe(f!.id);
    }
  });

  it('il carattere predefinito resta scelto', async () => {
    const s = await fresh();
    s.setFont({ family: 'Verdana', size: 12 });
    expect(s.font).toEqual({ family: 'Verdana', size: 12 });
  });
});

function $snap(s: MailStore) {
  return JSON.parse(JSON.stringify(s.c));
}
