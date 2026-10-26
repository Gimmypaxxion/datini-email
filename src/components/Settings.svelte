<script lang="ts">
  // Impostazioni: Aspetto, Account e sincronizzazione, Composizione e
  // risposta (firme e carattere), Informazioni su Better-Look.
  import Icon from '../lib/Icon.svelte';
  import About from './About.svelte';
  import RulesPage from './RulesPage.svelte';
  import AutoReplyPage from './AutoReplyPage.svelte';
  import CleanupPage from './CleanupPage.svelte';
  import AutoArchivePage from './AutoArchivePage.svelte';
  import QuickStepsPage from './QuickStepsPage.svelte';
  import TemplatesPage from './TemplatesPage.svelte';
  import RichEditor from './RichEditor.svelte';
  import LinkDialog from './LinkDialog.svelte';
  import Popover from './Popover.svelte';
  import { store, type Prefs, type ThemeChoice } from '../lib/store.svelte';
  import { EditorCtl, FONTS, SIZES, fontStack } from '../lib/editor.svelte';
  import { ACCENT_SWATCHES, DEFAULT_ACCENT, accentPalette, accentShift, normalizeHex, readableOn } from '../lib/accent';
  import { isMac } from '../lib/platform';
  import { CATEGORY_COLORS, type Category, type CategoryColor, type ReceiptReply, type ServerConfig, type Signature, type SyncOptions } from '../lib/types';

  const RECEIPT_CHOICES: [ReceiptReply, string][] = [
    ['chiedi', 'Chiedi ogni volta se inviarla'],
    ['sempre', 'Inviala sempre'],
    ['mai', 'Non inviarla mai'],
  ];

  interface Opt {
    on: boolean;
    title: string;
    text: string;
    pick: () => void;
  }
  interface Group {
    id: string;
    legend: string;
    opts: Opt[];
    extra?: boolean;
  }

  function pref<K extends keyof Prefs>(k: K, v: Prefs[K], title: string, text: string): Opt {
    return { on: store.prefs[k] === v, title, text, pick: () => store.setPref(k, v) };
  }
  function themeOpt(v: ThemeChoice, title: string, text: string): Opt {
    return { on: store.themeChoice === v, title, text, pick: () => store.setTheme(v) };
  }

  const where = isMac ? 'dall’icona nella barra dei menu o dal Dock' : 'dall’icona vicino all’orologio';
  const closeCards = $derived<Opt[]>([
    { on: store.background, title: 'Resta attivo in sottofondo', text: `Ricevi la posta e restano attivi regole, risposte automatiche, invio programmato e promemoria. Lo riapri ${where}.`, pick: () => store.setBackground(true) },
    { on: !store.background, title: 'Esci dal programma', text: 'Si chiude tutto: regole, risposte automatiche, invio programmato e promemoria riprendono alla prossima apertura.', pick: () => store.setBackground(false) },
  ]);

  const presetCards = $derived<Opt[]>([
    { on: store.presetName === 'classico', title: 'Outlook classico', text: 'Barra estesa, menu File e nuovi messaggi in una finestra separata.', pick: () => store.applyPreset('classico') },
    { on: store.presetName === 'nuovo', title: 'Nuovo Outlook', text: 'Barra compatta, ingranaggio e nuovi messaggi nel riquadro di lettura.', pick: () => store.applyPreset('nuovo') },
  ]);

  const groups = $derived<Group[]>([
    { id: 'set-rib', legend: 'Barra dei comandi', opts: [
      pref('ribbon', 'estesa', 'Estesa', 'Icone grandi con il nome sotto, come in Outlook classico. Il pulsante Invia sta a sinistra dei destinatari.'),
      pref('ribbon', 'compatta', 'Compatta', 'Una sola riga di comandi, come nel nuovo Outlook. Il pulsante Invia sta sopra i destinatari.'),
    ] },
    { id: 'set-entry', legend: 'Accesso a impostazioni e account', opts: [
      pref('entry', 'file', 'Menu File', 'Una scheda File a sinistra delle altre, come in Outlook classico.'),
      pref('entry', 'ingranaggio', 'Ingranaggio', 'Un’icona in alto a destra, come nel nuovo Outlook.'),
    ] },
    { id: 'set-cmp', legend: 'Nuovi messaggi', extra: true, opts: [
      pref('compose', 'finestra', 'In una finestra separata', 'Puoi continuare a leggere la posta mentre scrivi.'),
      pref('compose', 'pannello', 'Nel riquadro di lettura', 'Scrivi accanto all’elenco dei messaggi, nella stessa finestra.'),
    ] },
    { id: 'set-read', legend: 'Riquadro di lettura', opts: [
      { on: store.readingPane === 'destra', title: 'A destra', text: 'Elenco a sinistra e messaggio a destra, come nella vista predefinita.', pick: () => store.setReadingPane('destra') },
      { on: store.readingPane === 'basso', title: 'In basso', text: 'Elenco sopra e messaggio sotto: utile sugli schermi stretti.', pick: () => store.setReadingPane('basso') },
      { on: store.readingPane === 'nascosto', title: 'Nascosto', text: 'Solo l’elenco; il messaggio si apre con doppio clic o Invio.', pick: () => store.setReadingPane('nascosto') },
    ] },
    { id: 'set-dens', legend: 'Densità dell’elenco', opts: [
      { on: store.density === 'ampia', title: 'Ampia', text: 'Più spazio fra i messaggi.', pick: () => store.setDensity('ampia') },
      { on: store.density === 'media', title: 'Media', text: 'Foto, oggetto e anteprima del testo.', pick: () => store.setDensity('media') },
      { on: store.density === 'compatta', title: 'Compatta', text: 'Senza foto né anteprima: più messaggi sullo schermo.', pick: () => store.setDensity('compatta') },
    ] },
    { id: 'set-theme', legend: 'Tema', opts: [
      themeOpt('sistema', 'Come il sistema', 'Chiaro o scuro, seguendo l’impostazione del computer.'),
      themeOpt('chiaro', 'Chiaro', 'Fondo chiaro, adatto alla luce del giorno.'),
      themeOpt('scuro', 'Scuro', 'Fondo scuro, più riposante la sera.'),
    ] },
  ]);

  // ---------- Colore di accento ----------
  const accentNow = $derived(store.accent ?? DEFAULT_ACCENT);
  const accentCustom = $derived(!ACCENT_SWATCHES.some((s) => s.hex === accentNow));
  /** Il colore usato davvero si allontana da quello scelto, per la leggibilità. */
  const accentAdapted = $derived(accentShift(accentNow, accentPalette(accentNow, store.theme)['--brand'] ?? accentNow) > 0.05);
  let hexDraft = $state('');
  let hexFocus = $state(false);
  const hexBad = $derived(hexFocus && !!hexDraft.trim() && !normalizeHex(hexDraft));
  $effect(() => {
    const v = accentNow;
    if (!hexFocus) hexDraft = v;
  });
  function typeHex(v: string) {
    hexDraft = v;
    if (normalizeHex(v)) store.setAccent(v);
  }

  let dlgEl: HTMLDivElement | undefined = $state();
  let contentEl: HTMLDivElement | undefined = $state();

  // All'apertura il fuoco va alla finestra (non alla X): Esc chiude, Tab entra nei comandi.
  $effect(() => {
    if (store.settings && dlgEl && !dlgEl.contains(document.activeElement)) dlgEl.focus();
  });
  // Cambiando pagina si riparte dall'alto.
  $effect(() => {
    void store.setPage;
    if (contentEl) contentEl.scrollTop = 0;
  });

  const nav = [
    ['aspetto', 'Aspetto'],
    ['avvio', 'Avvio e chiusura'],
    ['account', 'Account e sincronizzazione'],
    ['composizione', 'Composizione e risposta'],
    ['modelli', 'Modelli di messaggio'],
    ['categorie', 'Categorie'],
    ['regole', 'Regole e avvisi'],
    ['azioni', 'Azioni rapide'],
    ['risposte', 'Risposte automatiche'],
    ['pulizia', 'Pulizia cassetta postale'],
    ['archiviazione', 'Archiviazione automatica'],
    ['info', 'Informazioni su Better-Look'],
  ] as const;

  const SEC: Record<string, string> = { ssl: 'SSL/TLS', starttls: 'STARTTLS' };
  const PROTO: Record<string, string> = { imap: 'IMAP', pop3: 'POP', local: 'File di dati' };
  const STATE: Record<string, string> = { pronto: 'Connesso', sincronizzazione: 'Sincronizzazione in corso', offline: 'Offline', errore: 'Errore' };

  const acct = $derived(store.accounts.find((a) => a.id === store.acctSel) ?? store.accounts[0] ?? null);
  const srv = (c: ServerConfig | null) => (c ? `${c.host}, porta ${c.port}, ${SEC[c.security] ?? c.security}` : '');

  const imapWindow = [['1m', 'Ultimo mese'], ['3m', 'Ultimi 3 mesi'], ['1a', 'Ultimo anno'], ['tutto', 'Tutta la posta']] as const;
  const attachOpts = [['subito', 'Scarica subito'], ['apertura', 'Quando li apro']] as const;
  const popKeep = [['mai', 'Non lasciare copie'], ['14', 'Lascia per 14 giorni'], ['sempre', 'Lascia sempre']] as const;

  let newPw = $state('');
  let confirmRemove = $state(false);
  let nameDraft = $state('');
  $effect(() => {
    nameDraft = acct?.name ?? '';
    confirmRemove = false;
    newPw = '';
  });

  // ---------- Firme ----------
  const sigEd = new EditorCtl();
  const senders = $derived(store.accounts.filter((a) => store.canSend(a)));
  let sigSel = $state<string | null>(null);
  let sigName = $state('');
  let sigHtml = $state('');
  let sigDirty = $state(false);
  let sigNote = $state('');
  let sigVersion = $state(0);
  let sigConfirm = $state(false);
  let linkBtn: HTMLButtonElement | undefined = $state();
  let linkOpen = $state(false);
  let sigImg: HTMLInputElement | undefined = $state();

  /** La prima firma parte già scritta: nome e indirizzo dell'account, da completare. */
  function starterSignature(): string {
    const a = senders[0];
    if (!a) return '';
    const esc = (t: string) => t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    const name = a.name && a.name.trim() && a.name !== a.email ? `<div><b>${esc(a.name.trim())}</b></div>` : '';
    return `${name}<div>${esc(a.email)}</div>`;
  }

  function pickSig(s: Signature | null) {
    sigSel = s?.id ?? null;
    sigName = s?.name ?? (store.signatures.length ? '' : 'Firma');
    sigHtml = s?.html ?? (store.signatures.length ? '' : starterSignature());
    sigDirty = false;
    sigNote = '';
    sigConfirm = false;
    sigVersion++;
  }

  // All'apertura della pagina: la prima firma, oppure una nuova da scrivere.
  let sigInit = false;
  $effect(() => {
    if (store.settings && store.setPage === 'composizione') {
      if (!sigInit) {
        sigInit = true;
        pickSig(store.signatures[0] ?? null);
      }
    } else {
      sigInit = false;
    }
  });

  async function saveSig() {
    const first = !store.signatures.length;
    const saved = await store.saveSignature({ id: sigSel ?? '', name: sigName.trim() || 'Firma', html: sigHtml });
    if (!saved) return;
    sigSel = saved.id;
    sigName = saved.name;
    sigDirty = false;
    sigNote = first && senders.length ? 'Firma salvata. Da ora viene aggiunta da sola ai nuovi messaggi e alle risposte: puoi cambiarlo qui sotto.' : 'Firma salvata.';
  }

  async function removeSig() {
    if (!sigSel) return;
    await store.deleteSignature(sigSel);
    pickSig(store.signatures[0] ?? null);
  }

  function sigImages(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = '';
    if (files.length) void sigEd.insertImageFiles(files);
  }

  const sizeText = (s: number) => String(s).replace('.', ',');

  // ---------- Categorie ----------
  let catDraft = $state<Category[]>([]);
  let catErr = $state('');
  let catColorFor = $state<{ index: number; el: HTMLElement } | null>(null);
  let catFocusLast = false;
  // Copia modificabile, aggiornata quando cambiano le categorie salvate.
  $effect(() => {
    catDraft = store.categories.map((c) => ({ ...c }));
  });

  async function saveCats() {
    const err = await store.saveCategories(catDraft.map((c) => ({ ...c })));
    catErr = err;
  }

  function addCat() {
    const used = new Set(catDraft.map((c) => c.color));
    const color = (CATEGORY_COLORS.find(([c]) => !used.has(c)) ?? CATEGORY_COLORS[0])[0];
    let n = 1;
    while (catDraft.some((c) => c.name.toLowerCase() === `nuova categoria${n > 1 ? ` ${n}` : ''}`)) n++;
    catDraft = [...catDraft, { id: '', name: `Nuova categoria${n > 1 ? ` ${n}` : ''}`, color }];
    catFocusLast = true;
    void saveCats();
  }

  function focusNew(el: HTMLInputElement, last: boolean) {
    if (last && catFocusLast) {
      catFocusLast = false;
      el.focus();
      el.select();
    }
  }

  async function removeCat(i: number) {
    const c = catDraft[i];
    const ok = await store.ask({ title: `Eliminare la categoria “${c.name}”?`, text: 'Verrà tolta da tutti i messaggi a cui è assegnata.', ok: 'Elimina', danger: true });
    if (!ok) return;
    catDraft = catDraft.filter((_, j) => j !== i);
    void saveCats();
  }

  function pickColor(color: CategoryColor) {
    if (!catColorFor) return;
    catDraft[catColorFor.index].color = color;
    catColorFor = null;
    void saveCats();
  }

  async function savePw() {
    if (!acct || !newPw) return;
    if (await store.setPassword(acct.id, newPw)) newPw = '';
  }
