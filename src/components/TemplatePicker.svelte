<script lang="ts">
  // I miei modelli, dal pulsante Modelli mentre si scrive: un clic inserisce
  // il testo al posto del cursore; il messaggio aperto si può salvare come modello.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { htmlToText } from '../lib/compose-html';
  import { cleanSubject } from '../lib/format';
  import type { MessageTemplate } from '../lib/types';

  let { onclose }: { onclose: () => void } = $props();

  let saving = $state(false);
  let name = $state('');
  let err = $state('');
  let busy = $state(false);
  let nameEl: HTMLInputElement | undefined = $state();

  const preview = (t: MessageTemplate) => htmlToText(t.html).replace(/\s+/g, ' ').trim();
  const replacing = $derived(store.templates.find((t) => t.name.trim().toLowerCase() === name.trim().toLowerCase()));

  function use(t: MessageTemplate) {
    store.insertTemplate(t);
    onclose();
  }

  function startSave() {
    saving = true;
    err = '';
    name = cleanSubject(store.c?.subject ?? '').slice(0, 80);
    queueMicrotask(() => nameEl?.select());
  }

  async function save() {
    const n = name.trim();
    if (!n) {
      err = 'Dai un nome al modello.';
      return;
    }
    busy = true;
    err = await store.saveDraftAsTemplate(n);
    busy = false;
    if (!err) {
      store.flash({ kind: 'info', text: `Modello «${n}» salvato.` }, 3000);
      onclose();
    }
  }
</script>

<div class="tp">
  <p class="h">I miei modelli</p>
  {#if store.templates.length}
    <div class="items" role="listbox" aria-label="Modelli">
      {#each store.templates as t, i (t.id)}
        <button class="it" role="option" aria-selected="false" data-first={i === 0 && !saving ? '' : undefined} onmousedown={(e) => e.preventDefault()} onclick={() => use(t)} title="Inserisci «{t.name}»">
          <span class="n">{t.name}</span>
          <span class="p">{preview(t) || 'Senza testo'}</span>
        </button>
      {/each}
    </div>
  {:else}
    <p class="empty">Nessun modello. Scrivi il testo che usi spesso, poi salvalo qui sotto per riusarlo con un clic.</p>
  {/if}

  <div class="foot">
    {#if saving}
      <form class="save" onsubmit={(e) => { e.preventDefault(); void save(); }}>
        <label class="tf">
          <span>Nome del modello</span>
          <input bind:this={nameEl} bind:value={name} oninput={() => (err = '')} maxlength="80" placeholder="Per esempio: Conferma d’ordine" />
        </label>
        {#if replacing}<p class="hint">Sostituisce il modello «{replacing.name}».</p>{/if}
        {#if err}<p class="bad" role="alert">{err}</p>{/if}
        <div class="tp-row">
          <button class="primary-btn" type="submit" disabled={busy}><Icon name="device-floppy" size={16} />{replacing ? 'Sostituisci' : 'Salva'}</button>
          <button class="sec-btn" type="button" onclick={() => (saving = false)}>Annulla</button>
        </div>
      </form>
    {:else}
      <button class="lbtn act" onclick={startSave} disabled={!store.c}><Icon name="device-floppy" size={16} />Salva questo messaggio come modello</button>
      <button class="lbtn act" onclick={() => { onclose(); store.openSettings('modelli'); }}><Icon name="settings" size={16} />Gestisci modelli</button>
    {/if}
  </div>
</div>

<style>
  .tp { width: 320px; display: flex; flex-direction: column; }
  .h { margin: 0; padding: 12px 14px 6px; font-size: 14px; font-weight: 600; }
  .items { max-height: 300px; overflow: auto; display: flex; flex-direction: column; padding: 0 6px 6px; }
  .it { display: flex; flex-direction: column; gap: 1px; min-width: 0; padding: 7px 8px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); text-align: left; }
  .it:hover, .it:focus-visible { background: var(--hover); }
  .n { font-size: 13.5px; font-weight: 600; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .p { font-size: 12.5px; color: var(--ink-2); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .empty { margin: 0; padding: 2px 14px 12px; font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .foot { display: flex; flex-direction: column; padding: 6px; box-shadow: inset 0 1px 0 var(--rule); }
  .act { height: 32px; display: flex; align-items: center; gap: 8px; padding: 0 8px; font-size: 13.5px; text-align: left; }
  .act:hover:not(:disabled) { background: var(--hover); }
  .act:disabled { color: var(--ink-3); }
  .save { display: flex; flex-direction: column; gap: 8px; padding: 6px 8px 8px; }
  .hint { margin: 0; font-size: 12.5px; color: var(--ink-2); }
  .bad { margin: 0; font-size: 12.5px; color: var(--danger); }
  .tp-row { display: flex; gap: 8px; }
</style>
