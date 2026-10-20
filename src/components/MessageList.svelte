<script lang="ts">
  // Elenco messaggi: schede Prioritaria/Altre, gruppi per data, azioni rapide
  // al passaggio del mouse, posta in uscita e avvisi (offline, invio, bozze).
  import Icon from '../lib/Icon.svelte';
  import { store, folderLabel } from '../lib/store.svelte';
  import { GROUP_ORDER, groupLabel, initials, listTime, peopleText, personLabel, tint } from '../lib/format';
  import type { MessageSummary, Role } from '../lib/types';

  const EMPTY: Partial<Record<Role, [string, string]>> = {
    inbox: ['Niente di nuovo qui', 'Non ci sono messaggi in questa scheda.'],
    sent: ['Nessun messaggio inviato', 'I messaggi che invii compaiono qui.'],
    drafts: ['Nessuna bozza', 'I messaggi che inizi e non invii restano qui.'],
    archive: ['L’archivio è vuoto', 'Archivia un messaggio per toglierlo dalla posta in arrivo senza eliminarlo.'],
    junk: ['Nessun messaggio indesiderato', 'Qui arrivano i messaggi che segnali come indesiderati.'],
    deleted: ['Nessun elemento eliminato', 'I messaggi eliminati restano qui finché non li elimini anche da questa cartella.'],
  };

  const OUT_STATE: Record<string, string> = {
    attesa: 'In partenza',
    invio: 'Invio in corso',
    offline: 'In attesa della connessione',
    errore: 'Non inviato',
  };

  const q = $derived(store.q.trim());
  const role = $derived(store.currentFolder?.role ?? null);
  const showRecipients = $derived(!q && (role === 'sent' || role === 'drafts'));
  const now = $derived(new Date(store.list.length ? Date.now() : 0));
  const groups = $derived.by(() => {
    const by = new Map<string, MessageSummary[]>();
    for (const m of store.list) {
      const g = groupLabel(m.date, now);
      if (!by.has(g)) by.set(g, []);
      by.get(g)!.push(m);
    }
    return GROUP_ORDER.filter((g) => by.has(g)).map((g) => ({ label: g, items: by.get(g)! }));
  });
  const empty = $derived<[string, string]>(
    q
      ? ['Nessun risultato', `Nessun messaggio contiene “${q}”. Prova con un nome o con una parola dell’oggetto.`]
      : !store.accounts.length
        ? ['Nessun account', 'Aggiungi il tuo account di posta per vedere qui i messaggi.']
        : (role && EMPTY[role]) || ['Cartella vuota', 'Usa Sposta in per mettere qui i messaggi.'],
  );
  const problem = $derived(store.accounts.find((a) => a.status.state === 'errore'));
  const outbox = $derived(store.outbox.filter((o) => store.folderSel === `uscita-${o.accountId}` || store.folderSel === 'uscita'));

  function who(m: MessageSummary): string {
    if (showRecipients) return m.to.length ? peopleText(m.to) : 'Nessun destinatario';
    return personLabel(m.from) || 'Mittente sconosciuto';
  }

  function folderOf(m: MessageSummary): string {
    return folderLabel(store.folders.find((f) => f.id === m.folderId)) || m.folderName || '';
  }

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) void store.loadMore();
  }
</script>

