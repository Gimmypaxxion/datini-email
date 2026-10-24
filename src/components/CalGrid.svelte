<script lang="ts">
  // Giorno, settimana lavorativa e settimana: intestazione dei giorni e
  // striscia "tutto il giorno" ferme in alto, ore che scorrono sotto. Doppio
  // clic su un orario libero: nuovo appuntamento; clic su un appuntamento: scheda.
  import { store } from '../lib/store.svelte';
  import { allDayBars, DAY, GIORNI_BREVI, layoutDay, MIN, sameDay, timeText, type Occurrence } from '../lib/calendar';

  let { onpick, onopen }: { onpick: (o: Occurrence, el: HTMLElement) => void; onopen: (o: Occurrence) => void } = $props();

  const HOUR = 48;
  const WORK_FROM = 8;
  const WORK_TO = 18;

  const cal = store.cal;
  const days = $derived(cal.range.days);
  const occs = $derived(cal.occs);
  const bars = $derived(allDayBars(occs, days));
  const barRows = $derived(bars.reduce((n, b) => Math.max(n, b.row + 1), 0));
  const columns = $derived(days.map((d) => ({ day: d, placed: layoutDay(occs, d, 22) })));
  const hours = Array.from({ length: 24 }, (_, h) => h);

  let scroller: HTMLDivElement | undefined = $state();
  let scrolledFor = '';
  // All'apertura si parte dalle 7:30, o da poco prima dell'ora attuale.
  $effect(() => {
    const key = `${cal.view}:${days[0]}`;
    if (!scroller || key === scrolledFor) return;
    scrolledFor = key;
    const now = new Date(cal.now);
    const today = days.some((d) => sameDay(d, cal.now));
    const hour = today && now.getHours() >= 10 ? now.getHours() - 2 : 7.5;
    scroller.scrollTop = hour * HOUR;
  });

  const weekend = (d: number) => {
    const w = new Date(d).getDay();
    return w === 0 || w === 6;
  };

  function slotStart(day: number, e: MouseEvent): number {
    const el = e.currentTarget as HTMLElement;
    const y = e.clientY - el.getBoundingClientRect().top;
    const minutes = Math.max(0, Math.min(23.5 * 60, Math.floor(y / HOUR * 2) * 30));
    const d = new Date(day);
    d.setHours(0, minutes, 0, 0);
    return d.getTime();
  }

  function onslot(day: number, e: MouseEvent) {
    if ((e.target as HTMLElement).closest('.ev')) return;
    cal.newEvent({ start: slotStart(day, e) });
  }

  const label = (o: Occurrence) => `${o.ev.title || 'Senza titolo'}, ${o.ev.allDay ? 'tutto il giorno' : `dalle ${timeText(o.start)} alle ${timeText(o.end)}`}${o.ev.location ? `, ${o.ev.location}` : ''}`;
  const nowTop = $derived(((cal.now - new Date(cal.now).setHours(0, 0, 0, 0)) / MIN) * (HOUR / 60));
</script>

