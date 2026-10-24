import { describe, it, expect } from 'vitest';
import {
  addDays,
  allDayBars,
  conflicts,
  dueReminders,
  emptyEvent,
  expand,
  fromInputs,
  layoutDay,
  nextOccurrence,
  nthWeekday,
  occurrences,
  rangeTitle,
  repeatChoices,
  repeatKey,
  repeatText,
  responsesText,
  startOfWeek,
  viewRange,
  whenText,
} from '../src/lib/calendar';
import type { CalEvent, Repeat } from '../src/lib/types';

const t = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min).getTime();
const rep = (r: Partial<Repeat>): Repeat => ({ freq: 'weekly', interval: 1, days: [], week: null, until: null, count: null, ...r });
const ev = (over: Partial<CalEvent>): CalEvent => ({ ...emptyEvent(t(2026, 10, 19, 9), t(2026, 10, 19, 10)), id: 'e', uid: 'u', title: 'Prova', ...over });
const starts = (list: { start: number }[]) => list.map((o) => new Date(o.start).toISOString().slice(0, 16));
const local = (list: { start: number }[]) => list.map((o) => { const d = new Date(o.start); return `${d.getDate()}/${d.getMonth() + 1} ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`; });

describe('calendario: viste', () => {
  it('settimane da lunedì, mese intero, titoli', () => {
    // Giovedì 22 ottobre 2026.
    const anchor = t(2026, 10, 22, 15);
    expect(startOfWeek(anchor)).toBe(t(2026, 10, 19));
    const wk = viewRange('settimana-lavorativa', anchor);
    expect(wk.days.length).toBe(5);
    expect(wk.to).toBe(t(2026, 10, 24));
    expect(viewRange('settimana', anchor).days.length).toBe(7);
    const month = viewRange('mese', anchor);
    expect(month.days[0]).toBe(t(2026, 9, 28));
    expect(month.days.length).toBe(35);
    expect(rangeTitle('giorno', anchor)).toBe('giovedì 22 ottobre 2026');
    expect(rangeTitle('settimana-lavorativa', anchor)).toBe('19-23 ottobre 2026');
    expect(rangeTitle('settimana', t(2026, 9, 30))).toBe('28 settembre - 4 ottobre 2026');
    expect(rangeTitle('settimana', t(2026, 12, 30))).toBe('28 dicembre 2026 - 3 gennaio 2027');
    expect(rangeTitle('mese', anchor)).toBe('ottobre 2026');
  });

  it('giorni della settimana nel mese', () => {
    expect(nthWeekday(2026, 9, 2, 2)?.getDate()).toBe(13); // secondo martedì di ottobre
    expect(nthWeekday(2026, 9, 0, -1)?.getDate()).toBe(25); // ultima domenica
    expect(nthWeekday(2026, 1, 1, 5)).toBeNull(); // febbraio 2026 non ha un quinto lunedì
  });
});

