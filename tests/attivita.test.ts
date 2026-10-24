import { describe, it, expect } from 'vitest';
import { MailStore, PRESETS } from '../src/lib/store.svelte';
import { MockBackend } from '../src/lib/mockBackend';
import { dayKey, dueChoices, dueInfo, nextTaskDate, openCount, plannedGroups, reminderChoices, tasksIn, emptyTask } from '../src/lib/tasks';
import type { Task } from '../src/lib/types';

const t = (y: number, m: number, d: number, h = 0) => new Date(y, m - 1, d, h).getTime();
const task = (over: Partial<Task>): Task => ({ ...emptyTask('', t(2026, 10, 20, 9)), id: Math.random().toString(36).slice(2), title: 'Prova', ...over });

async function fresh() {
  const s = new MailStore({ persist: false, backend: new MockBackend() });
  s.prefs = { ...PRESETS.classico };
  s.onboarding = false;
  await s.init();
  return s;
}

describe('attività: logica', () => {
  // Sabato 24 ottobre 2026, pomeriggio.
  const now = t(2026, 10, 24, 16);

  it('viste intelligenti e conteggi', () => {
    const all = [
      task({ id: 'a', myDay: dayKey(now), created: 3 }),
      task({ id: 'b', important: true, list: 'l1', created: 2 }),
      task({ id: 'c', due: t(2026, 10, 23), created: 1 }),
      task({ id: 'd', done: true, doneAt: now, myDay: dayKey(now) }),
      task({ id: 'e', myDay: dayKey(t(2026, 10, 23)), created: 0 }),
    ];
    expect(tasksIn(all, 'giornata', now).open.map((x) => x.id)).toEqual(['a']);
    expect(tasksIn(all, 'giornata', now).done.map((x) => x.id)).toEqual(['d']);
    expect(tasksIn(all, 'importanti', now).open.map((x) => x.id)).toEqual(['b']);
    expect(tasksIn(all, 'attivita', now).open.map((x) => x.id)).toEqual(['a', 'c', 'e']);
    expect(tasksIn(all, 'elenco:l1', now).open.map((x) => x.id)).toEqual(['b']);
    expect(openCount(all, 'pianificate', now)).toBe(1);
    // Ieri in La mia giornata: oggi no.
    expect(openCount(all, 'giornata', now)).toBe(1);
  });

  it('pianificate per gruppi e scadenze in parole', () => {
    const open = [
      task({ id: 'late', due: t(2026, 10, 22) }),
      task({ id: 'today', due: t(2026, 10, 24) }),
      task({ id: 'tom', due: t(2026, 10, 25) }),
      task({ id: 'later', due: t(2026, 11, 3) }),
    ];
    expect(plannedGroups(open, now).map((g) => [g.label, g.tasks.map((x) => x.id)])).toEqual([
      ['In ritardo', ['late']],
      ['Oggi', ['today']],
      ['Domani', ['tom']],
      ['Più avanti', ['later']],
    ]);
    expect(dueInfo(t(2026, 10, 23), now)).toEqual({ text: 'Scaduta ieri', state: 'scaduta' });
    expect(dueInfo(t(2026, 10, 24), now).state).toBe('oggi');
    expect(dueInfo(t(2026, 10, 30), now).text).toBe('Scade ven 30 ott');
    expect(dueChoices(now).map((c) => c.label)).toEqual(['Oggi', 'Domani', 'Settimana prossima']);
    expect(new Date(dueChoices(now)[2].at).getDay()).toBe(1);
    // Alle 16 c'è ancora tempo per "Più tardi oggi" (alle 19).
    expect(reminderChoices(now)[0]).toMatchObject({ label: 'Più tardi oggi', hint: '19:00' });
    expect(reminderChoices(t(2026, 10, 24, 22))[0].label).toBe('Domani');
  });

  it('ripetizioni', () => {
    expect(nextTaskDate(t(2026, 10, 23), 'weekdays')).toBe(t(2026, 10, 26));
    expect(nextTaskDate(t(2026, 1, 31), 'monthly')).toBe(t(2026, 2, 28));
    expect(nextTaskDate(t(2028, 2, 29), 'yearly')).toBe(t(2029, 2, 28));
  });
});

