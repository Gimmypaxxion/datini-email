<script lang="ts">
  // Impostazioni › Archiviazione automatica, come in Outlook: ogni tanto i
  // messaggi vecchi lasciano le cartelle di posta (in Archivio, in un archivio
  // sul computer o eliminati) e Posta eliminata si svuota da sola.
  import { onMount } from 'svelte';
  import Icon from '../lib/Icon.svelte';
  import { store, folderLabel, monthsText, sortFolders } from '../lib/store.svelte';
  import type { AutoArchiveAction, Folder } from '../lib/types';

  onMount(() => {
    void store.reloadAutoArchive();
  });

  const a = $derived(store.autoArchive);
  const mail = $derived(store.accounts.filter((x) => x.kind !== 'local'));

  const EVERY: [number, string][] = [
    [1, 'ogni giorno'],
    [7, 'ogni settimana'],
    [14, 'ogni 2 settimane'],
    [30, 'ogni mese'],
  ];
  const MONTHS = [1, 3, 6, 12, 24, 36];
  const TRASH: [number, string][] = [
    [7, '7 giorni'],
    [14, '14 giorni'],
    [30, '30 giorni'],
    [60, '60 giorni'],
    [90, '90 giorni'],
  ];

  const ACTIONS: { id: AutoArchiveAction; title: string; text: string }[] = [
    { id: 'archivio', title: 'Nella cartella Archivio', text: 'Restano sul server e su tutti i tuoi dispositivi, e si trovano con la ricerca.' },
    { id: 'computer', title: 'In un archivio sul computer', text: 'Liberano spazio sul server, utile se la casella si riempie. Li trovi solo su questo computer, nel file di dati “Archivio sul computer”.' },
    { id: 'elimina', title: 'Eliminali definitivamente', text: 'Spariscono per sempre, anche dal server. Non si possono recuperare.' },
  ];

  /** Le cartelle che l'archiviazione può toccare (non Archivio, Bozze, Posta eliminata e Indesiderata). */
  function archivable(accountId: string): Folder[] {
    return sortFolders(store.folders.filter((f) => f.accountId === accountId && f.selectable && f.role !== 'archive' && f.role !== 'drafts' && f.role !== 'deleted' && f.role !== 'junk'));
  }

  function when(secs: number): string {
    return new Date(secs * 1000).toLocaleDateString('it-IT', { day: 'numeric', month: 'long', year: 'numeric' });
  }

  const next = $derived(a?.enabled && a.lastRun ? Math.max(a.lastRun + a.everyDays * 86_400, Math.floor(Date.now() / 1000)) : null);
</script>

<h3>Archiviazione automatica</h3>
<p class="intro">Ogni tanto Better-Look toglie dalle cartelle di posta i messaggi vecchi: le cartelle restano leggere e la casella non si riempie. Bozze e messaggi contrassegnati restano sempre dove sono.</p>

