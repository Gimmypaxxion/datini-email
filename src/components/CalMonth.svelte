<script lang="ts">
  // Vista mese: una casella per giorno con i primi appuntamenti; "Altri" e il
  // numero del giorno aprono la vista giorno. Doppio clic: evento di tutto il giorno.
  import { store } from '../lib/store.svelte';
  import { dayItems, GIORNI, isAllDayLike, sameDay, timeText, type Occurrence } from '../lib/calendar';

  let { onpick, onopen }: { onpick: (o: Occurrence, el: HTMLElement) => void; onopen: (o: Occurrence) => void } = $props();

  const MAX = 3;
  const cal = store.cal;
  const days = $derived(cal.range.days);
  const occs = $derived(cal.occs);
  const month = $derived(new Date(cal.anchor).getMonth());
  const heads = [1, 2, 3, 4, 5, 6, 0].map((d) => GIORNI[d]);

  const label = (o: Occurrence) => `${o.ev.title || 'Senza titolo'}, ${isAllDayLike(o) ? 'tutto il giorno' : `alle ${timeText(o.start)}`}`;
</script>

<div class="mo">
  <div class="mo-head" aria-hidden="true">
    {#each heads as h (h)}<span>{h}</span>{/each}
  </div>
  <div class="mo-grid" style:--rows={days.length / 7}>
    {#each days as d (d)}
      {@const items = dayItems(occs, d)}
      {@const dt = new Date(d)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="cell" data-out={dt.getMonth() !== month ? 'si' : 'no'} ondblclick={(e) => { if (!(e.target as HTMLElement).closest('button')) cal.newEvent({ start: d, allDay: true }); }}>
        <button class="cd btn" data-today={sameDay(d, cal.now) ? 'si' : 'no'} onclick={() => cal.goTo(d, 'giorno')} aria-label="Apri {GIORNI[dt.getDay()]} {dt.getDate()}">
          {dt.getDate() === 1 ? `${dt.getDate()} ${dt.toLocaleDateString('it-IT', { month: 'short' })}` : dt.getDate()}
        </button>
        <div class="items">
          {#each items.slice(0, items.length > MAX ? MAX - 1 : MAX) as o (o.key)}
            {@const all = isAllDayLike(o)}
            <button
              class="ev it"
              class:all
              style={cal.tone(o.ev)}
              data-sel={cal.sel === o.key ? 'si' : 'no'}
              data-state={cal.state(o.ev)}
              aria-label={label(o)}
              onclick={(e) => onpick(o, e.currentTarget)}
              ondblclick={() => onopen(o)}
            >{#if !all}<span class="tm">{timeText(Math.max(o.start, d))}</span>{/if}<span class="ev-t">{o.ev.title || 'Senza titolo'}</span></button>
          {/each}
          {#if items.length > MAX}
            <button class="more lbtn" onclick={() => cal.goTo(d, 'giorno')}>Altri {items.length - (MAX - 1)}</button>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .mo { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .mo-head { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); box-shadow: 0 1px 0 var(--rule); }
  .mo-head span { padding: 8px 10px; font-size: 12.5px; color: var(--ink-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mo-grid { flex: 1; min-height: 0; display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); grid-template-rows: repeat(var(--rows), minmax(0, 1fr)); }
  .cell { min-width: 0; min-height: 0; display: flex; flex-direction: column; gap: 2px; padding: 4px 4px 6px; box-shadow: inset 1px 0 0 var(--rule), inset 0 -1px 0 var(--rule); overflow: hidden; }
  .cell[data-out="si"] { background: var(--rail); }
  .cell[data-out="si"] .cd { color: var(--ink-3); }
  .cd { align-self: flex-start; min-width: 26px; height: 24px; padding: 0 6px; border: 0; border-radius: 12px; background: transparent; color: var(--ink); font-size: 13px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .cd[data-today="si"] { background: var(--brand); color: var(--on-brand); }
  .items { min-height: 0; display: flex; flex-direction: column; gap: 2px; }
  .it { height: 22px; flex: none; display: flex; align-items: center; gap: 6px; padding: 0 6px; white-space: nowrap; }
  .it:not(.all):not([data-state="needs-action"]) { background: transparent; box-shadow: none; color: var(--ink); }
  .it:not(.all):not([data-state="needs-action"]):hover { background: var(--hover); }
  .it:not(.all)[data-sel="si"] { background: var(--ev-bg); box-shadow: inset 0 0 0 2px var(--ev-line); }
  .it:not(.all)::before { content: ""; width: 8px; height: 8px; flex: none; border-radius: 2px; background: var(--ev-line); }
  .it .ev-t { min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .tm { flex: none; font-size: 12px; color: var(--ink-2); font-variant-numeric: tabular-nums; }
  .more { align-self: flex-start; height: 22px; padding: 0 6px; font-size: 12.5px; font-weight: 600; }
  .more:hover { background: var(--hover); }
</style>