describe('attività: modulo', () => {
  it('aggiunge, completa una ripetuta, sposta ed elimina', async () => {
    const s = await fresh();
    s.setModule('attivita');
    await s.tasks.reload();
    const tk = s.tasks;
    tk.setView('giornata');
    const before = tk.shown.open.length;
    expect(await tk.add('Chiamare il fornitore di lampade')).toBe(true);
    expect(tk.shown.open.length).toBe(before + 1);
    expect(tk.shown.open[0].myDay).toBe(dayKey(Date.now()));

    // In Importanti nasce importante.
    tk.setView('importanti');
    await tk.add('Firmare il contratto');
    expect(tk.items.find((x) => x.title === 'Firmare il contratto')?.important).toBe(true);

    // Completare la ripetuta crea la prossima.
    const monthly = tk.items.find((x) => x.id === 't3')!;
    await tk.toggleDone(monthly);
    const next = tk.items.find((x) => x.title === monthly.title && !x.done)!;
    expect(next).toBeTruthy();
    expect(next.due).toBeGreaterThan(monthly.due!);
    expect(tk.items.find((x) => x.id === 't3')?.done).toBe(true);

    // Passaggi.
    const t1 = tk.items.find((x) => x.id === 't1')!;
    await tk.addStep(t1, 'Mandare le foto');
    expect(tk.items.find((x) => x.id === 't1')!.steps.map((x) => x.title)).toContain('Mandare le foto');

    // Nuovo elenco e spostamento.
    tk.startNewList();
    tk.listEdit!.name = 'Fiera di gennaio';
    await tk.commitList();
    const fiera = tk.lists.find((l) => l.name === 'Fiera di gennaio')!;
    expect(tk.view).toBe(`elenco:${fiera.id}`);
    await tk.moveTo(tk.items.find((x) => x.id === 't5')!, fiera.id);
    expect(tk.shown.open.map((x) => x.id)).toEqual(['t5']);

    // Eliminare l'elenco chiede conferma ed elimina le sue attività.
    const del = tk.deleteList(fiera);
    for (let i = 0; i < 10 && !s.confirm; i++) await new Promise((r) => setTimeout(r, 0));
    expect(s.confirm?.text).toMatch(/l’attività che contiene/);
    s.answer(true);
    await del;
    expect(tk.items.some((x) => x.id === 't5')).toBe(false);
    expect(tk.view).toBe('attivita');
  });

  it('posta contrassegnata: completare toglie il contrassegno', async () => {
    const s = await fresh();
    await s.tasks.reload();
    const tk = s.tasks;
    expect(tk.flagged.map((m) => m.id)).toEqual([2]);
    await tk.unflag(tk.flagged[0]);
    expect(tk.flagged).toEqual([]);
    expect((await s.backend.flaggedMessages()).length).toBe(0);
    expect(s.list.find((m) => m.id === 2)?.flagged).toBe(false);

    // Un messaggio diventa un'attività di La mia giornata.
    await tk.fromMessage(s.list.find((m) => m.id === 1)!);
    const made = tk.items.find((x) => x.message === 1)!;
    expect(made.title).toBe('Preventivo uffici Bassano, revisione 3');
    expect(made.myDay).toBe(dayKey(Date.now()));
  });

  it('promemoria delle attività', async () => {
    const s = await fresh();
    const tk = s.tasks;
    await tk.reload();
    const t1 = tk.items.find((x) => x.id === 't1')!;
    await tk.setReminder(t1, Date.now() - 60_000);
    await tk.checkReminders(Date.now());
    expect(tk.reminders.map((x) => x.id)).toEqual(['t1']);
    tk.dismiss(tk.reminders[0]);
    expect(tk.reminders).toEqual([]);
    // Ignorato: non torna.
    await tk.checkReminders(Date.now() + 1000);
    expect(tk.reminders).toEqual([]);
  });
});
