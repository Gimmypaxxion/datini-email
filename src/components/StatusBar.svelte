<script lang="ts">
  // Barra di stato in basso, come in Outlook classico: quanti elementi ha la
  // cartella e se la posta è aggiornata, in sincronizzazione o offline.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';

  const f = $derived(store.currentFolder);
  const problem = $derived(store.accounts.find((a) => a.status.state === 'errore'));
  const waiting = $derived(store.outbox.filter((o) => o.state === 'offline' || o.state === 'errore').length);
</script>

<footer class="status" aria-label="Stato">
  <span class="left">
    {#if store.q.trim()}
      Risultati: {store.list.length}
    {:else if store.view === 'uscita'}
      In uscita: {store.outbox.length}
    {:else if f}
      Elementi: {f.total}{#if f.unread}<span class="gap"></span>Da leggere: {f.unread}{/if}
    {/if}
  </span>
  <span class="right" role="status">
    {#if store.importing}
      {@const i = store.importing}
      <Icon name="loader-2" size={15} class="spin" style="color:var(--brand-ink)" />
      Importazione di {i.name}: {i.total ? `${i.done.toLocaleString('it-IT')} di ${i.total.toLocaleString('it-IT')} elementi` : `${i.done.toLocaleString('it-IT')} elementi`}
    {:else if store.offline}
      <Icon name="cloud-off" size={15} style="color:var(--warn)" />
      {waiting ? `Offline. ${waiting === 1 ? 'Un messaggio aspetta' : `${waiting} messaggi aspettano`} la connessione.` : 'Offline'}
    {:else if problem}
      <button class="lnk" onclick={() => { store.acctSel = problem.id; store.openSettings('account'); }}>
        <Icon name="alert-triangle" size={15} style="color:var(--warn)" />Problema con {problem.email}
      </button>
    {:else if store.syncing}
      <Icon name="loader-2" size={15} class="spin" style="color:var(--brand-ink)" />Aggiornamento in corso
    {:else if store.accounts.some((a) => a.kind !== 'local')}
      Tutte le cartelle sono aggiornate.<span class="gap"></span>Connesso
    {/if}
  </span>
</footer>

<style>
  .status { height: 26px; flex: none; display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 0 14px 0 66px; font-size: 12px; color: var(--ink-2); font-variant-numeric: tabular-nums; }
  .left, .right { display: inline-flex; align-items: center; gap: 6px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .gap { display: inline-block; width: 18px; }
  .lnk { display: inline-flex; align-items: center; gap: 6px; height: 22px; padding: 0 6px; border: 0; border-radius: 4px; background: transparent; color: var(--ink-2); font-size: 12px; }
  .lnk:hover { background: var(--hover); }
  @media (max-width: 760px) { .status { padding-left: 14px; } }
</style>
