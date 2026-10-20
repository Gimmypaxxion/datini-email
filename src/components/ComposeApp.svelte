<script lang="ts">
  // Finestra del sistema per scrivere un messaggio, come in Outlook classico.
  // Riceve la bozza dalla finestra principale; chiudendola la bozza va in
  // Bozze, inviando parte (con qualche secondo per annullare dalla finestra
  // principale).
  import Composer from './Composer.svelte';
  import RibbonCompose from './RibbonCompose.svelte';
  import { store } from '../lib/store.svelte';
  import { setWindowTitle } from '../lib/platform';

  let ready = $state(false);
  let closing = false;

  async function closeWindow(save: boolean) {
    if (closing) return;
    closing = true;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    if (save) await store.closeWin();
    await getCurrentWindow().destroy();
  }

  $effect(() => {
    document.documentElement.dataset.theme = store.theme;
  });

  $effect(() => {
    const s = store.c?.subject.trim();
    setWindowTitle(`${s || 'Senza titolo'} - Messaggio`);
  });

  // Inviato o eliminato: la finestra non serve più.
  $effect(() => {
    if (ready && !store.c) void closeWindow(false);
  });

  $effect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      await store.reloadAccounts();
      const d = await store.backend.takeComposeDraft();
      if (!d) {
        await closeWindow(false);
        return;
      }
      store.startCompose(d, true);
      ready = true;
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      unlisten = await getCurrentWindow().onCloseRequested(async (e) => {
        e.preventDefault();
        await closeWindow(true);
      });
    })();
    return () => unlisten?.();
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      void closeWindow(true);
    }
  }}
/>

<div class="app">
  {#if ready && store.c}
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
  {/if}
</div>

<style>
  .app { height: 100vh; display: flex; flex-direction: column; background: var(--ground); }
  .cmds { flex: none; padding: 4px 10px 8px; }
  .content { flex: 1; min-height: 0; display: flex; flex-direction: column; margin: 0 10px 10px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
</style>
