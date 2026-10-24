import { describe, it, expect, vi, afterEach } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';
import { MockBackend } from '../src/lib/mockBackend';
import { describeActions, describeConditions, emptyRule, ruleMatches, splitWords } from '../src/lib/rules';
import { whenText } from '../src/lib/format';

async function fresh(stile: 'classico' | 'nuovo' = 'classico') {
  const backend = new MockBackend();
  const s = new MailStore({ persist: false, backend });
  s.prefs = { ...PRESETS[stile] };
  s.onboarding = false;
  await s.init();
  return { s, backend };
}

afterEach(() => vi.useRealTimers());

describe('regole', () => {
  it('descrive e controlla le condizioni', () => {
    const r = emptyRule();
    r.name = 'Fatture';
    r.when.from = splitWords('studiobassi; Rinaldi');
    r.when.subject = ['fattura'];
    r.then.moveTo = 7;
    r.then.flag = true;
    expect(describeConditions(r.when)).toBe('Se il mittente contiene “studiobassi” o “Rinaldi” e l’oggetto contiene “fattura”');
    expect(describeActions(r, [{ id: 7, accountId: 'a', path: 'Fornitori', name: 'Fornitori', role: null, depth: 0, selectable: true, unread: 0, total: 0 }], [])).toBe('sposta in “Fornitori”, contrassegna, poi non applicare altre regole');
    expect(describeConditions(emptyRule().when)).toBe('Per tutti i messaggi in arrivo');
    const m = { id: 1, accountId: 'a', folderId: 1, from: { name: 'Studio Bassi & Rinaldi', email: 'amm@studiobassi.it' }, to: [], subject: 'Fattura n. 418', preview: '', date: 0, unread: true, flagged: false, answered: false, hasAttachments: false, focused: true, draft: false, hasBody: true, folderName: null, thread: null, categories: [], size: 0, threadCount: 1 };
    expect(ruleMatches(r, m)).toBe(true);
    expect(ruleMatches({ ...r, when: { ...r.when, hasAttachments: true } }, m)).toBe(false);
    expect(ruleMatches({ ...r, enabled: false }, m)).toBe(false);
  });

  it('crea una regola da un messaggio, la salva e la esegue ora', async () => {
    const { s } = await fresh();
    const msg = s.list.find((m) => m.from.email.includes('fabbrilegnami'))!;
    s.newRuleFrom(msg.id);
    expect(s.settings).toBe(true);
    expect(s.setPage).toBe('regole');
    expect(s.ruleDraft!.when.from).toEqual([msg.from.email]);
    const draft = s.ruleDraft!;
    // Senza azioni non si salva.
    expect(await s.saveRules([draft])).toMatch(/Scegli cosa deve fare/);
    const amministrazione = s.folders.find((f) => f.name === 'Amministrazione')!;
    draft.then.moveTo = amministrazione.id;
    expect(await s.saveRules([draft])).toBeNull();
    expect(s.rules).toHaveLength(1);
    const text = await s.runRulesNow([s.rules[0].id], msg.folderId);
    expect(text).toMatch(/Regola applicata a \d+ messagg/);
    await s.refresh();
    expect(s.list.some((m) => m.id === msg.id)).toBe(false);
  });

  it('avvisa sul desktop all’arrivo di nuovi messaggi, se richiesto', async () => {
    const { s, backend } = await fresh();
    const m = s.list[0];
    const ev = (count: number) => (backend as unknown as { emit(e: unknown): void }).emit({ type: 'newMail', accountId: m.accountId, count, messages: [m, s.list[1]] });
    ev(1);
    await Promise.resolve();
    expect(backend.notified.at(-1)).toEqual({ title: m.from.name, body: m.subject });
    ev(2);
    await Promise.resolve();
    expect(backend.notified.at(-1)!.title).toBe('2 nuovi messaggi');
    s.setNotify(false);
    ev(1);
    await Promise.resolve();
    expect(backend.notified).toHaveLength(2);
  });
});

