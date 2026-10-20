<script lang="ts">
  // Testo HTML di un messaggio, in un iframe isolato: niente script
  // (sandbox senza allow-scripts), e la Content-Security-Policy messa dal
  // motore nel documento blocca le risorse esterne non consentite.
  // I collegamenti si aprono nel browser predefinito, mai dentro Datini.

  let { html, onlink }: { html: string; onlink: (url: string) => void } = $props();

  let frame: HTMLIFrameElement | undefined = $state();
  let height = $state(120);
  let observer: ResizeObserver | null = null;

  function setup() {
    const doc = frame?.contentDocument;
    if (!doc) return;
    const measure = () => {
      height = Math.max(doc.documentElement?.scrollHeight ?? 0, doc.body?.scrollHeight ?? 0, 40) + 4;
    };
    measure();
    observer?.disconnect();
    observer = new ResizeObserver(measure);
    observer.observe(doc.documentElement);
    for (const img of doc.querySelectorAll('img')) img.addEventListener('load', measure);
    for (const a of doc.querySelectorAll('a[href]')) {
      // L'indirizzo vero si vede passandoci sopra: aiuta a riconoscere i collegamenti ingannevoli.
      if (!a.getAttribute('title')) a.setAttribute('title', a.getAttribute('href') ?? '');
    }
    doc.addEventListener('click', (e) => {
      const a = (e.target as Element | null)?.closest?.('a');
      if (!a) return;
      e.preventDefault();
      const href = a.getAttribute('href') ?? '';
      if (/^(https?:|mailto:)/i.test(href)) onlink(href);
    });
    // Le scorciatoie (Ctrl+R, Canc...) devono funzionare anche col fuoco nel messaggio.
    doc.addEventListener('keydown', (e) => {
      const copy = new KeyboardEvent('keydown', { key: e.key, code: e.code, ctrlKey: e.ctrlKey, metaKey: e.metaKey, shiftKey: e.shiftKey, altKey: e.altKey, bubbles: true, cancelable: true });
      window.dispatchEvent(copy);
      if (copy.defaultPrevented) e.preventDefault();
    });
  }

  $effect(() => () => observer?.disconnect());
</script>

<div class="mail">
  <iframe bind:this={frame} title="Contenuto del messaggio" sandbox="allow-same-origin" srcdoc={html} onload={setup} style:height="{height}px"></iframe>
</div>

<style>
  .mail { border-radius: 6px; background: #ffffff; overflow: hidden; }
  :global([data-theme="scuro"]) .mail { box-shadow: 0 0 0 1px var(--rule); padding: 12px 14px; }
  iframe { display: block; width: 100%; border: 0; background: #ffffff; color-scheme: light; }
</style>
