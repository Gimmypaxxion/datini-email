<script lang="ts">
  // Barra in alto: niente nome né logo, così il programma sembra parte del sistema.
  // Ricerca al centro, poi impostazioni (vista nuova), guida e account.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { initials } from '../lib/format';

  const me = $derived(store.currentAccount);

  let searchEl: HTMLInputElement | undefined = $state();

  export function focusSearch() {
    searchEl?.focus();
    searchEl?.select();
  }
</script>

<header class="top">
  <div class="side" aria-hidden="true"></div>
  <div class="search-wrap">
    <label class="search">
      <span class="sr">Cerca nella posta</span>
      <Icon name="search" class="lens" />
      <input
        bind:this={searchEl}
        class="ph"
        type="search"
        value={store.q}
        oninput={(e) => void store.search(e.currentTarget.value)}
        onkeydown={(e) => {
          if (e.key === 'Escape' && store.q) {
            e.stopPropagation();
            void store.clearSearch();
          }
        }}
        placeholder="Cerca"
        title="Cerca (Ctrl+E)"
      />
      {#if store.q}
        <button class="ibtn btn clear" onclick={() => void store.clearSearch()} aria-label="Cancella ricerca"><Icon name="x" size={16} /></button>
      {/if}
    </label>
  </div>
  <div class="side right">
    {#if store.prefs.entry === 'ingranaggio'}
      <button class="hbtn" onclick={() => store.openSettings('aspetto')} aria-label="Impostazioni" title="Impostazioni"><Icon name="settings" size={20} /></button>
    {/if}
    <button class="hbtn" aria-label="Guida" title="Guida (F1)" onclick={() => (store.ribbonTab = 'aiuto')}><Icon name="help-circle" size={20} /></button>
    {#if me}
      <button class="me" aria-label="Account {me.email}" title="{me.name}&#10;{me.email}" onclick={() => { store.acctSel = me.id; store.openSettings('account'); }}>{initials(me.name || me.email)}</button>
    {:else}
      <button class="me none" aria-label="Aggiungi account" title="Aggiungi account" onclick={() => store.openWizard(false)}><Icon name="user-plus" size={17} /></button>
    {/if}
  </div>
</header>

<style>
  .top { height: 48px; flex: none; display: flex; align-items: center; gap: 16px; padding: 0 10px 0 14px; background: var(--head); color: var(--on-head); }
  .side { width: 280px; flex: none; }
  .side.right { display: flex; align-items: center; justify-content: flex-end; gap: 2px; }
  .search-wrap { flex: 1; min-width: 0; display: flex; justify-content: center; }
  .search { position: relative; display: block; width: 100%; max-width: 560px; }
  .search :global(.lens) { position: absolute; left: 10px; top: 7px; color: var(--brand-ink); }
  .search input { width: 100%; box-sizing: border-box; height: 32px; padding: 0 38px; border: 0; border-radius: 4px; background: var(--search); color: var(--ink); font: inherit; font-size: 14px; }
  .search input::-webkit-search-cancel-button { display: none; }
  .clear { position: absolute; right: 3px; top: 3px; width: 26px; height: 26px; }
  .me { width: 32px; height: 32px; margin-left: 8px; border: 0; border-radius: 50%; background: var(--av1-bg); color: var(--av1-fg); font-size: 12.5px; font-weight: 600; }
  .me:focus-visible { outline-color: var(--on-head); }
  .me.none { display: grid; place-items: center; background: var(--head-hover); color: var(--on-head); }
  @media (max-width: 760px) { .side:not(.right), .search-wrap { display: none; } }
</style>
