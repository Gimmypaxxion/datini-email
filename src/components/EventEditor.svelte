<script lang="ts">
  // Modulo di un appuntamento o di una riunione (Ctrl+Maiusc+A, Ctrl+Maiusc+Q):
  // titolo, partecipanti, orari, ripetizione, luogo, promemoria, categoria e
  // note. Per una riunione organizzata da me il pulsante è Invia; per un invito
  // ricevuto si cambiano solo promemoria, categoria e note, e si risponde.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import {
    addDays,
    dateInput,
    durationText,
    fromInputs,
    MIN,
    partStatText,
    REMINDERS,
    repeatChoices,
    repeatKey,
    repeatText,
    startOfDay,
    timeInput,
  } from '../lib/calendar';
  import type { EventEdit } from '../lib/calendar.svelte';
  import { initials, parseRecipient, personLabel, validEmail } from '../lib/format';
  import type { InviteAction, PartStat, Person } from '../lib/types';

  const cal = store.cal;
  const ed = $derived(cal.edit);
  const ev = $derived(ed?.ev);
  const mine = $derived(ev ? cal.isMine(ev) : true);
  const ro = $derived(!!ev?.organizer && !mine);
  const meeting = $derived(!!ed && (ed.meeting || !!ev?.attendees.length));
  const sendable = $derived(meeting && mine && !!ev?.attendees.length);
  const senders = $derived(store.accounts.filter((a) => store.canSend(a)));
  const uid = Math.random().toString(36).slice(2, 8);

  let sd = $state('');
  let st = $state('');
  let endD = $state('');
  let et = $state('');
  let untilD = $state('');
  let rk = $state('mai');
  let err = $state('');
  let saving = $state(false);
  let who = $state('');
  let sugg = $state<{ items: Person[]; idx: number } | null>(null);
  let suggSeq = 0;
  let snapshot = '';
  let opened: EventEdit | null = null;
  let titleEl: HTMLInputElement | undefined = $state();
  let whoEl: HTMLInputElement | undefined = $state();
  let box: HTMLDivElement | undefined = $state();

  function fill() {
    if (!ev) return;
    sd = dateInput(ev.start);
    st = timeInput(ev.start);
    endD = dateInput(ev.allDay ? addDays(ev.end, -1) : ev.end);
    et = timeInput(ev.end);
    untilD = ev.repeat?.until ? dateInput(ev.repeat.until) : '';
    rk = repeatKey(ev.repeat, ev.start);
  }

  $effect(() => {
    const x = cal.edit;
    if (!x) {
      opened = null;
      return;
    }
    if (x === opened) return;
    opened = x;
    fill();
    err = '';
    who = '';
    sugg = null;
    snapshot = JSON.stringify(x.ev);
    setTimeout(() => (x.meeting && !x.ev.attendees.length && !x.ev.title ? whoEl : titleEl)?.focus(), 0);
  });

  const choices = $derived(ev ? repeatChoices(ev.start) : []);
  const duration = $derived(ev && !ev.allDay && ev.end > ev.start ? durationText(ev.end - ev.start) : '');
  const cats = $derived(store.categories);
  const catSel = $derived(ev ? (ev.categories.map((id) => cats.find((c) => c.id === id || c.name.toLowerCase() === id.toLowerCase())).find(Boolean)?.id ?? '') : '');

  function untilMs(): number | null {
    if (!untilD) return null;
    const t = fromInputs(untilD, '23:59');
    return t === null ? null : t + 59_999;
  }

  function onStart() {
    if (!ev) return;
    const ns = fromInputs(sd, ev.allDay ? '00:00' : st);
    if (ns === null) return;
    if (ev.allDay) {
      const days = Math.max(1, Math.round((startOfDay(ev.end) - startOfDay(ev.start)) / 86_400_000));
      ev.start = ns;
      ev.end = addDays(ns, days);
    } else {
      const dur = Math.max(0, ev.end - ev.start);
      ev.start = ns;
      ev.end = ns + dur;
    }
    // La ripetizione scelta segue il nuovo giorno (il martedì diventa mercoledì...).
    if (rk !== 'mai' && rk !== 'altra') setRepeat(rk);
    endD = dateInput(ev.allDay ? addDays(ev.end, -1) : ev.end);
    et = timeInput(ev.end);
  }

  function onEnd() {
    if (!ev) return;
    const ne = ev.allDay ? fromInputs(endD, '00:00') : fromInputs(endD, et);
    if (ne === null) return;
    ev.end = ev.allDay ? addDays(ne, 1) : ne;
  }

  function onAllDay(on: boolean) {
    if (!ev) return;
    ev.allDay = on;
    if (on) {
      const first = startOfDay(ev.start);
      const last = startOfDay(Math.max(ev.end - 1, ev.start));
      ev.start = first;
      ev.end = addDays(last, 1);
      if (ev.reminder === 15) ev.reminder = 1080;
    } else {
      ev.start = startOfDay(ev.start) + 9 * 60 * MIN;
      ev.end = ev.start + 30 * MIN;
      if (ev.reminder === 1080) ev.reminder = 15;
    }
    fill();
  }

  function setRepeat(key: string) {
    if (!ev) return;
    rk = key;
    if (key === 'altra') return;
    const c = repeatChoices(ev.start).find((x) => x.key === key);
    ev.repeat = c?.repeat ? { ...c.repeat, until: untilMs() } : null;
  }

  function onUntil() {
    if (ev?.repeat) ev.repeat = { ...ev.repeat, until: untilMs(), count: untilD ? null : ev.repeat.count };
  }

  // ---------- Partecipanti ----------

  async function onWho(value: string) {
    who = value;
    const seq = ++suggSeq;
    if (value.trim().length < 2) {
      sugg = null;
      return;
    }
    const items = await store.contacts(value);
    if (seq !== suggSeq || !ev) return;
    const chosen = new Set(ev.attendees.map((a) => a.email.toLowerCase()));
    const mineList = cal.myEmails.map((e) => e.toLowerCase());
    const fresh = items.filter((p) => !chosen.has(p.email.toLowerCase()) && !mineList.includes(p.email.toLowerCase()));
    sugg = fresh.length ? { items: fresh, idx: 0 } : null;
  }

  function addWho(p: Person | null = parseRecipient(who)) {
    if (!ev || !p) return;
    if (!ev.attendees.some((a) => a.email.toLowerCase() === p.email.toLowerCase())) ev.attendees.push({ name: p.name, email: p.email, status: 'needs-action', optional: false });
    who = '';
    sugg = null;
    suggSeq++;
  }

  function onWhoKey(e: KeyboardEvent) {
    if (sugg) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const n = sugg.items.length;
        sugg.idx = (sugg.idx + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        if (e.key === 'Enter') e.preventDefault();
        addWho(sugg.items[sugg.idx]);
        return;
      }
      if (e.key === 'Escape') {
        e.stopPropagation();
        sugg = null;
        return;
      }
    }
    if ((e.key === 'Enter' || e.key === ';' || e.key === ',') && who.trim()) {
      e.preventDefault();
      addWho();
    } else if (e.key === 'Backspace' && !who && ev?.attendees.length) {
      ev.attendees.pop();
    }
  }

  const STATUS_ICON: Record<PartStat, { icon: 'check' | 'x' | 'help' | 'clock'; color: string }> = {
    accepted: { icon: 'check', color: 'var(--green)' },
    declined: { icon: 'x', color: 'var(--danger)' },
    tentative: { icon: 'help', color: 'var(--amber)' },
    'needs-action': { icon: 'clock', color: 'var(--ink-3)' },
  };

  // ---------- Salvataggio ----------

  async function save(send: boolean) {
    if (!ev || saving) return;
    if (who.trim()) addWho();
    const bad = ev.attendees.find((a) => !validEmail(a.email));
    if (bad) {
      err = `L'indirizzo "${bad.email}" non è valido.`;
      return;
    }
    saving = true;
    err = (await cal.save(send)) ?? '';
    saving = false;
  }

  async function close() {
    if (ev && JSON.stringify(ev) !== snapshot && !ro) {
      const ok = await store.ask({ title: 'Chiudere senza salvare?', text: 'Le modifiche a questo appuntamento andranno perse.', ok: 'Chiudi senza salvare', danger: true });
      if (!ok) return;
    }
    cal.edit = null;
  }

  async function remove() {
    if (!ed || !ev) return;
    const occ = ed.occ ?? { ev, start: ev.start, end: ev.end, key: `${ev.id}:${ev.start}` };
    await cal.remove(occ);
  }

  async function respond(a: InviteAction) {
    if (!ed?.occ) return;
    const occ = ed.occ;
    cal.edit = null;
    await cal.respond(occ, a, true);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      void close();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
      e.preventDefault();
      void save(false);
    } else if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      void save(sendable);
    }
  }

  const heading = $derived(!ed || !ev ? '' : ro ? 'Riunione' : ed.original ? (meeting ? 'Modifica riunione' : 'Modifica appuntamento') : meeting ? 'Nuova riunione' : 'Nuovo appuntamento');
