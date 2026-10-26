import { describe, it, expect } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';
import { MockBackend } from '../src/lib/mockBackend';

async function fresh() {
  const s = new MailStore({ persist: false, backend: new MockBackend() });
  s.prefs = { ...PRESETS.classico };
  s.onboarding = false;
  await s.init();
  return s;
}

describe('cartelle', () => {
  it('crea, rinomina ed elimina una cartella', async () => {
    const s = await fresh();
    const clienti = s.folders.find((f) => f.path === 'Clienti')!;
    s.startNewFolder('valli', clienti.id);
    s.folderEdit!.value = 'Bassano';
    await s.commitFolderEdit();
    const nuova = s.folders.find((f) => f.name === 'Bassano')!;
    expect(nuova.path).toBe('Clienti/Bassano');
    expect(s.folderEdit).toBeNull();

    s.startRenameFolder(nuova.id);
    s.folderEdit!.value = 'Cantiere';
    await s.commitFolderEdit();
    expect(s.folders.find((f) => f.id === nuova.id)!.path).toBe('Clienti/Cantiere');

    // Un nome già usato nello stesso posto: l'errore resta nel campo.
    s.startNewFolder('valli', null);
    s.folderEdit!.value = 'Clienti';
    await s.commitFolderEdit();
    expect(s.folderEdit?.error).toMatch(/Esiste già/);
    s.cancelFolderEdit();

    // Le cartelle predefinite non si rinominano.
    const inbox = s.roleFolder('valli', 'inbox')!;
    s.startRenameFolder(inbox.id);
    expect(s.folderEdit).toBeNull();

    // Trascinare un messaggio sulla cartella lo sposta lì.
    const first = s.list[0];
    await s.dropOnFolder([first.id], nuova.id);
    expect(s.list.some((m) => m.id === first.id)).toBe(false);
    await s.reloadFolders();
    expect(s.folders.find((f) => f.id === nuova.id)!.total).toBe(1);

    // Eliminare chiede conferma; i messaggi vanno in Posta eliminata.
    const pending = s.deleteFolder(nuova.id);
    expect(s.confirm?.title).toMatch(/Cantiere/);
    s.answer(true);
    await pending;
    expect(s.folders.some((f) => f.id === nuova.id)).toBe(false);
    const trash = s.roleFolder('valli', 'deleted')!;
    const inTrash = await s.backend.messages(trash.id, null);
    expect(inTrash.some((m) => m.id === first.id)).toBe(true);
  });

  it('svuota Posta eliminata solo dopo la conferma', async () => {
    const s = await fresh();
    await s.deleteMsg(s.list[0].id);
    const trash = s.roleFolder('valli', 'deleted')!;
    await s.reloadFolders();
    expect(s.folders.find((f) => f.id === trash.id)!.total).toBe(1);
    const no = s.emptyFolder(trash.id);
    s.answer(false);
    await no;
    expect((await s.backend.messages(trash.id, null)).length).toBe(1);
    const yes = s.emptyFolder(trash.id);
    s.answer(true);
    await yes;
    expect((await s.backend.messages(trash.id, null)).length).toBe(0);
  });

  it('segna come letti tutti i messaggi della cartella', async () => {
    const s = await fresh();
    const inbox = s.roleFolder('valli', 'inbox')!;
    expect(inbox.unread).toBeGreaterThan(0);
    await s.markFolderRead(inbox.id);
    await s.reloadFolders();
    expect(s.roleFolder('valli', 'inbox')!.unread).toBe(0);
  });
});

