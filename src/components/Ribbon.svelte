<script lang="ts">
  // Schede e barra dei comandi della finestra principale.
  // Gruppi e ordine seguono Outlook; in modalità compatta restano solo
  // i comandi più usati, in una riga.
  import RB from './RB.svelte';
  import RibbonCompose from './RibbonCompose.svelte';
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import { store, type RibbonTab } from '../lib/store.svelte';
  import { categoryItems, moveItems } from '../lib/menus';
  import { REPO_URL } from '../lib/platform';

  const TAB_NAMES: Record<RibbonTab, string> = {
    home: 'Home',
    visualizza: 'Visualizza',
    aiuto: 'Aiuto',
    messaggio: 'Messaggio',
    inserisci: 'Inserisci',
    formato: 'Formato testo',
  };

  const noSel = $derived(!store.selected);
  const ext = $derived(store.prefs.ribbon === 'estesa');
  const rib = $derived(store.activeRibbon);
  const sel = $derived(store.sel);

  let moveAnchor = $state<HTMLElement | null>(null);
  let catAnchor = $state<HTMLElement | null>(null);
  let rulesAnchor = $state<HTMLElement | null>(null);
  const rulesItems = $derived([
    { label: 'Crea regola…', icon: 'list-check' as const, disabled: noSel, onclick: () => store.newRuleFrom() },
    { label: 'Gestisci regole e avvisi', icon: 'settings' as const, onclick: () => { store.ruleDraft = null; store.openSettings('regole'); } },
  ]);

  function openMove(e: MouseEvent) {
    moveAnchor = moveAnchor ? null : (e.currentTarget as HTMLElement);
  }

  function openCats(e: MouseEvent) {
    catAnchor = catAnchor ? null : (e.currentTarget as HTMLElement);
  }
</script>

