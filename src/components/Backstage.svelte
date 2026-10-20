<script lang="ts">
  // Menu File a tutta pagina, come in Outlook classico.
  import Icon from '../lib/Icon.svelte';
  import About from './About.svelte';
  import { store } from '../lib/store.svelte';
  import { initials } from '../lib/format';

  const acct = $derived(store.currentAccount);
  const kindText = $derived(
    !acct ? '' : acct.kind === 'local' ? 'File di dati su questo computer' : `${acct.kind === 'imap' ? 'IMAP' : 'POP'}, connessione diretta`,
  );

  const items = [
    ['info', 'Informazioni'],
    ['apri', 'Apri ed esporta'],
  ] as const;
</script>

{#if store.backstage}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="bs fade-in"
    role="dialog"
    aria-label="File"
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === 'Escape' && !store.settings) store.backstage = false;
    }}
  >
    <nav class="bs-nav" aria-label="Menu File">
      <button class="back" onclick={() => (store.backstage = false)} aria-label="Torna alla posta" title="Torna alla posta (Esc)"><Icon name="arrow-left" size={20} /></button>
      {#each items as [id, label] (id)}
        <button class="bs-it" data-on={store.bsPage === id ? 'si' : 'no'} aria-current={store.bsPage === id ? 'page' : undefined} onclick={() => (store.bsPage = id)}>{label}</button>
      {/each}
      <span style="flex:1"></span>
      <button class="bs-it" data-on={store.bsPage === 'about' ? 'si' : 'no'} aria-current={store.bsPage === 'about' ? 'page' : undefined} onclick={() => (store.bsPage = 'about')}>Informazioni su Datini</button>
      <button class="bs-it" onclick={() => store.openSettings('aspetto')}>Opzioni</button>
    </nav>
    <div class="scroll main">
      {#if store.bsPage === 'info'}
        <h1>Informazioni account</h1>
        {#if acct}
          <button class="btn acct" onclick={() => { store.acctSel = acct.id; store.openSettings('account'); }}>
            <span class="avatar" aria-hidden="true" style="width:36px;height:36px;font-size:13px;background:var(--av1-bg);color:var(--av1-fg)">{initials(acct.name || acct.email)}</span>
            <span class="col"><span class="a">{acct.kind === 'local' ? acct.name : acct.email}</span><span class="b">{kindText}</span></span>
            <Icon name="chevron-right" size={16} style="margin-left:14px;color:var(--ink-3)" />
          </button>
        {/if}
        <button class="btn addacc" onclick={() => store.openWizard(false)}><Icon name="plus" size={16} />Aggiungi account</button>
        <div class="rows">
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.openSettings('account')}><Icon name="user-cog" size={30} stroke={1.5} />Impostazioni account</button>
            <div><h2>Impostazioni account</h2><p>Server, posta offline e sincronizzazione di questo account. Da qui puoi anche aggiungerne altri o cambiare la password.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.soon('Risposte automatiche')}><Icon name="message-reply" size={30} stroke={1.5} />Risposte automatiche</button>
            <div><h2>Risposte automatiche</h2><p>Avvisa chi ti scrive che sei fuori ufficio e quando rientri. Arriverà in una prossima versione.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.soon('Pulizia cassetta postale')}><Icon name="trash-x" size={30} stroke={1.5} />Pulizia</button>
            <div><h2>Pulizia cassetta postale</h2><p>Libera spazio archiviando i messaggi vecchi o svuotando la posta eliminata. Arriverà in una prossima versione.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.soon('Regole e avvisi')}><Icon name="list-check" size={30} stroke={1.5} />Regole e avvisi</button>
            <div><h2>Regole e avvisi</h2><p>Sposta in automatico i messaggi nelle cartelle giuste. Arriverà in una prossima versione.</p></div>
          </div>
        </div>
      {:else if store.bsPage === 'apri'}
        <h1>Apri ed esporta</h1>
        {#if !store.backend.real}<p class="demo">Nell’anteprima nel browser l’accesso ai file non è disponibile: queste funzioni si usano nell’app installata.</p>{/if}
        <div class="rows">
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.importArchive('pst')}><Icon name="folder-open" size={30} stroke={1.5} />Apri PST</button>
            <div><h2>Apri file di dati di Outlook</h2><p>Apri un archivio .pst: compare nel riquadro cartelle come file di dati. Puoi leggerlo e spostare i messaggi nelle cartelle del tuo account. Il file originale non viene modificato.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.importArchive('mbox')}><Icon name="file-import" size={30} stroke={1.5} />Importa MBOX</button>
            <div><h2>Importa da file MBOX</h2><p>Porta in Datini la posta esportata da Thunderbird, Apple Mail o da un altro programma in formato MBOX.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.exportFolder()} disabled={!store.currentFolder}><Icon name="file-export" size={30} stroke={1.5} />Esporta cartella</button>
            <div><h2>Esporta in MBOX</h2><p>Salva una copia della cartella aperta in formato MBOX, leggibile da qualsiasi programma di posta. Per un solo messaggio usa Salva con nome dal messaggio aperto.</p></div>
          </div>
        </div>
      {:else}
        <h1>Informazioni su Datini</h1>
        <div style="margin-top:36px"><About /></div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .back { width: 40px; height: 40px; margin: 0 0 16px 6px; border: 0; border-radius: 50%; background: transparent; display: grid; place-items: center; color: var(--on-brand); box-shadow: inset 0 0 0 1.5px rgba(255, 255, 255, .8); }
  .main { flex: 1; min-width: 0; overflow: auto; padding: 40px 56px 56px; }
  h1 { margin: 0; font-size: 28px; font-weight: 600; }
  .acct { margin-top: 16px; height: 54px; display: flex; align-items: center; gap: 12px; padding: 0 14px 0 9px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); text-align: left; }
  .col { display: flex; flex-direction: column; }
  .col .a { font-size: 14px; font-weight: 600; }
  .col .b { font-size: 12.5px; color: var(--ink-3); }
  .rows { margin-top: 18px; max-width: 780px; }
  .addacc { margin-top: 8px; height: 32px; display: inline-flex; align-items: center; gap: 8px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; }
  .demo { margin: 10px 0 0; max-width: 62ch; font-size: 14px; color: var(--ink-2); }
</style>