<div class="wk scroll" bind:this={scroller} style:--n={days.length} style:--hour="{HOUR}px">
  <div class="wk-top">
    <div class="corner" aria-hidden="true"></div>
    {#each days as d (d)}
      {@const dt = new Date(d)}
      <button class="dh btn" data-today={sameDay(d, cal.now) ? 'si' : 'no'} onclick={() => cal.goTo(d, 'giorno')} title="Mostra solo questo giorno">
        <span class="dn">{GIORNI_BREVI[dt.getDay()]}</span>
        <span class="dd">{dt.getDate()}</span>
      </button>
    {/each}
    <div class="ad-gutter" aria-hidden="true"></div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="ad" style:--rows={Math.max(1, barRows)} ondblclick={(e) => {
      if ((e.target as HTMLElement).closest('.ev')) return;
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const i = Math.min(days.length - 1, Math.max(0, Math.floor(((e.clientX - r.left) / r.width) * days.length)));
      cal.newEvent({ start: days[i], allDay: true });
    }}>
      {#each bars as b (b.occ.key)}
        <button
          class="ev bar"
          class:cut-s={b.cutStart}
          class:cut-e={b.cutEnd}
          style="{cal.tone(b.occ.ev)}; grid-column: {b.first + 1} / span {b.span}; grid-row: {b.row + 1}"
          data-sel={cal.sel === b.occ.key ? 'si' : 'no'}
          data-state={cal.state(b.occ.ev)}
          aria-label={label(b.occ)}
          onclick={(e) => onpick(b.occ, e.currentTarget)}
          ondblclick={() => onopen(b.occ)}
        ><span class="ev-t">{b.occ.ev.title || 'Senza titolo'}</span>{#if b.occ.ev.location}<span class="bl-loc">{b.occ.ev.location}</span>{/if}</button>
      {/each}
    </div>
  </div>

  <div class="wk-body">
    <div class="gutter" aria-hidden="true">
      {#each hours as h (h)}<span class="hl" style:top="{h * HOUR}px">{h ? `${String(h).padStart(2, '0')}:00` : ''}</span>{/each}
    </div>
    {#each columns as c (c.day)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="col" data-weekend={weekend(c.day) ? 'si' : 'no'} ondblclick={(e) => onslot(c.day, e)}>
        {#if !weekend(c.day)}<div class="work" style:top="{WORK_FROM * HOUR}px" style:height="{(WORK_TO - WORK_FROM) * HOUR}px" aria-hidden="true"></div>{/if}
        {#each c.placed as p (p.occ.key)}
          {@const short = p.height * (HOUR / 60) < 40}
          <button
            class="ev slot"
            class:short
            style="{cal.tone(p.occ.ev)}; top: {p.top * (HOUR / 60)}px; height: {Math.max(p.height * (HOUR / 60) - 2, 18)}px; left: calc({(p.col / p.cols) * 100}% + 1px); width: calc({100 / p.cols}% - 4px)"
            data-sel={cal.sel === p.occ.key ? 'si' : 'no'}
            data-state={cal.state(p.occ.ev)}
            aria-label={label(p.occ)}
            onclick={(e) => onpick(p.occ, e.currentTarget)}
            ondblclick={() => onopen(p.occ)}
          >
            <span class="ev-t">{p.occ.ev.title || 'Senza titolo'}</span>
            {#if short}
              {#if p.occ.ev.location}<span class="ev-s">, {p.occ.ev.location}</span>{/if}
            {:else}
              <span class="ev-l">{timeText(p.occ.start)}-{timeText(p.occ.end)}</span>
              {#if p.occ.ev.location}<span class="ev-l">{p.occ.ev.location}</span>{/if}
            {/if}
          </button>
        {/each}
        {#if sameDay(c.day, cal.now) && cal.now < c.day + DAY}
          <div class="now" style:top="{nowTop}px" aria-hidden="true"></div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .wk { flex: 1; min-height: 0; overflow: auto; position: relative; }
  .wk-top { position: sticky; top: 0; z-index: 3; display: grid; grid-template-columns: 56px repeat(var(--n), minmax(0, 1fr)); background: var(--sheet); box-shadow: 0 1px 0 var(--rule); }
  .corner { grid-row: 1; }
  .dh { min-width: 0; height: 52px; display: flex; flex-direction: column; align-items: flex-start; justify-content: center; gap: 1px; padding: 0 10px; border: 0; border-radius: 0; background: transparent; color: var(--ink-2); text-align: left; box-shadow: inset 1px 0 0 var(--rule); }
  .dn { font-size: 12.5px; }
  .dd { font-size: 20px; font-weight: 600; line-height: 1.1; color: var(--ink); font-variant-numeric: tabular-nums; }
  .dh[data-today="si"] .dn { color: var(--brand-ink); font-weight: 600; }
  .dh[data-today="si"] .dd { align-self: flex-start; min-width: 30px; height: 30px; margin-left: -4px; padding: 0 4px; box-sizing: border-box; display: grid; place-items: center; border-radius: 15px; background: var(--brand); color: var(--on-brand); font-size: 17px; }
  .ad-gutter { grid-column: 1; }
  .ad { grid-column: 2 / -1; display: grid; grid-template-columns: repeat(var(--n), minmax(0, 1fr)); grid-template-rows: repeat(var(--rows), 24px); row-gap: 2px; padding: 2px 0 6px; min-height: 26px; background-image: linear-gradient(90deg, var(--rule) 1px, transparent 1px); background-size: calc(100% / var(--n)) 100%; }
  .bar { min-width: 0; height: 24px; margin: 0 3px; display: flex; align-items: center; gap: 8px; padding: 0 8px; white-space: nowrap; }
  .bar.cut-s { margin-left: 0; border-top-left-radius: 0; border-bottom-left-radius: 0; }
  .bar.cut-e { margin-right: 0; border-top-right-radius: 0; border-bottom-right-radius: 0; }
  .bar .ev-t, .bl-loc { overflow: hidden; text-overflow: ellipsis; }
  .bl-loc { opacity: .8; }

  .wk-body { position: relative; display: grid; grid-template-columns: 56px repeat(var(--n), minmax(0, 1fr)); height: calc(24 * var(--hour)); }
  .gutter { position: relative; }
  .hl { position: absolute; right: 8px; transform: translateY(-50%); font-size: 12px; color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .col { position: relative; min-width: 0; background-color: var(--rail); background-image: linear-gradient(var(--rule) 1px, transparent 1px), linear-gradient(color-mix(in srgb, var(--rule) 55%, transparent) 1px, transparent 1px); background-size: 100% var(--hour), 100% var(--hour); background-position: 0 0, 0 calc(var(--hour) / 2); box-shadow: inset 1px 0 0 var(--rule); }
  .work { position: absolute; left: 1px; right: 0; background: var(--sheet); background-image: linear-gradient(var(--rule) 1px, transparent 1px), linear-gradient(color-mix(in srgb, var(--rule) 55%, transparent) 1px, transparent 1px); background-size: 100% var(--hour), 100% var(--hour); background-position: 0 0, 0 calc(var(--hour) / 2); }
  .slot { position: absolute; z-index: 1; display: flex; flex-direction: column; align-items: stretch; padding: 3px 7px; }
  .slot > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .slot.short { flex-direction: row; align-items: center; padding-block: 0; }
  .slot.short .ev-t { flex: none; max-width: 100%; }
  .ev-l { font-size: 12px; opacity: .85; font-variant-numeric: tabular-nums; }
  .ev-s { opacity: .85; }
  .slot[data-sel="si"] { z-index: 2; }
  .now { position: absolute; left: 0; right: 0; z-index: 2; height: 2px; margin-top: -1px; background: var(--danger); pointer-events: none; }
  .now::before { content: ""; position: absolute; left: -5px; top: -4px; width: 10px; height: 10px; border-radius: 50%; background: var(--danger); }
</style>
