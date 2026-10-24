<script lang="ts">
  // Separatore trascinabile fra l'elenco dei messaggi e il riquadro di lettura,
  // come in Outlook: in verticale (riquadro a destra) o in orizzontale (in basso).
  // Doppio clic: torna alla misura predefinita. Con la tastiera: frecce.
  import { store, LIST_W, LIST_H } from '../lib/store.svelte';

  let { dir }: { dir: 'x' | 'y' } = $props();

  let el = $state<HTMLElement | null>(null);
  let dragging = $state(false);

  const value = $derived(dir === 'x' ? store.listW : Math.round(store.listH * 100));

  /** Il pannello che si ridimensiona è quello subito prima del separatore. */
  function bounds() {
    const parent = el?.parentElement?.getBoundingClientRect();
    const list = el?.previousElementSibling?.getBoundingClientRect();
    return { parent, list };
  }

  function setX(w: number) {
    const { parent, list } = bounds();
    if (!parent || !list) return;
    // Almeno 280 px all'elenco e 380 px al messaggio.
    const max = Math.max(280, parent.right - list.left - 380);
    store.listW = Math.round(Math.min(max, Math.max(280, w)));
  }

  function setY(h: number) {
    const { parent } = bounds();
    if (!parent?.height) return;
    store.listH = Math.min(0.75, Math.max(0.2, h / parent.height));
  }

  function down(e: PointerEvent) {
    if (e.button !== 0 || !el) return;
    e.preventDefault();
    el.setPointerCapture(e.pointerId);
    dragging = true;
  }

  function move(e: PointerEvent) {
    if (!dragging) return;
    const { parent, list } = bounds();
    if (!parent || !list) return;
    if (dir === 'x') setX(e.clientX - list.left);
    else setY(e.clientY - parent.top);
  }

  function up(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    el?.releasePointerCapture(e.pointerId);
    store.save();
  }

  function reset() {
    if (dir === 'x') store.listW = LIST_W;
    else store.listH = LIST_H;
    store.save();
  }

  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 4 : 1;
    const back = dir === 'x' ? 'ArrowLeft' : 'ArrowUp';
    const fwd = dir === 'x' ? 'ArrowRight' : 'ArrowDown';
    if (e.key !== back && e.key !== fwd && e.key !== 'Home') return;
    e.preventDefault();
    if (e.key === 'Home') return reset();
    const sign = e.key === fwd ? 1 : -1;
    if (dir === 'x') setX(store.listW + sign * 16 * step);
    else {
      const { parent } = bounds();
      if (parent) setY(store.listH * parent.height + sign * 16 * step);
    }
    store.save();
  }
</script>

<!-- Un separatore che si può mettere a fuoco è un controllo (ARIA "separator" con valore). -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  bind:this={el}
  class="split"
  class:dragging
  data-dir={dir}
  role="separator"
  tabindex="0"
  aria-orientation={dir === 'x' ? 'vertical' : 'horizontal'}
  aria-label={dir === 'x' ? "Larghezza dell'elenco dei messaggi" : "Altezza dell'elenco dei messaggi"}
  aria-valuenow={value}
  aria-valuemin={dir === 'x' ? 280 : 20}
  aria-valuemax={dir === 'x' ? 1600 : 75}
  title="Trascina per cambiare la misura. Doppio clic: misura predefinita"
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  ondblclick={reset}
  onkeydown={key}
></div>

<style>
  /* Il separatore non occupa spazio: la zona da afferrare è lo spazio vuoto fra i pannelli. */
  .split { position: relative; flex: none; z-index: 3; outline: none; touch-action: none; }
  .split[data-dir="x"] { width: 0; align-self: stretch; margin-bottom: 10px; }
  .split[data-dir="y"] { height: 0; margin-right: 10px; }
  .split::before { content: ''; position: absolute; }
  .split[data-dir="x"]::before { top: 0; bottom: 0; left: -1px; width: 10px; cursor: col-resize; }
  .split[data-dir="y"]::before { left: 0; right: 0; top: -9px; height: 10px; cursor: row-resize; }
  /* Linea sottile al centro dello spazio, solo mentre serve. */
  .split::after { content: ''; position: absolute; border-radius: 2px; background: var(--brand); opacity: 0; transition: opacity .12s ease; pointer-events: none; }
  .split[data-dir="x"]::after { top: 8px; bottom: 8px; left: 3px; width: 2px; }
  .split[data-dir="y"]::after { left: 8px; right: 8px; top: -5px; height: 2px; }
  .split:hover::after { opacity: .55; transition-delay: .25s; }
  .split.dragging::after, .split:focus-visible::after { opacity: 1; transition-delay: 0s; }
  @media (max-width: 760px) { .split { display: none; } }
</style>
