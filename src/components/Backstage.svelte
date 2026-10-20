<script lang="ts">
  // Menu File a tutta pagina, come in Outlook classico.
  import Icon from '../lib/Icon.svelte';
  import About from './About.svelte';
  import { store } from '../lib/store.svelte';
  import { ME } from '../lib/mock';

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
        <button class="btn acct">
          <span class="avatar" aria-hidden="true" style="width:36px;height:36px;font-size:13px;background:var(--av1-bg);color:var(--av1-fg)">{ME.initials}</span>
          <span class="col"><span class="a">{ME.email}</span><span class="b">IMAP, connessione diretta</span></span>
          <Icon name="chevron-down" size={16} style="margin-left:14px;color:var(--ink-3)" />
        </button>
        <div class="rows">
          <div class="bs-row">
            <button class="btn bs-tile" onclick={() => store.openSettings('account')}><Icon name="user-cog" size={30} stroke={1.5} />Impostazioni account</button>
            <div><h2>Impostazioni account</h2><p>Server, posta offline e sincronizzazione di questo account. Da qui puoi anche aggiungerne altri.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="message-reply" size={30} stroke={1.5} />Risposte automatiche</button>
            <div><h2>Risposte automatiche</h2><p>Avvisa chi ti scrive che sei fuori ufficio e quando rientri.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="trash-x" size={30} stroke={1.5} />Pulizia</button>
            <div><h2>Pulizia cassetta postale</h2><p>Libera spazio sul computer archiviando i messaggi vecchi o svuotando gli elementi eliminati.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="list-check" size={30} stroke={1.5} />Regole e avvisi</button>
            <div><h2>Regole e avvisi</h2><p>Sposta in automatico i messaggi nelle cartelle giuste e ricevi un avviso per quelli importanti.</p></div>
          </div>
        </div>
      {:else if store.bsPage === 'apri'}
        <h1>Apri ed esporta</h1>
        <div class="rows">
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="file-import" size={30} stroke={1.5} />Importa PST</button>
            <div><h2>Importa da file PST</h2><p>Porta in Datini posta, contatti e calendario da un archivio di Outlook.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="folder-open" size={30} stroke={1.5} />Apri PST</button>
            <div><h2>Apri file di dati di Outlook</h2><p>Consulta un archivio .pst senza importarlo.</p></div>
          </div>
          <div class="bs-row">
            <button class="btn bs-tile"><Icon name="file-export" size={30} stroke={1.5} />Esporta</button>
            <div><h2>Esporta in MBOX o EML</h2><p>Salva una copia della posta in un formato aperto, leggibile da qualsiasi programma di posta.</p></div>
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
</style>
