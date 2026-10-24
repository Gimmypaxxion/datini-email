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
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import AccountWizard from './components/AccountWizard.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import ContactsView from './components/ContactsView.svelte';
  import CalendarView from './components/CalendarView.svelte';
  import EventEditor from './components/EventEditor.svelte';
  import Reminders from './components/Reminders.svelte';
  import Splitter from './components/Splitter.svelte';
  import TasksView from './components/TasksView.svelte';
  import Icon from './lib/Icon.svelte';
  import { store } from './lib/store.svelte';
  import { setWindowTitle } from './lib/platform';
  import { applyAccent } from './lib/accent';
  import { handleShortcut } from './lib/shortcuts';

  let header: Header | undefined = $state();

  // Tema: l'attributo sull'elemento <html> attiva i colori chiari o scuri;
  // il colore di accento sostituisce il blu nelle variabili CSS.
  $effect(() => {
    document.documentElement.dataset.theme = store.theme;
    applyAccent(document.documentElement, store.accent, store.theme);
  });

  // Titolo della finestra del sistema, come in Outlook: "Posta in arrivo - Better-Look".
  $effect(() => {
    const where = store.module === 'contatti' ? 'Contatti' : store.module === 'calendario' ? 'Calendario' : store.c && store.cWin ? store.c.subject || 'Nuovo messaggio' : store.listTitle || 'Posta';
    setWindowTitle(`${where} - Better-Look`);
  });

  const apps = [
    { label: 'Posta', icon: 'mail', key: 'Ctrl+1', module: 'posta' },
    { label: 'Calendario', icon: 'calendar', key: 'Ctrl+2', module: 'calendario' },
    { label: 'Contatti', icon: 'users', key: 'Ctrl+3', module: 'contatti' },
    { label: 'Attività', icon: 'checkbox', key: 'Ctrl+4', module: 'attivita' },
  ] as const;
</script>

<svelte:window onkeydown={(e) => handleShortcut(e, { focusSearch: () => header?.focusSearch() })} />

<div class="shell">
  <Header bind:this={header} />
  <Ribbon />
  <div class="panes" data-reading={store.module === 'posta' ? store.readingPane : null} style="--list-w: {store.listW}px; --list-h: {(store.listH * 100).toFixed(1)}%">
    <nav class="apps" aria-label="App">
      {#each apps as a (a.label)}
        {@const current = a.module === store.module}
        <button
          class="app btn"
          class:current
          aria-label={a.label}
          aria-current={current ? 'page' : undefined}
          title="{a.label} ({a.key})"
          onclick={() => store.setModule(a.module)}
        >
          <Icon name={a.icon} size={20} stroke={current ? 1.7 : 1.6} />
        </button>
      {/each}
    </nav>
    {#if store.module === 'contatti'}
      <ContactsView />
    {:else if store.module === 'calendario'}
      <CalendarView />
    {:else if store.module === 'attivita'}
      <TasksView />
    {:else}
      {#if store.showFolders}<FolderPane />{/if}
      {#if store.readingPane === 'basso'}
        <div class="stack"><MessageList /><Splitter dir="y" /><ReadingPane /></div>
      {:else if store.readingPane === 'nascosto'}
        <!-- Senza riquadro di lettura il messaggio aperto prende il posto dell'elenco. -->
        {#if (store.fullRead && store.sel !== null) || store.paneCompose}<ReadingPane />{:else}<MessageList />{/if}
      {:else}
        <MessageList />
        <Splitter dir="x" />
        <ReadingPane />
      {/if}
    {/if}
  </div>
  <StatusBar />
  <ComposeWindow />
  <Backstage />
  <Settings />
  <AccountWizard />
  <Onboarding />
  <EventEditor />
  <Reminders />
  <ConfirmDialog />
</div>

<style>
  .shell { position: relative; height: 100vh; min-height: 560px; display: flex; flex-direction: column; overflow: hidden; }
  .panes { flex: 1; min-height: 0; display: flex; }
  /* Riquadro di lettura in basso: elenco sopra, messaggio sotto. */
  .stack { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
  .stack :global(.pane-list) { width: auto; flex: 0 0 var(--list-h, 46%); min-height: 180px; margin: 0 10px 8px 0; }
  .stack :global(.pane-read) { flex: 1; min-height: 0; padding-left: 0; }
  /* Riquadro nascosto: l'elenco occupa tutto lo spazio. */
  .panes[data-reading="nascosto"] :global(.pane-list) { width: auto; flex: 1 1 auto; margin-right: 10px; }
  .apps { width: 52px; flex: none; display: flex; flex-direction: column; align-items: center; gap: 4px; padding-top: 2px; }
  .app { width: 40px; height: 40px; border: 0; border-radius: 4px; background: transparent; display: grid; place-items: center; color: var(--ink-2); }
  .app.current { background: var(--brand-soft); color: var(--brand-ink); }
  @media (max-width: 760px) { .apps { display: none; } }
</style>