<div class="ribbon-wrap">
  <nav class="tabs-row" aria-label="Schede dei comandi">
    {#if store.prefs.entry === 'file'}
      <button class="tab-btn btn file-tab" onclick={() => store.openBackstage()} aria-haspopup="dialog" title="File">File</button>
    {/if}
    {#each store.ribbonTabs as t (t)}
      <button class="tab-btn btn" aria-pressed={t === rib} onclick={() => (store.ribbonTab = t)}>{TAB_NAMES[t]}</button>
    {/each}
  </nav>

  <div class="rib" data-mode={store.prefs.ribbon} role="toolbar" aria-label="Comandi">
    <div class="rib-groups">
    {#if rib === 'home'}
      <div class="rg"><div class="rgb">
        <RB big alwaysLabel cls="pri split" icon="mail-plus" label="Nuovo messaggio" title="Nuovo messaggio (Ctrl+N)" onclick={() => store.openCompose('new')} />
        <button class="rb pri pri-x only-c" aria-label="Altri elementi nuovi" title="Nuovo evento, contatto o attività"><Icon name="chevron-down" size={16} class="chev" /></button>
        <RB big menu cls="only-e" icon="layout-grid-add" tone="c-brand" label="Nuovi elementi" title="Nuovo evento, contatto o attività" onclick={() => store.soon('Calendario, contatti e attività')} />
      </div><span class="rgl">Nuovo</span></div>

      <div class="rg only-e"><div class="rgb">
        <RB big icon="refresh" tone="c-brand" label="Invia e ricevi" title="Invia/Ricevi tutte le cartelle (F9)" onclick={() => store.syncNow()} />
      </div><span class="rgl">Aggiorna</span></div>

      <div class="rg"><div class="rgb">
        <div class="rs only-e">
          <RB showLabel={false} icon="mail-off" label="Ignora conversazione" disabled={noSel} onclick={() => store.soon('Ignora conversazione')} />
          <RB showLabel={false} menu icon="eraser" tone="c-amber" label="Pulisci conversazione" disabled={noSel} onclick={() => store.soon('Pulisci conversazione')} />
          <RB showLabel={false} menu icon="ban" tone="c-red" label="Posta indesiderata" title="Segnala come posta indesiderata" disabled={noSel} onclick={() => store.moveMsg(sel, 'junk')} />
        </div>
        <RB big icon="trash" label="Elimina" title="Elimina (Canc)" disabled={noSel} onclick={() => store.deleteMsg(sel)} />
        <RB big icon="archive" tone="c-green" label="Archivia" title="Archivia (Backspace)" disabled={noSel} onclick={() => store.moveMsg(sel, 'archive')} />
        <RB iconOnly cls="only-c" icon="ban" tone="c-red" label="Segnala come posta indesiderata" disabled={noSel} onclick={() => store.moveMsg(sel, 'junk')} />
      </div><span class="rgl">Elimina</span></div>

      <div class="rg"><div class="rgb">
        <div class="rs">
          <RB icon="arrow-back-up" tone="c-brand" label="Rispondi" title="Rispondi (Ctrl+R)" disabled={noSel} onclick={() => store.openCompose('reply')} />
          <RB icon="arrow-back-up-double" tone="c-brand" label="Rispondi a tutti" title="Rispondi a tutti (Ctrl+Maiusc+R)" disabled={noSel} onclick={() => store.openCompose('replyAll')} />
          <RB icon="arrow-forward-up" tone="c-brand" label="Inoltra" title="Inoltra (Ctrl+F)" disabled={noSel} onclick={() => store.openCompose('forward')} />
        </div>
        <div class="rs only-e">
          <RB showLabel={false} icon="calendar-plus" tone="c-brand" label="Rispondi con una riunione" disabled={noSel} onclick={() => store.soon('Rispondi con una riunione')} />
          <RB showLabel={false} menu icon="message-dots" label="Altre risposte" disabled={noSel} onclick={() => store.soon('Altre risposte')} />
        </div>
      </div><span class="rgl">Rispondi</span></div>

      <div class="rg only-e"><div class="rgb">
        <RB big menu icon="bolt" tone="c-amber" label="Azioni rapide" title="Azioni rapide: più passaggi con un clic" onclick={() => store.soon('Azioni rapide')} />
      </div><span class="rgl">Azioni rapide</span></div>

      <div class="rg"><div class="rgb">
        <RB big menu icon="folder-symlink" tone="c-brand" label="Sposta in" title="Sposta in (Ctrl+Maiusc+V)" disabled={noSel} pressed={!!moveAnchor} onclick={openMove} />
        <RB big menu cls="only-e" icon="list-check" tone="c-brand" label="Regole" title="Regole: sposta in automatico i messaggi" pressed={!!rulesAnchor} onclick={(e) => (rulesAnchor = rulesAnchor ? null : (e.currentTarget as HTMLElement))} />
      </div><span class="rgl">Sposta</span></div>

      <div class="rg"><div class="rgb"><div class="rs">
        <RB icon="mail-opened" tone="c-brand" label="Letto/Non letto" title="Segna come letto (Ctrl+Q) o non letto (Ctrl+U)" disabled={noSel} onclick={() => store.toggleRead(sel)} />
        <RB menu labelExtOnly icon="tag" tone="c-amber" label="Categorizza" disabled={noSel} pressed={!!catAnchor} onclick={openCats} />
        <RB icon="flag" tone="c-red" label="Contrassegna" title="Contrassegna (Ins)" disabled={noSel} onclick={() => store.toggleFlag(sel)} />
      </div></div><span class="rgl">Tag</span></div>

      <div class="rg only-e p2"><div class="rgb">
        <RB big menu icon="search" label="Trova" title="Cerca persone, rubrica e filtri della posta" onclick={() => document.querySelector<HTMLInputElement>('.search input')?.focus()} />
      </div><span class="rgl">Trova</span></div>

      <div class="rg only-e p1"><div class="rgb">
        <RB big icon={store.speech === 'off' ? 'volume' : 'player-stop'} tone="c-brand" label={store.speech === 'off' ? 'Leggi ad alta voce' : 'Interrompi la lettura'} title="Leggi ad alta voce il messaggio (Ctrl+Alt+Spazio)" disabled={noSel && store.speech === 'off'} pressed={store.speech !== 'off'} onclick={() => store.toggleSpeech()} />
      </div><span class="rgl">Comandi vocali</span></div>

      <div class="rg only-e p1"><div class="rgb">
        <RB big icon="language" tone="c-brand" label="Traduci" title="Traduci il messaggio" onclick={() => store.soon('Traduci')} />
      </div><span class="rgl">Lingua</span></div>

      <div class="rg only-e p2"><div class="rgb">
        <RB big icon="puzzle" tone="c-brand" label="Tutte le estensioni" title="Estensioni installate" onclick={() => store.soon('Estensioni')} />
      </div><span class="rgl">Estensioni</span></div>
    {:else if rib === 'visualizza'}
      <div class="rg"><div class="rgb">
        <RB big icon="layout-sidebar" tone="c-brand" label="Riquadro cartelle" pressed={store.showFolders} onclick={() => store.toggleFolders()} />
        <RB big menu icon="layout-sidebar-right" tone="c-brand" label="Riquadro di lettura" onclick={() => store.soon('Posizione del riquadro di lettura')} />
      </div><span class="rgl">Layout</span></div>
      <div class="rg"><div class="rgb">
        <RB big menu icon="layout-rows" label="Densità" onclick={() => store.soon('Densità')} />
        <RB big icon="messages" label="Conversazioni" title="Mostra i messaggi raggruppati per conversazione" pressed={store.conversations} onclick={() => store.toggleConversations()} />
      </div><span class="rgl">Elenco messaggi</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon={store.theme === 'scuro' ? 'sun' : 'moon'} tone={store.theme === 'scuro' ? 'c-amber' : ''} label={store.theme === 'scuro' ? 'Tema chiaro' : 'Tema scuro'} onclick={() => store.toggleTheme()} />
        <RB big icon="app-window" label={ext ? 'Barra compatta' : 'Barra estesa'} onclick={() => store.toggleRibbon()} />
      </div><span class="rgl">Aspetto</span></div>
    {:else if rib === 'aiuto'}
      <div class="rg"><div class="rgb">
        <RB big icon="help-circle" tone="c-brand" label="Guida" onclick={() => store.openLink(`${REPO_URL}#readme`)} />
        <RB big icon="keyboard" label="Scorciatoie da tastiera" onclick={() => store.openLink(`${REPO_URL}/blob/main/docs/scorciatoie.md`)} />
        <RB big icon="news" tone="c-brand" label="Novità" onclick={() => store.openLink(`${REPO_URL}/releases`)} />
      </div><span class="rgl">Guida</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="bug" tone="c-red" label="Segnala un problema" onclick={() => store.openLink(`${REPO_URL}/issues`)} />
        <RB big icon="code" label="Codice sorgente" onclick={() => store.openLink(REPO_URL)} />
      </div><span class="rgl">Progetto open source</span></div>
    {:else}
      <RibbonCompose tab={rib} />
    {/if}
    </div>

    {#if moveAnchor && store.selected}
      <Menu anchor={moveAnchor} items={moveItems(store.selected)} label="Sposta in" onclose={() => (moveAnchor = null)} />
    {/if}
    {#if rulesAnchor}
      <Menu anchor={rulesAnchor} items={rulesItems} label="Regole" onclose={() => (rulesAnchor = null)} />
    {/if}
    {#if catAnchor && store.selected}
      <Menu anchor={catAnchor} items={categoryItems(store.selected)} label="Categorizza" onclose={() => (catAnchor = null)} />
    {/if}
    <button class="rb ico rib-tg" onclick={() => store.toggleRibbon()} aria-label={ext ? 'Passa alla barra compatta' : 'Passa alla barra estesa'} title={ext ? 'Passa alla barra compatta' : 'Passa alla barra estesa'}>
      <Icon name={ext ? 'chevron-up' : 'chevron-down'} />
    </button>
  </div>
</div>

<style>
  .ribbon-wrap { flex: none; background: var(--ground); padding: 0 10px 8px; }
  .file-tab { padding: 0 14px; color: var(--brand-ink); font-weight: 600; }
</style>
