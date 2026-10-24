<script lang="ts">
  // Riquadro dell'attività aperta: titolo, passaggi, La mia giornata,
  // promemoria, scadenza, ripetizione, elenco e note. Ogni modifica si salva da sola.
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import { store } from '../lib/store.svelte';
  import { REPEATS, dayKey, dayWord, dueChoices, dueInfo, reminderChoices, reminderWord, repeatLabel } from '../lib/tasks';
  import { dateInput, fromInputs, timeInput } from '../lib/calendar';
  import type { MenuItem } from '../lib/menus';
  import type { Task } from '../lib/types';

  let { task }: { task: Task } = $props();
  const tk = store.tasks;

  let title = $state('');
  let notes = $state('');
  let stepDraft = $state('');
  let loadedId = '';
  // Cambiando attività i campi ripartono da lei; durante la scrittura non si toccano.
  $effect(() => {
    if (task.id !== loadedId) {
      loadedId = task.id;
      title = task.title;
      notes = task.notes;
      stepDraft = '';
      picking = null;
    }
  });

  function saveTitle() {
    const t = title.trim();
    if (!t) title = task.title;
    else if (t !== task.title) void tk.update(task.id, { title: t });
  }

  function saveNotes() {
    if (notes !== task.notes) void tk.update(task.id, { notes });
  }

  async function addStep(e: SubmitEvent) {
    e.preventDefault();
    if (!stepDraft.trim()) return;
    await tk.addStep(task, stepDraft);
    stepDraft = '';
  }

  const today = $derived(dayKey(tk.now));
  const inMyDay = $derived(task.myDay === today);
  const due = $derived(task.due !== null ? dueInfo(task.due, tk.now) : null);

  // Scelta di una data a mano, sotto la riga.
  let picking = $state<'due' | 'reminder' | null>(null);
  let pickDate = $state('');
  let pickTime = $state('09:00');
  function startPick(kind: 'due' | 'reminder') {
    picking = kind;
    const base = kind === 'due' ? (task.due ?? Date.now()) : (task.reminder ?? Date.now() + 3_600_000);
    pickDate = dateInput(base);
    pickTime = kind === 'reminder' ? timeInput(task.reminder ?? new Date().setMinutes(0, 0, 0) + 3_600_000) : '09:00';
  }
  function applyPick() {
    const at = fromInputs(pickDate, picking === 'reminder' ? pickTime : '00:00');
    if (at === null) return;
    if (picking === 'due') void tk.setDue(task, at);
    else void tk.setReminder(task, at);
    picking = null;
  }

  let menu = $state<{ el: HTMLElement; items: MenuItem[]; label: string } | null>(null);
  function open(e: MouseEvent, kind: 'due' | 'reminder' | 'repeat') {
    const el = e.currentTarget as HTMLElement;
    if (menu?.el === el) {
      menu = null;
      return;
    }
    const now = Date.now();
    if (kind === 'due') {
      menu = {
        el,
        label: 'Scadenza',
        items: [
          ...dueChoices(now).map((c) => ({ label: `${c.label}`, keys: c.hint, icon: 'calendar-due' as const, onclick: () => tk.setDue(task, c.at) })),
          { kind: 'sep', label: '' },
          { label: 'Scegli una data…', icon: 'calendar-event', onclick: () => startPick('due') },
        ],
      };
    } else if (kind === 'reminder') {
      menu = {
        el,
        label: 'Promemoria',
        items: [
          ...reminderChoices(now).map((c) => ({ label: c.label, keys: c.hint, icon: 'bell' as const, onclick: () => tk.setReminder(task, c.at) })),
          { kind: 'sep', label: '' },
          { label: 'Scegli data e ora…', icon: 'calendar-event', onclick: () => startPick('reminder') },
        ],
      };
    } else {
      menu = {
        el,
        label: 'Ripeti',
        items: REPEATS.map(([r, label]) => ({ label, checked: task.repeat === r, onclick: () => tk.setRepeat(task, r) })),
      };
    }
  }

  /** Il titolo cresce con il testo, su più righe. */
  function autosize(el: HTMLTextAreaElement, _v: string) {
    const fit = () => {
      el.style.height = 'auto';
      el.style.height = `${el.scrollHeight}px`;
    };
    fit();
    return { update: () => queueMicrotask(fit) };
  }

  const created = $derived(task.done && task.doneAt ? `Completata ${dayWord(task.doneAt, tk.now).toLowerCase()}` : `Creata ${dayWord(task.created, tk.now).toLowerCase()}`);