</script>

{#if ed && ev}
  <div class="scrim fade-in ed-scrim">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="ed win-in" bind:this={box} role="dialog" aria-modal="true" aria-labelledby="ed-t-{uid}" tabindex="-1" onkeydown={onKey}>
      <header class="ed-h">
        <h2 id="ed-t-{uid}">{heading}</h2>
        <button class="ibtn btn" onclick={close} aria-label="Chiudi" title="Chiudi (Esc)"><Icon name="x" /></button>
      </header>

      <div class="ed-b scroll">
        {#if ed.series && ev.repeat}
          <p class="note-bar info series"><Icon name="repeat" size={16} style="color:var(--brand-ink)" /><span class="t">Stai modificando tutta la serie: {repeatText(ev.repeat, ev.start).toLowerCase()}.</span></p>
        {:else if ev.recurrenceId !== null && ev.recurrenceId !== undefined}
          <p class="note-bar info series"><Icon name="repeat" size={16} style="color:var(--brand-ink)" /><span class="t">Stai modificando solo questa occorrenza della serie.</span></p>
        {/if}
        {#if ro && ev.organizer}
          <div class="note-bar info inv">
            <Icon name="calendar-event" size={16} style="color:var(--brand-ink)" />
            <span class="t">Invito di {personLabel(ev.organizer)}. {ev.response && ev.response !== 'needs-action' ? `La tua risposta: ${partStatText(ev.response).toLowerCase()}.` : 'Non hai ancora risposto.'}</span>
            {#if ed.occ && !ev.cancelled}
              <button class="act" onclick={() => respond('accepted')}>Accetta</button>
              <button class="act" onclick={() => respond('tentative')}>Provvisorio</button>
              <button class="act" onclick={() => respond('declined')}>Rifiuta</button>
            {/if}
          </div>
        {/if}

        <form class="form" onsubmit={(e) => { e.preventDefault(); void save(sendable); }}>
          <label class="tf"><span>Titolo</span><input class="title" bind:this={titleEl} bind:value={ev.title} disabled={ro} autocomplete="off" placeholder="Per esempio: Riunione con il cliente" /></label>

          {#if meeting}
            <div class="tf who">
              <label for="who-{uid}">Partecipanti</label>
              <div class="chips-box" class:ro>
                {#each ev.attendees as a, i (a.email + i)}
                  {@const si = STATUS_ICON[a.status]}
                  <span class="chip" class:bad={!validEmail(a.email)} title="{a.email}{ed.original ? ` (${partStatText(a.status).toLowerCase()})` : ''}">
                    {#if ed.original && !ro}<Icon name={si.icon} size={13} stroke={2} style="color:{si.color}" />{:else}<span class="avatar" aria-hidden="true">{initials(personLabel(a))}</span>{/if}
                    {personLabel(a)}
                    {#if !ro}<button type="button" class="ibtn btn" onclick={() => ev.attendees.splice(i, 1)} aria-label="Togli {personLabel(a)}"><Icon name="x" size={13} stroke={1.8} /></button>{/if}
                  </span>
                {/each}
                {#if !ro}
                  <input
                    bind:this={whoEl}
                    id="who-{uid}"
                    class="bare ph grow"
                    value={who}
                    oninput={(e) => onWho(e.currentTarget.value)}
                    onkeydown={onWhoKey}
                    onblur={() => setTimeout(() => (sugg = null), 150)}
                    placeholder={ev.attendees.length ? '' : 'Nome o indirizzo email'}
                    autocomplete="off"
                    role="combobox"
                    aria-expanded={!!sugg}
                    aria-controls="who-s-{uid}"
                    aria-autocomplete="list"
                  />
                {/if}
              </div>
              {#if sugg}
                <ul class="sugg" id="who-s-{uid}" role="listbox" aria-label="Indirizzi suggeriti">
                  {#each sugg.items as p, i (p.email)}
                    <li role="option" aria-selected={i === sugg.idx}>
                      <button type="button" class="sg" class:on={i === sugg.idx} onmousedown={(e) => { e.preventDefault(); addWho(p); }}>
                        <span class="avatar" aria-hidden="true">{initials(personLabel(p))}</span>
                        <span class="sgt"><span class="sgn">{personLabel(p)}</span>{#if p.name}<span class="sge">{p.email}</span>{/if}</span>
                      </button>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
            {#if mine && senders.length > 1}
              <label class="tf from"><span>Da</span>
                <select value={ev.accountId ?? senders[0]?.id} onchange={(e) => (ev.accountId = e.currentTarget.value)}>
                  {#each senders as a (a.id)}<option value={a.id}>{a.email}</option>{/each}
                </select>
              </label>
            {/if}
          {:else if !ro}
            <button type="button" class="lbtn add-who" onclick={() => { ed.meeting = true; setTimeout(() => whoEl?.focus(), 0); }}><Icon name="user-plus" size={16} />Invita partecipanti</button>
          {/if}

          <label class="chk"><input type="checkbox" checked={ev.allDay} disabled={ro} onchange={(e) => onAllDay(e.currentTarget.checked)} />Tutto il giorno</label>

          <div class="times">
            <div class="tf">
              <span id="st-{uid}">Inizio</span>
              <div class="dt">
                <input type="date" aria-labelledby="st-{uid}" bind:value={sd} onchange={onStart} disabled={ro} />
                {#if !ev.allDay}<input type="time" aria-label="Ora di inizio" step="300" bind:value={st} onchange={onStart} disabled={ro} />{/if}
              </div>
            </div>
            <div class="tf">
              <span id="en-{uid}">Fine{#if duration}<span class="hint dur">({duration})</span>{/if}</span>
              <div class="dt">
                <input type="date" aria-labelledby="en-{uid}" bind:value={endD} onchange={onEnd} disabled={ro} />
                {#if !ev.allDay}<input type="time" aria-label="Ora di fine" step="300" bind:value={et} onchange={onEnd} disabled={ro} />{/if}
              </div>
            </div>
          </div>

          <div class="two-col">
            <label class="tf"><span>Ripeti</span>
              <select value={rk} onchange={(e) => setRepeat(e.currentTarget.value)} disabled={ro || (ev.recurrenceId !== null && ev.recurrenceId !== undefined)}>
                {#each choices as c (c.key)}<option value={c.key}>{c.label}</option>{/each}
                {#if rk === 'altra' && ev.repeat}<option value="altra">{repeatText(ev.repeat, ev.start)}</option>{/if}
              </select>
            </label>
            {#if ev.repeat}
              <label class="tf"><span>Fino al <span class="hint">(facoltativo)</span></span><input type="date" bind:value={untilD} onchange={onUntil} min={dateInput(ev.start)} disabled={ro} /></label>
            {/if}
          </div>

          <label class="tf"><span>Luogo</span><input bind:value={ev.location} disabled={ro} autocomplete="off" placeholder="Sala riunioni, indirizzo o collegamento" /></label>

          <div class="two-col">
            <label class="tf"><span>Promemoria</span>
              <select value={String(ev.reminder)} onchange={(e) => (ev.reminder = e.currentTarget.value === 'null' ? null : Number(e.currentTarget.value))}>
                {#each REMINDERS as [m, l] (String(m))}<option value={String(m)}>{l}</option>{/each}
                {#if !REMINDERS.some(([m]) => m === ev.reminder)}<option value={String(ev.reminder)}>{ev.reminder} minuti prima</option>{/if}
              </select>
            </label>
            <label class="tf"><span>Categoria</span>
              <select value={catSel} onchange={(e) => (ev.categories = e.currentTarget.value ? [e.currentTarget.value] : [])}>
                <option value="">Nessuna</option>
                {#each cats as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
              </select>
            </label>
          </div>

          <label class="tf"><span>Note</span><textarea rows="5" bind:value={ev.notes}></textarea></label>
          {#if err}<p class="bad" role="alert">{err}</p>{/if}
          <button type="submit" hidden aria-hidden="true" tabindex="-1"></button>
        </form>
      </div>

      <footer class="ed-f">
        {#if ed.original && (ev.id || ed.occ)}
          <button class="sec-btn danger-btn" onclick={remove}><Icon name="trash" size={16} />{meeting && mine && ev.attendees.length ? 'Annulla riunione' : 'Elimina'}</button>
        {/if}
        <span class="sp"></span>
        {#if sendable}
          {#if ed.original}<button class="sec-btn" disabled={saving} onclick={() => save(false)}>Salva senza inviare</button>{/if}
          <button class="primary-btn" disabled={saving} onclick={() => save(true)} title="Invia (Ctrl+Invio)"><Icon name="send" size={16} />{ed.original ? 'Invia aggiornamento' : 'Invia'}</button>
        {:else}
          <button class="primary-btn" disabled={saving} onclick={() => save(false)} title="Salva (Ctrl+S)"><Icon name="device-floppy" size={16} />Salva</button>
        {/if}
        <button class="sec-btn" onclick={close}>Chiudi</button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .ed-scrim { z-index: 50; position: fixed; }
  .ed { width: min(660px, 100%); max-height: 100%; display: flex; flex-direction: column; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); outline: none; overflow: hidden; }
  .ed-h { display: flex; align-items: center; gap: 12px; padding: 16px 16px 8px 24px; }
  .ed-h h2 { flex: 1; margin: 0; font-size: 20px; font-weight: 600; }
  .ed-b { flex: 1; min-height: 0; overflow: auto; padding: 4px 24px 18px; }
  .series, .inv { margin: 4px 0 14px; border-radius: 4px; }
  .inv { flex-wrap: wrap; }
  .form { display: flex; flex-direction: column; gap: 14px; }
  .title { font-size: 16px; height: 38px; }
  .tf select { appearance: auto; }
  .who { position: relative; }
  .who > label { font-size: 13.5px; font-weight: 600; }
  .chips-box { min-height: 34px; box-sizing: border-box; display: flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 4px 6px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -1px 0 var(--ink-3); }
  .chips-box:focus-within { box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .chips-box.ro { box-shadow: none; padding-left: 0; }
  .chips-box .avatar { width: 20px; height: 20px; font-size: 9.5px; background: var(--sheet); color: var(--brand-ink); }
  .chip.bad { background: color-mix(in srgb, var(--danger) 14%, transparent); }
  .chip .ibtn { width: 22px; height: 22px; }
  .chips-box .grow, .chips-box .grow:focus { flex: 1; min-width: 160px; height: 26px; padding: 0 4px; box-shadow: none; background: transparent; font-size: 14px; outline: none; }
  .dur { margin-left: 6px; }
  .sugg { position: absolute; left: 0; right: 0; top: 100%; z-index: 4; margin: 4px 0 0; padding: 4px; list-style: none; max-height: 240px; overflow: auto; border-radius: 6px; background: var(--sheet); box-shadow: var(--win-shadow), 0 0 0 1px var(--rule-2); }
  .sg { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); text-align: left; }
  .sg.on, .sg:hover { background: var(--hover); }
  .sg .avatar { width: 28px; height: 28px; font-size: 11px; background: var(--brand-soft); color: var(--brand-ink); }
  .sgt { min-width: 0; display: flex; flex-direction: column; }
  .sgn { font-size: 13.5px; }
  .sge { font-size: 12.5px; color: var(--ink-2); }
  .from { max-width: 320px; }
  .add-who { align-self: flex-start; height: 30px; display: inline-flex; align-items: center; gap: 8px; padding: 0 8px; margin: -4px 0 -4px -8px; font-size: 13.5px; }
  .add-who:hover { background: var(--hover); }
  .chk { display: flex; align-items: center; gap: 10px; font-size: 14px; width: fit-content; }
  .chk input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); }
  .times { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px 16px; }
  .dt { display: flex; gap: 6px; }
  .dt input[type="date"] { flex: 1; min-width: 0; }
  .dt input[type="time"] { width: 118px; flex: none; font-variant-numeric: tabular-nums; }
  .two-col { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px 16px; }
  .tf textarea { box-sizing: border-box; padding: 8px 10px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -1px 0 var(--ink-3); color: var(--ink); font: inherit; font-size: 14px; line-height: 1.45; resize: vertical; }
  .tf textarea:focus { outline: none; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .bad { margin: 0; font-size: 13.5px; color: var(--danger); }
  .ed-f { display: flex; align-items: center; gap: 8px; padding: 14px 24px 18px; box-shadow: inset 0 1px 0 var(--rule); }
  .sp { flex: 1; }
  @media (max-width: 620px) { .times, .two-col { grid-template-columns: minmax(0, 1fr); } }
</style>