describe('categorie', () => {
  it('assegna e toglie le categorie', async () => {
    const s = await fresh();
    const m = s.list.find((x) => x.subject.startsWith('Riunione'))!;
    expect(m.categories).toEqual([]);
    await s.toggleCategory(m.id, 'c4');
    expect(s.findMsg(m.id)!.categories).toEqual(['c4']);
    await s.toggleCategory(m.id, 'c1');
    await s.toggleCategory(m.id, 'c4');
    expect(s.findMsg(m.id)!.categories).toEqual(['c1']);
    await s.clearMsgCategories(m.id);
    expect(s.findMsg(m.id)!.categories).toEqual([]);
  });

  it('una categoria eliminata sparisce dai messaggi', async () => {
    const s = await fresh();
    const m = s.list.find((x) => x.categories.includes('c3'))!;
    const err = await s.saveCategories(s.categories.filter((c) => c.id !== 'c3'));
    expect(err).toBe('');
    await s.refresh();
    expect(s.findMsg(m.id)!.categories).not.toContain('c3');
    expect(s.categories.some((c) => c.id === 'c3')).toBe(false);
  });

  it('non accetta due categorie con lo stesso nome', async () => {
    const s = await fresh();
    const err = await s.saveCategories([...s.categories, { id: '', name: 'clienti', color: 'rosa' }]);
    expect(err).toMatch(/Esiste già/);
  });
});

describe('conversazioni', () => {
  it('raccoglie i messaggi della discussione anche da Posta inviata', async () => {
    const s = await fresh();
    const m = s.list.find((x) => x.subject.startsWith('Re: Campionario'))!;
    expect(m.threadCount).toBe(2);
    s.toggleConversations();
    await s.toggleThread(m);
    const items = s.threadOpen[m.thread!];
    expect(items.map((x) => x.subject)).toEqual(['Re: Campionario tessuti per lo showroom', 'Campionario tessuti per lo showroom']);
    // Si può aprire anche il messaggio in Posta inviata.
    await s.pickMsg(items[1].id);
    expect(s.detail?.id).toBe(items[1].id);
    await s.toggleThread(m);
    expect(s.threadOpen[m.thread!]).toBeUndefined();
  });
});

describe('scorciatoie sul Mac', () => {
  it('scrive le scorciatoie con i simboli del Mac', async () => {
    const { keysText } = await import('../src/lib/platform');
    expect(keysText('Rispondi a tutti (Ctrl+Maiusc+R)', true)).toBe('Rispondi a tutti (⇧⌘R)');
    expect(keysText('Leggi ad alta voce (Ctrl+Alt+Spazio)', true)).toBe('Leggi ad alta voce (⌥⌘Spazio)');
    expect(keysText('Ctrl+1 … Ctrl+4', true)).toBe('⌘1 … ⌘4');
    expect(keysText('Elimina (Canc)', true)).toBe('Elimina (Canc)');
    expect(keysText('Rispondi (Ctrl+R)', false)).toBe('Rispondi (Ctrl+R)');
  });
});

describe('conversazioni: ignora, pulisci, inoltra come allegato', () => {
  const until = async (ok: () => unknown) => {
    for (let i = 0; i < 50 && !ok(); i++) await new Promise((r) => setTimeout(r, 0));
  };

  it('ignora una conversazione e poi smette', async () => {
    const s = await fresh();
    const trash = s.roleFolder('valli', 'deleted')!;
    const p = s.ignoreConversation(1);
    await until(() => s.confirm);
    expect(s.confirm?.title).toBe('Ignora conversazione');
    s.answer(true);
    await p;
    expect(s.threadIgnored).toBe(true);
    expect(s.toast?.text).toMatch(/2 messaggi spostati in Posta eliminata/);
    const inTrash = await s.backend.messages(trash.id, null);
    expect(inTrash.map((m) => m.id).sort()).toEqual([1, 21]);
    expect(await s.backend.threadIgnored(1)).toBe(true);

    const back = s.ignoreConversation(1);
    await until(() => s.confirm);
    expect(s.confirm?.title).toBe('Smetti di ignorare');
    s.answer(true);
    await back;
    expect(s.threadIgnored).toBe(false);
    expect((await s.backend.messages(trash.id, null)).some((m) => m.id === 1)).toBe(false);
  });

  it('pulisce i messaggi già riportati in una risposta', async () => {
    const s = await fresh();
    const b = s.backend as unknown as { find(id: number): { text: string } };
    // Il messaggio 1 riporta per intero il 21, al quale risponde.
    b.find(1).text += `\n\nIl giovedì Chiara Valli ha scritto:\n${b.find(21).text}`;
    s.sel = 1;
    const p = s.cleanUp('conversazione');
    await until(() => s.confirm);
    expect(s.confirm?.ok).toBe('Pulisci');
    s.answer(true);
    await p;
    expect(s.toast?.text).toMatch(/1 messaggio superfluo/);
    const trash = s.roleFolder('valli', 'deleted')!;
    const inTrash = (await s.backend.messages(trash.id, null)).map((m) => m.id);
    // Il 21 è superfluo; l'1 resta (ha allegati ed è da leggere).
    expect(inTrash).toContain(21);
    expect(inTrash).not.toContain(1);

    // Una cartella dove ogni messaggio dice qualcosa di nuovo: nulla da pulire.
    const again = s.cleanUp('cartella');
    await until(() => s.confirm);
    s.answer(true);
    await again;
    expect(s.toast?.text).toMatch(/Nessun messaggio da pulire/);
  });

  it('inoltra un messaggio intero come allegato .eml', async () => {
    const s = await fresh();
    await s.forwardAsAttachment(3);
    expect(s.c?.subject).toBe('I: Campionario tessuti per lo showroom');
    expect(s.c?.attachments).toHaveLength(1);
    expect(s.c?.attachments[0]).toMatchObject({ name: 'Campionario tessuti per lo showroom.eml', mime: 'message/rfc822', message: 3 });
  });
});

