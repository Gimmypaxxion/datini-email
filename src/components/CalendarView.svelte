<script lang="ts">
  // Calendario (Ctrl+2), come in Outlook: a sinistra il mese per spostarsi e i
  // prossimi appuntamenti; a destra la barra (Oggi, avanti, indietro, vista) e
  // il giorno, la settimana o il mese. Con la ricerca, l'elenco dei risultati.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import Popover from './Popover.svelte';
  import Toast from './Toast.svelte';
  import CalGrid from './CalGrid.svelte';
  import CalMonth from './CalMonth.svelte';
  import EventPeek from './EventPeek.svelte';
  import { store } from '../lib/store.svelte';
  import { addMonths, dayText, GIORNI, MESI, monthGrid, sameDay, startOfDay, timeText, untilText, VIEW_NAMES, whenText, type CalView, type Occurrence } from '../lib/calendar';

  const cal = store.cal;
  const VIEWS: [CalView, IconName][] = [
    ['giorno', 'calendar-due'],
    ['settimana-lavorativa', 'briefcase'],
    ['settimana', 'calendar-week'],
    ['mese', 'calendar-month'],
  ];

  // ---------- Mini calendario ----------
  let mini = $state(startOfDay(Date.now()));
  let lastAnchor = 0;
  $effect(() => {
    // Segue il giorno mostrato, ma si può sfogliare da solo.
    if (cal.anchor !== lastAnchor) {
      lastAnchor = cal.anchor;
      mini = cal.anchor;
    }
  });
  const miniDays = $derived(monthGrid(mini));
  const miniMonth = $derived(new Date(mini).getMonth());
  const inView = (d: number) => d >= cal.range.from && d < cal.range.to;

  // ---------- Scheda rapida ----------
  let peek = $state<{ occ: Occurrence; el: HTMLElement } | null>(null);
  function pick(o: Occurrence, el: HTMLElement) {
    cal.sel = o.key;
    peek = { occ: o, el };
  }
  function open(o: Occurrence) {
    peek = null;
    void cal.open(o);
  }

  function goSearch(o: Occurrence) {
    void cal.search('');
    cal.goTo(o.start);
    cal.sel = o.key;
  }

  const upcomingDay = (o: Occurrence) => (sameDay(o.start, cal.now) ? 'Oggi' : sameDay(o.start, cal.now + 86_400_000) ? 'Domani' : dayText(o.start, false));
</script>

