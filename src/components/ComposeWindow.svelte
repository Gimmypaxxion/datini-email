<script lang="ts">
  // Messaggio in una finestra separata (vista classica).
  // Fase 1: è una finestra interna all'app. Nella fase 4 diventerà una vera
  // finestra del sistema operativo, come in Outlook classico.
  import Icon from '../lib/Icon.svelte';
  import Composer from './Composer.svelte';
  import RibbonCompose from './RibbonCompose.svelte';
  import { store } from '../lib/store.svelte';

  const title = $derived(store.c?.subj || 'Nuovo messaggio');
</script>

{#if store.winOpen}
  <div
    class="win win-in"
    data-max={store.winMax ? 'si' : 'no'}
    role="dialog"
    aria-label={title}
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        store.closeWin();
      }
    }}
  >
    <div class="bar">
      <span class="name">{title} - Messaggio</span>
      <button class="wbtn" onclick={() => (store.winMin = true)} aria-label="Riduci a icona" title="Riduci a icona"><Icon name="minus" size={16} /></button>
      <button class="wbtn" onclick={() => (store.winMax = !store.winMax)} aria-label={store.winMax ? 'Ripristina' : 'Ingrandisci'} title={store.winMax ? 'Ripristina' : 'Ingrandisci'}><Icon name="square" size={14} /></button>
      <button class="wbtn wx" onclick={() => store.closeWin()} aria-label="Chiudi e salva in Bozze" title="Chiudi e salva in Bozze"><Icon name="x" size={16} /></button>
    </div>
    <div class="cmds">
      <nav class="tabs-row" style="height:36px" aria-label="Schede del messaggio">
        {#each [['messaggio', 'Messaggio'], ['inserisci', 'Inserisci']] as [id, label] (id)}
          <button class="tab-btn btn" aria-pressed={store.winTab === id} onclick={() => (store.winTab = id as 'messaggio' | 'inserisci')}>{label}</button>
        {/each}
      </nav>
      <div class="rib" data-mode={store.prefs.ribbon} role="toolbar" aria-label="Comandi del messaggio">
        <RibbonCompose tab={store.winTab} />
      </div>
    </div>
    <div class="content">
      <Composer />
    </div>
  </div>
{/if}

{#if store.winMinimized}
  <button class="restore" onclick={() => (store.winMin = false)} title="Riapri il messaggio">
    <Icon name="file-pencil" style="color:var(--brand-ink)" /><span>{title}</span>
  </button>
{/if}

<style>
  .bar { height: 36px; flex: none; display: flex; align-items: center; gap: 10px; padding-left: 12px; background: var(--head); color: var(--on-head); }
  .name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; }
  .cmds { flex: none; background: var(--ground); padding: 0 10px 8px; }
  .content { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; background: var(--sheet); }
  .restore { position: absolute; right: 20px; bottom: 16px; z-index: 20; max-width: 340px; height: 40px; display: flex; align-items: center; gap: 10px; padding: 0 16px 0 12px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: var(--win-shadow), 0 0 0 1px var(--rule-2); color: var(--ink); font-size: 14px; }
  .restore span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