describe('azioni rapide', () => {
  it('Fatto segna come letto e archivia; Rispondi ed elimina apre la risposta', async () => {
    const s = await fresh();
    expect(s.quickSteps.map((q) => q.name)).toEqual(['Fatto', 'Rispondi ed elimina']);
    const archive = s.roleFolder('valli', 'archive')!;
    const trash = s.roleFolder('valli', 'deleted')!;

    await s.pickMsg(1);
    s.runQuickShortcut(1);
    await new Promise((r) => setTimeout(r, 0));
    await new Promise((r) => setTimeout(r, 0));
    const inArchive = await s.backend.messages(archive.id, null);
    expect(inArchive.find((m) => m.id === 1)?.unread).toBe(false);
    expect(s.list.some((m) => m.id === 1)).toBe(false);
    expect(s.toast?.text).toMatch(/Fatto/);

    await s.pickMsg(3);
    await s.runQuickStep(s.quickSteps[1]);
    expect(s.c?.subject).toBe('Re: Campionario tessuti per lo showroom');
    expect(s.c?.to[0]?.email).toBe('lucia@marchettitessuti.it');
    expect((await s.backend.messages(trash.id, null)).some((m) => m.id === 3)).toBe(true);
  });

  it('azione personalizzata: categoria, contrassegno, sposta e inoltro con testo', async () => {
    const s = await fresh();
    const clienti = s.folders.find((f) => f.path === 'Clienti')!;
    const err = await s.saveQuickSteps([
      ...s.quickSteps,
      {
        id: '',
        name: 'Al commercialista',
        icon: 'briefcase',
        shortcut: 3,
        actions: [
          { kind: 'categoria', category: 'c3' },
          { kind: 'contrassegna' },
          { kind: 'sposta', folder: clienti.id },
          { kind: 'inoltra', to: [{ name: '', email: 'studio@commercialista.example' }], text: 'Da registrare, grazie.' },
        ],
      },
    ]);
    expect(err).toBe('');
    const q = s.quickSteps.find((x) => x.name === 'Al commercialista')!;
    expect(q.id).not.toBe('');
    await s.pickMsg(4);
    await s.runQuickStep(q);
    const moved = (await s.backend.messages(clienti.id, null)).find((m) => m.id === 4)!;
    expect(moved.flagged).toBe(true);
    expect(moved.categories).toContain('c3');
    expect(s.c?.subject).toMatch(/^I: /);
    expect(s.c?.to.map((p) => p.email)).toEqual(['studio@commercialista.example']);
    expect(s.c?.html).toMatch(/^<div>Da registrare, grazie\.<\/div>/);

    // Due spostamenti, o una scorciatoia già presa: errore in parole.
    expect(await s.saveQuickSteps([{ ...q, actions: [{ kind: 'archivia' }, { kind: 'elimina' }] }])).toMatch(/un solo posto/);
    expect(await s.saveQuickSteps([...s.quickSteps, { ...q, id: '', name: 'Copia', shortcut: 1 }])).toMatch(/Ctrl\+Maiusc\+1/);
  });
});