describe('calendario: ripetizioni', () => {
  it('settimanale su più giorni con eccezioni e fine', () => {
    const e = ev({ repeat: rep({ days: [1, 3], until: t(2026, 11, 4, 23, 59) }), exceptions: [t(2026, 10, 26, 9)] });
    expect(local(occurrences(e, t(2026, 10, 1), t(2026, 12, 31)))).toEqual(['19/10 9:00', '21/10 9:00', '28/10 9:00', '2/11 9:00', '4/11 9:00']);
  });

  it("l'ora resta la stessa dopo il cambio dell'ora legale", () => {
    const e = ev({ start: t(2026, 10, 19, 9), end: t(2026, 10, 19, 9, 30), repeat: rep({ freq: 'daily', count: 14 }) });
    const list = occurrences(e, t(2026, 10, 1), t(2027, 1, 1));
    expect(list.length).toBe(14);
    expect(list.every((o) => new Date(o.start).getHours() === 9 && o.end - o.start === 30 * 60_000)).toBe(true);
    expect(local(list)[13]).toBe('1/11 9:00');
  });

  it('ogni 2 settimane, mensile per data e per giorno, annuale', () => {
    const two = ev({ repeat: rep({ interval: 2, days: [1] }) });
    expect(local(occurrences(two, t(2026, 10, 1), t(2026, 11, 20)))).toEqual(['19/10 9:00', '2/11 9:00', '16/11 9:00']);
    const monthly31 = ev({ start: t(2026, 10, 31, 8), end: t(2026, 10, 31, 9), repeat: rep({ freq: 'monthly' }) });
    expect(local(occurrences(monthly31, t(2026, 10, 1), t(2027, 2, 1)))).toEqual(['31/10 8:00', '31/12 8:00', '31/1 8:00']);
    const second = ev({ start: t(2026, 10, 13, 18), end: t(2026, 10, 13, 19), repeat: rep({ freq: 'monthly', days: [2], week: 2, count: 3 }) });
    expect(local(occurrences(second, t(2026, 1, 1), t(2028, 1, 1)))).toEqual(['13/10 18:00', '10/11 18:00', '8/12 18:00']);
    const last = ev({ start: t(2026, 10, 30, 12), end: t(2026, 10, 30, 13), repeat: rep({ freq: 'monthly', days: [5], week: -1 }) });
    expect(local(occurrences(last, t(2026, 11, 1), t(2027, 1, 1)))).toEqual(['27/11 12:00', '25/12 12:00']);
    const bday = ev({ start: t(2026, 3, 9), end: t(2026, 3, 10), allDay: true, repeat: rep({ freq: 'yearly' }) });
    expect(local(occurrences(bday, t(2030, 1, 1), t(2031, 1, 1)))).toEqual(['9/3 0:00']);
  });

  it('serie lunghe: salta subito vicino all’intervallo', () => {
    const daily = ev({ start: t(2000, 1, 3, 7), end: t(2000, 1, 3, 7, 15), repeat: rep({ freq: 'daily' }) });
    const list = occurrences(daily, t(2026, 10, 19), t(2026, 10, 24));
    expect(list.length).toBe(5);
    expect(new Date(list[0].start).getHours()).toBe(7);
    const yearly = ev({ start: t(1990, 3, 9), end: t(1990, 3, 10), allDay: true, repeat: rep({ freq: 'monthly' }) });
    expect(occurrences(yearly, t(2026, 10, 1), t(2026, 11, 1)).length).toBe(1);
  });

  it('eventi di tutto il giorno e di più giorni', () => {
    const ferie = ev({ start: t(2026, 10, 21), end: t(2026, 10, 24), allDay: true });
    expect(occurrences(ferie, t(2026, 10, 23), t(2026, 10, 24)).length).toBe(1);
    expect(occurrences(ferie, t(2026, 10, 24), t(2026, 10, 25)).length).toBe(0);
    const days = viewRange('settimana-lavorativa', t(2026, 10, 19)).days;
    const occs = expand([ferie, ev({ id: 'f2', start: t(2026, 10, 23), end: t(2026, 10, 24), allDay: true })], days[0], addDays(days[0], 5));
    const bars = allDayBars(occs, days);
    expect(bars.map((b) => [b.first, b.span, b.row])).toEqual([[2, 3, 0], [4, 1, 1]]);
  });

  it('prossima occorrenza', () => {
    const e = ev({ repeat: rep({ days: [1] }) });
    expect(local([nextOccurrence(e, t(2026, 11, 3))!])).toEqual(['9/11 9:00']);
    const done = ev({ repeat: rep({ days: [1], count: 2 }) });
    expect(local([nextOccurrence(done, t(2027, 1, 1))!])).toEqual(['26/10 9:00']);
  });
});