{#if !mail.length}
  <p class="hint" style="margin-top:22px">Aggiungi un account di posta per usare l’archiviazione automatica.</p>
{:else if a}
  <div class="aa-main">
    <label class="aa-check">
      <input type="checkbox" checked={a.enabled} onchange={(e) => store.setAutoArchive({ enabled: e.currentTarget.checked })} />
      <span>Esegui l’archiviazione automatica</span>
      <select class="aa-sel" aria-label="Ogni quanto" value={String(a.everyDays)} onchange={(e) => store.setAutoArchive({ everyDays: Number(e.currentTarget.value) })} disabled={!a.enabled}>
        {#each EVERY as [d, label] (d)}<option value={String(d)}>{label}</option>{/each}
      </select>
    </label>
    <p class="aa-status">
      {#if a.lastRun}Ultima archiviazione: {when(a.lastRun)}.{:else}Non è ancora mai stata eseguita.{/if}
      {#if next}Prossima: {next <= Date.now() / 1000 + 3_600 ? 'entro un’ora' : when(next)}.{/if}
    </p>
  </div>

  <section aria-labelledby="aa-what">
    <h4 id="aa-what">Messaggi vecchi</h4>
    <div class="aa-line">
      <label for="aa-age">Archivia i messaggi ricevuti più di</label>
      <select id="aa-age" class="aa-sel" value={String(a.olderMonths)} onchange={(e) => store.setAutoArchive({ olderMonths: Number(e.currentTarget.value) })}>
        {#each MONTHS as m (m)}<option value={String(m)}>{monthsText(m)}</option>{/each}
      </select>
      <span>fa</span>
    </div>
    <div class="aa-opts" role="radiogroup" aria-label="Dove metterli">
      {#each ACTIONS as o (o.id)}
        {@const on = a.action === o.id}
        <label class="opt" data-on={on ? 'si' : 'no'}>
          <input type="radio" name="aa-action" checked={on} onchange={() => store.setAutoArchive({ action: o.id })} />
          <span class="dot" aria-hidden="true"></span>
          <span class="aa-otext"><span class="aa-ot" class:aa-danger={o.id === 'elimina'}>{o.title}</span><span class="aa-od">{o.text}</span></span>
        </label>
      {/each}
    </div>
  </section>

  <section aria-labelledby="aa-trash">
    <h4 id="aa-trash">Posta eliminata e Posta indesiderata</h4>
    <label class="aa-check">
      <input type="checkbox" checked={a.emptyTrash} onchange={(e) => store.setAutoArchive({ emptyTrash: e.currentTarget.checked })} />
      <span>Svuotale dai messaggi che ci stanno da più di</span>
      <select class="aa-sel" aria-label="Giorni" value={String(a.trashDays)} onchange={(e) => store.setAutoArchive({ trashDays: Number(e.currentTarget.value) })} disabled={!a.emptyTrash}>
        {#each TRASH as [d, label] (d)}<option value={String(d)}>{label}</option>{/each}
      </select>
    </label>
    <p class="hint aa-sub">Conta da quando il messaggio è finito lì: quello eliminato per sbaglio oggi resta recuperabile.</p>
  </section>

  <section aria-labelledby="aa-folders">
    <h4 id="aa-folders">Cartelle</h4>
    <p class="hint">Togli il segno alle cartelle da lasciare come sono.</p>
    {#each mail as acc (acc.id)}
      {@const list = archivable(acc.id)}
      {#if list.length}
        <fieldset class="aa-acct">
          <legend>{acc.email}</legend>
          <div class="aa-folders">
            {#each list as f (f.id)}
              <label class="aa-f">
                <input type="checkbox" checked={!a.excluded.includes(f.id)} onchange={(e) => store.toggleArchiveFolder(f.id, e.currentTarget.checked)} />
                {folderLabel(f)}
              </label>
            {/each}
          </div>
        </fieldset>
      {/if}
    {/each}
  </section>

  <div class="aa-run">
    <button class="sec-btn" onclick={() => store.runAutoArchiveNow()} disabled={store.archiving}>
      <Icon name={store.archiving ? 'loader-2' : 'archive'} size={16} class={store.archiving ? 'spin' : ''} />{store.archiving ? 'Archiviazione in corso…' : 'Archivia ora'}
    </button>
    <span class="hint">Con le impostazioni qui sopra, anche se l’archiviazione automatica è spenta.</span>
  </div>
{/if}

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 62ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .hint { margin: 0; max-width: 60ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  section { margin-top: 24px; padding-top: 20px; box-shadow: inset 0 1px 0 var(--rule); max-width: 680px; }
  .aa-main { margin-top: 22px; }
  .aa-check { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; font-size: 14px; }
  .aa-check input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); }
  .aa-status { margin: 8px 0 0 28px; font-size: 13px; line-height: 1.5; color: var(--ink-2); }
  .aa-line { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; font-size: 14px; }
  .aa-sel { height: 32px; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .aa-sel:disabled { opacity: .55; }
  .aa-opts { margin-top: 14px; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
  .aa-otext { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .aa-ot { font-size: 14px; font-weight: 600; }
  .aa-ot.aa-danger { color: var(--danger); }
  .aa-od { font-size: 12.5px; line-height: 1.45; color: var(--ink-2); }
  .aa-sub { margin: 6px 0 0 28px; }
  .aa-acct { margin: 14px 0 0; padding: 0; border: 0; min-width: 0; }
  .aa-acct legend { padding: 0; margin-bottom: 6px; font-size: 13px; font-weight: 600; color: var(--ink-2); }
  .aa-folders { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 4px 16px; }
  .aa-f { min-width: 0; display: flex; align-items: center; gap: 8px; height: 28px; font-size: 14px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .aa-f input { width: 17px; height: 17px; margin: 0; flex: none; accent-color: var(--brand); }
  .aa-run { margin-top: 26px; display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
  .aa-run .sec-btn { display: inline-flex; align-items: center; gap: 8px; }
  @media (max-width: 760px) { .aa-opts { grid-template-columns: 1fr; } }
</style>
