<script lang="ts">
  // Impostazioni › Pulizia cassetta postale, come lo strumento di Outlook:
  // quanto occupa ogni cartella, messaggi grandi, messaggi vecchi da
  // archiviare, Posta eliminata e Indesiderata da svuotare.
  import Icon from '../lib/Icon.svelte';
  import { store, folderLabel } from '../lib/store.svelte';
  import { fileSize, listTime, personLabel } from '../lib/format';
  import { errorMessage, type FolderUsage, type MessageSummary } from '../lib/types';

  const accounts = $derived(store.accounts);
  const acct = $derived(accounts.find((a) => a.id === store.acctSel) ?? accounts[0] ?? null);

  let usage = $state<FolderUsage[]>([]);
  let loading = $state(false);
  let err = $state('');

  async function load() {
    if (!acct) return;
    loading = true;
    err = '';
    try {
      usage = await store.backend.mailboxUsage(acct.id);
    } catch (e) {
      err = errorMessage(e);
    } finally {
      loading = false;
    }
  }

  // Si ricarica quando cambia account o quando cambiano i messaggi.
  let shownFor = '';
  $effect(() => {
    const id = acct?.id ?? '';
    void store.folders;
    if (id !== shownFor) {
      shownFor = id;
      big = null;
      older = null;
    }
    void load();
  });

  const total = $derived(usage.reduce((t, u) => ({ count: t.count + u.count, size: t.size + u.size }), { count: 0, size: 0 }));
  const top = $derived(usage.filter((u) => u.count > 0).slice(0, 8));
  const folder = (id: number) => store.folders.find((f) => f.id === id);
  const roleUsage = (role: 'deleted' | 'junk') => {
    const f = store.folders.find((x) => x.accountId === acct?.id && x.role === role);
    return f ? { f, u: usage.find((u) => u.folderId === f.id) } : null;
  };
  const trash = $derived(roleUsage('deleted'));
  const junk = $derived(roleUsage('junk'));

  // ---------- Messaggi grandi ----------
  const SIZES = [
    [1, '1 MB'],
    [5, '5 MB'],
    [10, '10 MB'],
    [20, '20 MB'],
  ] as const;
  let minMb = $state(5);
  let big = $state<MessageSummary[] | null>(null);
  let picked = $state<Record<number, boolean>>({});
  const pickedIds = $derived(Object.entries(picked).filter(([, v]) => v).map(([k]) => Number(k)));

  async function findBig() {
    if (!acct) return;
    try {
      big = await store.backend.largeMessages(acct.id, minMb * 1024 * 1024);
      picked = {};
    } catch (e) {
      err = errorMessage(e);
    }
  }

  async function bigAction(role: 'deleted' | 'archive') {
    const ids = pickedIds;
    if (!ids.length) return;
    try {
      await store.backend.moveToRole(ids, role);
      big = (big ?? []).filter((m) => !ids.includes(m.id));
      picked = {};
      await store.refresh();
    } catch (e) {
      err = errorMessage(e);
    }
  }

  // ---------- Messaggi vecchi ----------
  const AGES = [
    [3, '3 mesi'],
    [6, '6 mesi'],
    [12, '1 anno'],
    [24, '2 anni'],
  ] as const;
  let months = $state(12);
  let older = $state<{ folderId: number; count: number } | null>(null);
  const inbox = $derived(store.folders.find((f) => f.accountId === acct?.id && f.role === 'inbox'));
  const before = () => {
    const d = new Date();
    d.setMonth(d.getMonth() - months);
    return d.getTime();
  };

  async function countOlder() {
    if (!inbox) return;
    try {
      older = { folderId: inbox.id, count: await store.backend.olderMessages(inbox.id, before(), false) };
    } catch (e) {
      err = errorMessage(e);
    }
  }

  async function archiveOlder() {
    if (!inbox || !older?.count) return;
    const ok = await store.ask({
      title: 'Archiviare i messaggi vecchi?',
      text: `${older.count.toLocaleString('it-IT')} messaggi di Posta in arrivo più vecchi di ${AGES.find(([m]) => m === months)?.[1]} vanno in Archivio. Restano cercabili e puoi riportarli indietro quando vuoi.`,
      ok: 'Archivia',
    });
    if (!ok) return;
    try {
      const n = await store.backend.olderMessages(inbox.id, before(), true);
      older = { folderId: inbox.id, count: 0 };
      store.hint(n === 1 ? '1 messaggio archiviato.' : `${n.toLocaleString('it-IT')} messaggi archiviati.`);
      await store.refresh();
    } catch (e) {
      err = errorMessage(e);
    }
  }
