<script lang="ts">
  // Riquadro cartelle: Preferiti, cartelle dell'account principale e un
  // secondo account (POP) richiudibile, come in Outlook.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import { store } from '../lib/store.svelte';
  import { ME, type FolderKey } from '../lib/mock';

  interface Entry {
    key: FolderKey;
    id: string;
    label: string;
    icon: IconName;
  }

  const favs: Entry[] = [
    { key: 'inbox', id: 'fav-inbox', label: 'Posta in arrivo', icon: 'inbox' },
    { key: 'sent', id: 'fav-sent', label: 'Posta inviata', icon: 'send' },
    { key: 'drafts', id: 'fav-drafts', label: 'Bozze', icon: 'file-pencil' },
  ];
  const account: Entry[] = [
    { key: 'inbox', id: 'acc-inbox', label: 'Posta in arrivo', icon: 'inbox' },
    { key: 'drafts', id: 'acc-drafts', label: 'Bozze', icon: 'file-pencil' },
    { key: 'sent', id: 'acc-sent', label: 'Posta inviata', icon: 'send' },
    { key: 'archive', id: 'acc-archive', label: 'Archivio', icon: 'archive' },
    { key: 'junk', id: 'acc-junk', label: 'Posta indesiderata', icon: 'ban' },
    { key: 'deleted', id: 'acc-deleted', label: 'Elementi eliminati', icon: 'trash' },
    { key: 'clienti', id: 'acc-clienti', label: 'Clienti', icon: 'folder' },
    { key: 'fornitori', id: 'acc-fornitori', label: 'Fornitori', icon: 'folder' },
    { key: 'amministrazione', id: 'acc-amministrazione', label: 'Amministrazione', icon: 'folder' },
  ];
  const account2: Entry[] = [{ key: 'ordini', id: 'acc2-inbox', label: 'Posta in arrivo', icon: 'inbox' }];

  function countFor(key: FolderKey): number {
    if (key === 'inbox') return store.count('inbox', true);
    if (key === 'drafts') return store.count('drafts', false);
    if (key === 'junk') return store.count('junk', true);
    return 0;
  }
</script>

{#snippet folderRow(f: Entry)}
  {@const on = store.folderSel === f.id}
  {@const n = countFor(f.key)}
  <button class="btn fold" data-sel={on ? 'si' : 'no'} aria-current={on ? 'page' : undefined} onclick={() => store.pickFolder(f.key, f.id)}>
    <Icon name={f.icon} />
    <span class="name" style:font-weight={on ? 600 : 400}>{f.label}</span>
    {#if n}<span class="cnt" style:color={f.key === 'drafts' ? 'var(--ink-3)' : 'var(--brand-ink)'}>{n}</span>{/if}
  </button>
{/snippet}

<nav class="pane-folders scroll" aria-label="Cartelle">
  <div class="top">
    <button class="ibtn btn" onclick={() => store.toggleFolders()} aria-label="Comprimi riquadro cartelle" title="Comprimi riquadro cartelle"><Icon name="menu-2" size={20} /></button>
  </div>
  <h2 class="grp-h"><Icon name="chevron-down" size={14} stroke={1.8} />Preferiti</h2>
  {#each favs as f (f.id)}{@render folderRow(f)}{/each}

  <h2 class="grp-h" style="margin-top:14px"><Icon name="chevron-down" size={14} stroke={1.8} />{ME.email}</h2>
  {#each account as f (f.id)}{@render folderRow(f)}{/each}

  <h2 class="acct2">
    <button class="btn grp-h toggle" onclick={() => (store.acct2Open = !store.acct2Open)} aria-expanded={store.acct2Open}>
      <Icon name={store.acct2Open ? 'chevron-down' : 'chevron-right'} size={14} stroke={1.8} />
      <span>ordini@valliarredi.it</span>
    </button>
  </h2>
  {#if store.acct2Open}
    {#each account2 as f (f.id)}{@render folderRow(f)}{/each}
  {/if}

  <button class="btn add" onclick={() => store.openSettings('account')}><Icon name="user-plus" />Aggiungi account</button>
</nav>

<style>
  .pane-folders { width: 240px; flex: none; box-sizing: border-box; overflow: auto; padding: 0 8px 20px 2px; background: var(--ground); }
  .top { height: 34px; display: flex; align-items: center; }
  .acct2 { margin: 12px 0 2px; padding: 0; }
  .toggle { width: 100%; margin: 0; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .toggle span { overflow: hidden; text-overflow: ellipsis; }
  .add { width: 100%; height: 32px; margin-top: 10px; display: flex; align-items: center; gap: 10px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; text-align: left; }
</style>
