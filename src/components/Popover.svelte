<script lang="ts">
  // Riquadro a comparsa sotto un pulsante (o vicino a un punto): elenchi dei
  // caratteri, colori, griglia della tabella, collegamento. Si chiude con Esc
  // o con un clic fuori, e rimette il fuoco al pulsante.
  import type { Snippet } from 'svelte';

  let {
    anchor,
    label,
    onclose,
    children,
    role = 'dialog',
    width = undefined,
  }: { anchor: HTMLElement; label: string; onclose: () => void; children: Snippet; role?: string; width?: number } = $props();

  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  // Il riquadro va in fondo alla pagina: dentro una finestra animata la
  // posizione fissa sarebbe calcolata dalla finestra, non dallo schermo.
  $effect(() => {
    if (el && el.parentElement !== document.body) document.body.appendChild(el);
  });

  $effect(() => {
    const place = () => {
      const r = anchor.getBoundingClientRect();
      const w = el?.offsetWidth ?? 240;
      const h = el?.offsetHeight ?? 200;
      pos = {
        left: Math.max(8, Math.min(r.left, window.innerWidth - w - 8)),
        top: r.bottom + 4 + h > window.innerHeight - 8 ? Math.max(8, r.top - h - 4) : r.bottom + 4,
      };
    };
    place();
    requestAnimationFrame(place);
    const first = el?.querySelector<HTMLElement>('[data-first]') ?? el?.querySelector<HTMLElement>('input, button:not(:disabled)');
    first?.focus();
    const away = (e: MouseEvent) => {
      if (!el?.contains(e.target as Node) && !anchor.contains(e.target as Node)) onclose();
    };
    const t = setTimeout(() => document.addEventListener('mousedown', away), 0);
    window.addEventListener('blur', onclose);
    return () => {
      clearTimeout(t);
      document.removeEventListener('mousedown', away);
      window.removeEventListener('blur', onclose);
    };
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      e.preventDefault();
      const a = anchor;
      onclose();
      a?.focus();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="pop rise" bind:this={el} {role} aria-label={label} style:left="{pos.left}px" style:top="{pos.top}px" style:width={width ? `${width}px` : undefined} onkeydown={onkey}>
  {@render children()}
</div>

<style>
  .pop { position: fixed; z-index: 60; min-width: 160px; max-height: min(460px, calc(100vh - 16px)); overflow: auto; padding: 6px; border-radius: 6px; background: var(--sheet); box-shadow: var(--win-shadow), 0 0 0 1px var(--rule-2); color: var(--ink); }
</style>
