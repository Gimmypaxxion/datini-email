<script lang="ts">
  // "Inserisci collegamento", come in Outlook: testo da visualizzare e
  // indirizzo. Con il cursore su un collegamento esistente lo modifica o lo toglie.
  import { untrack } from 'svelte';
  import Popover from './Popover.svelte';
  import type { EditorCtl } from '../lib/editor.svelte';

  let { ctl, anchor, onclose }: { ctl: EditorCtl; anchor: HTMLElement; onclose: () => void } = $props();

  const start = untrack(() => ctl.linkInfo());
  let text = $state(start.text);
  let href = $state(start.href);
  const existing = !!start.href;

  function done() {
    onclose();
    ctl.focus('keep');
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!href.trim()) return;
    ctl.setLink(text.trim(), href.trim());
    onclose();
  }
</script>

<Popover {anchor} label="Inserisci collegamento" onclose={done} width={360}>
  <form class="lk" onsubmit={submit}>
    <h3>{existing ? 'Modifica collegamento' : 'Inserisci collegamento'}</h3>
    <label class="tf">
      <span>Testo da visualizzare</span>
      <input bind:value={text} autocomplete="off" placeholder="Per esempio: il nostro catalogo" />
    </label>
    <label class="tf">
      <span>Indirizzo</span>
      <input bind:value={href} autocomplete="off" spellcheck="false" placeholder="www.esempio.it oppure nome@azienda.it" data-first />
    </label>
    <div class="acts">
      {#if existing}
        <button type="button" class="sec-btn rm" onclick={() => { ctl.removeLink(); onclose(); }}>Rimuovi collegamento</button>
      {/if}
      <button type="button" class="sec-btn" onclick={done}>Annulla</button>
      <button type="submit" class="primary-btn" disabled={!href.trim()}>{existing ? 'Salva' : 'Inserisci'}</button>
    </div>
  </form>
</Popover>

<style>
  .lk { display: flex; flex-direction: column; gap: 14px; padding: 10px 10px 8px; }
  h3 { margin: 0; font-size: 15px; font-weight: 600; }
  .acts { display: flex; justify-content: flex-end; gap: 8px; margin-top: 2px; }
  .rm { margin-right: auto; }
</style>
