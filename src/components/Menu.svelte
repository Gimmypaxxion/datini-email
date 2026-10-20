<script lang="ts" module>
  import type { IconName } from '../lib/icons';

  export interface MenuItem {
    label: string;
    icon?: IconName;
    disabled?: boolean;
    /** Rientro per le sottocartelle. */
    depth?: number;
    onclick?: () => void;
    kind?: 'item' | 'sep' | 'head';
  }
</script>

<script lang="ts">
  // Menu a comparsa sotto un pulsante. Si chiude con Esc, con un clic fuori
  // o dopo aver scelto una voce; le frecce spostano il fuoco tra le voci.
  import Icon from '../lib/Icon.svelte';

  let { anchor, items, label, onclose }: { anchor: HTMLElement; items: MenuItem[]; label: string; onclose: () => void } = $props();

  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ left: 0, top: 0 });

  $effect(() => {
    const r = anchor.getBoundingClientRect();
    const w = el?.offsetWidth ?? 240;
    const h = el?.offsetHeight ?? 200;
    pos = {
      left: Math.max(8, Math.min(r.left, window.innerWidth - w - 8)),
      top: r.bottom + 4 + h > window.innerHeight - 8 ? Math.max(8, r.top - h - 4) : r.bottom + 4,
    };
    el?.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    const away = (e: MouseEvent) => {
      if (!el?.contains(e.target as Node) && !anchor.contains(e.target as Node)) onclose();
    };
    const t = setTimeout(() => document.addEventListener('mousedown', away), 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener('mousedown', away);
    };
  });

  function onkey(e: KeyboardEvent) {
    const buttons = [...(el?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? [])];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
      anchor.focus();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      buttons[(i + 1) % buttons.length]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      buttons[(i - 1 + buttons.length) % buttons.length]?.focus();
    }
  }
</script>

<!-- svelte-ignore a11y_interactive_supports_focus -->
<div class="menu rise" bind:this={el} role="menu" aria-label={label} style:left="{pos.left}px" style:top="{pos.top}px" onkeydown={onkey}>
  {#each items as it, i (i)}
    {#if it.kind === 'sep'}
      <div class="menu-sep" role="separator"></div>
    {:else if it.kind === 'head'}
      <div class="menu-h">{it.label}</div>
    {:else}
      <button
        class="menu-it"
        role="menuitem"
        disabled={it.disabled}
        style:padding-left={it.depth ? `${10 + it.depth * 16}px` : undefined}
        onclick={() => {
          onclose();
          it.onclick?.();
        }}
      >
        {#if it.icon}<Icon name={it.icon} size={17} />{/if}
        <span class="lab">{it.label}</span>
      </button>
    {/if}
  {/each}
</div>
