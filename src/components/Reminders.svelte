<script lang="ts">
  // Promemoria degli appuntamenti, in basso a destra come la finestra dei
  // promemoria di Outlook: Apri, Posticipa, Ignora. L'avviso sul desktop parte
  // insieme (calendar.svelte.ts).
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { timeText, untilText } from '../lib/calendar';
  import { dueInfo } from '../lib/tasks';

  const cal = store.cal;
  const tk = store.tasks;
  const total = $derived(cal.reminders.length + tk.reminders.length);
  function dismissAll() {
    cal.dismissAll();
    tk.dismissAll();
  }
  const SNOOZE: [number, string][] = [
    [5, '5 minuti'],
    [10, '10 minuti'],
    [15, '15 minuti'],
    [30, '30 minuti'],
    [60, '1 ora'],
  ];
  let snoozeFor = $state(5);
</script>

{#if total}
  <section class="rem rise" aria-labelledby="rem-h" aria-live="assertive">
    <header class="rem-h">
      <Icon name="bell-ringing" size={18} style="color:var(--brand-ink)" />
      <h2 id="rem-h">{total === 1 ? 'Promemoria' : `${total} promemoria`}</h2>
      {#if total > 1}<button class="lbtn all" onclick={dismissAll}>Ignora tutti</button>{/if}
    </header>
    <ul class="scroll">
      {#each cal.reminders as r (r.key)}
        {@const o = r.occ}
        <li style={cal.tone(o.ev)}>
          <span class="sw" aria-hidden="true"></span>
          <div class="txt">
            <p class="t">{o.ev.title || 'Senza titolo'}</p>
            <p class="w">{untilText(o.start, cal.now)}, {o.ev.allDay ? 'tutto il giorno' : `${timeText(o.start)}-${timeText(o.end)}`}</p>
            {#if o.ev.location}<p class="w">{o.ev.location}</p>{/if}
            <div class="acts">
              <button class="sec-btn" onclick={() => cal.showReminder(r)}>Apri</button>
              <button class="sec-btn" onclick={() => cal.snooze(r.key, snoozeFor)} title="Ricorda di nuovo tra {SNOOZE.find(([m]) => m === snoozeFor)?.[1]}">Posticipa</button>
              <button class="sec-btn" onclick={() => cal.dismiss(r.key)}>Ignora</button>
            </div>
          </div>
        </li>
      {/each}
      {#each tk.reminders as t (t.id)}
        <li class="task">
          <Icon name="circle-check" size={16} style="color:var(--brand-ink);margin-top:2px;flex:none" />
          <div class="txt">
            <p class="t">{t.title}</p>
            <p class="w">Attività{t.due !== null ? `, ${dueInfo(t.due, cal.now).text.toLowerCase()}` : ''}</p>
            <div class="acts">
              <button class="sec-btn" onclick={() => tk.showReminder(t)}>Apri</button>
              <button class="sec-btn" onclick={() => tk.completeFromReminder(t)}>Completata</button>
              <button class="sec-btn" onclick={() => tk.snooze(t, snoozeFor)} title="Ricorda di nuovo tra {SNOOZE.find(([m]) => m === snoozeFor)?.[1]}">Posticipa</button>
              <button class="sec-btn" onclick={() => tk.dismiss(t)}>Ignora</button>
            </div>
          </div>
        </li>
      {/each}
    </ul>
    <footer class="rem-f">
      <label>Posticipa di
        <select bind:value={snoozeFor}>
          {#each SNOOZE as [m, l] (m)}<option value={m}>{l}</option>{/each}
        </select>
      </label>
    </footer>
  </section>
{/if}

<style>
  .rem { position: fixed; right: 18px; bottom: 36px; z-index: 45; width: 360px; max-width: calc(100vw - 36px); max-height: min(460px, calc(100vh - 120px)); display: flex; flex-direction: column; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow), 0 0 0 1px var(--rule-2); overflow: hidden; }
  .rem-h { display: flex; align-items: center; gap: 10px; padding: 12px 14px 8px 16px; }
  .rem-h h2 { flex: 1; margin: 0; font-size: 15px; font-weight: 600; }
  .all { height: 28px; padding: 0 8px; font-size: 13px; font-weight: 600; }
  .all:hover { background: var(--hover); }
  ul { margin: 0; padding: 0 8px; list-style: none; overflow: auto; }
  li { --ev-line: var(--brand); display: flex; gap: 12px; padding: 10px 8px 12px; }
  li + li { box-shadow: inset 0 1px 0 var(--rule); }
  .sw { width: 10px; height: 10px; flex: none; margin-top: 5px; border-radius: 3px; background: var(--ev-line); }
  .txt { flex: 1; min-width: 0; }
  .t { margin: 0; font-size: 14.5px; font-weight: 600; overflow-wrap: anywhere; }
  .w { margin: 2px 0 0; font-size: 13px; color: var(--ink-2); font-variant-numeric: tabular-nums; }
  .acts { margin-top: 10px; display: flex; flex-wrap: wrap; gap: 6px; }
  .acts .sec-btn { height: 30px; padding: 0 12px; font-size: 13.5px; }
  .rem-f { padding: 8px 16px 12px; box-shadow: inset 0 1px 0 var(--rule); font-size: 13px; color: var(--ink-2); }
  .rem-f label { display: flex; align-items: center; gap: 8px; }
  .rem-f select { height: 28px; padding: 0 6px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13px; }
</style>
