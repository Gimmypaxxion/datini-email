<script lang="ts">
  // Domanda di conferma prima di un'azione che non si può annullare
  // (eliminare una cartella, svuotare la posta eliminata...). Esc annulla;
  // per le azioni distruttive il fuoco parte da Annulla, come in Outlook.
  import { store } from '../lib/store.svelte';

  let cancelBtn: HTMLButtonElement | undefined = $state();
  let okBtn: HTMLButtonElement | undefined = $state();

  $effect(() => {
    if (!store.confirm) return;
    (store.confirm.danger ? cancelBtn : okBtn)?.focus();
  });
</script>

{#if store.confirm}
  {@const c = store.confirm}
  <div class="scrim fade-in confirm-scrim">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="box rise"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="conf-t"
      aria-describedby="conf-d"
      tabindex="-1"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          store.answer(false);
        }
      }}
    >
      <h2 id="conf-t">{c.title}</h2>
      <p id="conf-d">{c.text}</p>
      {#if c.check}
        <label class="chk"><input type="checkbox" checked={c.check.value} onchange={(e) => (c.check!.value = e.currentTarget.checked)} />{c.check.label}</label>
      {/if}
      <div class="acts">
        <button class="primary-btn" class:danger={c.danger} bind:this={okBtn} onclick={() => store.answer(true)}>{c.ok}</button>
        {#if c.alt}<button class="sec-btn" onclick={() => store.answer('alt')}>{c.alt}</button>{/if}
        <button class="sec-btn" bind:this={cancelBtn} onclick={() => store.answer(false)}>Annulla</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .confirm-scrim { z-index: 70; position: fixed; }
  .box { width: min(440px, 100%); box-sizing: border-box; padding: 22px 24px 20px; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); outline: none; }
  h2 { margin: 0; font-size: 18px; font-weight: 600; line-height: 1.3; text-wrap: balance; overflow-wrap: anywhere; }
  p { margin: 10px 0 0; font-size: 14px; line-height: 1.5; color: var(--ink-2); }
  .chk { margin-top: 14px; display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .chk input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); }
  .acts { margin-top: 22px; display: flex; justify-content: flex-end; gap: 8px; }
  .danger { background: var(--danger-fill); color: var(--on-danger); }
</style>
