<script lang="ts">
  // Elenco messaggi: schede Prioritaria/Altre, gruppi per data, azioni rapide
  // al passaggio del mouse, posta in uscita e avvisi (offline, invio, bozze).
  // Con la vista per conversazioni i messaggi della stessa discussione stanno
  // in una riga sola, che si apre con la freccia. Clic destro: menu del messaggio.
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import Toast from './Toast.svelte';
  import { store, folderLabel, DRAG_TYPE } from '../lib/store.svelte';
  import { categoryItems, messageItems, moveItems, type MenuItem } from '../lib/menus';
  import { GROUP_ORDER, groupLabel, initials, listTime, peopleText, personLabel, tint, whenText } from '../lib/format';
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
  /** Una riga dell'elenco: un messaggio, o l'ultimo messaggio di una conversazione. */
  interface Row {
    m: MessageSummary;
    /** Vista per conversazioni: chiave e numero di messaggi (anche di altre cartelle). */
    conv?: { key: string; count: number; unread: boolean };
  }

  const convMode = $derived(store.conversations && !q && store.view === 'cartella');

  const rows = $derived.by<Row[]>(() => {
    if (!convMode) return store.list.map((m) => ({ m }));
    const out: Row[] = [];
    const seen = new Map<string, Row>();
    for (const m of store.list) {
      const key = m.thread;
      const prev = key ? seen.get(key) : undefined;
      if (prev?.conv) {
        if (m.unread) prev.conv.unread = true;
        continue;
      }
      const row: Row = { m, conv: key ? { key, count: Math.max(m.threadCount, 1), unread: m.unread } : undefined };
      if (key) seen.set(key, row);
      out.push(row);
    }
    return out;
  });

  const groups = $derived.by(() => {
    const by = new Map<string, Row[]>();
    for (const r of rows) {
      const g = groupLabel(r.m.date, now);
      if (!by.has(g)) by.set(g, []);
      by.get(g)!.push(r);
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
  /** Risposta automatica attiva: prima quella dell'account aperto. */
  const away = $derived(store.autoReplyOn.find((a) => a.id === store.currentFolder?.accountId) ?? store.autoReplyOn[0]);
  const outState = (o: (typeof store.outbox)[number]) =>
    o.state === 'attesa' && o.sendAfter > Date.now() + 60_000 ? `Programmato per ${whenText(o.sendAfter)}` : (OUT_STATE[o.state] ?? o.state);
  const outbox = $derived(store.outbox.filter((o) => store.folderSel === `uscita-${o.accountId}` || store.folderSel === 'uscita'));

  function who(m: MessageSummary): string {
    if (showRecipients) return m.to.length ? peopleText(m.to) : 'Nessun destinatario';
    return personLabel(m.from) || 'Mittente sconosciuto';
  }

  function folderOf(m: MessageSummary): string {
    return folderLabel(store.folders.find((f) => f.id === m.folderId)) || m.folderName || '';
  }

  // ---------- Menu del clic destro ----------

  let ctx = $state<{ el: HTMLElement; x: number; y: number; items: MenuItem[]; label: string } | null>(null);

  function openMenu(e: MouseEvent, m: MessageSummary) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const at = { x: e.clientX, y: e.clientY };
    const sub = (kind: 'categorie' | 'sposta') => {
      // Il secondo menu si apre nello stesso punto, appena chiuso il primo.
      setTimeout(() => {
        ctx = kind === 'categorie' ? { el, ...at, items: categoryItems(m), label: 'Categorizza' } : { el, ...at, items: moveItems(m), label: 'Sposta in' };
      }, 0);
    };
    ctx = { el, ...at, items: messageItems(m, sub), label: 'Azioni sul messaggio' };
  }

  function menuKey(e: KeyboardEvent, m: MessageSummary) {
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      openMenu(new MouseEvent('contextmenu', { clientX: r.left + 60, clientY: r.top + 30 }), m);
      e.preventDefault();
    }
  }

  /** Trascinare un messaggio su una cartella lo sposta lì. */
  function dragStart(e: DragEvent, m: MessageSummary) {
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData(DRAG_TYPE, JSON.stringify([m.id]));
    e.dataTransfer.setData('text/plain', m.subject || '(nessun oggetto)');
  }

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (el.scrollTop + el.clientHeight > el.scrollHeight - 400) void store.loadMore();
  }
</script>