</script>

{#snippet option(o: Opt, name: string)}
  <label class="opt" data-on={o.on ? 'si' : 'no'}>
    <input type="radio" {name} checked={o.on} onchange={o.pick} />
    <span class="dot" aria-hidden="true"></span>
    <span class="otext"><span class="ot">{o.title}</span><span class="od">{o.text}</span></span>
  </label>
{/snippet}

{#snippet pills(key: keyof SyncOptions, items: readonly (readonly [string, string])[])}
  <div class="pills">
    {#each items as [v, label] (v)}
      {@const on = acct?.sync[key] === v}
      <label class="pill" data-on={on ? 'si' : 'no'}>
        <input type="radio" name="sync-{key}" checked={on} onchange={() => acct && store.updateSync(acct.id, { [key]: v })} />{label}
      </label>
    {/each}
  </div>
{/snippet}

{#if store.settings}
  <div class="scrim fade-in">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      bind:this={dlgEl}
      class="dlg rise"
      role="dialog"
      aria-modal="true"
      aria-labelledby="set-title"
      tabindex="-1"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.stopPropagation();
          store.settings = false;
        }
      }}
    >
      <nav class="nav" aria-label="Sezioni delle impostazioni">
        <h2 id="set-title">Impostazioni</h2>
        {#each nav as [id, label] (id)}
          {@const on = store.setPage === id}
          <button class="btn fold navit" data-sel={on ? 'si' : 'no'} aria-current={on ? 'page' : undefined} style:font-weight={on ? 600 : 400} onclick={() => (store.setPage = id)}>{label}</button>
        {/each}
      </nav>
      <div class="pane">
        <button class="dlg-x close" onclick={() => (store.settings = false)} aria-label="Chiudi impostazioni" title="Chiudi (Esc)"><Icon name="x" size={18} /></button>
        <div class="scroll content" bind:this={contentEl}>

        {#if store.setPage === 'aspetto'}
          <h3>Aspetto</h3>
          <p class="intro">Scegli dove trovare i comandi, il tema e il colore. Le modifiche valgono subito.</p>
          <div role="radiogroup" aria-labelledby="set-preset" style="margin-top:26px">
            <h4 id="set-preset">Stile di Outlook</h4>
            <div class="two">
              {#each presetCards as o (o.title)}{@render option(o, 'set-preset')}{/each}
            </div>
            {#if !store.presetName}<p class="custom">Stai usando una combinazione personalizzata delle due viste.</p>{/if}
          </div>
          {#each groups as g (g.id)}
            <div role="radiogroup" aria-labelledby={g.id} class="group">
              <h4 id={g.id}>{g.legend}</h4>
              <div class="two">
                {#each g.opts as o (o.title)}{@render option(o, g.id)}{/each}
              </div>
              {#if g.extra}
                <label class="check" style:color={store.prefs.compose === 'finestra' ? 'var(--ink)' : 'var(--ink-3)'}>
                  <input type="checkbox" checked={store.prefs.replyWin} disabled={store.prefs.compose !== 'finestra'} onchange={() => store.setPref('replyWin', !store.prefs.replyWin)} />
                  Apri in una finestra anche risposte e inoltri
                </label>
              {/if}
            </div>
          {/each}
          <div class="group">
            <h4 id="set-accent">Colore di accento</h4>
            <p class="hint">Colora la barra in alto, i pulsanti principali e la selezione, nel tema chiaro e in quello scuro.</p>
            <div class="accent-row">
              <div role="radiogroup" aria-labelledby="set-accent" class="swatches">
                {#each ACCENT_SWATCHES as s (s.hex)}
                  {@const on = accentNow === s.hex}
                  <label class="sw" data-on={on ? 'si' : 'no'} title={s.hex === DEFAULT_ACCENT ? `${s.name} (predefinito)` : s.name} style:background={s.hex} style:color={readableOn(s.hex)}>
                    <input type="radio" name="set-accent" checked={on} onchange={() => store.setAccent(s.hex)} aria-label={s.hex === DEFAULT_ACCENT ? `${s.name} (predefinito)` : s.name} />
                    {#if on}<Icon name="check" size={18} stroke={2.5} />{/if}
                  </label>
                {/each}
              </div>
              <label class="sw pick" data-on={accentCustom ? 'si' : 'no'} title="Altro colore" style:background={accentCustom ? accentNow : null} style:color={accentCustom ? readableOn(accentNow) : null}>
                <input type="color" aria-label="Altro colore" value={accentNow.toLowerCase()} oninput={(e) => store.setAccent(e.currentTarget.value)} />
                <Icon name={accentCustom ? 'check' : 'color-picker'} size={18} stroke={accentCustom ? 2.5 : 2} />
              </label>
              <label class="tf hex" data-bad={hexBad ? 'si' : 'no'}>
                <span class="sr">Codice del colore</span>
                <input
                  value={hexDraft}
                  maxlength="7"
                  spellcheck="false"
                  autocomplete="off"
                  aria-invalid={hexBad}
                  title="Codice esadecimale, per esempio #0F6CBD"
                  onfocus={() => (hexFocus = true)}
                  onblur={() => { hexFocus = false; hexDraft = accentNow; }}
                  oninput={(e) => typeHex(e.currentTarget.value)}
                />
              </label>
            </div>
            {#if hexBad}
              <p class="custom">Scrivi un codice come #0F6CBD: il cancelletto e sei cifre o lettere da A a F.</p>
            {:else if accentAdapted}
              <p class="custom">Nel tema {store.theme} questo colore è leggermente adattato, per tenere leggibili testi e icone.</p>
            {/if}
          </div>
        {:else if store.setPage === 'avvio'}
          <h3>Avvio e chiusura</h3>
          <p class="intro">Cosa succede quando chiudi la finestra di Better-Look e quando accendi il computer.</p>
          <div role="radiogroup" aria-labelledby="set-close" style="margin-top:26px">
            <h4 id="set-close">Quando chiudi la finestra</h4>
            <div class="two">
              {#each closeCards as o (o.title)}{@render option(o, 'set-close')}{/each}
            </div>
          </div>
          <div class="group">
            <h4 id="set-start">All’accensione del computer</h4>
            <label class="check">
              <input type="checkbox" checked={store.autoStart} onchange={(e) => store.setAutoStart(e.currentTarget.checked)} />
              Avvia Better-Look in sottofondo, senza aprire la finestra
            </label>
            <p class="hint sub">Così la posta arriva e i promemoria suonano anche se non apri il programma. {isMac ? 'Puoi toglierlo anche da Impostazioni di Sistema › Generali › Elementi login.' : 'Puoi toglierlo anche dalle app di avvio del sistema.'}</p>
          </div>
          <p class="hint" style="margin-top:22px">Per chiudere davvero Better-Look scegli Esci dal menu dell’icona{isMac ? ' o premi Cmd+Q' : ''}.</p>
        {:else if store.setPage === 'account'}
          <h3>Account e sincronizzazione</h3>
          <p class="intro">La posta viaggia direttamente tra questo computer e il server del tuo provider. Better-Look non usa server intermedi.</p>
          <div class="acc-grid">
            <div class="acc-list">
              {#each store.accounts as a (a.id)}
                {@const on = acct?.id === a.id}
                <button class="btn fold acc" data-sel={on ? 'si' : 'no'} aria-current={on ? 'true' : undefined} onclick={() => (store.acctSel = a.id)}>
                  <span class="addr">{a.kind === 'local' ? a.name : a.email}</span><span class="kind">{PROTO[a.kind]}</span>
                </button>
              {/each}
              <button class="btn add" onclick={() => store.openWizard(false)}><Icon name="plus" size={16} />Aggiungi account</button>
            </div>
            {#if acct}
              <div style="min-width:0">
                {#if acct.kind === 'local'}
                  <div class="srv">
                    <span class="k">Tipo</span><span style="font-weight:600">File di dati</span>
                    {#if acct.sourceFile}<span class="k">Importato da</span><span class="path">{acct.sourceFile}</span>{/if}
                    <span class="k">Dove si trova</span><span>Solo su questo computer, nell’archivio di Better-Look</span>
                  </div>
                {:else}
                  <div class="srv">
                    <span class="k">Stato</span>
                    <span class="state" data-s={acct.status.state}>
                      <Icon name={acct.status.state === 'offline' ? 'cloud-off' : acct.status.state === 'errore' ? 'alert-triangle' : 'circle-check'} size={16} />
                      {STATE[acct.status.state] ?? acct.status.state}{acct.status.message ? `: ${acct.status.message}` : ''}
                    </span>
                    <span class="k">Protocollo</span><span style="font-weight:600">{PROTO[acct.kind]}</span>
                    <span class="k">Server in arrivo</span><span>{srv(acct.incoming)}</span>
                    <span class="k">Server in uscita</span><span>{srv(acct.outgoing)}</span>
                    <span class="k">Nome utente</span><span>{acct.incoming?.username}</span>
                    <span class="k">Connessione</span><span class="conn"><Icon name="server-2" size={16} style="color:var(--brand-ink)" />Diretta, senza server intermedi</span>
                  </div>

                  <div class="group-a">
                    <label class="tf" style="max-width:380px">
                      <span>Il tuo nome <span class="hint">come lo vedono i destinatari</span></span>
                      <input bind:value={nameDraft} onblur={() => nameDraft.trim() && nameDraft !== acct.name && store.renameAccount(acct.id, nameDraft)} />
                    </label>
                  </div>

                  {#if acct.kind === 'imap'}
                    <div role="radiogroup" aria-labelledby="sync-win" class="group-a">
                      <h4 id="sync-win" class="tight">Posta da tenere sul computer</h4>
                      <p class="hint">I messaggi più vecchi restano sul server e si scaricano quando li apri. Senza connessione vedi solo quelli tenuti sul computer.</p>
                      {@render pills('keep', imapWindow)}
                    </div>
                    <div role="radiogroup" aria-labelledby="sync-att" class="group-a">
                      <h4 id="sync-att" class="tight">Allegati</h4>
                      <p class="hint">Con “Quando li apro” i messaggi con allegati oltre 3 MB si scaricano solo quando li apri.</p>
                      {@render pills('attachments', attachOpts)}
                    </div>
                  {:else}
                    <div role="note" class="note"><Icon name="alert-triangle" style="color:var(--warn);margin-top:1px" /><span>Con POP la posta viene scaricata su questo computer, che ne conserva l’unica copia completa. Includi la cartella di Better-Look nei backup.</span></div>
                    <div role="radiogroup" aria-labelledby="sync-pop" class="group-a">
                      <h4 id="sync-pop" class="tight">Copia sul server</h4>
                      <p class="hint">Per quanto tempo lasciare i messaggi anche sul server dopo averli scaricati.</p>
                      {@render pills('popKeep', popKeep)}
                    </div>
                  {/if}

                  <form class="group-a pwrow" onsubmit={(e) => { e.preventDefault(); void savePw(); }}>
                    <label class="tf" style="flex:1;max-width:300px">
                      <span>Cambia password</span>
                      <input type="password" bind:value={newPw} autocomplete="new-password" placeholder="Nuova password" />
                    </label>
                    <button type="submit" class="sec-btn" disabled={!newPw}>Salva password</button>
                  </form>
                {/if}

                <div class="group-a remove">
                  {#if confirmRemove}
                    <p class="warn-t">
                      {acct.kind === 'local'
                        ? 'Il file di dati verrà tolto da Better-Look. Il file originale sul disco resta dov’è.'
                        : acct.kind === 'pop3'
                          ? 'L’account e la posta scaricata su questo computer verranno tolti da Better-Look. I messaggi ancora sul server restano lì.'
                          : 'L’account verrà tolto da Better-Look. La posta resta sul server del tuo provider.'}
                    </p>
                    <div class="rm-acts">
                      <button class="sec-btn danger-btn" onclick={() => { const id = acct.id; confirmRemove = false; void store.removeAccount(id); }}>{acct.kind === 'local' ? 'Chiudi file di dati' : 'Rimuovi account'}</button>
                      <button class="sec-btn" onclick={() => (confirmRemove = false)}>Annulla</button>
                    </div>
                  {:else}
                    <button class="sec-btn danger-btn" onclick={() => (confirmRemove = true)}><Icon name="trash" size={16} />{acct.kind === 'local' ? 'Chiudi file di dati' : 'Rimuovi account'}</button>
                  {/if}
                </div>
              </div>
            {:else}
              <div class="noacc">
                <p>Non hai ancora aggiunto account.</p>
                <button class="primary-btn" onclick={() => store.openWizard(false)}><Icon name="user-plus" size={17} />Aggiungi account</button>
              </div>
            {/if}
          </div>
        {:else if store.setPage === 'composizione'}
          <h3>Composizione e risposta</h3>
          <p class="intro">La firma in fondo ai messaggi, il carattere del testo che scrivi e le conferme di lettura.</p>

          <div class="group-a">
            <h4 class="tight">Firma e-mail</h4>
            {#if store.signatures.length}
              <p class="hint">Scrivila qui sotto oppure incollala da un documento o da un sito: testo formattato e logo compresi.</p>
            {:else}
              <p class="hint">Nome e indirizzo sono già scritti: aggiungi ruolo, azienda e telefono, poi Salva. Puoi anche incollare la firma che usi oggi, copiandola da un messaggio inviato, logo compreso.</p>
            {/if}
            <div class="sig-grid">
              <div class="sig-list" role="listbox" aria-label="Firme">
                {#each store.signatures as s (s.id)}
                  {@const on = sigSel === s.id}
                  <button class="btn fold sig-it" role="option" aria-selected={on} data-sel={on ? 'si' : 'no'} onclick={() => pickSig(s)}><Icon name="signature" size={16} />{s.name}</button>
                {/each}
                {#if !store.signatures.length}<p class="sig-empty">Non hai ancora una firma.</p>{/if}
                <button class="btn add" onclick={() => pickSig(null)}><Icon name="plus" size={16} />Nuova firma</button>
              </div>
              <div class="sig-edit">
                <label class="tf" style="max-width:320px">
                  <span>Nome della firma <span class="hint">serve solo a te, per riconoscerla</span></span>
                  <input bind:value={sigName} oninput={() => { sigDirty = true; sigNote = ''; }} placeholder="Per esempio: Firma ufficio" />
                </label>
                <div class="sig-box">
                  <div class="sig-bar" role="toolbar" aria-label="Formato della firma">
                    <select class="sig-sel" aria-label="Tipo di carattere" value={sigEd.font} onchange={(e) => sigEd.setFont(e.currentTarget.value)} disabled={!sigEd.ready} style:width="118px">
                      {#if sigEd.font && !FONTS.some((f) => f.name === sigEd.font)}<option value={sigEd.font}>{sigEd.font}</option>{/if}
                      {#each FONTS as f (f.name)}<option value={f.name}>{f.name}</option>{/each}
                    </select>
                    <select class="sig-sel" aria-label="Dimensione carattere" value={String(sigEd.size ?? store.font.size)} onchange={(e) => sigEd.setSize(Number(e.currentTarget.value))} disabled={!sigEd.ready} style:width="58px">
                      {#if sigEd.size && !SIZES.includes(sigEd.size)}<option value={String(sigEd.size)}>{sizeText(sigEd.size)}</option>{/if}
                      {#each SIZES as sz (sz)}<option value={String(sz)}>{sizeText(sz)}</option>{/each}
                    </select>
                    <span class="sig-sep"></span>
                    <button class="rb ico lt" aria-label="Grassetto" title="Grassetto (Ctrl+G)" aria-pressed={sigEd.bold} onmousedown={(e) => e.preventDefault()} onclick={() => sigEd.toggleBold()}><span style="font-weight:700">G</span></button>
                    <button class="rb ico lt" aria-label="Corsivo" title="Corsivo (Ctrl+I)" aria-pressed={sigEd.italic} onmousedown={(e) => e.preventDefault()} onclick={() => sigEd.toggleItalic()}><span style="font-style:italic;font-family:Georgia,serif">C</span></button>
                    <button class="rb ico lt" aria-label="Sottolineato" title="Sottolineato (Ctrl+S)" aria-pressed={sigEd.underline} onmousedown={(e) => e.preventDefault()} onclick={() => sigEd.toggleUnderline()}><span style="text-decoration:underline;text-underline-offset:2px">S</span></button>
                    <label class="rb ico sig-color" title="Colore carattere">
                      <Icon name="text-color" /><span class="bar" style:background={sigEd.lastColor}></span>
                      <input type="color" aria-label="Colore carattere" value={sigEd.lastColor.toLowerCase()} onchange={(e) => sigEd.setColor(e.currentTarget.value.toUpperCase())} />
                    </label>
                    <span class="sig-sep"></span>
                    <button class="rb ico" bind:this={linkBtn} aria-label="Collegamento" title="Inserisci collegamento (Ctrl+K)" aria-pressed={sigEd.link} onmousedown={(e) => e.preventDefault()} onclick={() => (linkOpen = true)}><Icon name="link" /></button>
                    <button class="rb ico" aria-label="Immagine" title="Inserisci un'immagine, per esempio il logo" onmousedown={(e) => e.preventDefault()} onclick={() => sigImg?.click()}><Icon name="photo" /></button>
                    <button class="rb ico" aria-label="Cancella formattazione" title="Cancella formattazione" onmousedown={(e) => e.preventDefault()} onclick={() => sigEd.clearFormat()}><Icon name="clear-formatting" /></button>
                    <input bind:this={sigImg} type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden onchange={sigImages} />
                  </div>
                  <div class="sig-area">
                    <RichEditor
                      ctl={sigEd}
                      docKey="firma-{sigVersion}"
                      html={sigHtml}
                      font={store.font}
                      label="Testo della firma"
                      placeholder="Per esempio: nome, ruolo, azienda, telefono"
                      onchange={(h, t) => { sigHtml = t || /<img/i.test(h) ? h : ''; sigDirty = true; sigNote = ''; }}
                      onlink={() => (linkOpen = true)}
                      fetchImage={(u) => store.backend.fetchImage(u)}
                    />
                  </div>
                </div>
                {#if linkOpen && linkBtn}
                  <LinkDialog ctl={sigEd} anchor={linkBtn} onclose={() => (linkOpen = false)} />
                {/if}
                <div class="sig-acts">
                  <button class="primary-btn" onclick={saveSig} disabled={!sigHtml || (!sigDirty && !!sigSel)}><Icon name="device-floppy" size={17} />Salva</button>
                  {#if sigSel}
                    {#if sigConfirm}
                      <span class="sig-q">Eliminare “{sigName}”?</span>
                      <button class="sec-btn danger-btn" onclick={removeSig}>Elimina</button>
                      <button class="sec-btn" onclick={() => (sigConfirm = false)}>Annulla</button>
                    {:else}
                      <button class="sec-btn" onclick={() => (sigConfirm = true)}><Icon name="trash" size={16} />Elimina</button>
                    {/if}
                  {/if}
                  <span class="sig-note" role="status">{sigNote || (sigDirty && sigHtml ? 'Modifiche non ancora salvate' : '')}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="group-a">
            <h4 class="tight">Firma predefinita</h4>
            <p class="hint">Quale firma aggiungere da sola. Puoi sempre metterne un'altra dal pulsante Firma mentre scrivi.</p>
            {#if !senders.length}
              <p class="hint">Aggiungi un account di posta per scegliere la sua firma.</p>
            {:else}
              <div class="defs">
                <span class="dh">Account</span><span class="dh">Nuovi messaggi</span><span class="dh">Risposte e inoltri</span>
                {#each senders as a (a.id)}
                  <span class="da">{a.email}</span>
                  <select class="sig-sel" aria-label="Firma per i nuovi messaggi da {a.email}" value={a.signatureNew ?? ''} onchange={(e) => store.setAccountSignature(a.id, 'new', e.currentTarget.value || null)}>
                    <option value="">(nessuna)</option>
                    {#each store.signatures as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
                  </select>
                  <select class="sig-sel" aria-label="Firma per risposte e inoltri da {a.email}" value={a.signatureReply ?? ''} onchange={(e) => store.setAccountSignature(a.id, 'reply', e.currentTarget.value || null)}>
                    <option value="">(nessuna)</option>
                    {#each store.signatures as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
                  </select>
                {/each}
              </div>
            {/if}
          </div>

          <div class="group-a">
            <h4 class="tight">Carattere dei nuovi messaggi</h4>
            <p class="hint">Vale per i messaggi che scrivi, le risposte e gli inoltri.</p>
            <div class="font-row">
              <select class="sig-sel" aria-label="Tipo di carattere predefinito" value={store.font.family} onchange={(e) => store.setFont({ ...store.font, family: e.currentTarget.value })} style:width="180px">
                {#each FONTS as f (f.name)}<option value={f.name}>{f.name}</option>{/each}
              </select>
              <select class="sig-sel" aria-label="Dimensione predefinita" value={String(store.font.size)} onchange={(e) => store.setFont({ ...store.font, size: Number(e.currentTarget.value) })} style:width="72px">
                {#each SIZES as sz (sz)}<option value={String(sz)}>{sizeText(sz)}</option>{/each}
              </select>
            </div>
            <p class="preview" style:font-family={fontStack(store.font.family)} style:font-size="{store.font.size}pt">Gentile cliente, le invio il preventivo aggiornato.</p>
          </div>

          <div class="group-a">
            <h4 class="tight">Conferme di recapito e di lettura</h4>
            <p class="hint">Per sapere se un messaggio è arrivato e se è stato aperto. La conferma di lettura la manda chi riceve, solo se è d’accordo; quella di recapito arriva dal suo server, se la offre.</p>
            <p class="rc-h">Per tutti i messaggi che invii, richiedi:</p>
            <label class="check rc">
              <input type="checkbox" checked={store.trackDelivery} onchange={(e) => store.setTracking({ trackDelivery: e.currentTarget.checked })} />
              La conferma di recapito
            </label>
            <label class="check rc">
              <input type="checkbox" checked={store.trackRead} onchange={(e) => store.setTracking({ trackRead: e.currentTarget.checked })} />
              La conferma di lettura
            </label>
            <p class="hint sub">Per un solo messaggio usa la scheda Opzioni, mentre lo scrivi.</p>
            <p class="rc-h" id="set-rcpt">Quando qualcuno ti chiede una conferma di lettura:</p>
            <div role="radiogroup" aria-labelledby="set-rcpt">
              {#each RECEIPT_CHOICES as [v, label] (v)}
                <label class="check rc">
                  <input type="radio" name="set-rcpt" checked={store.receiptReply === v} onchange={() => store.setTracking({ receiptReply: v })} />
                  {label}
                </label>
              {/each}
            </div>
          </div>
        {:else if store.setPage === 'categorie'}
          <h3>Categorie</h3>
          <p class="intro">Le categorie colorano i messaggi per ritrovarli a colpo d’occhio. Assegnale con Categorizza o con il clic destro su un messaggio.</p>
          <ul class="cats" aria-label="Categorie">
            {#each catDraft as c, i (c.id || `nuova-${i}`)}
              <li class="cat-row">
                <button class="cat-sw" style:background="var(--cat-{c.color})" aria-label="Colore di {c.name}: {CATEGORY_COLORS.find(([k]) => k === c.color)?.[1]}" title="Cambia colore" onclick={(e) => (catColorFor = { index: i, el: e.currentTarget })}></button>
                <input
                  class="cat-name"
                  value={c.name}
                  aria-label="Nome della categoria"
                  use:focusNew={i === catDraft.length - 1}
                  oninput={(e) => (c.name = e.currentTarget.value)}
                  onchange={() => void saveCats()}
                  onkeydown={(e) => {
                    if (e.key === 'Enter') e.currentTarget.blur();
                  }}
                />
                <span class="cat" style:background="var(--cat-{c.color}-bg)" style:color="var(--cat-{c.color}-fg)" aria-hidden="true"><span class="cat-t">{c.name || 'Senza nome'}</span></span>
                <button class="ibtn btn" onclick={() => removeCat(i)} aria-label="Elimina {c.name}" title="Elimina"><Icon name="trash" size={17} /></button>
              </li>
            {/each}
          </ul>
          {#if catErr}<p class="cat-err" role="alert">{catErr}</p>{/if}
          <button class="btn add" style="margin-top:8px" onclick={addCat}><Icon name="plus" size={16} />Nuova categoria</button>
          {#if catColorFor}
            <Popover anchor={catColorFor.el} label="Colore della categoria" onclose={() => (catColorFor = null)}>
              <div class="cat-colors" role="radiogroup" aria-label="Colore">
                {#each CATEGORY_COLORS as [k, label] (k)}
                  {@const on = catDraft[catColorFor.index]?.color === k}
                  <button class="cat-pick" role="radio" aria-checked={on} style:background="var(--cat-{k})" aria-label={label} title={label} data-first={on ? '' : undefined} onclick={() => pickColor(k)}>
                  </button>
                {/each}
              </div>
            </Popover>
          {/if}
        {:else if store.setPage === 'modelli'}
          <TemplatesPage />
        {:else if store.setPage === 'regole'}
          <RulesPage />
        {:else if store.setPage === 'azioni'}
          <QuickStepsPage />
        {:else if store.setPage === 'risposte'}
          <AutoReplyPage />
        {:else if store.setPage === 'pulizia'}
          <CleanupPage />
        {:else if store.setPage === 'archiviazione'}
          <AutoArchivePage />
        {:else}
          <h3>Informazioni su Better-Look</h3>
          <div style="margin-top:30px"><About /></div>
        {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .dlg { width: min(1040px, 100%); height: min(720px, 100%); display: flex; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); overflow: hidden; outline: none; }
  .nav { width: 264px; flex: none; display: flex; flex-direction: column; gap: 2px; box-sizing: border-box; padding: 24px 12px; background: var(--rail); overflow-y: auto; }
  .nav h2 { margin: 0 0 14px 12px; font-size: 20px; font-weight: 600; line-height: 1.3; }
  .navit { height: 36px; flex: none; padding: 0 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .pane { position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .content { flex: 1; min-height: 0; overflow: auto; padding: 28px 72px 40px 40px; }
  /* Ferma in alto a destra, allineata al titolo della pagina, anche quando il contenuto scorre. */
  .close { position: absolute; top: 26px; right: 20px; z-index: 2; background: var(--sheet); }
  .close:hover { background: color-mix(in srgb, var(--ink) 6%, var(--sheet)); }
  .two:has(> :nth-child(3)) { grid-template-columns: repeat(3, minmax(0, 1fr)); max-width: 840px; }
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  h4.tight { margin-bottom: 4px; }
  .intro { margin: 6px 0 0; max-width: 60ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .custom { margin: 10px 0 0; font-size: 13px; color: var(--ink-2); }
  .group { margin-top: 28px; padding-top: 24px; box-shadow: inset 0 1px 0 var(--rule); }
  .otext { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .ot { font-size: 14.5px; font-weight: 600; }
  .od { font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .accent-row { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .swatches { display: flex; flex-wrap: wrap; gap: 8px; }
  .sw { position: relative; width: 34px; height: 34px; flex: none; display: grid; place-items: center; border-radius: 4px; cursor: pointer; box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 14%, transparent); transition: box-shadow .15s ease-out; }
  .sw:hover { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 14%, transparent), 0 0 0 2px var(--sheet), 0 0 0 3px var(--rule-2); }
  .sw[data-on="si"] { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 14%, transparent), 0 0 0 2px var(--sheet), 0 0 0 4px var(--ink); }
  .sw:focus-within { outline: 2px solid var(--focus); outline-offset: 6px; }
  .sw input { position: absolute; inset: 0; width: 100%; height: 100%; margin: 0; opacity: 0; cursor: pointer; }
  .sw.pick { margin-left: 8px; background: var(--sheet); color: var(--ink-2); box-shadow: inset 0 0 0 1px var(--rule-2); }
  .sw.pick:hover { box-shadow: inset 0 0 0 1px var(--rule-2), 0 0 0 2px var(--sheet), 0 0 0 3px var(--rule-2); }
  .sw.pick[data-on="si"] { box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 14%, transparent), 0 0 0 2px var(--sheet), 0 0 0 4px var(--ink); }
  .hex input { width: 96px; font-variant-numeric: tabular-nums; text-transform: uppercase; }
  .cats { margin: 22px 0 0; padding: 0; list-style: none; max-width: 560px; }
  .cat-row { display: grid; grid-template-columns: 28px minmax(0, 1fr) 150px 34px; gap: 12px; align-items: center; padding: 6px 0; }
  .cat-row + .cat-row { box-shadow: inset 0 1px 0 var(--rule); }
  .cat-sw { width: 24px; height: 24px; border: 0; border-radius: 4px; box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 18%, transparent); cursor: pointer; }
  .cat-name { height: 32px; min-width: 0; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font: inherit; font-size: 14px; }
  .cat-name:hover { box-shadow: inset 0 0 0 1px var(--rule-2); }
  .cat-name:focus { outline: none; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .cat-row .cat { justify-self: start; }
  .cat-err { margin: 8px 0 0; font-size: 13px; color: var(--danger); }
  .cat-colors { display: grid; grid-template-columns: repeat(5, 30px); gap: 8px; padding: 6px; }
  .cat-pick { width: 30px; height: 30px; border: 0; border-radius: 4px; display: grid; place-items: center; cursor: pointer; box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 18%, transparent); }
  .cat-pick[aria-checked="true"] { box-shadow: 0 0 0 2px var(--sheet), 0 0 0 4px var(--ink); }
  .check { margin-top: 14px; display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .check input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); }
  .acc-grid { margin-top: 24px; display: grid; grid-template-columns: 232px minmax(0, 1fr); gap: 28px; align-items: start; }
  .acc-list { display: flex; flex-direction: column; gap: 4px; }
  .acc { height: auto; flex-direction: column; align-items: flex-start; gap: 1px; padding: 10px 12px; }
  .acc .addr { max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; font-weight: 600; }
  .acc .kind { font-size: 12.5px; color: var(--ink-3); }
  .add { margin-top: 6px; height: 36px; display: flex; align-items: center; gap: 8px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 13.5px; }
  .srv { display: grid; grid-template-columns: auto minmax(0, 1fr); column-gap: 24px; row-gap: 8px; padding: 16px 18px; border-radius: 4px; background: var(--rail); font-size: 13.5px; }
  .srv .k { color: var(--ink-2); }
  .conn { display: flex; align-items: center; gap: 6px; }
  .hint { margin: 0 0 12px; max-width: 58ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .hint.sub { margin: 6px 0 0 28px; }
  .pills { display: flex; flex-wrap: wrap; gap: 8px; }
  .group-a { margin-top: 26px; }
  .rc-h { margin: 18px 0 0; font-size: 14px; font-weight: 600; }
  .check.rc { margin-top: 10px; }
  .state { display: flex; align-items: center; gap: 6px; }
  .state[data-s="pronto"] :global(svg), .state[data-s="sincronizzazione"] :global(svg) { color: var(--green); }
  .state[data-s="offline"] :global(svg), .state[data-s="errore"] :global(svg) { color: var(--warn); }
  .path { overflow-wrap: anywhere; }
  .pwrow { display: flex; align-items: flex-end; gap: 10px; flex-wrap: wrap; }
  .remove { padding-top: 22px; box-shadow: inset 0 1px 0 var(--rule); }
  .warn-t { margin: 0 0 12px; max-width: 58ch; font-size: 14px; line-height: 1.5; }
  .rm-acts { display: flex; gap: 8px; }
  .noacc { display: flex; flex-direction: column; align-items: flex-start; gap: 12px; font-size: 14px; color: var(--ink-2); }
  .noacc p { margin: 0; }
  .sig-grid { margin-top: 4px; display: grid; grid-template-columns: 200px minmax(0, 1fr); gap: 24px; align-items: start; }
  .sig-list { display: flex; flex-direction: column; gap: 4px; }
  .sig-it { height: 36px; gap: 8px; padding: 0 12px; font-size: 13.5px; }
  .sig-it :global(svg) { color: var(--brand-ink); flex: none; }
  .sig-empty { margin: 4px 12px 2px; font-size: 13.5px; color: var(--ink-2); }
  .sig-edit { min-width: 0; display: flex; flex-direction: column; gap: 14px; }
  .sig-box { border: 1px solid var(--rule-2); border-radius: 4px; overflow: hidden; }
  .sig-bar { display: flex; align-items: center; flex-wrap: wrap; gap: 2px; padding: 4px 6px; background: var(--rail); box-shadow: inset 0 -1px 0 var(--rule-2); }
  .sig-bar .rb { height: 30px; }
  .sig-bar .rb.ico { width: 30px; }
  .sig-sep { width: 1px; height: 20px; margin: 0 3px; background: var(--rule-2); }
  .sig-sel { height: 30px; box-sizing: border-box; padding: 0 6px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .sig-sel:focus { outline: none; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .sig-area { height: 210px; display: flex; flex-direction: column; background: #ffffff; }
  .lt span { font-size: 15px; line-height: 1; color: var(--ink); }
  .sig-color { position: relative; cursor: pointer; }
  .sig-color .bar { position: absolute; left: 7px; right: 7px; bottom: 4px; height: 3px; border-radius: 1px; box-shadow: 0 0 0 1px color-mix(in srgb, var(--ink) 18%, transparent); }
  .sig-color input { position: absolute; inset: 0; opacity: 0; width: 100%; height: 100%; cursor: pointer; }
  .sig-acts { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .sig-q { font-size: 13.5px; }
  .sig-note { flex: 1; min-width: 180px; font-size: 13px; color: var(--ink-2); }
  .defs { display: grid; grid-template-columns: minmax(0, 1fr) 190px 190px; column-gap: 12px; row-gap: 8px; align-items: center; max-width: 640px; }
  .dh { font-size: 12.5px; font-weight: 600; color: var(--ink-2); }
  .da { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .font-row { display: flex; gap: 8px; }
  .preview { margin: 14px 0 0; padding: 12px 14px; border-radius: 4px; background: #ffffff; color: #000000; box-shadow: inset 0 0 0 1px var(--rule-2); max-width: 520px; }
  .note { margin-top: 22px; display: flex; gap: 12px; padding: 12px 14px; border-radius: 4px; background: var(--warn-soft); font-size: 13.5px; line-height: 1.5; }
</style>
