<script lang="ts">
  // Invito dentro un messaggio: cosa, quando, dove, chi organizza; gli
  // impegni già presi in quell'orario; Accetta, Provvisorio, Rifiuta (con o
  // senza risposta all'organizzatore). Per gli annullamenti: togli dal
  // calendario; per le risposte ricevute: chi ha risposto e come.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import { store } from '../lib/store.svelte';
  import { addDays, conflicts, expand, MESI, partStatText, repeatText, responsesText, startOfDay, timeText, whenText, type Occurrence } from '../lib/calendar';
  import { personLabel } from '../lib/format';
  import type { Invite, InviteAction } from '../lib/types';

  let { invite }: { invite: Invite } = $props();

  const cal = store.cal;
  const ev = $derived(invite.event);
  const local = $derived(invite.local);
  const method = $derived(invite.method);
  const isRequest = $derived(method === 'request');
  const answered = $derived(local?.response && local.response !== 'needs-action' ? local.response : null);
  const day = $derived(new Date(ev.start));

  let sendReply = $state(true);
  let busy = $state(false);
  let err = $state('');
  let clash = $state<Occurrence[]>([]);

  // Impegni dello stesso giorno che si sovrappongono all'invito.
  $effect(() => {
    const from = startOfDay(ev.start);
    const to = addDays(startOfDay(Math.max(ev.end - 1, ev.start)), 1);
    const uid = ev.uid;
    const start = ev.start;
    const end = Math.max(ev.end, ev.start + 1);
    let alive = true;
    clash = [];
    if (!isRequest || ev.allDay) return;
    void store.backend
      .events(from, to)
      .then((list) => {
        if (alive) clash = conflicts({ start, end, uid }, expand(list, from, to));
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  });

  async function act(a: InviteAction) {
    busy = true;
    err = (await store.respondInvite(a, sendReply && a !== 'add' && a !== 'remove')) ?? '';
    busy = false;
  }

  function show() {
    const target = local ?? ev;
    store.setModule('calendario');
    cal.goTo(ev.start);
    cal.sel = `${target.id}:${ev.start}`;
  }

  const head = $derived.by((): { icon: IconName; color: string; text: string } => {
    if (method === 'cancel') return { icon: 'calendar-x', color: 'var(--danger)', text: "L'organizzatore ha annullato questo evento." };
    if (method === 'reply' && invite.replier) {
      const r = invite.replier;
      const name = personLabel(r);
      if (r.status === 'accepted') return { icon: 'calendar-check', color: 'var(--green)', text: `${name} ha accettato.` };
      if (r.status === 'declined') return { icon: 'calendar-x', color: 'var(--danger)', text: `${name} ha rifiutato.` };
      if (r.status === 'tentative') return { icon: 'calendar-question', color: 'var(--amber)', text: `${name} potrebbe partecipare (risposta provvisoria).` };
      return { icon: 'calendar-event', color: 'var(--brand-ink)', text: `${name} ha risposto.` };
    }
    if (invite.outdated) return { icon: 'calendar-time', color: 'var(--ink-3)', text: "Questo invito è superato: nel calendario c'è già una versione più recente." };
    if (isRequest) {
      if (answered) return { icon: answered === 'declined' ? 'calendar-x' : answered === 'tentative' ? 'calendar-question' : 'calendar-check', color: 'var(--brand-ink)', text: `Hai risposto: ${partStatText(answered).toLowerCase()}.` };
      return { icon: 'calendar-event', color: 'var(--brand-ink)', text: invite.forMe ? 'Non hai ancora risposto.' : 'Invito inoltrato: non sei tra i partecipanti.' };
    }
    return { icon: 'calendar-event', color: 'var(--brand-ink)', text: local ? 'È già nel tuo calendario.' : 'Evento da aggiungere al calendario.' };
  });

  const RESP: [InviteAction, string, IconName][] = [
    ['accepted', 'Accetta', 'check'],
    ['tentative', 'Provvisorio', 'help'],
    ['declined', 'Rifiuta', 'x'],
  ];
</script>

<section class="inv" aria-label="Invito: {ev.title || 'evento'}">
  <div class="leaf" aria-hidden="true">
    <span class="lm">{MESI[day.getMonth()].slice(0, 3)}</span>
    <span class="ld">{day.getDate()}</span>
  </div>
  <div class="body">
    <h3 class:gone={method === 'cancel'}>{ev.title || 'Evento senza titolo'}</h3>
    <p class="when">{whenText(ev.start, ev.end, ev.allDay, cal.now)}</p>
    {#if ev.repeat}<p class="meta"><Icon name="repeat" size={15} />{repeatText(ev.repeat, ev.start)}</p>{/if}
    {#if ev.location}<p class="meta"><Icon name="map-pin" size={15} />{ev.location}</p>{/if}
    {#if ev.organizer && method !== 'reply'}
      <p class="meta"><Icon name="user" size={15} />Organizzatore: {personLabel(ev.organizer)}{ev.attendees.length ? `, ${ev.attendees.length === 1 ? '1 partecipante' : `${ev.attendees.length} partecipanti`}` : ''}</p>
    {/if}

    <p class="status"><Icon name={head.icon} size={16} style="color:{head.color}" />{head.text}</p>
    {#if method === 'reply' && local && local.attendees.length}
      <p class="meta tally">Risposte finora: {responsesText(local.attendees)}.</p>
    {/if}
    {#if isRequest && !invite.outdated && !ev.allDay && method !== 'cancel'}
      <p class="meta clash" class:busy={clash.length > 0}>
        {#if clash.length}
          <Icon name="alert-triangle" size={15} />In conflitto con {clash.slice(0, 2).map((o) => `“${o.ev.title || 'Senza titolo'}” (${timeText(o.start)}-${timeText(o.end)})`).join(' e ')}{clash.length > 2 ? ` e altri ${clash.length - 2}` : ''}.
        {:else}
          <Icon name="circle-check" size={15} />Nessun altro impegno in quell'orario.
        {/if}
      </p>
    {/if}

    {#if isRequest && !invite.outdated}
      <div class="acts" role="group" aria-label="Rispondi all'invito">
        {#each RESP as [a, label, icon] (a)}
          <button class="sec-btn rbtn" aria-pressed={answered === a} disabled={busy} onclick={() => act(a)}><Icon name={icon} size={16} />{label}</button>
        {/each}
        <label class="send"><input type="checkbox" bind:checked={sendReply} />Invia la risposta all'organizzatore</label>
      </div>
    {:else if method === 'cancel'}
      <div class="acts">
        {#if local}
          <button class="primary-btn" disabled={busy} onclick={() => act('remove')}><Icon name="calendar-x" size={16} />Togli dal calendario</button>
        {:else}
          <span class="meta">Non è nel tuo calendario.</span>
        {/if}
      </div>
    {:else if method !== 'reply' && !invite.outdated && !local}
      <div class="acts"><button class="primary-btn" disabled={busy} onclick={() => act('add')}><Icon name="calendar-plus" size={16} />Aggiungi al calendario</button></div>
    {/if}
    {#if local && method !== 'cancel'}
      <button class="lbtn show" onclick={show}>Mostra nel calendario</button>
    {/if}
    {#if err}<p class="bad" role="alert">{err}</p>{/if}
  </div>
</section>

<style>
  .inv { margin: 16px 0 0 58px; display: flex; gap: 16px; padding: 14px 16px 14px 14px; border-radius: 8px; box-shadow: inset 0 0 0 1px var(--rule-2); background: var(--sheet); }
  .leaf { width: 50px; height: 56px; flex: none; display: flex; flex-direction: column; align-items: center; overflow: hidden; border-radius: 6px; box-shadow: inset 0 0 0 1px var(--rule-2); }
  .lm { width: 100%; height: 18px; display: grid; place-items: center; background: var(--brand); color: var(--on-brand); font-size: 11.5px; font-weight: 600; }
  .ld { flex: 1; display: grid; place-items: center; font-size: 21px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .body { flex: 1; min-width: 0; }
  h3 { margin: 0; font-size: 16px; font-weight: 600; line-height: 1.3; overflow-wrap: anywhere; }
  h3.gone { text-decoration: line-through; color: var(--ink-2); }
  .when { margin: 3px 0 0; font-size: 14px; color: var(--ink); }
  .meta { margin: 4px 0 0; display: flex; align-items: flex-start; gap: 8px; font-size: 13.5px; line-height: 1.4; color: var(--ink-2); }
  .meta :global(svg) { margin-top: 1px; flex: none; color: var(--ink-3); }
  .status { margin: 12px 0 0; display: flex; align-items: center; gap: 8px; font-size: 14px; font-weight: 600; }
  .clash.busy, .clash.busy :global(svg) { color: var(--warn); }
  .acts { margin-top: 12px; display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .rbtn[aria-pressed="true"] { background: var(--brand-soft); box-shadow: inset 0 0 0 1px var(--brand); font-weight: 600; }
  .send { margin-left: 6px; display: inline-flex; align-items: center; gap: 8px; font-size: 13.5px; color: var(--ink-2); }
  .send input { width: 16px; height: 16px; margin: 0; accent-color: var(--brand); }
  .show { margin-top: 10px; height: 28px; padding: 0 6px; margin-left: -6px; font-size: 13.5px; }
  .show:hover { background: var(--hover); }
  .bad { margin: 8px 0 0; font-size: 13.5px; color: var(--danger); }
  @media (max-width: 1200px) { .inv { margin-left: 0; } }
</style>