<aside class="cal-side scroll" aria-label="Navigazione del calendario">
  <section class="mini" aria-label="Scegli il giorno">
    <div class="mini-h">
      <h2>{MESI[miniMonth]} {new Date(mini).getFullYear()}</h2>
      <button class="ibtn btn" onclick={() => (mini = addMonths(mini, -1))} aria-label="Mese precedente" title="Mese precedente"><Icon name="chevron-up" size={16} /></button>
      <button class="ibtn btn" onclick={() => (mini = addMonths(mini, 1))} aria-label="Mese successivo" title="Mese successivo"><Icon name="chevron-down" size={16} /></button>
    </div>
    <div class="mini-g" role="grid" aria-label="{MESI[miniMonth]} {new Date(mini).getFullYear()}">
      {#each ['L', 'M', 'M', 'G', 'V', 'S', 'D'] as w, i (i)}<span class="mw" aria-hidden="true">{w}</span>{/each}
      {#each miniDays as d (d)}
        {@const dt = new Date(d)}
        <button
          class="md"
          data-out={dt.getMonth() !== miniMonth ? 'si' : 'no'}
          data-today={sameDay(d, cal.now) ? 'si' : 'no'}
          data-view={inView(d) ? 'si' : 'no'}
          aria-label="{GIORNI[dt.getDay()]} {dt.getDate()} {MESI[dt.getMonth()]}"
          aria-current={sameDay(d, cal.now) ? 'date' : undefined}
          onclick={() => cal.goTo(d)}
        >{dt.getDate()}</button>
      {/each}
    </div>
  </section>

  <section class="up" aria-labelledby="up-h">
    <h3 id="up-h">Prossimi appuntamenti</h3>
    {#if cal.upcoming.length}
      <ul>
        {#each cal.upcoming as o (o.key)}
          <li>
            <button class="up-it btn" style={cal.tone(o.ev)} onclick={() => { cal.goTo(o.start); cal.sel = o.key; }}>
              <span class="up-sw" aria-hidden="true"></span>
              <span class="up-t">
                <span class="up-n">{o.ev.title || 'Senza titolo'}</span>
                <span class="up-w">{upcomingDay(o)}, {o.ev.allDay ? 'tutto il giorno' : `${timeText(o.start)}-${timeText(o.end)}`}{o.start <= cal.now && o.end > cal.now ? ' (adesso)' : ''}</span>
              </span>
            </button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="up-none">Nessun appuntamento nei prossimi 7 giorni.</p>
    {/if}
  </section>

  <section class="cals" aria-labelledby="cals-h">
    <h3 id="cals-h">I miei calendari</h3>
    <div class="cal-it">
      <span class="cal-sw" aria-hidden="true"></span>
      <span class="cal-t"><span>Calendario</span><span class="cal-d">Su questo computer</span></span>
    </div>
    <button class="lbtn imp" onclick={() => cal.importIcs()}><Icon name="file-import" size={16} />Importa un calendario (.ics)</button>
  </section>
</aside>

<section class="cal-main" aria-label="Calendario">
  <div class="bar">
    <button class="sec-btn today" onclick={() => cal.today()} title="Vai a oggi"><Icon name="calendar-event" size={16} />Oggi</button>
    <button class="ibtn btn" onclick={() => cal.move(-1)} aria-label="Indietro" title="Indietro"><Icon name="chevron-left" size={18} /></button>
    <button class="ibtn btn" onclick={() => cal.move(1)} aria-label="Avanti" title="Avanti"><Icon name="chevron-right" size={18} /></button>
    <h1 aria-live="polite">{cal.query.trim() ? 'Risultati della ricerca' : cal.title}</h1>
    <div class="views" role="group" aria-label="Vista">
      {#each VIEWS as [v, icon], i (v)}
        <button class="vbtn btn" aria-pressed={cal.view === v} onclick={() => cal.setView(v)} title="{VIEW_NAMES[v]} (Ctrl+Alt+{i + 1})"><Icon name={icon} size={16} /><span class="vl">{VIEW_NAMES[v]}</span></button>
      {/each}
    </div>
  </div>

  {#if cal.query.trim()}
    <div class="found scroll">
      {#if !cal.found.length}
        <p class="none">Nessun appuntamento corrisponde a “{cal.query.trim()}”.</p>
      {:else}
        <ul>
          {#each cal.found as o (o.key)}
            {@const d = new Date(o.start)}
            <li>
              <button class="f-it btn" style={cal.tone(o.ev)} onclick={() => goSearch(o)} ondblclick={() => open(o)}>
                <span class="f-d" aria-hidden="true"><span class="f-m">{MESI[d.getMonth()].slice(0, 3)}</span><span class="f-n">{d.getDate()}</span></span>
                <span class="f-t">
                  <span class="f-title">{o.ev.title || 'Senza titolo'}</span>
                  <span class="f-w">{whenText(o.start, o.end, o.ev.allDay, cal.now)}{o.ev.location ? `, ${o.ev.location}` : ''}</span>
                </span>
                {#if o.start > cal.now}<span class="f-u">{untilText(o.start, cal.now)}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else if cal.view === 'mese'}
    <CalMonth onpick={pick} onopen={open} />
  {:else}
    <CalGrid onpick={pick} onopen={open} />
  {/if}
  <Toast />
</section>

{#if peek}
  {@const live = cal.occs.find((o) => o.key === peek!.occ.key) ?? peek.occ}
  <Popover anchor={peek.el} label={live.ev.title || 'Appuntamento'} onclose={() => (peek = null)}>
    <EventPeek occ={live} onclose={() => (peek = null)} />
  </Popover>
{/if}

<style>
  .cal-side { width: 252px; flex: none; min-height: 0; overflow: auto; box-sizing: border-box; padding: 6px 8px 20px 4px; display: flex; flex-direction: column; gap: 22px; }
  .mini-h { display: flex; align-items: center; gap: 2px; padding-left: 8px; }
  .mini-h h2 { flex: 1; margin: 0; font-size: 14px; font-weight: 600; }
  .mini-h h2::first-letter { text-transform: uppercase; }
  .mini-h .ibtn { width: 30px; height: 30px; }
  .mini-g { margin-top: 4px; display: grid; grid-template-columns: repeat(7, 32px); justify-content: start; padding-left: 4px; }
  .mw { height: 26px; display: grid; place-items: center; font-size: 12px; color: var(--ink-3); }
  .md { width: 32px; height: 30px; padding: 0; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font-size: 13px; font-variant-numeric: tabular-nums; transition: background-color .15s ease-out; }
  .md:hover { background: var(--hover); }
  .md[data-out="si"] { color: var(--ink-3); }
  .md[data-view="si"] { background: var(--brand-sel); border-radius: 0; }
  .md[data-today="si"] { position: relative; color: var(--on-brand); font-weight: 600; background: transparent; z-index: 0; }
  .md[data-today="si"]::before { content: ""; position: absolute; inset: 2px 3px; z-index: -1; border-radius: 13px; background: var(--brand); }
  .md[data-today="si"][data-view="si"] { background: var(--brand-sel); }

  .up h3, .cals h3 { margin: 0 0 6px 8px; font-size: 13px; font-weight: 600; color: var(--ink); }
  .up ul { margin: 0; padding: 0; list-style: none; }
  .up-it { width: 100%; display: flex; align-items: flex-start; gap: 10px; padding: 7px 8px; border: 0; border-radius: 4px; background: transparent; text-align: left; color: var(--ink); }
  .up-sw { --ev-line: var(--brand); width: 8px; height: 8px; flex: none; margin-top: 6px; border-radius: 2px; background: var(--ev-line); }
  .up-t { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .up-n { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .up-w { font-size: 12.5px; color: var(--ink-2); font-variant-numeric: tabular-nums; }
  .up-none { margin: 0 8px; font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .cal-it { display: flex; align-items: center; gap: 10px; padding: 6px 8px; }
  .cal-sw { width: 14px; height: 14px; flex: none; border-radius: 3px; background: var(--brand); }
  .cal-t { display: flex; flex-direction: column; font-size: 13.5px; }
  .cal-d { font-size: 12.5px; color: var(--ink-3); }
  .imp { margin: 6px 0 0; height: 30px; display: inline-flex; align-items: center; gap: 8px; padding: 0 8px; font-size: 13.5px; }
  .imp:hover { background: var(--hover); }

  .cal-main { position: relative; flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; margin: 0 10px 10px 2px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .bar { flex: none; display: flex; align-items: center; gap: 4px; padding: 10px 12px 10px 14px; box-shadow: inset 0 -1px 0 var(--rule); }
  .today { margin-right: 6px; }
  .bar .ibtn { width: 32px; height: 32px; }
  h1 { flex: 1; min-width: 0; margin: 0 0 0 8px; font-size: 18px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .views { display: flex; gap: 2px; padding: 2px; border-radius: 6px; background: var(--rail); }
  .vbtn { height: 30px; display: flex; align-items: center; gap: 6px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink-2); font-size: 13px; white-space: nowrap; }
  .vbtn[aria-pressed="true"] { background: var(--sheet); color: var(--ink); font-weight: 600; box-shadow: var(--sheet-shadow); }
  .vbtn[aria-pressed="true"] :global(svg) { color: var(--brand-ink); }

  .found { flex: 1; min-height: 0; overflow: auto; padding: 8px 10px 70px; }
  .found ul { margin: 0; padding: 0; list-style: none; max-width: 860px; }
  .f-it { --ev-line: var(--brand); width: 100%; display: flex; align-items: center; gap: 14px; padding: 8px 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); text-align: left; }
  .f-d { width: 44px; flex: none; display: flex; flex-direction: column; align-items: center; padding: 4px 0; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--rule-2); }
  .f-title::before { content: ""; display: inline-block; width: 8px; height: 8px; margin-right: 8px; border-radius: 2px; background: var(--ev-line); vertical-align: 1px; }
  .f-m { font-size: 11.5px; color: var(--ink-2); }
  .f-n { font-size: 18px; font-weight: 600; line-height: 1.1; font-variant-numeric: tabular-nums; }
  .f-t { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .f-title { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .f-w { font-size: 13px; color: var(--ink-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .f-u { flex: none; font-size: 12.5px; color: var(--ink-3); }
  .none { margin: 18px 6px; font-size: 14px; color: var(--ink-2); }

  @media (max-width: 1280px) { .vl { display: none; } .vbtn { padding: 0 9px; } }
  @media (max-width: 980px) { .cal-side { display: none; } .cal-main { margin-left: 10px; } }
</style>
