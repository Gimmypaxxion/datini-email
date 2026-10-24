<script lang="ts">
  // Impostazioni › Risposte automatiche: come "Risposte automatiche" di
  // Outlook. La risposta parte dal computer, quindi solo a programma aperto.
  import Icon from '../lib/Icon.svelte';
  import MiniEditor from './MiniEditor.svelte';
  import { store } from '../lib/store.svelte';
  import type { AutoReply } from '../lib/types';

  const accounts = $derived(store.accounts.filter((a) => store.canSend(a)));
  const acct = $derived(accounts.find((a) => a.id === store.acctSel) ?? accounts[0] ?? null);

  const pad = (n: number) => String(n).padStart(2, '0');
  /** Millisecondi → valore di un campo data e ora ("2026-10-26T08:00"). */
  function toInput(ms: number): string {
    const d = new Date(ms);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function fromInput(v: string): number | null {
    const t = new Date(v).getTime();
    return Number.isFinite(t) ? t : null;
  }
  function nextHour(): number {
    const d = new Date();
    d.setMinutes(0, 0, 0);
    return d.getTime() + 3_600_000;
  }

  let enabled = $state(false);
  let timed = $state(false);
  let start = $state('');
  let end = $state('');
  let html = $state('');
  let text = $state('');
  let version = $state(0);
  let dirty = $state(false);
  let note = $state('');
  let loadedFor = '';

  // Carica la risposta dell'account scelto (e solo quando cambia account).
  $effect(() => {
    const a = acct;
    if (!a || a.id === loadedFor) return;
    loadedFor = a.id;
    const r = a.autoReply;
    enabled = !!r?.enabled;
    timed = r?.start != null || r?.end != null;
    const s = r?.start ?? nextHour();
    start = toInput(s);
    end = toInput(r?.end ?? s + 86_400_000);
    html = r?.html ?? '';
    text = r?.text ?? '';
    dirty = false;
    note = '';
    version++;
  });

  const touch = () => {
    dirty = true;
    note = '';
  };

  const periodBad = $derived(timed && enabled && (fromInput(start) ?? 0) >= (fromInput(end) ?? 0));

  async function save() {
    if (!acct) return;
    const reply: AutoReply = {
      enabled,
      start: timed ? fromInput(start) : null,
      end: timed ? fromInput(end) : null,
      html: text.trim() ? html : '',
      text,
    };
    const err = await store.saveAutoReply(acct.id, reply);
    if (err) {
      note = err;
      return;
    }
    dirty = false;
    note = !enabled ? 'Salvato: le risposte automatiche sono spente.' : timed ? 'Salvato: le risposte automatiche partono nel periodo scelto.' : 'Salvato: le risposte automatiche sono attive.';
  }
</script>

<h3>Risposte automatiche</h3>
<p class="intro">Quando sei fuori ufficio, chi ti scrive riceve un messaggio che lo avvisa. Ogni mittente lo riceve una volta sola.</p>

{#if !acct}
  <p class="hint" style="margin-top:22px">Aggiungi un account di posta per usare le risposte automatiche.</p>
{:else}
  {#if accounts.length > 1}
    <label class="tf" style="max-width:380px;margin-top:22px">
      <span>Account</span>
      <select value={acct.id} onchange={(e) => (store.acctSel = e.currentTarget.value)}>
        {#each accounts as a (a.id)}<option value={a.id}>{a.email}</option>{/each}
      </select>
    </label>
  {/if}

  <div role="radiogroup" aria-label="Risposte automatiche" class="two" style="margin-top:22px">
    <label class="opt" data-on={!enabled ? 'si' : 'no'}>
      <input type="radio" name="ar-on" checked={!enabled} onchange={() => { enabled = false; touch(); }} />
      <span class="dot" aria-hidden="true"></span>
      <span class="otext"><span class="ot">Non inviare risposte automatiche</span></span>
    </label>
    <label class="opt" data-on={enabled ? 'si' : 'no'}>
      <input type="radio" name="ar-on" checked={enabled} onchange={() => { enabled = true; touch(); }} />
      <span class="dot" aria-hidden="true"></span>
      <span class="otext"><span class="ot">Invia risposte automatiche</span></span>
    </label>
  </div>

  <fieldset class="body" disabled={!enabled}>
    <label class="check">
      <input type="checkbox" checked={timed} onchange={(e) => { timed = e.currentTarget.checked; touch(); }} />
      Invia solo in questo periodo
    </label>
    {#if timed}
      <div class="period">
        <label class="tf"><span>Inizio</span><input type="datetime-local" value={start} oninput={(e) => { start = e.currentTarget.value; touch(); }} /></label>
        <label class="tf" data-bad={periodBad ? 'si' : 'no'}><span>Fine</span><input type="datetime-local" value={end} oninput={(e) => { end = e.currentTarget.value; touch(); }} /></label>
      </div>
      {#if periodBad}<p class="bad" role="alert">La fine deve venire dopo l’inizio.</p>{/if}
    {/if}

    <h4>Messaggio</h4>
    <MiniEditor
      docKey="ar-{acct.id}-{version}"
      {html}
      label="Messaggio della risposta automatica"
      placeholder="Per esempio: sono fuori ufficio fino al 2 novembre, con accesso limitato alla posta. Per urgenze scrivete a ordini@valliarredi.it."
      onchange={(h, t) => { html = h; text = t; touch(); }}
    />
  </fieldset>

  <div class="acts">
    <button class="primary-btn" onclick={save} disabled={!dirty || periodBad || (enabled && !text.trim())}><Icon name="device-floppy" size={17} />Salva</button>
    <span class="note" role="status">{note || (dirty ? 'Modifiche non ancora salvate' : '')}</span>
  </div>

  <div role="note" class="limit">
    <Icon name="alert-circle" size={18} style="color:var(--brand-ink);margin-top:1px" />
    <span>La risposta parte da questo computer, solo mentre Better-Look è aperto: i messaggi arrivati a computer spento ricevono la risposta alla riaccensione. Se resterai a lungo senza computer, attiva la risposta automatica anche nella webmail del tuo provider. Better-Look non risponde a newsletter, notifiche e altre risposte automatiche.</span>
  </div>
{/if}

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 22px 0 10px; font-size: 15px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 60ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .hint { margin: 0; max-width: 58ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .otext { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .ot { font-size: 14.5px; font-weight: 600; }
  .body { margin: 18px 0 0; padding: 0; border: 0; min-width: 0; }
  .body:disabled { opacity: .55; }
  .check { display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .check input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); }
  .period { margin-top: 12px; display: flex; flex-wrap: wrap; gap: 12px; }
  .period .tf { width: 220px; }
  .bad { margin: 8px 0 0; font-size: 13px; color: var(--danger); }
  .acts { margin-top: 16px; display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .note { font-size: 13px; color: var(--ink-2); }
  .limit { margin-top: 22px; max-width: 680px; display: flex; gap: 12px; padding: 12px 14px; border-radius: 4px; background: var(--brand-soft); font-size: 13.5px; line-height: 1.5; }
</style>