describe('modelli di messaggio', () => {
  it('inserisce un modello e salva il messaggio come modello', async () => {
    const s = await fresh();
    expect(s.templates.length).toBe(2);
    await s.openCompose('new');
    const t = s.templates.find((x) => x.name === 'Richiesta campioni')!;
    s.insertTemplate(t);
    expect(s.c?.subject).toBe('Richiesta campioni');
    expect(s.c?.html).toContain('ci servirebbero i campioni');
    // L'oggetto già scritto non cambia.
    s.c!.subject = 'Campioni per Bassano';
    s.insertTemplate(s.templates[0]);
    expect(s.c?.subject).toBe('Campioni per Bassano');

    // Salvato come modello: senza firma.
    expect(await s.saveDraftAsTemplate('Campioni Bassano')).toBe('');
    const saved = s.templates.find((x) => x.name === 'Campioni Bassano')!;
    expect(saved.subject).toBe('Campioni per Bassano');
    expect(saved.html).toContain('ci servirebbero i campioni');
    expect(saved.html).not.toContain('Ufficio acquisti');

    // Stesso nome: sostituisce.
    expect(await s.saveDraftAsTemplate('campioni bassano')).toBe('');
    expect(s.templates.filter((x) => x.name.toLowerCase() === 'campioni bassano').length).toBe(1);
    await s.deleteTemplate(saved.id);
    expect(s.templates.some((x) => x.id === saved.id)).toBe(false);
  });
});

describe('selezione di più messaggi', () => {
  it('Ctrl+clic, Maiusc+clic, Ctrl+A e Maiusc+frecce', async () => {
    const s = await fresh();
    const ids = s.list.map((m) => m.id);
    await s.clickMsg(ids[0]);
    expect(s.sel).toBe(ids[0]);
    await s.clickMsg(ids[2], { ctrl: true });
    expect(s.isMulti).toBe(true);
    expect(s.selection).toEqual([ids[0], ids[2]]);
    expect(s.sel).toBeNull();
    // Ctrl+clic di nuovo toglie: ne resta uno, che si apre.
    await s.clickMsg(ids[2], { ctrl: true });
    expect(s.isMulti).toBe(false);
    expect(s.sel).toBe(ids[0]);
    // Maiusc+clic: l'intervallo.
    await s.clickMsg(ids[3], { shift: true });
    expect(s.selection).toEqual(ids.slice(0, 4));
    // Maiusc+freccia su: si restringe.
    await s.extendSelection(-1);
    expect(s.selection).toEqual(ids.slice(0, 3));
    await s.selectAll();
    expect(s.selection.length).toBe(ids.length);
    // Un clic normale torna alla selezione singola.
    await s.clickMsg(ids[1]);
    expect(s.isMulti).toBe(false);
    expect(s.selection).toEqual([ids[1]]);
  });

  it('i comandi valgono per tutti i selezionati', async () => {
    const s = await fresh();
    const [a, b, c] = s.list.slice(0, 3).map((m) => m.id);
    await s.clickMsg(a);
    await s.clickMsg(c, { shift: true });
    await s.markRead(s.selection, true);
    expect(s.list.slice(0, 3).every((m) => !m.unread)).toBe(true);
    await s.toggleFlagSelected();
    expect(s.list.slice(0, 3).every((m) => m.flagged)).toBe(true);
    await s.setCategoryOn(s.selection, 'c5', true);
    expect(s.list.slice(0, 3).every((m) => m.categories.includes('c5'))).toBe(true);

    // Inoltra con più messaggi: tutti allegati, oggetto da scrivere.
    await s.forwardAsAttachment();
    expect(s.c?.attachments.map((x) => x.message)).toEqual([a, b, c]);
    expect(s.c?.subject).toBe('');

    // Archivia: spariscono tutti e si apre il messaggio che segue.
    const after = s.list[3].id;
    await s.clickMsg(a);
    await s.clickMsg(c, { shift: true });
    await s.moveSelected('archive');
    expect(s.list.some((m) => [a, b, c].includes(m.id))).toBe(false);
    expect(s.isMulti).toBe(false);
    expect(s.sel).toBe(after);
    const archive = s.roleFolder('valli', 'archive')!;
    const archived = (await s.backend.messages(archive.id, null)).map((m) => m.id);
    expect(archived).toEqual(expect.arrayContaining([a, b, c]));
  });

  it('azione rapida e spostamento su più messaggi', async () => {
    const s = await fresh();
    const [a, b] = s.list.slice(0, 2).map((m) => m.id);
    await s.clickMsg(a);
    await s.clickMsg(b, { ctrl: true });
    // Rispondi ed elimina: niente risposta con più messaggi, ma l'eliminazione sì.
    await s.runQuickStep(s.quickSteps[1]);
    expect(s.c).toBeNull();
    expect(s.toast?.text).toMatch(/un messaggio alla volta/);
    const trash = s.roleFolder('valli', 'deleted')!;
    expect((await s.backend.messages(trash.id, null)).map((m) => m.id)).toEqual(expect.arrayContaining([a, b]));

    const clienti = s.folders.find((f) => f.path === 'Clienti')!;
    const [c, d] = s.list.slice(0, 2).map((m) => m.id);
    await s.clickMsg(c);
    await s.clickMsg(d, { ctrl: true });
    await s.moveToFolder(s.selection, clienti.id);
    expect((await s.backend.messages(clienti.id, null)).map((m) => m.id)).toEqual(expect.arrayContaining([c, d]));
  });
});

