<script lang="ts">
  // Riquadro cartelle: Preferiti, poi le cartelle di ogni account e dei file
  // di dati, richiudibili, con i conteggi come in Outlook. Clic destro su una
  // cartella o sull'account: nuova cartella, rinomina, elimina, segna tutti
  // come letti, svuota. I messaggi si trascinano sulle cartelle. Sotto ogni
  // account le cartelle di ricerca, ricerche salvate che si aprono come cartelle.
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import Menu, { type MenuItem } from './Menu.svelte';
  import { store, folderLabel, DRAG_TYPE } from '../lib/store.svelte';
  import type { Account, Folder, Role, SearchFolder } from '../lib/types';
  import { describeCriteria, presetOf, searchFolderCount } from '../lib/searchfolders';

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

  // ---------- Menu contestuale ----------

  let ctx = $state<{ el: HTMLElement; x: number; y: number; items: MenuItem[]; label: string } | null>(null);

  function folderMenu(f: Folder): MenuItem[] {
    const items: MenuItem[] = [
      { label: 'Nuova cartella', icon: 'folder-plus', onclick: () => store.startNewFolder(f.accountId, f.id) },
      { label: 'Nuova cartella di ricerca…', icon: 'folder-search', keys: 'Ctrl+Maiusc+P', onclick: () => store.openSearchFolderDialog({ accountId: f.accountId }) },
      { label: 'Rinomina cartella', icon: 'pencil', keys: 'F2', disabled: !!f.role, onclick: () => store.startRenameFolder(f.id) },
      { label: 'Elimina cartella', icon: 'trash', disabled: !!f.role, onclick: () => store.deleteFolder(f.id) },
      { kind: 'sep', label: '' },
      { label: 'Segna tutti come letti', icon: 'mail-opened', disabled: f.unread === 0, onclick: () => store.markFolderRead(f.id) },
    ];
    if (f.role === 'deleted' || f.role === 'junk') {
      items.push({ label: f.role === 'deleted' ? 'Svuota Posta eliminata' : 'Svuota Posta indesiderata', icon: 'trash-x', disabled: f.total === 0, onclick: () => store.emptyFolder(f.id) });
    }
    return items;
  }

  function openFolderMenu(e: MouseEvent, f: Folder) {
    e.preventDefault();
    ctx = { el: e.currentTarget as HTMLElement, x: e.clientX, y: e.clientY, items: folderMenu(f), label: `Cartella ${folderLabel(f)}` };
  }

  function openAccountMenu(e: MouseEvent, a: Account) {
    e.preventDefault();
    ctx = {
      el: e.currentTarget as HTMLElement,
      x: e.clientX,
      y: e.clientY,
      label: `Account ${title(a)}`,
      items: [
        { label: 'Nuova cartella', icon: 'folder-plus', onclick: () => store.startNewFolder(a.id, null) },
        ...(a.kind !== 'local' ? [{ label: 'Nuova cartella di ricerca…', icon: 'folder-search' as IconName, keys: 'Ctrl+Maiusc+P', onclick: () => store.openSearchFolderDialog({ accountId: a.id }) }] : []),
      ],
    };
  }

  function searchFolderMenu(f: SearchFolder): MenuItem[] {
    return [
      { label: 'Modifica cartella di ricerca…', icon: 'pencil', onclick: () => store.openSearchFolderDialog({ edit: f }) },
      { label: 'Segna tutti come letti', icon: 'mail-opened', disabled: f.unread === 0, onclick: () => store.markSearchFolderRead(f.id) },
      { kind: 'sep', label: '' },
      { label: 'Nuova cartella di ricerca…', icon: 'folder-search', onclick: () => store.openSearchFolderDialog({ accountId: f.accountId }) },
      { label: 'Elimina cartella di ricerca', icon: 'trash', onclick: () => store.deleteSearchFolder(f.id) },
    ];
  }

  function openSearchFolderMenu(e: MouseEvent | KeyboardEvent, f: SearchFolder) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    const at = e instanceof MouseEvent ? { x: e.clientX, y: e.clientY } : { x: r.left + 24, y: r.bottom };
    ctx = { el, ...at, items: searchFolderMenu(f), label: `Cartella di ricerca ${f.name}` };
  }

  function sfKey(e: KeyboardEvent, f: SearchFolder) {
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) openSearchFolderMenu(e, f);
    else if (e.key === 'Delete') {
      e.preventDefault();
      void store.deleteSearchFolder(f.id);
    }
  }

  function openSfNodeMenu(e: MouseEvent, a: Account) {
    e.preventDefault();
    ctx = {
      el: e.currentTarget as HTMLElement,
      x: e.clientX,
      y: e.clientY,
      label: 'Cartelle di ricerca',
      items: [{ label: 'Nuova cartella di ricerca…', icon: 'folder-search', keys: 'Ctrl+Maiusc+P', onclick: () => store.openSearchFolderDialog({ accountId: a.id }) }],
    };
  }

  /** Tasto Menu o Maiusc+F10 sulla cartella: lo stesso menu, sotto la cartella. */
  function folderKey(e: KeyboardEvent, f: Folder) {
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const el = e.currentTarget as HTMLElement;
      const r = el.getBoundingClientRect();
      ctx = { el, x: r.left + 24, y: r.bottom, items: folderMenu(f), label: `Cartella ${folderLabel(f)}` };
    } else if (e.key === 'F2' && !f.role) {
      e.preventDefault();
      store.startRenameFolder(f.id);
    }
  }

  // ---------- Trascinamento dei messaggi ----------

  let dropOn = $state<number | null>(null);

  function canDrop(e: DragEvent, f: Folder): boolean {
    return f.selectable && !!e.dataTransfer?.types.includes(DRAG_TYPE);
  }

  function ondragover(e: DragEvent, f: Folder) {
    if (!canDrop(e, f)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dropOn = f.id;
  }

  function ondrop(e: DragEvent, f: Folder) {
    dropOn = null;
    if (!canDrop(e, f)) return;
    e.preventDefault();
    try {
      const ids = JSON.parse(e.dataTransfer!.getData(DRAG_TYPE)) as number[];
      if (Array.isArray(ids) && ids.length) void store.dropOnFolder(ids, f.id);
    } catch {
      /* dati trascinati da altrove */
    }
  }

  function focusInput(el: HTMLInputElement) {
    el.focus();
    el.select();
  }
</script>

{#snippet nameInput(depth: number)}
  {@const e = store.folderEdit!}
  <div class="edit" style:padding-left="{28 + depth * 16}px">
    <Icon name="folder" />
    <input
      use:focusInput
      class="ph"
      value={e.value}
      aria-label={e.kind === 'new' ? 'Nome della nuova cartella' : 'Nuovo nome della cartella'}
      aria-invalid={!!e.error}
      aria-describedby={e.error ? 'fold-err' : undefined}
      placeholder="Nome della cartella"
      disabled={e.busy}
      oninput={(ev) => {
        e.value = ev.currentTarget.value;
        e.error = '';
      }}
      onkeydown={(ev) => {
        if (ev.key === 'Enter') {
          ev.preventDefault();
          void store.commitFolderEdit();
        } else if (ev.key === 'Escape') {
          ev.preventDefault();
          ev.stopPropagation();
          store.cancelFolderEdit();
        }
      }}
      onblur={() => {
        if (store.folderEdit && !store.folderEdit.error) void store.commitFolderEdit();
      }}
    />
  </div>
  {#if e.error}<p class="err" id="fold-err" role="alert" style:padding-left="{28 + depth * 16}px">{e.error}</p>{/if}
{/snippet}

{#snippet folderRow(f: Folder, prefix: string, depth: number)}
  {@const id = `${prefix}-${f.id}`}
  {@const on = store.folderSel === id}
  {@const c = count(f)}
  {@const edit = store.folderEdit}
  {#if edit?.kind === 'rename' && edit.folderId === f.id && prefix === 'acc'}
    {@render nameInput(depth)}
  {:else}
    <button
      class="btn fold"
      data-sel={on ? 'si' : 'no'}
      data-drop={dropOn === f.id ? 'si' : undefined}
      aria-current={on ? 'page' : undefined}
      style:padding-left="{28 + depth * 16}px"
      disabled={!f.selectable}
      onclick={() => store.pickFolder(f.id, id)}
      oncontextmenu={(e) => openFolderMenu(e, f)}
      onkeydown={(e) => folderKey(e, f)}
      ondragover={(e) => ondragover(e, f)}
      ondragenter={(e) => ondragover(e, f)}
      ondragleave={() => (dropOn = dropOn === f.id ? null : dropOn)}
      ondrop={(e) => ondrop(e, f)}
    >
      <Icon name={f.role ? ICON[f.role] : 'folder'} />
      <span class="name" style:font-weight={on ? 600 : 400}>{folderLabel(f)}</span>
      {#if c.n}<span class="cnt" style:color={c.muted ? 'var(--ink-3)' : 'var(--brand-ink)'}>{c.n}</span>{/if}
    </button>
  {/if}
  {#if edit?.kind === 'new' && edit.parentId === f.id && prefix === 'acc'}
    {@render nameInput(depth + 1)}
  {/if}
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
      <button class="btn grp-h toggle" onclick={() => store.toggleSection(a.id)} oncontextmenu={(e) => openAccountMenu(e, a)} aria-expanded={open} title={title(a)}>
        <Icon name={open ? 'chevron-down' : 'chevron-right'} size={14} stroke={1.8} />
        {#if a.kind === 'local'}<Icon name="database" size={15} style="color:var(--ink-2)" />{/if}
        <span class="aname">{title(a)}</span>
        {#if st}<span class="st" title={st.text}><Icon name={st.icon} size={15} style="color:var(--warn)" /><span class="sr">{st.text}</span></span>{/if}
      </button>
    </h2>
    {#if open}
      {#each store.foldersOf(a.id) as f (f.id)}{@render folderRow(f, 'acc', depthOf(f))}{/each}
      {#if store.folderEdit?.kind === 'new' && store.folderEdit.parentId === null && store.folderEdit.accountId === a.id}
        {@render nameInput(0)}
      {/if}
      {#if out}
        {@const on = store.folderSel === `uscita-${a.id}`}
        <button class="btn fold" data-sel={on ? 'si' : 'no'} aria-current={on ? 'page' : undefined} onclick={() => store.pickOutbox(`uscita-${a.id}`)}>
          <Icon name="mail-up" />
          <span class="name" style:font-weight={on ? 600 : 400}>Posta in uscita</span>
          <span class="cnt" style:color="var(--brand-ink)">{out}</span>
        </button>
      {/if}
      {#if a.kind !== 'local'}
        {@const sfs = store.searchFoldersOf(a.id)}
        {@const sfOpen = !store.collapsed[`ricerca:${a.id}`]}
        <button class="btn fold sf-node" aria-expanded={sfOpen} onclick={() => store.toggleSection(`ricerca:${a.id}`)} oncontextmenu={(e) => openSfNodeMenu(e, a)} title="Ricerche salvate che si aprono come cartelle">
          <Icon name={sfOpen ? 'chevron-down' : 'chevron-right'} size={14} stroke={1.8} />
          <Icon name="folder-search" />
          <span class="name">Cartelle di ricerca</span>
        </button>
        {#if sfOpen}
          {#each sfs as f (f.id)}
            {@const id = `sf-${f.id}`}
            {@const on = store.folderSel === id}
            {@const c = searchFolderCount(f)}
            <button
              class="btn fold"
              data-sel={on ? 'si' : 'no'}
              aria-current={on ? 'page' : undefined}
              style:padding-left="44px"
              title={describeCriteria(f.criteria, (cid) => store.category(cid)?.name ?? null)}
              onclick={() => store.pickSearchFolder(f.id, id)}
              oncontextmenu={(e) => openSearchFolderMenu(e, f)}
              onkeydown={(e) => sfKey(e, f)}
            >
              <Icon name={presetOf(f.kind).icon} />
              <span class="name" style:font-weight={on ? 600 : 400}>{f.name}</span>
              {#if c.n}<span class="cnt" style:color={c.muted ? 'var(--ink-3)' : 'var(--brand-ink)'}>{c.n}</span>{/if}
            </button>
          {/each}
          {#if !sfs.length}
            <button class="btn fold sf-new" style:padding-left="44px" onclick={() => store.openSearchFolderDialog({ accountId: a.id })}>
              <Icon name="plus" />
              <span class="name">Nuova cartella di ricerca</span>
            </button>
          {/if}
        {/if}
      {/if}
    {/if}
  {/each}

  {#if !store.accounts.length && store.ready}
    <p class="none">Nessun account. Aggiungine uno per vedere qui le tue cartelle.</p>
  {/if}

  <button class="btn add" onclick={() => store.openWizard(false)}><Icon name="user-plus" />Aggiungi account</button>
</nav>

{#if ctx}
  <Menu anchor={ctx.el} at={{ x: ctx.x, y: ctx.y }} items={ctx.items} label={ctx.label} onclose={() => (ctx = null)} />
{/if}

<style>
  .pane-folders { width: 240px; flex: none; box-sizing: border-box; overflow: auto; padding: 0 8px 20px 2px; background: var(--ground); }
  .top { height: 34px; display: flex; align-items: center; }
  .acct { margin: 12px 0 2px; padding: 0; font: inherit; }
  .toggle { width: 100%; margin: 0; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .aname { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; }
  .st { display: inline-flex; }
  .none { margin: 16px 10px 0; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .add { width: 100%; height: 32px; margin-top: 10px; display: flex; align-items: center; gap: 10px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; text-align: left; }
  .fold[data-drop="si"] { background: var(--brand-sel); box-shadow: inset 0 0 0 1px var(--brand); }
  .edit { height: 32px; display: flex; align-items: center; gap: 10px; padding-right: 4px; color: var(--ink-2); }
  .edit input { flex: 1; min-width: 0; height: 28px; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); color: var(--ink); font: inherit; font-size: 14px; }
  .edit input:focus { outline: none; }
  .edit input[aria-invalid="true"] { box-shadow: inset 0 0 0 1px var(--danger), inset 0 -2px 0 var(--danger); }
  .sf-node { padding-left: 8px; gap: 6px; color: var(--ink-2); }
  .sf-node :global(svg.i + svg.i) { margin-right: 4px; }
  .sf-new { color: var(--brand-ink); }
  .sf-new :global(svg.i) { color: var(--brand-ink); }
  .err { margin: 2px 4px 6px; font-size: 12.5px; line-height: 1.4; color: var(--danger); }
</style>
