<script lang="ts">
  // Pulsante della barra dei comandi. Lo stesso markup funziona in modalità
  // estesa e compatta: è il CSS di .rib[data-mode] a cambiarne la forma.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';

  let {
    icon,
    label,
    tone = '',
    big = false,
    menu = false,
    showLabel = true,
    labelExtOnly = false,
    alwaysLabel = false,
    iconOnly = false,
    disabled = false,
    title = undefined,
    cls = '',
    pressed = undefined,
    onclick = undefined,
  }: {
    icon: IconName;
    label: string;
    tone?: string;
    big?: boolean;
    menu?: boolean;
    showLabel?: boolean;
    labelExtOnly?: boolean;
    /** Etichetta sempre visibile, anche quando la finestra è stretta. */
    alwaysLabel?: boolean;
    iconOnly?: boolean;
    disabled?: boolean;
    title?: string;
    cls?: string;
    pressed?: boolean;
    onclick?: () => void;
  } = $props();
</script>

<button
  class="rb {big ? 'big' : ''} {iconOnly ? 'ico' : ''} {cls}"
  {disabled}
  title={title ?? label}
  aria-label={label}
  aria-pressed={pressed}
  {onclick}
>
  <Icon name={icon} class={tone} />
  {#if showLabel && !iconOnly}
    {#if menu && big}
      <span class="bl"><span class="lbl">{label}</span><Icon name="chevron-down" size={14} class="chev" /></span>
    {:else}
      <span class={alwaysLabel ? '' : `lbl ${labelExtOnly ? 'lbl-x' : ''}`}>{label}</span>
      {#if menu}<Icon name="chevron-down" size={14} class="chev" />{/if}
    {/if}
  {:else if menu}
    <Icon name="chevron-down" size={14} class="chev" />
  {/if}
</button>
