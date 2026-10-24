<script lang="ts">
  // Pianifica invio: come nel nuovo Outlook (menu del pulsante Invia) e
  // "Ritarda recapito" di Outlook classico. Il messaggio aspetta nella posta
  // in uscita fino all'ora scelta.
  import Popover from './Popover.svelte';
  import { store } from '../lib/store.svelte';
  import { whenText } from '../lib/format';

  let { anchor, onclose }: { anchor: HTMLElement; onclose: () => void } = $props();

  const pad = (n: number) => String(n).padStart(2, '0');
  const at = (d: Date, h: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate(), h, 0, 0, 0).getTime();

  const now = new Date();
  const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  const monday = new Date(now.getFullYear(), now.getMonth(), now.getDate() + (((8 - now.getDay()) % 7) || 7));

  const presets: { label: string; at: number }[] = [];
  if (now.getHours() < 13) presets.push({ label: 'Oggi pomeriggio', at: at(now, 14) });
  presets.push({ label: 'Domani mattina', at: at(tomorrow, 8) });
  if (monday.getTime() !== tomorrow.getTime()) presets.push({ label: 'Lunedì mattina', at: at(monday, 8) });

  const dayStr = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  let day = $state(dayStr(tomorrow));
  let time = $state('08:00');
  const custom = $derived(new Date(`${day}T${time || '00:00'}`).getTime());
  const bad = $derived(!Number.isFinite(custom) || custom <= Date.now() + 60_000);

  async function pick(ms: number) {
    onclose();
    await store.send(false, ms);
  }
</script>

<Popover {anchor} label="Pianifica invio" {onclose} width={300}>
  <div class="sp">
    <p class="sp-t">Pianifica invio</p>
    {#each presets as p (p.label)}
      <button class="sp-it" onclick={() => pick(p.at)}><span>{p.label}</span><span class="sp-w">{whenText(p.at)}</span></button>
    {/each}
    <div class="sp-sep"></div>
    <p class="sp-l" id="sp-custom">Data e ora a scelta</p>
    <div class="sp-row" role="group" aria-labelledby="sp-custom">
      <input type="date" aria-label="Giorno" bind:value={day} min={dayStr(now)} />
      <input type="time" aria-label="Ora" bind:value={time} step="300" />
    </div>
    {#if bad}<p class="sp-bad">Scegli un giorno e un’ora futuri.</p>{/if}
    <button class="primary-btn sp-go" disabled={bad} onclick={() => pick(custom)}>Pianifica</button>
    <p class="sp-note">Il messaggio aspetta in Posta in uscita e parte all’ora scelta, se Better-Look è aperto; altrimenti appena lo riapri.</p>
  </div>
</Popover>

<style>
  .sp { width: 300px; box-sizing: border-box; padding: 8px 6px 12px; }
  .sp-t { margin: 4px 10px 8px; font-size: 14px; font-weight: 600; }
  .sp-it { width: 100%; height: 36px; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font-size: 13.5px; text-align: left; }
  .sp-it:hover { background: var(--hover); }
  .sp-w { color: var(--ink-3); font-size: 12.5px; white-space: nowrap; }
  .sp-sep { height: 1px; margin: 8px 4px; background: var(--rule); }
  .sp-l { margin: 4px 10px 8px; font-size: 13px; font-weight: 600; }
  .sp-row { display: flex; gap: 8px; padding: 0 10px; }
  .sp-row input { height: 32px; box-sizing: border-box; min-width: 0; flex: 1; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .sp-row input[type="time"] { flex: 0 0 96px; }
  .sp-bad { margin: 8px 10px 0; font-size: 12.5px; color: var(--danger); }
  .sp-go { margin: 12px 10px 0; }
  .sp-note { margin: 10px 10px 0; font-size: 12.5px; line-height: 1.45; color: var(--ink-2); }
</style>