</script>

<h3>Pulizia cassetta postale</h3>
<p class="intro">Libera spazio sul computer e sul server: guarda quali cartelle occupano di più, togli i messaggi grandi e archivia quelli vecchi.</p>

{#if !acct}
  <p class="hint" style="margin-top:22px">Aggiungi un account di posta per usare la pulizia.</p>
{:else}
  {#if accounts.length > 1}
    <label class="tf" style="max-width:380px;margin-top:22px">
      <span>Account</span>
      <select value={acct.id} onchange={(e) => (store.acctSel = e.currentTarget.value)}>
        {#each accounts as a (a.id)}<option value={a.id}>{a.kind === 'local' ? a.name : a.email}</option>{/each}
      </select>
    </label>
  {/if}
  {#if err}<p class="bad" role="alert">{err}</p>{/if}

  <section aria-labelledby="cl-size">
    <h4 id="cl-size">Spazio occupato</h4>
    {#if loading && !usage.length}
      <p class="hint">Calcolo in corso…</p>
    {:else}
      <p class="sum"><strong>{fileSize(total.size)}</strong> in {total.count.toLocaleString('it-IT')} messaggi{acct.kind === 'imap' ? ', contando anche quelli rimasti solo sul server' : ''}.</p>
      {#if top.length}
        <table class="usage">
          <thead><tr><th scope="col">Cartella</th><th scope="col" class="n">Messaggi</th><th scope="col" class="n">Spazio</th></tr></thead>
          <tbody>
            {#each top as u (u.folderId)}
              <tr><td>{folderLabel(folder(u.folderId))}</td><td class="n">{u.count.toLocaleString('it-IT')}</td><td class="n">{fileSize(u.size)}</td></tr>
            {/each}
          </tbody>
        </table>
      {/if}
    {/if}
  </section>

  {#if trash || junk}
    <section aria-labelledby="cl-empty">
      <h4 id="cl-empty">Svuota</h4>
      <div class="empties">
        {#each [trash, junk].filter((x) => x !== null) as x (x.f.id)}
          <div class="empty-row">
            <span class="er-t"><strong>{folderLabel(x.f)}</strong> <span class="muted">{x.u?.count ? `${x.u.count.toLocaleString('it-IT')} messaggi, ${fileSize(x.u.size)}` : 'vuota'}</span></span>
            <button class="sec-btn" disabled={!x.u?.count} onclick={() => store.emptyFolder(x.f.id)}><Icon name="trash-x" size={16} />Svuota</button>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <section aria-labelledby="cl-big">
    <h4 id="cl-big">Messaggi grandi</h4>
    <div class="line">
      <label class="flabel" for="cl-min">Più grandi di</label>
      <select id="cl-min" class="sel" value={String(minMb)} onchange={(e) => (minMb = Number(e.currentTarget.value))}>
        {#each SIZES as [mb, label] (mb)}<option value={String(mb)}>{label}</option>{/each}
      </select>
      <button class="sec-btn" onclick={findBig}><Icon name="search" size={16} />Trova</button>
    </div>
    {#if big}
      {#if !big.length}
        <p class="hint" style="margin-top:12px">Nessun messaggio supera questa dimensione.</p>
      {:else}
        <ul class="big" aria-label="Messaggi grandi">
          {#each big as m (m.id)}
            <li>
              <label class="big-row">
                <input type="checkbox" checked={!!picked[m.id]} onchange={(e) => (picked[m.id] = e.currentTarget.checked)} />
                <span class="bt"><span class="b1">{m.subject || '(nessun oggetto)'}</span><span class="b2">{personLabel(m.from)}, {listTime(m.date)}, {folderLabel(folder(m.folderId)) || m.folderName}</span></span>
                <span class="bsize">{fileSize(m.size)}</span>
              </label>
            </li>
          {/each}
        </ul>
        <div class="line">
          <button class="sec-btn" disabled={!pickedIds.length} onclick={() => bigAction('archive')}><Icon name="archive" size={16} />Archivia</button>
          <button class="sec-btn danger-btn" disabled={!pickedIds.length} onclick={() => bigAction('deleted')}><Icon name="trash" size={16} />Elimina</button>
          <span class="muted">{pickedIds.length ? `${pickedIds.length} scelti` : 'Scegli i messaggi da togliere'}</span>
        </div>
      {/if}
    {/if}
  </section>

  {#if inbox}
    <section aria-labelledby="cl-old">
      <h4 id="cl-old">Archivia i messaggi vecchi</h4>
      <div class="line">
        <label class="flabel" for="cl-age">Posta in arrivo più vecchia di</label>
        <select id="cl-age" class="sel" value={String(months)} onchange={(e) => { months = Number(e.currentTarget.value); older = null; }}>
          {#each AGES as [m, label] (m)}<option value={String(m)}>{label}</option>{/each}
        </select>
        <button class="sec-btn" onclick={countOlder}>Conta</button>
        {#if older}
          <span class="muted">{older.count === 0 ? 'Nessun messaggio da archiviare.' : `${older.count.toLocaleString('it-IT')} messaggi`}</span>
          {#if older.count > 0}<button class="primary-btn" onclick={archiveOlder}><Icon name="archive" size={16} />Archivia</button>{/if}
        {/if}
      </div>
    </section>
  {/if}
{/if}

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 62ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .hint { margin: 0; max-width: 58ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .bad { margin: 14px 0 0; font-size: 13.5px; color: var(--danger); }
  section { margin-top: 24px; padding-top: 20px; box-shadow: inset 0 1px 0 var(--rule); max-width: 680px; }
  .sum { margin: 0 0 12px; font-size: 14px; }
  .usage { width: 100%; max-width: 480px; border-collapse: collapse; font-size: 13.5px; }
  .usage th { padding: 0 0 6px; font-size: 12.5px; font-weight: 600; color: var(--ink-2); text-align: left; }
  .usage td { padding: 6px 0; box-shadow: inset 0 -1px 0 var(--rule); }
  .usage tbody tr:last-child td { box-shadow: none; }
  .n { text-align: right !important; font-variant-numeric: tabular-nums; padding-left: 16px !important; white-space: nowrap; }
  .empties { display: flex; flex-direction: column; gap: 8px; }
  .empty-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .er-t { font-size: 14px; }
  .muted { color: var(--ink-2); font-size: 13.5px; font-variant-numeric: tabular-nums; }
  .line { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-top: 4px; }
  .flabel { font-size: 14px; }
  .sel { height: 32px; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .big { margin: 12px 0 10px; padding: 0; list-style: none; max-height: 300px; overflow: auto; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--rule-2); }
  .big li + li { box-shadow: inset 0 1px 0 var(--rule); }
  .big-row { display: flex; align-items: center; gap: 12px; padding: 8px 12px; cursor: pointer; }
  .big-row:hover { background: var(--hover); }
  .big-row input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); flex: none; }
  .bt { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .b1 { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .b2 { font-size: 12.5px; color: var(--ink-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bsize { font-size: 13px; font-variant-numeric: tabular-nums; color: var(--ink-2); white-space: nowrap; }
</style>