{#snippet cats(m: MessageSummary)}
  {#each m.categories as id (id)}
    {@const c = store.category(id)}
    {#if c}<span class="cat" style:background="var(--cat-{c.color}-bg)" style:color="var(--cat-{c.color}-fg)"><span class="cat-t">{c.name}</span></span>{/if}
  {/each}
{/snippet}

{#snippet msgRow(m: MessageSummary, conv: Row['conv'])}
  {@const on = m.id === store.sel}
  {@const name = who(m)}
  {@const t = tint(name)}
  {@const unread = conv ? conv.unread : m.unread}
  {@const strong = unread && !on}
  {@const open = !!(conv && store.threadOpen[conv.key])}
  {@const many = !!conv && conv.count > 1}
  <div class="row" class:conv={convMode} data-sel={on ? 'si' : 'no'}>
    <button
      class="row-main"
      draggable="true"
      ondragstart={(e) => dragStart(e, m)}
      oncontextmenu={(e) => openMenu(e, m)}
      onkeydown={(e) => menuKey(e, m)}
      onclick={() => store.pickMsg(m.id)}
      aria-current={on ? 'true' : undefined}
      aria-label={`${unread ? 'Non letto. ' : ''}${m.draft ? 'Bozza. ' : ''}${many ? `Conversazione di ${conv!.count} messaggi. ` : ''}${name}, ${m.subject || 'nessun oggetto'}, ${listTime(m.date, now)}`}
    >
      <span class="bar" aria-hidden="true" style:background={unread ? 'var(--brand)' : 'transparent'}></span>
      <span class="avatar" aria-hidden="true" style="width:36px;height:36px;font-size:13px;background:{t.bg};color:{t.fg}">{initials(name)}</span>
      <span class="txt">
        <span class="l1">
          <span class="from" style:font-weight={unread ? 600 : 400}>{name}</span>
          {#if m.answered}<Icon name="arrow-back-up" size={14} stroke={1.8} style="color:var(--ink-3)" />{/if}
          {#if many}<span class="count" title="{conv!.count} messaggi nella conversazione">{conv!.count}</span>{/if}
          <span class="time" style:font-weight={unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink-3)'}>{listTime(m.date, now)}</span>
        </span>
        <span class="l2">
          <span class="subj" style:font-weight={unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink)'}>{m.subject || '(nessun oggetto)'}</span>
          {#if m.hasAttachments}<Icon name="paperclip" size={15} stroke={1.7} style="color:var(--ink-3)" />{/if}
          {#if m.flagged}<Icon name="flag-filled" size={15} style="color:var(--danger)" />{/if}
        </span>
        <span class="l3">
          {#if m.categories.length}{@render cats(m)}{/if}
          <span class="prev">
            {#if q}<span class="in">{folderOf(m)}</span>{/if}
            {#if m.draft}<span class="draft">[Bozza] </span>{/if}{m.preview}
          </span>
        </span>
      </span>
    </button>
    {#if many}
      <button class="ibtn btn tog" aria-expanded={open} onclick={() => store.toggleThread(m)} aria-label={open ? 'Chiudi la conversazione' : `Mostra i ${conv!.count} messaggi della conversazione`} title={open ? 'Chiudi la conversazione' : 'Mostra la conversazione'}>
        <Icon name={open ? 'chevron-down' : 'chevron-right'} size={16} stroke={1.8} />
      </button>
    {/if}
    {#if !m.draft || role !== 'drafts'}
      <div class="row-acts">
        <button class="ibtn btn" onclick={() => store.deleteMsg(m.id)} aria-label="Elimina" title="Elimina"><Icon name="trash" size={16} /></button>
        <button class="ibtn btn" style="color:var(--danger)" onclick={() => store.toggleFlag(m.id)} aria-label={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'} title={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'}><Icon name="flag" size={16} /></button>
        <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.toggleRead(m.id)} aria-label={m.unread ? 'Segna come letto' : 'Segna come non letto'} title={m.unread ? 'Segna come letto' : 'Segna come non letto'}><Icon name="mail-opened" size={16} /></button>
      </div>
    {/if}
  </div>
{/snippet}

{#snippet childRow(k: MessageSummary)}
  {@const on = k.id === store.sel}
  {@const elsewhere = k.folderId !== store.folderId}
  <div class="row kid" data-sel={on ? 'si' : 'no'}>
    <button
      class="row-main"
      draggable={!elsewhere}
      ondragstart={(e) => dragStart(e, k)}
      oncontextmenu={(e) => openMenu(e, k)}
      onkeydown={(e) => menuKey(e, k)}
      onclick={() => store.pickMsg(k.id)}
      aria-current={on ? 'true' : undefined}
      aria-label={`${k.unread ? 'Non letto. ' : ''}${personLabel(k.from)}, ${listTime(k.date, now)}${elsewhere ? `, in ${folderOf(k)}` : ''}`}
    >
      <span class="bar" aria-hidden="true" style:background={k.unread ? 'var(--brand)' : 'transparent'}></span>
      <span class="txt">
        <span class="l1">
          <span class="from" style:font-weight={k.unread ? 600 : 400}>{personLabel(k.from) || 'Mittente sconosciuto'}</span>
          {#if k.hasAttachments}<Icon name="paperclip" size={14} stroke={1.7} style="color:var(--ink-3)" />{/if}
          <span class="time" style:color="var(--ink-3)">{listTime(k.date, now)}</span>
        </span>
        <span class="l3">
          {#if elsewhere}<span class="in">{folderOf(k)}</span>{/if}
          <span class="prev">{k.preview || k.subject}</span>
        </span>
      </span>
    </button>
  </div>
{/snippet}

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
  {:else if away}
    <div role="status" class="note-bar info">
      <Icon name="message-reply" style="color:var(--brand-ink)" />
      <span class="t">Risposte automatiche attive{store.accounts.length > 1 ? ` per ${away.email}` : ''}.</span>
      <button class="act" onclick={() => { store.acctSel = away.id; store.openSettings('risposte'); }}>Modifica</button>
      <button class="act" onclick={() => store.turnOffAutoReply(away.id)}>Disattiva</button>
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
              <span class="prev" class:err={o.state === 'errore'}>{outState(o)}{o.error ? `: ${o.error}` : ''}</span>
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
        {#each g.items as r (r.m.id)}
          {@render msgRow(r.m, r.conv)}
          {#if r.conv && store.threadOpen[r.conv.key]}
            {@const items = store.threadOpen[r.conv.key]}
            {#if !items.length}
              <div class="kid-wait" aria-hidden="true"><span class="skel" style="width:60%;height:10px"></span></div>
            {/if}
            {#each items as k (k.id)}
              {@render childRow(k)}
            {/each}
          {/if}
        {/each}
      {/each}
      {#if !store.listEnd && store.list.length}
        <div class="more" aria-hidden="true"><span class="skel" style="width:60%;height:10px"></span></div>
      {/if}
    {/if}
  </div>

  {#if ctx}
    <Menu anchor={ctx.el} at={{ x: ctx.x, y: ctx.y }} items={ctx.items} label={ctx.label} onclose={() => (ctx = null)} />
  {/if}

  <Toast />
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
  .l3 { display: flex; align-items: center; gap: 5px; min-width: 0; }
  .prev { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--ink-3); }
  .l3 .cat { height: 18px; font-size: 11.5px; max-width: 120px; }
  .count { flex: none; min-width: 18px; height: 18px; box-sizing: border-box; padding: 0 5px; border-radius: 9px; background: var(--rail); color: var(--ink-2); font-size: 11.5px; font-weight: 600; line-height: 18px; text-align: center; font-variant-numeric: tabular-nums; }
  .row.conv :global(.row-main) { padding-left: 32px; }
  .tog { position: absolute; left: 6px; top: 18px; width: 24px; height: 24px; color: var(--ink-2); }
  .row.kid :global(.row-main) { grid-template-columns: minmax(0, 1fr); padding: 6px 12px 7px 62px; }
  .row.kid .bar { left: 50px; top: 8px; bottom: 8px; }
  .row.kid .from { font-size: 13.5px; }
  .kid-wait { padding: 8px 20px 8px 62px; }
  .prev.err { color: var(--danger); white-space: normal; }
  .in { flex: none; margin-right: 6px; padding: 1px 6px; border-radius: 3px; background: var(--rail); color: var(--ink-2); font-size: 12px; }
  .draft { color: var(--danger); font-weight: 600; }
  .out { cursor: default; }
  .open { align-self: flex-start; margin-top: 6px; padding: 2px 0; font-size: 13.5px; font-weight: 600; }
  .ghost { display: grid; grid-template-columns: 36px minmax(0, 1fr); gap: 12px; padding: 12px 18px 12px 20px; }
  .lines { display: flex; flex-direction: column; gap: 7px; padding-top: 2px; }
  .more { padding: 14px 20px; }
  @media (max-width: 760px) { .pane-list { width: auto; flex: 1 1 auto; margin-right: 10px; } }
</style>
