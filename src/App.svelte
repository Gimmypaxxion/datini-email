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
    { label: 'Attività', icon: 'checkbox', key: 'Ctrl+4', module: null },
  ] as const;
</script>

<svelte:window onkeydown={(e) => handleShortcut(e, { focusSearch: () => header?.focusSearch() })} />

<div class="shell">
  <Header bind:this={header} />
  <Ribbon />
  <div class="panes">
    <nav class="apps" aria-label="App">
      {#each apps as a (a.label)}
        {@const current = a.module === store.module}
        <button
          class="app btn"
          class:current
          aria-label={a.label}
          aria-current={current ? 'page' : undefined}
          title="{a.label} ({a.key})"
          onclick={() => (a.module ? store.setModule(a.module) : store.soon(a.label))}
        >
          <Icon name={a.icon} size={20} stroke={current ? 1.7 : 1.6} />
        </button>
      {/each}
    </nav>
    {#if store.module === 'contatti'}
      <ContactsView />
    {:else if store.module === 'calendario'}
      <CalendarView />
    {:else}
      {#if store.showFolders}<FolderPane />{/if}
      <MessageList />
      <ReadingPane />
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
  .apps { width: 52px; flex: none; display: flex; flex-direction: column; align-items: center; gap: 4px; padding-top: 2px; }
  .app { width: 40px; height: 40px; border: 0; border-radius: 4px; background: transparent; display: grid; place-items: center; color: var(--ink-2); }
  .app.current { background: var(--brand-soft); color: var(--brand-ink); }
  @media (max-width: 760px) { .apps { display: none; } }
</style>
