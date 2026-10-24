import { describe, it, expect } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';
import { MockBackend } from '../src/lib/mockBackend';
import { birthdayText, contactLetter, contactName } from '../src/lib/contacts';

async function fresh() {
  const s = new MailStore({ persist: false, backend: new MockBackend() });
  s.prefs = { ...PRESETS.classico };
  s.onboarding = false;
  await s.init();
  return s;
}

describe('rubrica', () => {
  it('nomi, lettere e compleanni', () => {
    const base = { id: '', firstName: '', lastName: '', displayName: '', emails: [], phones: [], company: '', jobTitle: '', address: '', website: '', birthday: '', notes: '' };
    expect(contactName({ ...base, firstName: 'Elena', lastName: 'Fabbri' })).toBe('Elena Fabbri');
    expect(contactName({ ...base, company: 'Trasporti Ferri' })).toBe('Trasporti Ferri');
    expect(contactName({ ...base, emails: ['x@y.it'] })).toBe('x@y.it');
    expect(contactLetter({ ...base, firstName: 'Ugo', lastName: 'Òrsini' })).toBe('O');
    expect(contactLetter({ ...base, displayName: '3M Italia' })).toBe('#');
    expect(birthdayText('1980-05-14')).toBe('14 maggio 1980');
    expect(birthdayText('--03-09')).toBe('9 marzo');
  });

  it('apre i contatti, cerca, crea, modifica ed elimina', async () => {
    const s = await fresh();
    s.setModule('contatti');
    await s.reloadContacts();
    expect(s.book.length).toBe(6);
    expect(s.ribbonTabs).toEqual(['home', 'aiuto']);
    s.contactQuery = 'valli';
    expect(s.filteredContacts.map(contactName)).toEqual(['Giorgio Valli', 'Marco Ventura']);
    s.contactQuery = '0574 222';
    expect(s.filteredContacts.map(contactName)).toEqual(['Elena Fabbri']);
    s.contactQuery = '';

    s.newContact();
    s.contactEdit!.emails = ['non valido'];
    expect(await s.saveContactEdit()).toMatch(/non è valido/);
    s.contactEdit!.emails = [];
    expect(await s.saveContactEdit()).toMatch(/almeno un nome/);
    s.contactEdit!.firstName = 'Paolo';
    s.contactEdit!.lastName = 'Rossi';
    s.contactEdit!.emails = ['paolo@rossi.example'];
    expect(await s.saveContactEdit()).toBeNull();
    expect(s.selectedContact && contactName(s.selectedContact)).toBe('Paolo Rossi');

    s.editContact();
    s.contactEdit!.company = 'Rossi Srl';
    await s.saveContactEdit();
    expect(s.selectedContact!.company).toBe('Rossi Srl');

    const del = s.deleteContacts([s.contactSel!]);
    expect(s.confirm?.title).toBe('Eliminare “Paolo Rossi”?');
    s.answer(true);
    await del;
    expect(s.book.length).toBe(6);
  });

  it('aggiunge il mittente di un messaggio e porta alla sua posta', async () => {
    const s = await fresh();
    const m = s.list.find((x) => x.from.email === 'consegne@trasportiferri.it') ?? s.list[0];
    await s.addToContacts({ name: 'Nuovo Fornitore', email: 'info@nuovofornitore.example' });
    expect(s.module).toBe('contatti');
    expect(s.contactEdit).toMatchObject({ firstName: 'Nuovo', lastName: 'Fornitore', emails: ['info@nuovofornitore.example'] });
    s.contactEdit = null;
    // Già in rubrica: si apre la sua scheda.
    await s.addToContacts({ name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' });
    expect(s.selectedContact && contactName(s.selectedContact)).toBe('Davide Orlandi');
    await s.showMailFrom('d.orlandi@studiorlandi.it');
    expect(s.module).toBe('posta');
    expect(s.q).toBe('d.orlandi@studiorlandi.it');
    expect(s.list.length).toBeGreaterThan(0);
    expect(m).toBeTruthy();
  });

  it('importa senza duplicare', async () => {
    const s = await fresh();
    s.setModule('contatti');
    await s.importContacts();
    expect(s.book.length).toBe(8);
    expect(s.toast?.text).toBe('Contatti importati: 2 nuovi.');
    await s.importContacts();
    expect(s.book.length).toBe(8);
    expect(s.toast?.text).toBe('Contatti importati: 2 già presenti.');
  });
});