describe('risposte automatiche', () => {
  it('si attivano con un periodo e si spengono dall’avviso', async () => {
    const { s } = await fresh();
    const now = Date.now();
    expect(await s.saveAutoReply('valli', { enabled: true, start: now + 1000, end: now, html: '', text: 'Fuori ufficio' })).toMatch(/fine del periodo/);
    expect(await s.saveAutoReply('valli', { enabled: true, start: null, end: null, html: '', text: ' ' })).toMatch(/Scrivi il messaggio/);
    expect(await s.saveAutoReply('valli', { enabled: true, start: now - 1000, end: now + 86_400_000, html: '<div>Fuori ufficio</div>', text: 'Fuori ufficio' })).toBeNull();
    expect(s.autoReplyOn.map((a) => a.id)).toEqual(['valli']);
    await s.turnOffAutoReply('valli');
    expect(s.autoReplyOn).toEqual([]);
    // Un periodo che deve ancora iniziare non è attivo.
    await s.saveAutoReply('valli', { enabled: true, start: now + 3_600_000, end: null, html: '', text: 'Dal mese prossimo' });
    expect(s.autoReplyOn).toEqual([]);
  });
});

describe('invio programmato', () => {
  it('mette il messaggio in uscita per l’ora scelta e si può annullare', async () => {
    const { s } = await fresh('nuovo');
    await s.openCompose('new');
    s.c!.to = [{ name: 'Davide Orlandi', email: 'd.orlandi@studiorlandi.it' }];
    s.setSubject('Campioni del rovere');
    // Un'ora già passata non va.
    expect(await s.send(false, Date.now() - 60_000)).toBe(false);
    expect(s.err).toMatch(/ora futura/);
    const at = new Date(Date.now() + 2 * 86_400_000).setHours(8, 0, 0, 0);
    expect(await s.send(false, at)).toBe(true);
    await new Promise((r) => setTimeout(r, 0));
    await s.reloadOutbox();
    expect(s.outbox).toHaveLength(1);
    expect(s.outbox[0].sendAfter).toBe(at);
    expect(s.toast?.text).toBe(`Invio programmato per ${whenText(at)}`);
    await s.undoSend();
    await s.reloadOutbox();
    expect(s.outbox).toHaveLength(0);
    expect(s.c?.subject).toBe('Campioni del rovere');
  });

  it('chiede conferma dell’oggetto vuoto senza perdere l’ora', async () => {
    const { s } = await fresh('nuovo');
    await s.openCompose('new');
    s.c!.to = [{ name: '', email: 'ordini@valliarredi.it' }];
    const at = Date.now() + 3_600_000;
    expect(await s.send(false, at)).toBe(false);
    expect(s.askNoSubject).toBe(true);
    expect(await s.send(true)).toBe(true);
    await s.reloadOutbox();
    expect(s.outbox[0].sendAfter).toBe(at);
  });
});

describe('pulizia cassetta postale', () => {
  it('mostra lo spazio, trova i messaggi grandi e archivia quelli vecchi', async () => {
    const { backend } = await fresh();
    const usage = await backend.mailboxUsage('valli');
    expect(usage[0].size).toBeGreaterThanOrEqual(usage[usage.length - 1].size);
    const big = await backend.largeMessages('valli', 1);
    expect(big.length).toBeGreaterThan(0);
    expect(big[0].folderName).toBeTruthy();
    const inbox = (await backend.folders()).find((f) => f.accountId === 'valli' && f.role === 'inbox')!;
    const old = Date.now() - 3 * 86_400_000;
    const n = await backend.olderMessages(inbox.id, old, false);
    expect(n).toBeGreaterThan(0);
    expect(await backend.olderMessages(inbox.id, old, true)).toBe(n);
    expect(await backend.olderMessages(inbox.id, old, false)).toBe(0);
  });
});
