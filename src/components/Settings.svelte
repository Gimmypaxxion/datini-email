<script lang="ts">
  // Impostazioni: Aspetto, Account e sincronizzazione, Informazioni su Datini.
  import Icon from '../lib/Icon.svelte';
  import About from './About.svelte';
  import { store, type Prefs, type ThemeChoice } from '../lib/store.svelte';
  import type { ServerConfig, SyncOptions } from '../lib/types';

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
    { id: 'set-theme', legend: 'Tema', opts: [
      themeOpt('sistema', 'Come il sistema', 'Chiaro o scuro, seguendo l’impostazione del computer.'),
      themeOpt('chiaro', 'Chiaro', 'Fondo chiaro, adatto alla luce del giorno.'),
      themeOpt('scuro', 'Scuro', 'Fondo scuro, più riposante la sera.'),
    ] },
  ]);

  const nav = [
    ['aspetto', 'Aspetto'],
    ['account', 'Account e sincronizzazione'],
    ['info', 'Informazioni su Datini'],
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
      <div class="scroll content">
        <button class="ibtn btn close" onclick={() => (store.settings = false)} aria-label="Chiudi impostazioni" title="Chiudi (Esc)"><Icon name="x" /></button>

        {#if store.setPage === 'aspetto'}
          <h3>Aspetto</h3>
          <p class="intro">Scegli dove trovare i comandi. Le modifiche valgono subito.</p>
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
        {:else if store.setPage === 'account'}
          <h3>Account e sincronizzazione</h3>
          <p class="intro">La posta viaggia direttamente tra questo computer e il server del tuo provider. Datini non usa server intermedi.</p>
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
                    <span class="k">Dove si trova</span><span>Solo su questo computer, nell’archivio di Datini</span>
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
                    <div role="note" class="note"><Icon name="alert-triangle" style="color:var(--warn);margin-top:1px" /><span>Con POP la posta viene scaricata su questo computer, che ne conserva l’unica copia completa. Includi la cartella di Datini nei backup.</span></div>
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
                        ? 'Il file di dati verrà tolto da Datini. Il file originale sul disco resta dov’è.'
                        : acct.kind === 'pop3'
                          ? 'L’account e la posta scaricata su questo computer verranno tolti da Datini. I messaggi ancora sul server restano lì.'
                          : 'L’account verrà tolto da Datini. La posta resta sul server del tuo provider.'}
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
        {:else}
          <h3>Informazioni su Datini</h3>
          <div style="margin-top:30px"><About /></div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .dlg { width: min(1000px, 100%); height: min(720px, 100%); display: flex; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); overflow: hidden; outline: none; }
  .nav { width: 240px; flex: none; display: flex; flex-direction: column; gap: 2px; box-sizing: border-box; padding: 22px 12px; background: var(--rail); }
  .nav h2 { margin: 0 0 14px 12px; font-size: 20px; font-weight: 600; }
  .navit { height: 36px; padding: 0 12px; }
  .content { position: relative; flex: 1; min-width: 0; overflow: auto; padding: 28px 40px 40px; }
  .close { position: absolute; top: 16px; right: 16px; }
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  h4.tight { margin-bottom: 4px; }
  .intro { margin: 6px 0 0; max-width: 60ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .custom { margin: 10px 0 0; font-size: 13px; color: var(--ink-2); }
  .group { margin-top: 28px; padding-top: 24px; box-shadow: inset 0 1px 0 var(--rule); }
  .otext { min-width: 0; display: flex; flex-direction: column; gap: 3px; }
  .ot { font-size: 14.5px; font-weight: 600; }
  .od { font-size: 13px; line-height: 1.45; color: var(--ink-2); }
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
  .pills { display: flex; flex-wrap: wrap; gap: 8px; }
  .group-a { margin-top: 26px; }
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
  .note { margin-top: 22px; display: flex; gap: 12px; padding: 12px 14px; border-radius: 4px; background: var(--warn-soft); font-size: 13.5px; line-height: 1.5; }
</style>
