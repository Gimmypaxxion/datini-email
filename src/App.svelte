<script lang="ts">
  import Header from './components/Header.svelte';
  import Ribbon from './components/Ribbon.svelte';
  import FolderPane from './components/FolderPane.svelte';
  import MessageList from './components/MessageList.svelte';
  import ReadingPane from './components/ReadingPane.svelte';
  import ComposeWindow from './components/ComposeWindow.svelte';
  import Backstage from './components/Backstage.svelte';
  import Settings from './components/Settings.svelte';
  import Onboarding from './components/Onboarding.svelte';
  import AccountWizard from './components/AccountWizard.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import Icon from './lib/Icon.svelte';
  import { store } from './lib/store.svelte';
  import { setWindowTitle } from './lib/platform';
  import { handleShortcut } from './lib/shortcuts';

  let header: Header | undefined = $state();

  // Tema: l'attributo sull'elemento <html> attiva i colori chiari o scuri.
  $effect(() => {
    document.documentElement.dataset.theme = store.theme;
  });

  // Titolo della finestra del sistema, come in Outlook: "Posta in arrivo - Datini".
  $effect(() => {
    const where = store.c && store.cWin ? store.c.subject || 'Nuovo messaggio' : store.listTitle || 'Posta';
    setWindowTitle(`${where} - Datini`);
  });

  const apps = [
    { label: 'Posta', icon: 'mail', key: 'Ctrl+1', current: true },
    { label: 'Calendario', icon: 'calendar', key: 'Ctrl+2', current: false },
    { label: 'Contatti', icon: 'users', key: 'Ctrl+3', current: false },
    { label: 'Attività', icon: 'checkbox', key: 'Ctrl+4', current: false },
  ] as const;
</script>

<svelte:window onkeydown={(e) => handleShortcut(e, { focusSearch: () => header?.focusSearch() })} />

<div class="shell">
  <Header bind:this={header} />
  <Ribbon />
  <div class="panes">
    <nav class="apps" aria-label="App">
      {#each apps as a (a.label)}
        <button
          class="app btn"
          class:current={a.current}
          aria-label={a.label}
          aria-current={a.current ? 'page' : undefined}
          title="{a.label} ({a.key})"
          onclick={() => !a.current && store.soon(a.label)}
        >
          <Icon name={a.icon} size={20} stroke={a.current ? 1.7 : 1.6} />
        </button>
      {/each}
    </nav>
    {#if store.showFolders}<FolderPane />{/if}
    <MessageList />
    <ReadingPane />
  </div>
  <StatusBar />
  <ComposeWindow />
  <Backstage />
  <Settings />
  <AccountWizard />
  <Onboarding />
</div>

<style>
  .shell { position: relative; height: 100vh; min-height: 560px; display: flex; flex-direction: column; overflow: hidden; }
  .panes { flex: 1; min-height: 0; display: flex; }
  .apps { width: 52px; flex: none; display: flex; flex-direction: column; align-items: center; gap: 4px; padding-top: 2px; }
  .app { width: 40px; height: 40px; border: 0; border-radius: 4px; background: transparent; display: grid; place-items: center; color: var(--ink-2); }
  .app.current { background: var(--brand-soft); color: var(--brand-ink); }
  @media (max-width: 760px) { .apps { display: none; } }
</style>
