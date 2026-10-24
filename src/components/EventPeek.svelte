<script lang="ts">
  // Scheda rapida di un appuntamento, aperta con un clic nel calendario:
  // quando, dove, ripetizione, partecipanti; risposta agli inviti; Apri ed Elimina.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { partStatText, reminderText, repeatText, responsesText, whenText, type Occurrence } from '../lib/calendar';
  import { personLabel } from '../lib/format';
  import type { InviteAction } from '../lib/types';

  let { occ, onclose }: { occ: Occurrence; onclose: () => void } = $props();

  const cal = store.cal;
  const ev = $derived(occ.ev);
  const mine = $derived(cal.isMine(ev));
  const invited = $derived(!!ev.organizer && !mine);
  const cat = $derived(ev.categories.map((id) => store.categories.find((c) => c.id === id || c.name.toLowerCase() === id.toLowerCase())).find(Boolean));

  const RESP: [InviteAction, string][] = [
    ['accepted', 'Accetta'],
    ['tentative', 'Provvisorio'],
    ['declined', 'Rifiuta'],
  ];

  async function respond(a: InviteAction) {
    onclose();
    await cal.respond(occ, a, true);
  }
</script>

<div class="pk" style={cal.tone(ev)}>
  <div class="pk-h">
    <span class="sw" aria-hidden="true"></span>
    <h3>{ev.title || 'Senza titolo'}</h3>
    <button class="dlg-x" onclick={onclose} aria-label="Chiudi" title="Chiudi (Esc)"><Icon name="x" size={16} /></button>
  </div>
  {#if ev.cancelled}<p class="cancel">L'organizzatore ha annullato questo evento.</p>{/if}
  <ul class="facts">
    <li><Icon name="clock" size={16} /><span>{whenText(occ.start, occ.end, ev.allDay)}</span></li>
    {#if ev.repeat}<li><Icon name="repeat" size={16} /><span>{repeatText(ev.repeat, ev.start)}</span></li>{/if}
    {#if ev.location}<li><Icon name="map-pin" size={16} /><span>{ev.location}</span></li>{/if}
    {#if invited && ev.organizer}
      <li><Icon name="user" size={16} /><span>Organizzatore: {personLabel(ev.organizer)}</span></li>
    {/if}
    {#if ev.attendees.length}
      <li><Icon name="users" size={16} /><span>{ev.attendees.length === 1 ? '1 partecipante' : `${ev.attendees.length} partecipanti`}{mine ? `: ${responsesText(ev.attendees)}` : ''}</span></li>
    {/if}
    <li><Icon name="bell" size={16} /><span>Promemoria: {reminderText(ev.reminder).toLowerCase()}</span></li>
    {#if cat}<li><Icon name="tag" size={16} /><span>{cat.name}</span></li>{/if}
  </ul>
  {#if ev.notes}<p class="notes">{ev.notes}</p>{/if}

  {#if invited && !ev.cancelled}
    <div class="resp" role="group" aria-label="La tua risposta">
      <p class="rs">{ev.response && ev.response !== 'needs-action' ? `La tua risposta: ${partStatText(ev.response).toLowerCase()}.` : 'Non hai ancora risposto.'}</p>
      <div class="rb-row">
        {#each RESP as [a, l] (a)}
          <button class="sec-btn" aria-pressed={ev.response === a} onclick={() => respond(a)}>{l}</button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="acts">
    <button class="primary-btn" onclick={() => { onclose(); void cal.open(occ); }}><Icon name="pencil" size={16} />{invited ? 'Apri' : 'Modifica'}</button>
    <button class="sec-btn" onclick={() => { onclose(); void cal.remove(occ); }}><Icon name="trash" size={16} />{ev.cancelled ? 'Togli dal calendario' : 'Elimina'}</button>
  </div>
</div>

<style>
  .pk { --ev-line: var(--brand); width: 340px; box-sizing: border-box; padding: 12px 14px 14px; }
  .pk-h { display: flex; align-items: flex-start; gap: 10px; }
  .sw { width: 12px; height: 12px; flex: none; margin-top: 6px; border-radius: 3px; background: var(--ev-line); }
  h3 { flex: 1; min-width: 0; margin: 0; padding-top: 1px; font-size: 16px; font-weight: 600; line-height: 1.35; overflow-wrap: anywhere; }
  .pk-h .dlg-x { width: 28px; height: 28px; margin: -2px -6px 0 0; }
  .cancel { margin: 8px 0 0; font-size: 13.5px; color: var(--danger); }
  .facts { margin: 10px 0 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 6px; font-size: 13.5px; color: var(--ink); }
  .facts li { display: flex; align-items: flex-start; gap: 10px; line-height: 1.4; }
  .facts li :global(svg) { margin-top: 2px; color: var(--ink-3); }
  .facts span { min-width: 0; overflow-wrap: anywhere; }
  .notes { margin: 10px 0 0 26px; font-size: 13px; line-height: 1.45; color: var(--ink-2); white-space: pre-line; display: -webkit-box; -webkit-line-clamp: 4; line-clamp: 4; -webkit-box-orient: vertical; overflow: hidden; }
  .resp { margin-top: 12px; padding-top: 10px; box-shadow: inset 0 1px 0 var(--rule); }
  .rs { margin: 0 0 8px; font-size: 13px; color: var(--ink-2); }
  .rb-row { display: flex; gap: 6px; }
  .rb-row .sec-btn { padding: 0 10px; }
  .rb-row .sec-btn[aria-pressed="true"] { background: var(--brand-soft); box-shadow: inset 0 0 0 1px var(--brand); font-weight: 600; }
  .acts { margin-top: 14px; display: flex; gap: 8px; }
</style>