describe('calendario: griglia, testi e promemoria', () => {
  it('appuntamenti sovrapposti vanno in colonne', () => {
    const day = t(2026, 10, 19);
    const occs = expand(
      [
        ev({ id: 'a', start: t(2026, 10, 19, 9), end: t(2026, 10, 19, 10) }),
        ev({ id: 'b', start: t(2026, 10, 19, 9, 30), end: t(2026, 10, 19, 11) }),
        ev({ id: 'c', start: t(2026, 10, 19, 10), end: t(2026, 10, 19, 10, 30) }),
        ev({ id: 'd', start: t(2026, 10, 19, 14), end: t(2026, 10, 19, 15) }),
        ev({ id: 'x', start: t(2026, 10, 18, 23), end: t(2026, 10, 19, 1) }),
      ],
      day,
      addDays(day, 1),
    );
    const placed = layoutDay(occs, day);
    const by = (id: string) => placed.find((p) => p.occ.ev.id === id)!;
    expect([by('a').col, by('b').col, by('c').col, by('a').cols]).toEqual([0, 1, 0, 2]);
    expect([by('d').col, by('d').cols, by('d').top]).toEqual([0, 1, 14 * 60]);
    expect([by('x').top, by('x').height, by('x').cutStart]).toEqual([0, 60, true]);
    const c = conflicts({ start: t(2026, 10, 19, 10, 15), end: t(2026, 10, 19, 10, 45), uid: 'nuovo' }, occs);
    expect(c.map((o) => o.ev.id).sort()).toEqual(['b', 'c']);
  });

  it('testi in italiano', () => {
    const now = t(2026, 10, 15);
    expect(whenText(t(2026, 10, 19, 10, 30), t(2026, 10, 19, 12), false, now)).toBe('lunedì 19 ottobre, 10:30-12:00');
    expect(whenText(t(2026, 12, 28), t(2026, 12, 31), true, now)).toBe('Da lunedì 28 dicembre a mercoledì 30 dicembre');
    expect(whenText(t(2027, 1, 18), t(2027, 1, 19), true, now)).toBe('lunedì 18 gennaio 2027, tutto il giorno');
    const s = t(2026, 10, 13, 9);
    expect(repeatText(rep({ days: [1, 3, 5] }), s)).toBe('Ogni settimana il lunedì, il mercoledì e il venerdì');
    expect(repeatText(rep({ days: [1, 2, 3, 4, 5] }), s)).toBe('Ogni giorno feriale');
    expect(repeatText(rep({ freq: 'monthly', days: [2], week: 2 }), s)).toBe('Ogni mese il secondo martedì');
    expect(repeatText(rep({ freq: 'monthly', days: [0], week: -1 }), s)).toBe("Ogni mese l'ultima domenica");
    expect(repeatText(rep({ freq: 'monthly', interval: 3 }), s)).toBe('Ogni 3 mesi il giorno 13');
    expect(repeatText(rep({ freq: 'yearly', until: t(2030, 1, 1) }), t(2026, 3, 1))).toBe('Ogni anno il 1° marzo, fino al 1° gennaio 2030');
    expect(repeatText(rep({ freq: 'daily', count: 5 }), s)).toBe('Ogni giorno, per 5 volte');
    expect(responsesText([
      { name: '', email: 'a@x.it', status: 'accepted', optional: false },
      { name: '', email: 'b@x.it', status: 'accepted', optional: false },
      { name: '', email: 'c@x.it', status: 'needs-action', optional: false },
    ])).toBe('2 accettati, 1 senza risposta');
  });

  it('scelte di ripetizione del modulo', () => {
    // Venerdì 30 ottobre 2026: è anche l'ultimo venerdì del mese.
    const s = t(2026, 10, 30, 12);
    const keys = repeatChoices(s).map((c) => `${c.key}: ${c.label}`);
    expect(keys).toEqual([
      'mai: Non si ripete',
      'giorno: Ogni giorno',
      'feriali: Ogni giorno feriale (lun-ven)',
      'settimana: Ogni settimana il venerdì',
      'mese: Ogni mese il giorno 30',
      "mese-ultimo: Ogni mese l'ultimo venerdì",
      'anno: Ogni anno il 30 ottobre',
    ]);
    const fourth = repeatChoices(t(2026, 10, 23, 12)).map((c) => c.key);
    expect(fourth).toContain('mese-n');
    expect(fourth).not.toContain('mese-ultimo');
    expect(repeatKey(rep({ days: [5] }), s)).toBe('settimana');
    expect(repeatKey(rep({ days: [] }), s)).toBe('settimana');
    expect(repeatKey(rep({ freq: 'monthly', days: [5], week: -1 }), s)).toBe('mese-ultimo');
    expect(repeatKey(rep({ interval: 3 }), s)).toBe('altra');
    expect(repeatKey(null, s)).toBe('mai');
  });

  it('promemoria e campi', () => {
    const e = ev({ reminder: 15 });
    const occs = expand([e, ev({ id: 'n', reminder: null }), ev({ id: 'r', reminder: 15, response: 'declined' })], t(2026, 10, 19), t(2026, 10, 20));
    expect(dueReminders(occs, t(2026, 10, 19, 8, 40), t(2026, 10, 19, 8, 45)).map((o) => o.ev.id)).toEqual(['e']);
    expect(dueReminders(occs, t(2026, 10, 19, 8, 45), t(2026, 10, 19, 8, 50))).toEqual([]);
    expect(fromInputs('2026-10-19', '10:30')).toBe(t(2026, 10, 19, 10, 30));
    expect(fromInputs('19/10/2026', '10:30')).toBeNull();
    expect(starts([{ start: Date.UTC(2026, 9, 19, 8, 30) }])).toEqual(['2026-10-19T08:30']);
  });
});