<section class="pane-list" aria-label="Elenco messaggi" aria-busy={store.listLoading}>
  {#if store.offline}
    <div role="status" class="note-bar warn">
      <Icon name="wifi-off" style="color:var(--warn)" />
      <span class="t">Sei offline. I messaggi in uscita partono appena torna la connessione.</span>
      <button class="act" onclick={() => store.syncNow()}>Riprova</button>
    </div>
  {:else if problem}
    <div role="status" class="note-bar warn">
      <Icon name="alert-triangle" style="color:var(--warn)" />
      <span class="t">{problem.email}: {problem.status.message}</span>
      <button class="act" onclick={() => { store.acctSel = problem.id; store.openSettings('account'); }}>Controlla</button>
    </div>
  {/if}

  <div class="head">
    {#if !store.showFolders}
      <button class="ibtn btn" style="margin-left:-6px" onclick={() => store.toggleFolders()} aria-label="Mostra riquadro cartelle" title="Mostra riquadro cartelle"><Icon name="menu-2" size={20} /></button>
    {/if}
    <h1>{store.listTitle}</h1>
    {#if store.view === 'cartella'}
      <button class="ibtn btn" aria-label="Aggiorna" title="Invia e ricevi (F9)" onclick={() => store.syncNow()}>
        <Icon name="refresh" class={store.syncing ? 'spin' : ''} />
      </button>
    {/if}
  </div>

  {#if store.view === 'cartella' && store.isInbox && !q}
    <div class="tabs" role="tablist" aria-label="Schede della posta in arrivo">
      {#each [['prio', 'Prioritaria'], ['altre', 'Altre']] as [id, label] (id)}
        {@const on = store.tab === id}
        <button role="tab" aria-selected={on} class:on onclick={() => store.pickTab(id as 'prio' | 'altre')}>{label}</button>
      {/each}
    </div>
  {/if}

  <div class="scroll body" {onscroll}>
    {#if store.view === 'uscita'}
      {#if !outbox.length}
        <div class="empty">
          <Icon name="mail-up" size={56} stroke={1.2} style="color:var(--brand-ink)" />
          <p class="t">Posta in uscita vuota</p>
          <p class="d">Tutti i messaggi sono partiti.</p>
        </div>
      {/if}
      {#each outbox as o (o.id)}
        <div class="row" data-sel="no">
          <div class="row-main out">
            <span class="avatar" aria-hidden="true" style="width:36px;height:36px;background:var(--brand-soft);color:var(--brand-ink)"><Icon name={o.state === 'errore' ? 'mail-exclamation' : 'mail-up'} size={18} /></span>
            <span class="txt">
              <span class="l1"><span class="from">{o.to.length ? peopleText(o.to) : 'Nessun destinatario'}</span></span>
              <span class="l2"><span class="subj">{o.subject || '(nessun oggetto)'}</span></span>
              <span class="prev" class:err={o.state === 'errore'}>{OUT_STATE[o.state] ?? o.state}{o.error ? `: ${o.error}` : ''}</span>
              {#if o.state !== 'invio'}
                <button class="lbtn open" onclick={() => store.openOutboxItem(o.id)}>Apri e modifica</button>
              {/if}
            </span>
          </div>
        </div>
      {/each}
    {:else}
      {#if store.listLoading && !store.list.length}
        {#each [0, 1, 2, 3, 4, 5] as i (i)}
          <div class="ghost" aria-hidden="true">
            <span class="skel" style="width:36px;height:36px;border-radius:50%"></span>
            <span class="lines"><span class="skel" style="width:46%;height:12px"></span><span class="skel" style="width:78%;height:11px"></span><span class="skel" style="width:92%;height:10px"></span></span>
          </div>
        {/each}
      {:else if !store.list.length}
        <div class="empty">
          <Icon name="inbox" size={56} stroke={1.2} style="color:var(--brand-ink)" />
          <p class="t">{empty[0]}</p>
          <p class="d">{empty[1]}</p>
          {#if !store.accounts.length && store.ready}
            <button class="primary-btn" style="margin-top:18px" onclick={() => store.openWizard(false)}><Icon name="user-plus" size={17} />Aggiungi account</button>
          {/if}
        </div>
      {/if}

      {#each groups as g (g.label)}
        <h2 class="group">{g.label}</h2>
        {#each g.items as m (m.id)}
          {@const on = m.id === store.sel}
          {@const name = who(m)}
          {@const t = tint(name)}
          {@const strong = m.unread && !on}
          <div class="row" data-sel={on ? 'si' : 'no'}>
            <button
              class="row-main"
              onclick={() => store.pickMsg(m.id)}
              aria-current={on ? 'true' : undefined}
              aria-label={`${m.unread ? 'Non letto. ' : ''}${m.draft ? 'Bozza. ' : ''}${name}, ${m.subject || 'nessun oggetto'}, ${listTime(m.date, now)}`}
            >
              <span class="bar" aria-hidden="true" style:background={m.unread ? 'var(--brand)' : 'transparent'}></span>
              <span class="avatar" aria-hidden="true" style="width:36px;height:36px;font-size:13px;background:{t.bg};color:{t.fg}">{initials(name)}</span>
              <span class="txt">
                <span class="l1">
                  <span class="from" style:font-weight={m.unread ? 600 : 400}>{name}</span>
                  {#if m.answered}<Icon name="arrow-back-up" size={14} stroke={1.8} style="color:var(--ink-3)" />{/if}
                  <span class="time" style:font-weight={m.unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink-3)'}>{listTime(m.date, now)}</span>
                </span>
                <span class="l2">
                  <span class="subj" style:font-weight={m.unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink)'}>{m.subject || '(nessun oggetto)'}</span>
                  {#if m.hasAttachments}<Icon name="paperclip" size={15} stroke={1.7} style="color:var(--ink-3)" />{/if}
                  {#if m.flagged}<Icon name="flag-filled" size={15} style="color:var(--danger)" />{/if}
                </span>
                <span class="prev">
                  {#if q}<span class="in">{folderOf(m)}</span>{/if}
                  {#if m.draft}<span class="draft">[Bozza] </span>{/if}{m.preview}
                </span>
              </span>
            </button>
            {#if !m.draft || role !== 'drafts'}
              <div class="row-acts">
                <button class="ibtn btn" onclick={() => store.deleteMsg(m.id)} aria-label="Elimina" title="Elimina"><Icon name="trash" size={16} /></button>
                <button class="ibtn btn" style="color:var(--danger)" onclick={() => store.toggleFlag(m.id)} aria-label={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'} title={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'}><Icon name="flag" size={16} /></button>
                <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.toggleRead(m.id)} aria-label={m.unread ? 'Segna come letto' : 'Segna come non letto'} title={m.unread ? 'Segna come letto' : 'Segna come non letto'}><Icon name="mail-opened" size={16} /></button>
              </div>
            {/if}
          </div>
        {/each}
      {/each}
      {#if !store.listEnd && store.list.length}
        <div class="more" aria-hidden="true"><span class="skel" style="width:60%;height:10px"></span></div>
      {/if}
    {/if}
  </div>

  {#if store.toast}
    {@const t = store.toast}
    <div role={t.kind === 'error' ? 'alert' : 'status'} class="toast rise" data-kind={t.kind}>
      <Icon name={t.kind === 'sent' ? 'send' : t.kind === 'draft' ? 'file-pencil' : t.kind === 'error' ? 'alert-circle' : 'circle-check'} />
      <span class="msg">{t.text}</span>
      {#if t.kind === 'sent' && t.outboxId}
        <button class="undo" onclick={() => store.undoSend()}>Annulla</button>
      {:else if t.kind === 'error' && t.outboxId}
        <button class="undo" onclick={() => store.openOutboxItem(t.outboxId!)}>Apri</button>
      {/if}
      <button class="close" onclick={() => store.dismissToast()} aria-label="Chiudi avviso"><Icon name="x" size={16} /></button>
    </div>
  {/if}
</section>

<style>
  .pane-list { position: relative; width: 420px; flex: none; min-height: 0; display: flex; flex-direction: column; margin: 0 0 10px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .head { display: flex; align-items: center; gap: 6px; padding: 10px 10px 0 16px; }
  h1 { margin: 0; flex: 1; min-width: 0; font-size: 18px; font-weight: 600; line-height: 34px; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .tabs { display: flex; gap: 18px; padding: 2px 16px 0; box-shadow: inset 0 -1px 0 var(--rule); }
  .tabs button { height: 34px; padding: 0 2px; border: 0; background: transparent; color: var(--ink-2); font-size: 14px; }
  .tabs button.on { color: var(--ink); font-weight: 600; box-shadow: inset 0 -3px 0 var(--brand); }
  .body { flex: 1; min-height: 0; overflow: auto; padding: 2px 0 28px; }
  .empty { padding: 72px 36px 0; display: flex; flex-direction: column; align-items: center; text-align: center; }
  .empty .t { margin: 14px 0 4px; font-size: 15px; font-weight: 600; }
  .empty .d { margin: 0; max-width: 300px; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .group { margin: 12px 16px 4px; font-size: 12.5px; font-weight: 600; color: var(--ink-2); }
  .bar { position: absolute; left: 4px; top: 10px; bottom: 10px; width: 3px; border-radius: 2px; }
  .txt { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .l1 { display: flex; align-items: center; gap: 8px; }
  .from { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 14px; }
  .time { flex: none; font-size: 12.5px; font-variant-numeric: tabular-nums; }
  .l2 { display: flex; align-items: center; gap: 6px; }
  .subj { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .prev { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--ink-3); }
  .prev.err { color: var(--danger); white-space: normal; }
  .in { margin-right: 6px; padding: 1px 6px; border-radius: 3px; background: var(--rail); color: var(--ink-2); font-size: 12px; }
  .draft { color: var(--danger); font-weight: 600; }
  .out { cursor: default; }
  .open { align-self: flex-start; margin-top: 6px; padding: 2px 0; font-size: 13.5px; font-weight: 600; }
  .ghost { display: grid; grid-template-columns: 36px minmax(0, 1fr); gap: 12px; padding: 12px 18px 12px 20px; }
  .lines { display: flex; flex-direction: column; gap: 7px; padding-top: 2px; }
  .more { padding: 14px 20px; }
  .toast { position: absolute; left: 14px; right: 14px; bottom: 14px; z-index: 5; display: flex; align-items: center; gap: 12px; min-height: 44px; box-sizing: border-box; padding: 6px 6px 6px 14px; border-radius: 4px; background: var(--ink); color: var(--sheet); box-shadow: var(--lift); font-size: 14px; }
  .toast[data-kind="error"] { background: #A4262C; color: #fff; }
  .toast .msg { flex: 1; min-width: 0; }
  .undo { height: 32px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: inherit; font-size: 14px; font-weight: 600; text-decoration: underline; text-underline-offset: 3px; }
  .close { width: 30px; height: 30px; flex: none; border: 0; border-radius: 4px; background: transparent; color: inherit; display: grid; place-items: center; opacity: .8; }
  .close:hover { opacity: 1; }
  .undo:focus-visible, .close:focus-visible { outline-color: currentColor; }
  @media (max-width: 760px) { .pane-list { width: auto; flex: 1 1 auto; margin-right: 10px; } }
</style>