</script>

<div class="td">
  <div class="scroll inner">
    <div class="block head">
      <button class="chk" role="checkbox" aria-checked={task.done} aria-label="Completata" title={task.done ? 'Segna come da fare' : 'Segna come completata'} onclick={() => tk.toggleDone(task)}>
        {#if task.done}<Icon name="circle-check-filled" size={22} class="on" />{:else}<Icon name="circle" size={22} class="c-off" /><Icon name="circle-check" size={22} class="c-on" />{/if}
      </button>
      <textarea
        class="title"
        rows="1"
        aria-label="Titolo dell’attività"
        data-done={task.done ? '' : undefined}
        bind:value={title}
        use:autosize={title}
        maxlength="400"
        onblur={saveTitle}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            (e.currentTarget as HTMLTextAreaElement).blur();
          } else if (e.key === 'Escape') {
            e.stopPropagation();
            title = task.title;
            (e.currentTarget as HTMLTextAreaElement).blur();
          }
        }}
      ></textarea>
      <button class="star" aria-pressed={task.important} aria-label="Importante" title={task.important ? 'Togli importanza' : 'Segna come importante'} onclick={() => tk.toggleImportant(task)}>
        <Icon name={task.important ? 'star-filled' : 'star'} size={20} />
      </button>
    </div>

    <div class="block steps-b">
      {#if task.steps.length}
        <ul class="steps" aria-label="Passaggi">
          {#each task.steps as s (s.id)}
            <li class="step">
              <button class="chk sm" role="checkbox" aria-checked={s.done} aria-label="Passaggio completato: {s.title}" onclick={() => tk.toggleStep(task, s.id)}>
                {#if s.done}<Icon name="circle-check-filled" size={18} class="on" />{:else}<Icon name="circle" size={18} class="c-off" /><Icon name="circle-check" size={18} class="c-on" />{/if}
              </button>
              <input class="st" data-done={s.done ? '' : undefined} value={s.title} aria-label="Passaggio" maxlength="400" onchange={(e) => tk.renameStep(task, s.id, e.currentTarget.value)} onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()} />
              <button class="ibtn btn rm" aria-label="Togli il passaggio {s.title}" title="Togli il passaggio" onclick={() => tk.removeStep(task, s.id)}><Icon name="x" size={14} /></button>
            </li>
          {/each}
        </ul>
      {/if}
      <form class="addstep" onsubmit={addStep}>
        <Icon name="plus" size={18} class="c-brand" />
        <input bind:value={stepDraft} placeholder={task.steps.length ? 'Passaggio successivo' : 'Aggiungi un passaggio'} aria-label="Aggiungi un passaggio" maxlength="400" />
      </form>
    </div>

    <div class="block">
      <div class="tdo" data-on={inMyDay ? '' : undefined}>
        <button class="ob" onclick={() => tk.toggleMyDay(task)}><Icon name="sun" size={18} />{inMyDay ? 'Aggiunta a La mia giornata' : 'Aggiungi a La mia giornata'}</button>
        {#if inMyDay}<button class="ibtn btn x" aria-label="Togli da La mia giornata" title="Togli da La mia giornata" onclick={() => tk.toggleMyDay(task)}><Icon name="x" size={14} /></button>{/if}
      </div>
    </div>

    <div class="block">
      <div class="tdo" data-on={task.reminder !== null ? '' : undefined}>
        <button class="ob" aria-haspopup="menu" onclick={(e) => open(e, 'reminder')}><Icon name="bell" size={18} />{task.reminder !== null ? `Promemoria: ${reminderWord(task.reminder, tk.now)}` : 'Ricordamelo'}</button>
        {#if task.reminder !== null}<button class="ibtn btn x" aria-label="Togli il promemoria" title="Togli il promemoria" onclick={() => tk.setReminder(task, null)}><Icon name="x" size={14} /></button>{/if}
      </div>
      {#if picking === 'reminder'}
        <form class="pick" onsubmit={(e) => { e.preventDefault(); applyPick(); }}>
          <input type="date" aria-label="Giorno del promemoria" bind:value={pickDate} />
          <input type="time" aria-label="Ora del promemoria" step="300" bind:value={pickTime} />
          <button class="primary-btn" type="submit">Imposta</button>
          <button class="sec-btn" type="button" onclick={() => (picking = null)}>Annulla</button>
        </form>
      {/if}
      <div class="tdo" data-on={due ? '' : undefined} data-state={due?.state}>
        <button class="ob" aria-haspopup="menu" onclick={(e) => open(e, 'due')}><Icon name="calendar-due" size={18} />{due ? due.text : 'Aggiungi una scadenza'}</button>
        {#if due}<button class="ibtn btn x" aria-label="Togli la scadenza" title="Togli la scadenza" onclick={() => tk.setDue(task, null)}><Icon name="x" size={14} /></button>{/if}
      </div>
      {#if picking === 'due'}
        <form class="pick" onsubmit={(e) => { e.preventDefault(); applyPick(); }}>
          <input type="date" aria-label="Giorno di scadenza" bind:value={pickDate} />
          <button class="primary-btn" type="submit">Imposta</button>
          <button class="sec-btn" type="button" onclick={() => (picking = null)}>Annulla</button>
        </form>
      {/if}
      <div class="tdo" data-on={task.repeat ? '' : undefined}>
        <button class="ob" aria-haspopup="menu" onclick={(e) => open(e, 'repeat')}><Icon name="repeat" size={18} />{task.repeat ? repeatLabel(task.repeat) : 'Ripeti'}</button>
        {#if task.repeat}<button class="ibtn btn x" aria-label="Non ripetere" title="Non ripetere" onclick={() => tk.setRepeat(task, null)}><Icon name="x" size={14} /></button>{/if}
      </div>
    </div>

    <div class="block">
      <label class="tdo lst">
        <span class="ob static"><Icon name="list" size={18} />Elenco</span>
        <select aria-label="Elenco dell’attività" value={task.list} onchange={(e) => tk.moveTo(task, e.currentTarget.value)}>
          <option value="">Attività</option>
          {#each tk.lists as l (l.id)}<option value={l.id}>{l.name}</option>{/each}
        </select>
      </label>
      {#if task.message !== null}
        {@const mid = task.message}
        <div class="tdo">
          <button class="ob" onclick={async () => { const m = await store.backend.message(mid, false).catch(() => null); if (m) await store.revealMessage(m); else store.flash({ kind: 'info', text: 'Il messaggio non c’è più.' }, 3000); }}><Icon name="mail" size={18} />Apri il messaggio</button>
        </div>
      {/if}
    </div>

    <label class="block notes">
      <span class="sr">Note</span>
      <textarea bind:value={notes} placeholder="Aggiungi una nota" rows="5" onblur={saveNotes}></textarea>
    </label>
  </div>

  <footer class="foot">
    <button class="ibtn btn" aria-label="Chiudi i dettagli" title="Chiudi i dettagli (Esc)" onclick={() => (tk.sel = null)}><Icon name="layout-sidebar-right" size={18} /></button>
    <span class="when">{created}</span>
    <button class="ibtn btn" aria-label="Elimina attività" title="Elimina attività" onclick={() => tk.remove(task)}><Icon name="trash" size={18} /></button>
  </footer>
</div>

{#if menu}
  <Menu anchor={menu.el} items={menu.items} label={menu.label} onclose={() => (menu = null)} />
{/if}

<style>
  .td { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--ground); }
  .inner { flex: 1; min-height: 0; overflow: auto; padding: 12px 12px 20px; display: flex; flex-direction: column; gap: 8px; }
  .block { border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule); }
  .head { display: flex; align-items: flex-start; gap: 4px; padding: 10px 6px 10px 6px; }
  .chk { width: 36px; height: 36px; flex: none; display: grid; place-items: center; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--ink-2); }
  .chk.sm { width: 30px; height: 30px; }
  .chk :global(.c-on) { display: none; color: var(--brand-ink); }
  .chk :global(.on) { color: var(--brand-ink); }
  .chk:hover :global(.c-on), .chk:focus-visible :global(.c-on) { display: block; }
  .chk:hover :global(.c-off), .chk:focus-visible :global(.c-off) { display: none; }
  .title { flex: 1; min-width: 0; margin-top: 4px; padding: 2px 4px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font: inherit; font-size: 17px; font-weight: 600; line-height: 1.35; resize: none; overflow: hidden; }
  .title:focus { outline: none; box-shadow: inset 0 -2px 0 var(--brand); }
  .title[data-done] { color: var(--ink-2); text-decoration: line-through; }
  .star { width: 36px; height: 36px; flex: none; display: grid; place-items: center; padding: 0; border: 0; border-radius: 4px; background: transparent; color: var(--ink-3); }
  .star:hover { background: var(--hover); color: var(--ink-2); }
  .star[aria-pressed="true"] { color: var(--brand-ink); }
  .steps-b { padding: 4px 6px; }
  .steps { margin: 0; padding: 0; list-style: none; }
  .step { display: flex; align-items: center; gap: 4px; }
  .st { flex: 1; min-width: 0; height: 30px; padding: 0 4px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font: inherit; font-size: 13.5px; }
  .st:focus { outline: none; box-shadow: inset 0 -2px 0 var(--brand); }
  .st[data-done] { color: var(--ink-2); text-decoration: line-through; }
  .rm { width: 28px; height: 28px; opacity: 0; }
  .step:hover .rm, .rm:focus-visible { opacity: 1; }
  .addstep { height: 36px; display: flex; align-items: center; gap: 10px; padding: 0 6px 0 8px; }
  .addstep :global(svg.c-brand) { color: var(--brand-ink); }
  .addstep input { flex: 1; min-width: 0; height: 30px; border: 0; background: transparent; color: var(--ink); font: inherit; font-size: 13.5px; }
  .addstep input::placeholder { color: var(--brand-ink); opacity: 1; }
  .addstep input:focus { outline: none; }
  .addstep input:focus::placeholder { color: var(--ink-3); }
  .tdo { display: flex; align-items: center; min-height: 44px; padding-right: 6px; }
  .tdo + .tdo { box-shadow: inset 0 1px 0 var(--rule); }
  .ob { flex: 1; min-width: 0; height: 44px; display: flex; align-items: center; gap: 12px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: var(--ink-2); font-size: 13.5px; text-align: left; }
  button.ob:hover { background: var(--hover); color: var(--ink); }
  .ob :global(svg) { flex: none; color: var(--ink-3); }
  .tdo[data-on] .ob { color: var(--brand-ink); }
  .tdo[data-on] .ob :global(svg) { color: var(--brand-ink); }
  .tdo[data-state="scaduta"] .ob, .tdo[data-state="scaduta"] .ob :global(svg) { color: var(--danger); }
  .x { width: 28px; height: 28px; flex: none; }
  .lst select { height: 30px; max-width: 170px; margin-left: auto; padding: 0 8px; border: 0; border-radius: 4px; background-color: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .static { cursor: default; }
  .pick { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 4px 12px 12px 42px; }
  .pick input { height: 30px; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -1px 0 var(--ink-3); color: var(--ink); font: inherit; font-size: 13.5px; }
  .pick input:focus { outline: none; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .pick .primary-btn, .pick .sec-btn { height: 30px; padding: 0 12px; font-size: 13.5px; }
  .notes { display: block; padding: 0; }
  .notes textarea { display: block; width: 100%; box-sizing: border-box; min-height: 110px; padding: 10px 12px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font: inherit; font-size: 13.5px; line-height: 1.5; resize: vertical; }
  .notes textarea:focus { outline: none; box-shadow: inset 0 -2px 0 var(--brand); }
  .foot { height: 44px; flex: none; display: flex; align-items: center; gap: 6px; padding: 0 6px; box-shadow: inset 0 1px 0 var(--rule-2); background: var(--sheet); }
  .when { flex: 1; min-width: 0; text-align: center; font-size: 12.5px; color: var(--ink-2); }
</style>
