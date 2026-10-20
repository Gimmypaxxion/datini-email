<script lang="ts">
  // Schede "Messaggio" e "Inserisci", usate sia nella barra principale
  // (scrittura nel riquadro) sia nelle finestre del messaggio. La
  // formattazione del testo arriverà con l'editor completo: per ora quei
  // comandi restano al loro posto ma non attivi.
  import RB from './RB.svelte';
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';

  let { tab }: { tab: 'messaggio' | 'inserisci' } = $props();
  const SOON = 'Arriverà in una prossima versione';
</script>

{#if tab === 'messaggio'}
  <div class="rg"><div class="rgb">
    <RB big icon="clipboard" tone="c-amber" label="Incolla" title="Incolla (Ctrl+V)" disabled />
  </div><span class="rgl">Appunti</span></div>
  <div class="rg"><div class="rgb"><div class="rs">
    <div class="rr">
      <button class="rb sel" aria-label="Tipo di carattere" style="width:132px" disabled title={SOON}>Aptos<Icon name="chevron-down" size={14} class="chev" /></button>
      <button class="rb sel" aria-label="Dimensione carattere" style="width:58px" disabled title={SOON}>11<Icon name="chevron-down" size={14} class="chev" /></button>
    </div>
    <div class="rr">
      <RB iconOnly icon="bold" label="Grassetto" title="Grassetto (Ctrl+G): {SOON.toLowerCase()}" disabled />
      <RB iconOnly icon="italic" label="Corsivo" title="Corsivo (Ctrl+I): {SOON.toLowerCase()}" disabled />
      <RB iconOnly icon="underline" label="Sottolineato" title="Sottolineato (Ctrl+S): {SOON.toLowerCase()}" disabled />
      <RB iconOnly icon="list" label="Elenco puntato" title={SOON} disabled />
      <RB iconOnly icon="list-numbers" label="Elenco numerato" title={SOON} disabled />
    </div>
  </div></div><span class="rgl">Testo</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="paperclip" tone="c-brand" label="Allega file" title="Allega file" onclick={() => store.addFiles()} />
    <RB big icon="link" tone="c-brand" label="Collegamento" title={SOON} disabled />
    <RB big icon="signature" label="Firma" title={SOON} disabled />
  </div><span class="rgl">Includi</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="clock" tone="c-brand" label="Invio programmato" title={SOON} disabled />
  </div><span class="rgl">Invio</span></div>
{:else}
  <div class="rg"><div class="rgb">
    <RB big icon="paperclip" tone="c-brand" label="Allega file" title="Allega file" onclick={() => store.addFiles()} />
    <RB big icon="photo" tone="c-green" label="Immagine" title={SOON} disabled />
    <RB big icon="table" tone="c-brand" label="Tabella" title={SOON} disabled />
    <RB big icon="link" tone="c-brand" label="Collegamento" title={SOON} disabled />
  </div><span class="rgl">Includi</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="calendar-plus" tone="c-brand" label="Proposta di riunione" title={SOON} disabled />
  </div><span class="rgl">Riunione</span></div>
{/if}
