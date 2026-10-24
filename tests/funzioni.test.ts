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
