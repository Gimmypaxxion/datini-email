<script lang="ts">
  import { icons, type IconName } from './icons';

  let {
    name,
    size = 18,
    stroke = 1.6,
    class: cls = '',
    style = '',
  }: { name: IconName; size?: number; stroke?: number; class?: string; style?: string } = $props();

  const Component = $derived(icons[name]);
</script>

<Component {size} {stroke} class={`i ${cls}`} {style} aria-hidden="true" focusable="false" />
