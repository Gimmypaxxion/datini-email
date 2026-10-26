import { describe, it, expect } from 'vitest';
import { describeCriteria, emptyCriteria, matchesCriteria, searchFolderCount, splitPeople } from '../src/lib/searchfolders';
import type { MessageSummary } from '../src/lib/types';

const base: MessageSummary = {
  id: 1, accountId: 'a', folderId: 1, from: { name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' }, to: [{ name: 'Chiara', email: 'c@valli.it' }],
  subject: 'Preventivo Bassano', preview: 'ti allego il preventivo', date: Date.UTC(2026, 9, 20), unread: true, flagged: false, answered: false,
  hasAttachments: true, focused: true, draft: false, hasBody: true, folderName: null, thread: null, categories: ['c1'], size: 2_000_000, threadCount: 1, importance: 1,
};

describe('condizioni delle cartelle di ricerca', () => {
  it('ogni condizione restringe i messaggi', () => {
    const now = Date.UTC(2026, 9, 26);
    const c = emptyCriteria();
    expect(matchesCriteria(base, c, now)).toBe(true);
    expect(matchesCriteria({ ...base, unread: false }, { ...c, unread: true }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, flagged: true }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, important: true, attachments: true }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, minKb: 1024 }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, minKb: 5120 }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, days: 7 }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, days: 1 }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, categorized: true }, now)).toBe(true);
    expect(matchesCriteria({ ...base, categories: [] }, { ...c, categorized: true }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, category: 'c2' }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, people: ['ORLANDI'] }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, people: ['bassi', 'c@valli'] }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, people: ['bassi'] }, now)).toBe(false);
    expect(matchesCriteria(base, { ...c, text: 'preventivo bassano' }, now)).toBe(true);
    expect(matchesCriteria(base, { ...c, text: 'fattura' }, now)).toBe(false);
    expect(matchesCriteria({ ...base, draft: true }, c, now)).toBe(false);
  });

  it('persone, conteggio e descrizione', () => {
    expect(splitPeople(' Davide; studiobassi.it ,\n ')).toEqual(['Davide', 'studiobassi.it']);
    const f = { id: 'x', accountId: 'a', name: 'X', kind: 'contrassegnata', criteria: { ...emptyCriteria(), flagged: true }, total: 4, unread: 1 };
    expect(searchFolderCount(f)).toEqual({ n: 4, muted: true });
    expect(searchFolderCount({ ...f, criteria: { ...f.criteria, flagged: false, unread: true } })).toEqual({ n: 1, muted: false });
    expect(describeCriteria(emptyCriteria())).toBe('Tutti i messaggi');
    expect(describeCriteria({ ...emptyCriteria(), unread: true, people: ['Davide'], minKb: 1024, days: 7 })).toBe('Messaggi da leggere, da o verso Davide, più grandi di 1 MB, degli ultimi 7 giorni');
  });
});