describe('priorità e conferme', () => {
  it('priorità e conferme del messaggio in scrittura', async () => {
    const s = await fresh();
    await s.openCompose('new');
    expect(s.c!.importance ?? null).toBeNull();
    s.setImportance('high');
    expect(s.c!.importance).toBe('high');
    s.setImportance('low');
    expect(s.c!.importance).toBe('low');
    s.setImportance('low');
    expect(s.c!.importance).toBeNull();
    s.toggleReceipt('lettura');
    s.toggleReceipt('recapito');
    expect(s.c!.readReceipt && s.c!.deliveryReceipt).toBe(true);

    // Richiesta per tutti i messaggi: vale per i nuovi.
    s.setTracking({ trackRead: true });
    await s.discard();
    await s.openCompose('new');
    expect(s.c!.readReceipt).toBe(true);
    expect(s.c!.deliveryReceipt).toBeFalsy();
  });

  it('la conferma di lettura chiesta dal mittente', async () => {
    const s = await fresh();
    const mock = s.backend as MockBackend;
    expect(s.list.find((m) => m.id === 4)!.importance).toBe(1);
    expect(s.list.find((m) => m.id === 12)?.importance ?? -1).toBe(-1);

    await s.pickMsg(4);
    expect(s.detail!.receiptTo?.email).toBe('prestampa@tipografiagualtieri.it');
    await s.answerReceipt(false);
    expect(s.detail!.receiptTo).toBeNull();
    expect(mock.receiptsSent).toEqual([]);
    await s.loadDetail(4);
    expect(s.detail!.receiptTo).toBeNull();

    // Con "Inviala sempre" parte da sola, una volta.
    mock.receiptsDone.clear();
    s.setTracking({ receiptReply: 'sempre' });
    await s.loadDetail(4);
    await new Promise((r) => setTimeout(r, 0));
    expect(mock.receiptsSent).toEqual([{ id: 4, automatic: true }]);
    expect(s.detail!.receiptTo).toBeNull();
    await s.loadDetail(4);
    expect(mock.receiptsSent.length).toBe(1);
  });
});