describe('calendario: stato e motore di esempio', () => {
  async function fresh() {
    const { MailStore, PRESETS } = await import('../src/lib/store.svelte');
    const { MockBackend } = await import('../src/lib/mockBackend');
    const backend = new MockBackend();
    const s = new MailStore({ persist: false, backend });
    s.prefs = { ...PRESETS.classico };
    s.onboarding = false;
    await s.init();
    return { s, backend };
  }
  const tick = () => new Promise((r) => setTimeout(r, 0));

  it('apre il calendario, crea, modifica ed elimina un appuntamento', async () => {
    const { s } = await fresh();
    s.setModule('calendario');
    await s.cal.reload();
    expect(s.cal.occs.length).toBeGreaterThan(5);
    const day = s.cal.range.days[1];
    s.cal.newEvent({ start: day + 11 * 3_600_000 });
    expect(s.cal.edit?.ev.end).toBe(day + 11.5 * 3_600_000);
    s.cal.edit!.ev.title = 'Prova colori';
    s.cal.edit!.ev.end = s.cal.edit!.ev.start - 1;
    expect(await s.cal.save(false)).toMatch(/fine/);
    s.cal.edit!.ev.end = day + 12 * 3_600_000;
    expect(await s.cal.save(false)).toBeNull();
    const o = s.cal.occs.find((x) => x.ev.title === 'Prova colori')!;
    expect(o).toBeTruthy();
    expect(s.cal.sel).toBe(o.key);
    const del = s.cal.remove(o);
    await tick();
    expect(s.confirm?.title).toContain('Prova colori');
    s.answer(true);
    await del;
    expect(s.cal.occs.some((x) => x.ev.title === 'Prova colori')).toBe(false);
  });

  it('serie: modifica solo un’occorrenza o elimina solo quella', async () => {
    const { s } = await fresh();
    s.setModule('calendario');
    await s.cal.reload();
    const weekly = s.cal.occs.find((x) => x.ev.id === 'ev1')!;
    const opening = s.cal.open(weekly);
    await tick();
    expect(s.confirm?.alt).toBe('Tutta la serie');
    s.answer(true);
    await opening;
    expect(s.cal.edit?.ev.recurrenceId).toBe(weekly.start);
    expect(s.cal.edit?.ev.repeat).toBeNull();
    s.cal.edit!.ev.title = 'Punto della settimana (spostato)';
    s.cal.edit!.ev.start += 3_600_000;
    s.cal.edit!.ev.end += 3_600_000;
    expect(await s.cal.save(false)).toBeNull();
    const titles = s.cal.occs.filter((x) => x.ev.uid === weekly.ev.uid).map((x) => x.ev.title);
    expect(titles).toContain('Punto della settimana (spostato)');
    expect(s.cal.occs.filter((x) => x.ev.uid === weekly.ev.uid && x.start === weekly.start).length).toBe(0);

    // Settimana dopo: elimina solo quella occorrenza.
    s.cal.move(1);
    await s.cal.reload();
    const next = s.cal.occs.find((x) => x.ev.id === 'ev1')!;
    const del = s.cal.remove(next);
    await tick();
    s.answer(true);
    await tick();
    s.answer(true);
    await del;
    expect(s.cal.occs.some((x) => x.ev.id === 'ev1')).toBe(false);
    s.cal.move(1);
    await s.cal.reload();
    expect(s.cal.occs.some((x) => x.ev.id === 'ev1')).toBe(true);
  });

  it('accetta un invito dalla posta e risponde all’organizzatore', async () => {
    const { s, backend } = await fresh();
    await s.pickMsg(14);
    expect(s.detail?.invite?.method).toBe('request');
    expect(s.detail?.invite?.local).toBeNull();
    expect(await s.respondInvite('accepted', true)).toBeNull();
    expect(s.detail?.invite?.local?.response).toBe('accepted');
    expect(backend.itip.map((m) => `${m.method} ${m.to.join()} ${m.subject}`)).toEqual(['REPLY m.ventura@valliarredi.it Accettato: Riunione commerciale']);
    expect(s.toast?.text).toContain('Risposta inviata a Marco Ventura');
    const inv = s.detail!.invite!.event;
    s.setModule('calendario');
    s.cal.goTo(inv.start);
    await s.cal.reload();
    const o = s.cal.occs.find((x) => x.ev.uid === inv.uid)!;
    expect(o.ev.response).toBe('accepted');
    expect(s.cal.state(o.ev)).toBe('');
    // Dal calendario si cambia idea: rifiuto con risposta, e l'evento sparisce.
    await s.cal.respond(o, 'declined', true);
    expect(s.cal.occs.some((x) => x.ev.uid === inv.uid)).toBe(false);
    expect(backend.itip[backend.itip.length - 1]?.subject).toBe('Rifiutato: Riunione commerciale');
  });

  it('riunioni: invito ai partecipanti, rispondi con una riunione, annullamento', async () => {
    const { s, backend } = await fresh();
    await s.pickMsg(1);
    s.meetingFrom();
    const ed = s.cal.edit!;
    expect(ed.meeting).toBe(true);
    expect(ed.ev.title).toBe('Preventivo uffici Bassano, revisione 3');
    expect(ed.ev.attendees.map((a) => a.email)).toEqual(['d.orlandi@studiorlandi.it', 'm.ventura@valliarredi.it']);
    const start = ed.ev.start;
    expect(await s.cal.save(true)).toBeNull();
    expect(backend.itip[0]).toMatchObject({ method: 'REQUEST', to: ['d.orlandi@studiorlandi.it', 'm.ventura@valliarredi.it'] });
    expect(s.toast?.text).toBe('Invito inviato a 2 partecipanti.');
    s.cal.goTo(start, 'giorno');
    await s.cal.reload();
    const o = s.cal.occs.find((x) => x.ev.title === 'Preventivo uffici Bassano, revisione 3')!;
    expect(o.ev.organizer?.email).toBe('chiara.valli@valliarredi.it');
    const del = s.cal.remove(o);
    await tick();
    expect(s.confirm?.title).toBe('Annullare la riunione?');
    expect(s.confirm?.check?.value).toBe(true);
    s.answer(true);
    await del;
    expect(backend.itip[backend.itip.length - 1]).toMatchObject({ method: 'CANCEL', subject: 'Annullato: Preventivo uffici Bassano, revisione 3' });
  });

  it('promemoria: avviso sul desktop, posticipa, ignora', async () => {
    const { s, backend } = await fresh();
    const start = Date.now() + 10 * 60_000;
    await backend.saveEvent({ ...emptyEvent(start, start + 30 * 60_000), title: 'Chiamata con Elena', reminder: 15 }, false);
    await s.cal.checkReminders(Date.now() - 60_000);
    await s.cal.checkReminders(Date.now());
    expect(s.cal.reminders.map((r) => r.occ.ev.title)).toEqual(['Chiamata con Elena']);
    expect(backend.notified[backend.notified.length - 1]?.title).toBe('Chiamata con Elena');
    const key = s.cal.reminders[0].key;
    s.cal.snooze(key, 5);
    expect(s.cal.reminders).toEqual([]);
    await s.cal.checkReminders(Date.now() + 6 * 60_000);
    expect(s.cal.reminders.length).toBe(1);
    s.cal.dismiss(key);
    await s.cal.checkReminders(Date.now() + 7 * 60_000);
    expect(s.cal.reminders).toEqual([]);
    expect(s.cal.upcoming.some((o) => o.ev.title === 'Chiamata con Elena')).toBe(true);
  });
});
