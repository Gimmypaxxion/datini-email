<script lang="ts">
  // Schede e barra dei comandi della finestra principale.
  // Gruppi e ordine seguono Outlook; in modalità compatta restano solo
  // i comandi più usati, in una riga.
  import RB from './RB.svelte';
  import RibbonCompose from './RibbonCompose.svelte';
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import { store, type RibbonTab } from '../lib/store.svelte';
  import { categoryItems, moveItems, quickItems } from '../lib/menus';
  import { iconOr } from '../lib/icons';
  import { dayKey, dueChoices } from '../lib/tasks';
  import { REPO_URL } from '../lib/platform';
  import { fitGroups } from '../lib/fit';

  const TAB_NAMES: Record<RibbonTab, string> = {
    home: 'Home',
    visualizza: 'Visualizza',
    aiuto: 'Aiuto',
    messaggio: 'Messaggio',
    inserisci: 'Inserisci',
    opzioni: 'Opzioni',
    formato: 'Formato testo',
  };

  const noSel = $derived(!store.selected);
  /** Nessun messaggio su cui agire (né uno aperto né più selezionati). */
  const none = $derived(!store.selection.length);
  /** Il primo dei selezionati, per i menu Sposta in e Categorizza. */
  const first = $derived(store.selection.length ? store.findMsg(store.selection[0]) : null);
  const ext = $derived(store.prefs.ribbon === 'estesa');
  const rib = $derived(store.activeRibbon);

  let newAnchor = $state<HTMLElement | null>(null);
  const newItems = [
    { label: 'Messaggio', icon: 'mail-plus' as const, keys: 'Ctrl+N', onclick: () => store.openCompose('new') },
    { label: 'Appuntamento', icon: 'calendar-plus' as const, keys: 'Ctrl+Maiusc+A', onclick: () => store.cal.newEvent() },
    { label: 'Riunione', icon: 'users-group' as const, keys: 'Ctrl+Maiusc+Q', onclick: () => store.cal.newEvent({ meeting: true }) },
    { label: 'Contatto', icon: 'user-plus' as const, keys: 'Ctrl+Maiusc+C', onclick: () => store.newContact() },
    { label: 'Attività', icon: 'playlist-add' as const, keys: 'Ctrl+Maiusc+K', onclick: () => store.tasks.newTask() },
  ];
  const VIEWS = [
    ['giorno', 'calendar-due', 'Giorno', 'Ctrl+Alt+1'],
    ['settimana-lavorativa', 'briefcase', 'Settimana lavorativa', 'Ctrl+Alt+2'],
    ['settimana', 'calendar-week', 'Settimana', 'Ctrl+Alt+3'],
    ['mese', 'calendar-month', 'Mese', 'Ctrl+Alt+4'],
  ] as const;

  let paneAnchor = $state<HTMLElement | null>(null);
  let densAnchor = $state<HTMLElement | null>(null);
  const paneItems = $derived([
    { label: 'A destra', icon: 'layout-sidebar-right' as const, checked: store.readingPane === 'destra', onclick: () => store.setReadingPane('destra') },
    { label: 'In basso', icon: 'layout-rows' as const, checked: store.readingPane === 'basso', onclick: () => store.setReadingPane('basso') },
    { label: 'Nascosto', icon: 'layout-list' as const, checked: store.readingPane === 'nascosto', onclick: () => store.setReadingPane('nascosto') },
  ]);
  const densItems = $derived([
    { label: 'Ampia', checked: store.density === 'ampia', onclick: () => store.setDensity('ampia') },
    { label: 'Media', checked: store.density === 'media', onclick: () => store.setDensity('media') },
    { label: 'Compatta', checked: store.density === 'compatta', onclick: () => store.setDensity('compatta') },
  ]);

  let cleanAnchor = $state<HTMLElement | null>(null);
  const cleanItems = $derived([
    { label: 'Pulisci conversazione', icon: 'messages' as const, disabled: noSel, onclick: () => store.cleanUp('conversazione') },
    { label: 'Pulisci cartella', icon: 'folder' as const, onclick: () => store.cleanUp('cartella') },
    { label: 'Pulisci cartella e sottocartelle', icon: 'folders' as const, onclick: () => store.cleanUp('sottocartelle') },
  ]);
  let moreAnchor = $state<HTMLElement | null>(null);
  const moreItems = $derived([
    { label: 'Inoltra come allegato', icon: 'paperclip' as const, keys: 'Ctrl+Alt+F', onclick: () => store.forwardAsAttachment() },
    { label: 'Rispondi con una riunione', icon: 'calendar-plus' as const, disabled: !store.detail, onclick: () => store.meetingFrom() },
  ]);

  let quickAnchor = $state<HTMLElement | null>(null);
  const quickMenu = $derived(quickItems(none));

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
    <div class="rib-groups" use:fitGroups>
    {#if store.module === 'contatti' && rib === 'home'}
      {@const ct = store.selectedContact}
      <div class="rg"><div class="rgb">
        <RB big alwaysLabel cls="pri" icon="user-plus" label="Nuovo contatto" title="Nuovo contatto (Ctrl+N)" onclick={() => store.newContact()} />
      </div><span class="rgl">Nuovo</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="pencil" tone="c-brand" label="Modifica" disabled={!ct || !!store.contactEdit} onclick={() => store.editContact()} />
        <RB big icon="trash" label="Elimina" title="Elimina (Canc)" disabled={!ct} onclick={() => ct && store.deleteContacts([ct.id])} />
      </div><span class="rgl">Azioni</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="mail" tone="c-brand" label="Invia un messaggio" disabled={!ct?.emails.length} onclick={() => ct && store.composeTo(ct.emails[0], ct.displayName || `${ct.firstName} ${ct.lastName}`.trim())} />
      </div><span class="rgl">Comunica</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="file-import" tone="c-brand" label="Importa" title="Importa contatti da vCard, CSV o LDIF" onclick={() => store.importContacts()} />
        <RB big icon="file-export" label="Esporta" title="Esporta la rubrica in vCard" disabled={!store.book.length} onclick={() => store.exportContacts()} />
      </div><span class="rgl">Gestisci</span></div>
    {:else if store.module === 'calendario' && rib === 'home'}
      {@const cal = store.cal}
      {@const occ = cal.selected}
      <div class="rg"><div class="rgb">
        <RB big alwaysLabel cls="pri" icon="calendar-plus" label="Nuovo appuntamento" title="Nuovo appuntamento (Ctrl+N)" onclick={() => cal.newEvent()} />
        <RB big icon="users-group" tone="c-brand" label="Nuova riunione" title="Nuova riunione (Ctrl+Maiusc+Q)" onclick={() => cal.newEvent({ meeting: true })} />
      </div><span class="rgl">Nuovo</span></div>
      <div class="rg" data-drop="2"><div class="rgb">
        <RB big icon="calendar-event" tone="c-brand" label="Oggi" title="Vai a oggi" onclick={() => cal.today()} />
      </div><span class="rgl">Vai a</span></div>
      <div class="rg"><div class="rgb">
        {#each VIEWS as [v, icon, label, keys] (v)}
          <RB big icon={icon} tone="c-brand" label={label} title="{label} ({keys})" pressed={cal.view === v} onclick={() => cal.setView(v)} />
        {/each}
      </div><span class="rgl">Disponi</span></div>
      <div class="rg" data-drop="1"><div class="rgb">
        <RB big icon="pencil" tone="c-brand" label="Apri" title="Apri l'appuntamento (Invio)" disabled={!occ} onclick={() => occ && cal.open(occ)} />
        <RB big icon="trash" label="Elimina" title="Elimina (Canc)" disabled={!occ} onclick={() => occ && cal.remove(occ)} />
      </div><span class="rgl">Appuntamento</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="file-import" tone="c-brand" label="Importa" title="Importa un calendario (.ics) da Outlook, Google o Apple" onclick={() => cal.importIcs()} />
        <RB big icon="file-export" label="Esporta" title="Salva il calendario in un file .ics" onclick={() => cal.exportIcs()} />
      </div><span class="rgl">Gestisci calendari</span></div>
    {:else if store.module === 'attivita' && rib === 'home'}
      {@const tk = store.tasks}
      {@const t = tk.selected}
      {@const myDay = !!t && t.myDay === dayKey(tk.now)}
      <div class="rg"><div class="rgb">
        <RB big alwaysLabel cls="pri" icon="playlist-add" label="Nuova attività" title="Nuova attività (Ctrl+N)" onclick={() => tk.newTask()} />
        <RB big icon="list" tone="c-brand" label="Nuovo elenco" title="Nuovo elenco di attività" onclick={() => tk.startNewList()} />
      </div><span class="rgl">Nuovo</span></div>
      <div class="rg"><div class="rgb">
        <RB big icon="circle-check" tone="c-green" label={t?.done ? 'Da fare' : 'Completata'} title={t?.done ? 'Segna come da fare' : 'Segna come completata'} disabled={!t} onclick={() => t && tk.toggleDone(t)} />
        <RB big icon={t?.important ? 'star-filled' : 'star'} tone="c-brand" label="Importante" title="Importante (Ins)" disabled={!t} pressed={!!t?.important} onclick={() => t && tk.toggleImportant(t)} />
        <RB big icon="sun" tone="c-amber" label="La mia giornata" title={myDay ? 'Togli da La mia giornata' : 'Aggiungi a La mia giornata'} disabled={!t} pressed={myDay} onclick={() => t && tk.toggleMyDay(t)} />
        <RB big icon="trash" label="Elimina" title="Elimina (Canc)" disabled={!t} onclick={() => t && tk.remove(t)} />
      </div><span class="rgl">Gestisci</span></div>
      <div class="rg" data-drop="1"><div class="rgb"><div class="rs">
        {#each dueChoices(tk.now) as c (c.label)}
          <RB icon="calendar-due" tone="c-brand" label={c.label} title="Scade {c.label.toLowerCase()} ({c.hint})" disabled={!t} onclick={() => t && tk.setDue(t, c.at)} />
        {/each}
      </div></div><span class="rgl">Scadenza</span></div>
      <div class="rg" data-drop="2"><div class="rgb">
        <RB big icon="checklist" tone="c-brand" label="Completate" title={tk.doneOpen ? 'Nascondi le attività completate' : 'Mostra le attività completate'} pressed={tk.doneOpen} onclick={() => (tk.doneOpen = !tk.doneOpen)} />
      </div><span class="rgl">Visualizza</span></div>
    {:else if rib === 'home'}
      <div class="rg"><div class="rgb">
        <RB big alwaysLabel cls="pri split" icon="mail-plus" label="Nuovo messaggio" title="Nuovo messaggio (Ctrl+N)" onclick={() => store.openCompose('new')} />
        <button class="rb pri pri-x only-c" aria-label="Altri elementi nuovi" aria-haspopup="menu" aria-expanded={!!newAnchor} title="Nuovo appuntamento, riunione, contatto o attività" onclick={(e) => (newAnchor = newAnchor ? null : e.currentTarget)}><Icon name="chevron-down" size={16} class="chev" /></button>
        <RB big menu cls="only-e" icon="layout-grid-add" tone="c-brand" label="Nuovi elementi" title="Nuovo appuntamento, riunione, contatto o attività" pressed={!!newAnchor} onclick={(e) => (newAnchor = newAnchor ? null : (e.currentTarget as HTMLElement))} />
      </div><span class="rgl">Nuovo</span></div>

      <div class="rg only-e" data-drop="6"><div class="rgb">
        <RB big icon="refresh" tone="c-brand" label="Invia e ricevi" title="Invia/Ricevi tutte le cartelle (F9)" onclick={() => store.syncNow()} />
      </div><span class="rgl">Aggiorna</span></div>

      <div class="rg"><div class="rgb">
        <div class="rs only-e">
          <RB showLabel={false} icon="mail-off" label="Ignora conversazione" title={store.threadIgnored ? 'Smetti di ignorare questa conversazione' : 'Ignora conversazione: questa e le risposte future vanno in Posta eliminata'} disabled={noSel} pressed={store.threadIgnored} onclick={() => store.ignoreConversation()} />
          <RB showLabel={false} menu icon="eraser" tone="c-amber" label="Pulisci" title="Pulisci: togli i messaggi già riportati nelle risposte" disabled={!store.currentFolder} pressed={!!cleanAnchor} onclick={(e) => (cleanAnchor = cleanAnchor ? null : (e.currentTarget as HTMLElement))} />
          <RB showLabel={false} menu icon="ban" tone="c-red" label="Posta indesiderata" title="Segnala come posta indesiderata" disabled={none} onclick={() => store.moveSelected('junk')} />
        </div>
        <RB big icon="trash" label="Elimina" title="Elimina (Canc)" disabled={none} onclick={() => store.deleteMany()} />
        <RB big icon="archive" tone="c-green" label="Archivia" title="Archivia (Backspace)" disabled={none} onclick={() => store.moveSelected('archive')} />
        <RB iconOnly cls="only-c" icon="ban" tone="c-red" label="Segnala come posta indesiderata" disabled={none} onclick={() => store.moveSelected('junk')} />
      </div><span class="rgl">Elimina</span></div>

      <div class="rg"><div class="rgb">
        <div class="rs">
          <RB icon="arrow-back-up" tone="c-brand" label="Rispondi" title="Rispondi (Ctrl+R)" disabled={noSel} onclick={() => store.openCompose('reply')} />
          <RB icon="arrow-back-up-double" tone="c-brand" label="Rispondi a tutti" title="Rispondi a tutti (Ctrl+Maiusc+R)" disabled={noSel} onclick={() => store.openCompose('replyAll')} />
          <RB icon="arrow-forward-up" tone="c-brand" label="Inoltra" title={store.isMulti ? 'Inoltra i messaggi selezionati come allegati (Ctrl+F)' : 'Inoltra (Ctrl+F)'} disabled={none} onclick={() => (store.isMulti ? store.forwardAsAttachment() : store.openCompose('forward'))} />
        </div>
        <div class="rs only-e">
          <RB showLabel={false} icon="calendar-plus" tone="c-brand" label="Rispondi con una riunione" disabled={noSel || !store.detail} onclick={() => store.meetingFrom()} />
          <RB showLabel={false} menu icon="message-dots" label="Altro" title="Altre risposte" disabled={noSel} pressed={!!moreAnchor} onclick={(e) => (moreAnchor = moreAnchor ? null : (e.currentTarget as HTMLElement))} />
        </div>
      </div><span class="rgl">Rispondi</span></div>

      <div class="rg" data-drop="5"><div class="rgb">
        {#if store.quickSteps.length}
          <div class="rs only-e qs-gal">
            {#each store.quickSteps.slice(0, 3) as q (q.id)}
              <RB icon={iconOr(q.icon, 'bolt')} tone="c-brand" label={q.name} title={q.shortcut ? `${q.name} (Ctrl+Maiusc+${q.shortcut})` : q.name} disabled={none} onclick={() => store.runQuickStep(q)} />
            {/each}
          </div>
        {/if}
        <RB big menu icon="bolt" tone="c-amber" label="Azioni rapide" title="Azioni rapide: più passaggi con un clic" pressed={!!quickAnchor} onclick={(e) => (quickAnchor = quickAnchor ? null : (e.currentTarget as HTMLElement))} />
      </div><span class="rgl">Azioni rapide</span></div>

      <div class="rg"><div class="rgb">
        <RB big menu icon="folder-symlink" tone="c-brand" label="Sposta in" title="Sposta in (Ctrl+Maiusc+V)" disabled={none} pressed={!!moveAnchor} onclick={openMove} />
        <RB big menu cls="only-e" icon="list-check" tone="c-brand" label="Regole" title="Regole: sposta in automatico i messaggi" pressed={!!rulesAnchor} onclick={(e) => (rulesAnchor = rulesAnchor ? null : (e.currentTarget as HTMLElement))} />
      </div><span class="rgl">Sposta</span></div>

      <div class="rg"><div class="rgb"><div class="rs">
        <RB icon="mail-opened" tone="c-brand" label="Letto/Non letto" title="Segna come letto (Ctrl+Q) o non letto (Ctrl+U)" disabled={none} onclick={() => store.toggleReadSelected()} />
        <RB menu labelExtOnly icon="tag" tone="c-amber" label="Categorizza" disabled={none} pressed={!!catAnchor} onclick={openCats} />
        <RB icon="flag" tone="c-red" label="Contrassegna" title="Contrassegna (Ins)" disabled={none} onclick={() => store.toggleFlagSelected()} />
      </div></div><span class="rgl">Tag</span></div>

      <div class="rg only-e" data-drop="4"><div class="rgb">
        <RB big menu icon="search" label="Trova" title="Cerca persone, rubrica e filtri della posta" onclick={() => document.querySelector<HTMLInputElement>('.search input')?.focus()} />
      </div><span class="rgl">Trova</span></div>

      <div class="rg only-e" data-drop="3"><div class="rgb">
        <RB big icon={store.speech === 'off' ? 'volume' : 'player-stop'} tone="c-brand" label={store.speech === 'off' ? 'Leggi ad alta voce' : 'Interrompi la lettura'} title="Leggi ad alta voce il messaggio (Ctrl+Alt+Spazio)" disabled={noSel && store.speech === 'off'} pressed={store.speech !== 'off'} onclick={() => store.toggleSpeech()} />
      </div><span class="rgl">Comandi vocali</span></div>

      <div class="rg only-e" data-drop="2"><div class="rgb">
        <RB big icon="language" tone="c-brand" label="Traduci" title="Traduci il messaggio" onclick={() => store.soon('Traduci')} />
      </div><span class="rgl">Lingua</span></div>

      <div class="rg only-e" data-drop="1"><div class="rgb">
        <RB big icon="puzzle" tone="c-brand" label="Tutte le estensioni" title="Estensioni installate" onclick={() => store.soon('Estensioni')} />
      </div><span class="rgl">Estensioni</span></div>
    {:else if rib === 'visualizza'}
      <div class="rg"><div class="rgb">
        <RB big icon="layout-sidebar" tone="c-brand" label="Riquadro cartelle" pressed={store.showFolders} onclick={() => store.toggleFolders()} />
        <RB big menu icon="layout-sidebar-right" tone="c-brand" label="Riquadro di lettura" title="Riquadro di lettura: a destra, in basso o nascosto" pressed={!!paneAnchor} onclick={(e) => (paneAnchor = paneAnchor ? null : (e.currentTarget as HTMLElement))} />
      </div><span class="rgl">Layout</span></div>
      <div class="rg"><div class="rgb">
        <RB big menu icon="layout-rows" label="Densità" title="Spazio fra le righe dell'elenco" pressed={!!densAnchor} onclick={(e) => (densAnchor = densAnchor ? null : (e.currentTarget as HTMLElement))} />
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

    {#if paneAnchor}
      <Menu anchor={paneAnchor} items={paneItems} label="Riquadro di lettura" onclose={() => (paneAnchor = null)} />
    {/if}
    {#if densAnchor}
      <Menu anchor={densAnchor} items={densItems} label="Densità" onclose={() => (densAnchor = null)} />
    {/if}
    {#if newAnchor}
      <Menu anchor={newAnchor} items={newItems} label="Nuovi elementi" onclose={() => (newAnchor = null)} />
    {/if}
    {#if moveAnchor && first}
      <Menu anchor={moveAnchor} items={moveItems(first, store.selection)} label="Sposta in" onclose={() => (moveAnchor = null)} />
    {/if}
    {#if quickAnchor}
      <Menu anchor={quickAnchor} items={quickMenu} label="Azioni rapide" onclose={() => (quickAnchor = null)} />
    {/if}
    {#if cleanAnchor}
      <Menu anchor={cleanAnchor} items={cleanItems} label="Pulisci" onclose={() => (cleanAnchor = null)} />
    {/if}
    {#if moreAnchor && store.selected}
      <Menu anchor={moreAnchor} items={moreItems} label="Altre risposte" onclose={() => (moreAnchor = null)} />
    {/if}
    {#if rulesAnchor}
      <Menu anchor={rulesAnchor} items={rulesItems} label="Regole" onclose={() => (rulesAnchor = null)} />
    {/if}
    {#if catAnchor && first}
      <Menu anchor={catAnchor} items={categoryItems(first, store.selection)} label="Categorizza" onclose={() => (catAnchor = null)} />
    {/if}
    <button class="rb ico rib-tg" onclick={() => store.toggleRibbon()} aria-label={ext ? 'Passa alla barra compatta' : 'Passa alla barra estesa'} title={ext ? 'Passa alla barra compatta' : 'Passa alla barra estesa'}>
      <Icon name={ext ? 'chevron-up' : 'chevron-down'} />
    </button>
  </div>
</div>

<style>
  .ribbon-wrap { flex: none; background: var(--ground); padding: 0 10px 8px; }
  .file-tab { padding: 0 14px; color: var(--brand-ink); font-weight: 600; }
  /* Le prime azioni rapide, come la raccolta di Outlook: nomi lunghi accorciati. */
  .qs-gal :global(.rb) { max-width: 168px; }
  .qs-gal :global(.rb .lbl) { overflow: hidden; text-overflow: ellipsis; }
</style>
