<script lang="ts">
  // Area di scrittura con testo formattato. Il contenuto si carica quando
  // cambia `docKey` (un altro messaggio, un'altra firma); da lì in poi è
  // l'editor a dire cosa è cambiato, con `onchange`.
  import { onDestroy } from 'svelte';
  import { editorDocument, type EditorCtl } from '../lib/editor.svelte';

  let {
    ctl,
    docKey,
    html,
    font,
    placeholder = 'Scrivi il messaggio',
    focusOnLoad = 'none',
    label = 'Testo del messaggio',
    onchange,
    onsubmit,
    onescape,
    onlink,
    fetchImage,
  }: {
    ctl: EditorCtl;
    docKey: string;
    html: string;
    font: { family: string; size: number };
    placeholder?: string;
    focusOnLoad?: 'none' | 'start';
    label?: string;
    onchange: (html: string, text: string) => void;
    onsubmit?: () => void;
    onescape?: () => void;
    onlink?: () => void;
    fetchImage?: (url: string) => Promise<string | null>;
  } = $props();

  let frame: HTMLIFrameElement | undefined = $state();
  let loadedKey = '';

  // Il documento dell'iframe si crea una volta per carattere predefinito;
  // il testo si carica dopo, a ogni nuovo messaggio.
  const srcdoc = $derived(editorDocument(font));

  function load() {
    if (!frame?.contentDocument?.body) return;
    loadedKey = docKey;
    ctl.attach(frame, html, font, { onchange, onsubmit, onescape, onlink, fetchImage }, placeholder, label);
    if (focusOnLoad === 'start') setTimeout(() => ctl.focus('start'), 0);
  }

  $effect(() => {
    // Nuovo messaggio nello stesso editor: si ricarica il contenuto.
    if (docKey !== loadedKey && frame?.contentDocument?.readyState === 'complete' && frame.contentDocument.body) load();
  });

  onDestroy(() => ctl.detach());
</script>

<!-- Niente attributo sandbox: WebKit (macOS e Linux) non eseguirebbe i
     gestori di eventi dell'editor. Gli script restano comunque impossibili:
     la Content-Security-Policy del documento li vieta tutti e il contenuto
     viene ripulito prima di entrare. -->
<iframe bind:this={frame} class="rte" title={label} {srcdoc} onload={load}></iframe>

<style>
  .rte { display: block; flex: 1; min-height: 0; width: 100%; border: 0; background: #ffffff; color-scheme: light; }
</style>
