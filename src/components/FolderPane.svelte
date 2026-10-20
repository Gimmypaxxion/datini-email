<script lang="ts">
  // Riquadro cartelle: Preferiti, poi le cartelle di ogni account e dei file
  // di dati, richiudibili, con i conteggi come in Outlook.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import { store, folderLabel } from '../lib/store.svelte';
  import type { Account, Folder, Role } from '../lib/types';

  const ICON: Record<Role, IconName> = {
    inbox: 'inbox',
    drafts: 'file-pencil',
    sent: 'send',
    deleted: 'trash',
    archive: 'archive',
    junk: 'ban',
  };

  const primary = $derived(store.accounts.find((a) => a.kind !== 'local') ?? store.accounts[0]);
  const favs = $derived(
    primary ? (['inbox', 'sent', 'drafts'] as Role[]).map((r) => store.roleFolder(primary.id, r)).filter((f): f is Folder => !!f) : [],
  );

  function count(f: Folder): { n: number; muted: boolean } {
    if (f.role === 'drafts') return { n: f.total, muted: true };
    if (f.role === 'sent' || f.role === 'archive') return { n: 0, muted: false };
    return { n: f.unread, muted: false };
  }

  /** Rientro: le cartelle speciali stanno a sinistra; sui server che mettono
   *  tutto dentro INBOX (Courier, cPanel) quel livello non si conta. */
  function depthOf(f: Folder): number {
    if (f.role) return 0;
    return Math.max(0, f.depth - (/^inbox[./]/i.test(f.path) ? 1 : 0));
  }

  function title(a: Account): string {
    return a.kind === 'local' ? a.name : a.email;
  }

  function statusIcon(a: Account): { icon: IconName; text: string } | null {
    if (a.status.state === 'offline') return { icon: 'cloud-off', text: a.status.message ?? 'Offline' };
    if (a.status.state === 'errore') return { icon: 'alert-triangle', text: a.status.message ?? 'Errore di sincronizzazione' };
    return null;
  }
</script>

{#snippet folderRow(f: Folder, prefix: string, depth: number)}
  {@const id = `${prefix}-${f.id}`}
  {@const on = store.folderSel === id}
  {@const c = count(f)}
  <button
    class="btn fold"
    data-sel={on ? 'si' : 'no'}
    aria-current={on ? 'page' : undefined}
    style:padding-left="{28 + depth * 16}px"
    disabled={!f.selectable}
    onclick={() => store.pickFolder(f.id, id)}
  >
    <Icon name={f.role ? ICON[f.role] : 'folder'} />
    <span class="name" style:font-weight={on ? 600 : 400}>{folderLabel(f)}</span>
    {#if c.n}<span class="cnt" style:color={c.muted ? 'var(--ink-3)' : 'var(--brand-ink)'}>{c.n}</span>{/if}
  </button>
{/snippet}

<nav class="pane-folders scroll" aria-label="Cartelle">
  <div class="top">
    <button class="ibtn btn" onclick={() => store.toggleFolders()} aria-label="Comprimi riquadro cartelle" title="Comprimi riquadro cartelle"><Icon name="menu-2" size={20} /></button>
  </div>

  {#if favs.length}
    <h2 class="grp-h"><Icon name="chevron-down" size={14} stroke={1.8} />Preferiti</h2>
    {#each favs as f (f.id)}{@render folderRow(f, 'fav', 0)}{/each}
  {/if}

  {#each store.accounts as a (a.id)}
    {@const open = !store.collapsed[a.id]}
    {@const st = statusIcon(a)}
    {@const out = store.outbox.filter((o) => o.accountId === a.id).length}
    <h2 class="acct">
      <button class="btn grp-h toggle" onclick={() => store.toggleSection(a.id)} aria-expanded={open} title={title(a)}>
        <Icon name={open ? 'chevron-down' : 'chevron-right'} size={14} stroke={1.8} />
        {#if a.kind === 'local'}<Icon name="database" size={15} style="color:var(--ink-2)" />{/if}
        <span class="aname">{title(a)}</span>
        {#if st}<span class="st" title={st.text}><Icon name={st.icon} size={15} style="color:var(--warn)" /><span class="sr">{st.text}</span></span>{/if}
      </button>
    </h2>
    {#if open}
      {#each store.foldersOf(a.id) as f (f.id)}{@render folderRow(f, 'acc', depthOf(f))}{/each}
      {#if out}
        {@const on = store.folderSel === `uscita-${a.id}`}
        <button class="btn fold" data-sel={on ? 'si' : 'no'} aria-current={on ? 'page' : undefined} onclick={() => store.pickOutbox(`uscita-${a.id}`)}>
          <Icon name="mail-up" />
          <span class="name" style:font-weight={on ? 600 : 400}>Posta in uscita</span>
          <span class="cnt" style:color="var(--brand-ink)">{out}</span>
        </button>
      {/if}
    {/if}
  {/each}

  {#if !store.accounts.length && store.ready}
    <p class="none">Nessun account. Aggiungine uno per vedere qui le tue cartelle.</p>
  {/if}

  <button class="btn add" onclick={() => store.openWizard(false)}><Icon name="user-plus" />Aggiungi account</button>
</nav>

<style>
  .pane-folders { width: 240px; flex: none; box-sizing: border-box; overflow: auto; padding: 0 8px 20px 2px; background: var(--ground); }
  .top { height: 34px; display: flex; align-items: center; }
  .acct { margin: 12px 0 2px; padding: 0; font: inherit; }
  .toggle { width: 100%; margin: 0; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .aname { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .st { display: inline-flex; }
  .none { margin: 16px 10px 0; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .add { width: 100%; height: 32px; margin-top: 10px; display: flex; align-items: center; gap: 10px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; text-align: left; }
</style>
