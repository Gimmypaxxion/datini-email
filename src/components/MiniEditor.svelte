<script lang="ts">
  // Editor di testo formattato con pochi comandi (grassetto, corsivo,
  // sottolineato, colore, collegamento), per testi brevi come la risposta
  // automatica.
  import Icon from '../lib/Icon.svelte';
  import RichEditor from './RichEditor.svelte';
  import LinkDialog from './LinkDialog.svelte';
  import { EditorCtl } from '../lib/editor.svelte';
  import { store } from '../lib/store.svelte';

  let {
    docKey,
    html,
    label,
    placeholder,
    height = 200,
    onchange,
  }: {
    docKey: string;
    html: string;
    label: string;
    placeholder: string;
    height?: number;
    onchange: (html: string, text: string) => void;
  } = $props();

  const ed = new EditorCtl();
  let linkBtn: HTMLButtonElement | undefined = $state();
  let linkOpen = $state(false);
</script>

<div class="box">
  <div class="bar" role="toolbar" aria-label="Formato di {label.toLowerCase()}">
    <button class="rb ico lt" aria-label="Grassetto" title="Grassetto (Ctrl+G)" aria-pressed={ed.bold} disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => ed.toggleBold()}><span style="font-weight:700">G</span></button>
    <button class="rb ico lt" aria-label="Corsivo" title="Corsivo (Ctrl+I)" aria-pressed={ed.italic} disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => ed.toggleItalic()}><span style="font-style:italic;font-family:Georgia,serif">C</span></button>
    <button class="rb ico lt" aria-label="Sottolineato" title="Sottolineato (Ctrl+S)" aria-pressed={ed.underline} disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => ed.toggleUnderline()}><span style="text-decoration:underline;text-underline-offset:2px">S</span></button>
    <label class="rb ico color" title="Colore carattere">
      <Icon name="text-color" /><span class="cbar" style:background={ed.lastColor}></span>
      <input type="color" aria-label="Colore carattere" value={ed.lastColor.toLowerCase()} onchange={(e) => ed.setColor(e.currentTarget.value.toUpperCase())} />
    </label>
    <span class="sep"></span>
    <button class="rb ico" aria-label="Elenco puntato" title="Elenco puntato" disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => ed.bullets()}><Icon name="list" /></button>
    <button class="rb ico" bind:this={linkBtn} aria-label="Collegamento" title="Inserisci collegamento (Ctrl+K)" aria-pressed={ed.link} disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => (linkOpen = true)}><Icon name="link" /></button>
    <button class="rb ico" aria-label="Cancella formattazione" title="Cancella formattazione" disabled={!ed.ready} onmousedown={(e) => e.preventDefault()} onclick={() => ed.clearFormat()}><Icon name="clear-formatting" /></button>
  </div>
  <div class="area" style:height="{height}px">
    <RichEditor ctl={ed} {docKey} {html} font={store.font} {label} {placeholder} {onchange} onlink={() => (linkOpen = true)} />
  </div>
</div>
{#if linkOpen && linkBtn}
  <LinkDialog ctl={ed} anchor={linkBtn} onclose={() => (linkOpen = false)} />
{/if}

<style>
  .box { border: 1px solid var(--rule-2); border-radius: 4px; overflow: hidden; max-width: 680px; }
  .bar { display: flex; align-items: center; flex-wrap: wrap; gap: 2px; padding: 4px 6px; background: var(--rail); box-shadow: inset 0 -1px 0 var(--rule-2); }
  .bar .rb { height: 30px; }
  .bar .rb.ico { width: 30px; }
  .lt span { font-size: 15px; line-height: 1; color: var(--ink); }
  .sep { width: 1px; height: 20px; margin: 0 3px; background: var(--rule-2); }
  .color { position: relative; cursor: pointer; }
  .color .cbar { position: absolute; left: 7px; right: 7px; bottom: 4px; height: 3px; border-radius: 1px; box-shadow: 0 0 0 1px color-mix(in srgb, var(--ink) 18%, transparent); }
  .color input { position: absolute; inset: 0; opacity: 0; width: 100%; height: 100%; cursor: pointer; }
  .area { display: flex; flex-direction: column; background: #ffffff; }
</style>
