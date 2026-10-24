<script lang="ts">
  // Avviso in basso nel riquadro (messaggio inviato, errori, conferme).
  // Si usa in fondo all'elenco della posta, della rubrica e del calendario.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
</script>

{#if store.toast}
  {@const t = store.toast}
  <div role={t.kind === 'error' ? 'alert' : 'status'} class="toast rise" data-kind={t.kind}>
    <Icon name={t.kind === 'sent' ? 'send' : t.kind === 'draft' ? 'file-pencil' : t.kind === 'error' ? 'alert-circle' : 'circle-check'} />
    <span class="msg">{t.text}</span>
    {#if t.kind === 'sent' && t.outboxId}
      <button class="undo" onclick={() => store.undoSend()}>Annulla</button>
    {:else if t.kind === 'error' && t.outboxId}
      <button class="undo" onclick={() => store.openOutboxItem(t.outboxId!)}>Apri</button>
    {/if}
    <button class="close" onclick={() => store.dismissToast()} aria-label="Chiudi avviso"><Icon name="x" size={16} /></button>
  </div>
{/if}

<style>
  .toast { position: absolute; left: 14px; right: 14px; bottom: 14px; z-index: 5; display: flex; align-items: center; gap: 12px; min-height: 44px; box-sizing: border-box; padding: 6px 6px 6px 14px; border-radius: 4px; background: var(--ink); color: var(--sheet); box-shadow: var(--lift); font-size: 14px; }
  .toast[data-kind="error"] { background: #A4262C; color: #fff; }
  .toast .msg { flex: 1; min-width: 0; }
  .undo { height: 32px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: inherit; font-size: 14px; font-weight: 600; text-decoration: underline; text-underline-offset: 3px; }
  .close { width: 30px; height: 30px; flex: none; border: 0; border-radius: 4px; background: transparent; color: inherit; display: grid; place-items: center; opacity: .8; }
  .close:hover { opacity: 1; }
  .undo:focus-visible, .close:focus-visible { outline-color: currentColor; }
</style>
