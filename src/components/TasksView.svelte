<script lang="ts">
  // Modulo Attività (Ctrl+4): a sinistra gli elenchi (La mia giornata,
  // Importanti, Pianificate, Attività, Messaggi contrassegnati e quelli
  // dell'utente), al centro le attività, a destra quella aperta.
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import Toast from './Toast.svelte';
  import TaskDetail from './TaskDetail.svelte';
  import { store, folderLabel } from '../lib/store.svelte';
  import { dayKey, dueInfo, plannedGroups, reminderWord, repeatLabel, stepsText, type TaskView } from '../lib/tasks';
  import { listTime, personLabel } from '../lib/format';
  import type { MenuItem } from '../lib/menus';
  import type { IconName } from '../lib/icons';
  import type { MessageSummary, Task, TaskList } from '../lib/types';

  const tk = store.tasks;

  const TONE: Record<string, string> = { giornata: 'c-amber', importanti: 'c-red', pianificate: 'c-green', attivita: 'c-brand', contrassegnati: 'c-red' };
  const EMPTY: Record<string, { icon: IconName; t: string; d: string }> = {
    giornata: { icon: 'sun', t: 'Concentrati sulla tua giornata', d: 'Scrivi qui sopra le cose da fare oggi, oppure aggiungi un’attività già pianificata con La mia giornata.' },
    importanti: { icon: 'star', t: 'Nessuna attività importante', d: 'Tocca la stella accanto a un’attività per ritrovarla qui.' },
    pianificate: { icon: 'calendar-due', t: 'Niente in programma', d: 'Le attività con una scadenza o un promemoria compaiono qui, in ordine di data.' },
    attivita: { icon: 'checklist', t: 'Nessuna attività', d: 'Scrivi qui sopra la prima cosa da fare e premi Invio.' },
    elenco: { icon: 'list', t: 'Elenco vuoto', d: 'Scrivi qui sopra la prima attività di questo elenco e premi Invio.' },
  };

  let draft = $state('');
  let addEl: HTMLInputElement | undefined = $state();
  let listEl: HTMLInputElement | undefined = $state();
  let menu = $state<{ el: HTMLElement; x?: number; y?: number; items: MenuItem[]; label: string } | null>(null);

  const shown = $derived(tk.shown);
  const flagged = $derived(tk.shownFlagged);
  const smartIcon = $derived(tk.isSmart ? (tk.smart.find((s) => s.id === tk.view)?.icon ?? 'list') : 'list');
  const groups = $derived(tk.view === 'pianificate' && !tk.query.trim() ? plannedGroups(shown.open, tk.now) : [{ label: '', tasks: shown.open }]);
  const empty = $derived(EMPTY[tk.isSmart ? tk.view : 'elenco']);

  $effect(() => {
    if (tk.addFocus) queueMicrotask(() => addEl?.focus());
  });
  $effect(() => {
    if (tk.listEdit) queueMicrotask(() => listEl?.focus());
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (await tk.add(draft)) draft = '';
  }

  function listMenu(e: MouseEvent, l: TaskList, at = false) {
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    menu = {
      el,
      ...(at ? { x: e.clientX, y: e.clientY } : {}),
      label: `Elenco ${l.name}`,
      items: [
        { label: 'Rinomina elenco', icon: 'pencil', onclick: () => tk.startRenameList(l) },
        { label: 'Elimina elenco', icon: 'trash', onclick: () => tk.deleteList(l) },
      ],
    };
  }

  function taskMenu(e: MouseEvent, t: Task) {
    e.preventDefault();
    const today = dayKey(Date.now());
    menu = {
      el: e.currentTarget as HTMLElement,
      x: e.clientX,
      y: e.clientY,
      label: 'Azioni sull’attività',
      items: [
        { label: t.done ? 'Segna come da fare' : 'Segna come completata', icon: 'circle-check', onclick: () => tk.toggleDone(t) },
        { label: t.important ? 'Togli importanza' : 'Segna come importante', icon: 'star', onclick: () => tk.toggleImportant(t) },
        { label: t.myDay === today ? 'Togli da La mia giornata' : 'Aggiungi a La mia giornata', icon: 'sun', onclick: () => tk.toggleMyDay(t) },
        { kind: 'sep', label: '' },
        { label: 'Scade oggi', icon: 'calendar-due', onclick: () => tk.setDue(t, Date.now()) },
        { label: 'Scade domani', icon: 'calendar-due', onclick: () => tk.setDue(t, Date.now() + 86_400_000) },
        ...(t.due !== null ? [{ label: 'Togli la scadenza', icon: 'calendar-off' as IconName, onclick: () => tk.setDue(t, null) }] : []),
        { kind: 'sep', label: '' },
        ...[{ id: '', name: 'Attività' }, ...tk.lists].filter((l) => l.id !== t.list).map((l) => ({ label: `Sposta in ${l.name}`, icon: 'list' as IconName, onclick: () => tk.moveTo(t, l.id) })),
        { kind: 'sep', label: '' },
        { label: 'Elimina attività', icon: 'trash', onclick: () => tk.remove(t) },
      ],
    };
  }

  function flagMenu(e: MouseEvent, m: MessageSummary) {
    e.preventDefault();
    menu = {
      el: e.currentTarget as HTMLElement,
      x: e.clientX,
      y: e.clientY,
      label: 'Azioni sul messaggio',
      items: [
        { label: 'Apri il messaggio', icon: 'mail-opened', onclick: () => store.revealMessage(m) },
        { label: 'Segna come completato', icon: 'circle-check', onclick: () => tk.unflag(m) },
        { label: 'Aggiungi a La mia giornata', icon: 'sun', onclick: () => tk.fromMessage(m) },
      ],
    };
  }

  function rowKey(e: KeyboardEvent, t: Task) {
    if (e.key === 'Delete') {
      e.preventDefault();
      void tk.remove(t);
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
      taskMenu(new MouseEvent('contextmenu', { clientX: r.left + 40, clientY: r.top + 24 }), t);
    }
  }

  const listCount = (id: TaskView) => tk.count(id);
  const folderOf = (m: MessageSummary) => {
    const f = store.folders.find((x) => x.id === m.folderId);
    return f ? folderLabel(f) : (m.folderName ?? '');
  };
</script>

{#snippet navItem(id: TaskView, label: string, icon: IconName, tone: string, l: TaskList | null)}
  {@const on = tk.view === id}
  {@const n = listCount(id)}
  <button
    class="btn fold nav-it"
    data-sel={on ? 'si' : 'no'}
    aria-current={on ? 'page' : undefined}
    onclick={() => tk.setView(id)}
    oncontextmenu={(e) => l && listMenu(e, l, true)}
    onkeydown={(e) => {
      if (l && e.key === 'F2') {
        e.preventDefault();
        tk.startRenameList(l);
      }
    }}
  >
    <Icon name={icon} class={tone} />
    <span class="name" style:font-weight={on ? 600 : 400}>{label}</span>
    {#if n}<span class="cnt">{n}</span>{/if}
  </button>
{/snippet}

{#snippet listInput()}
  {@const e = tk.listEdit!}
  <div class="edit">
    <Icon name="list" />
    <input
      bind:this={listEl}
      value={e.name}
      aria-label={e.id ? 'Nuovo nome dell’elenco' : 'Nome del nuovo elenco'}
      aria-invalid={e.error ? 'true' : undefined}
      placeholder="Nome dell’elenco"
      maxlength="80"
      oninput={(ev) => {
        e.name = ev.currentTarget.value;
        e.error = '';
      }}
      onkeydown={(ev) => {
        if (ev.key === 'Enter') {
          ev.preventDefault();
          void tk.commitList();
        } else if (ev.key === 'Escape') {
          ev.preventDefault();
          ev.stopPropagation();
          tk.listEdit = null;
        }
      }}
      onblur={() => {
        if (tk.listEdit && !tk.listEdit.error) void tk.commitList();
      }}
    />
  </div>
  {#if e.error}<p class="err" role="alert">{e.error}</p>{/if}
{/snippet}

<nav class="tk-nav scroll" aria-label="Elenchi di attività">
  {#each tk.smart as s (s.id)}
    {@render navItem(s.id, s.label, s.icon, TONE[s.id], null)}
  {/each}
  <div class="sep" role="separator"></div>
  {#each tk.lists as l (l.id)}
    {#if tk.listEdit?.id === l.id}
      {@render listInput()}
    {:else}
      {@render navItem(`elenco:${l.id}`, l.name, 'list', 'c-brand', l)}
    {/if}
  {/each}
  {#if tk.listEdit && !tk.listEdit.id}
    {@render listInput()}
  {/if}
  <button class="add" onclick={() => tk.startNewList()}><Icon name="plus" size={18} />Nuovo elenco</button>
</nav>

<section class="tk-main" aria-label={tk.title}>
  <div class="head">
    <div class="ht">
      <h1><Icon name={smartIcon} size={22} class={tk.isSmart ? TONE[tk.view] : 'c-brand'} />{tk.title}</h1>
      {#if tk.subtitle}<p class="sub">{tk.subtitle}</p>{/if}
    </div>
    {#if tk.currentList}
      {@const l = tk.currentList}
      <button class="ibtn btn" aria-label="Altre azioni per l’elenco" title="Rinomina o elimina l’elenco" aria-haspopup="menu" onclick={(e) => listMenu(e, l)}><Icon name="dots" /></button>
    {/if}
  </div>

  {#if tk.view !== 'contrassegnati'}
    <form class="addrow" onsubmit={submit}>
      <Icon name="plus" size={20} class="c-brand" />
      <input bind:this={addEl} bind:value={draft} aria-label="Aggiungi un’attività" placeholder="Aggiungi un’attività" maxlength="400" />
      {#if draft.trim()}<button class="primary-btn" type="submit">Aggiungi</button>{/if}
    </form>
  {/if}

  <div class="scroll body">
    {#if !tk.loaded}
      <div class="ghosts" aria-hidden="true">
        {#each [0, 1, 2] as i (i)}<div class="ghost"><span class="g-c"></span><span class="g-l" style:width="{62 - i * 14}%"></span></div>{/each}
      </div>
    {:else if tk.view === 'contrassegnati'}
      {#if !flagged.length}
        <div class="empty">
          <Icon name="flag" size={48} stroke={1.2} class="c-red" />
          <p class="t">{tk.query.trim() ? 'Nessun messaggio trovato' : 'Nessun messaggio contrassegnato'}</p>
          <p class="d">Contrassegna un messaggio (Ins) per ritrovarlo qui come cosa da fare. Quando hai finito, segnalo come completato.</p>
        </div>
      {:else}
        <ul class="rows">
          {#each flagged as m (m.id)}
            <li class="trow" oncontextmenu={(e) => flagMenu(e, m)}>
              <button class="chk" role="checkbox" aria-checked="false" aria-label="Segna come completato: {m.subject || 'nessun oggetto'}" title="Segna come completato (toglie il contrassegno)" onclick={() => tk.unflag(m)}>
                <Icon name="circle" size={20} class="c-off" /><Icon name="circle-check" size={20} class="c-on" />
              </button>
              <button class="tmain" onclick={() => store.revealMessage(m)} title="Apri il messaggio">
                <span class="tt">{m.subject || '(nessun oggetto)'}</span>
                <span class="tm">
                  <span class="mi"><Icon name="mail" size={14} />{personLabel(m.from) || 'Mittente sconosciuto'}</span>
                  {#if folderOf(m)}<span class="mi">{folderOf(m)}</span>{/if}
                  <span class="mi num">{listTime(m.date)}</span>
                </span>
              </button>
              <Icon name="flag-filled" size={16} class="c-red fl" />
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      {#if !shown.open.length && !shown.done.length}
        <div class="empty">
          {#if tk.query.trim()}
            <Icon name="search" size={44} stroke={1.2} class="c-brand" />
            <p class="t">Nessuna attività trovata</p>
            <p class="d">Prova con altre parole: si cerca nei titoli, nei passaggi e nelle note.</p>
          {:else}
            <Icon name={empty.icon} size={48} stroke={1.2} class={tk.isSmart ? TONE[tk.view] : 'c-brand'} />
            <p class="t">{empty.t}</p>
            <p class="d">{empty.d}</p>
          {/if}
        </div>
      {/if}
      {#each groups as g (g.label)}
        {#if g.label}<h2 class="grp" data-late={g.label === 'In ritardo' ? '' : undefined}>{g.label}</h2>{/if}
        <ul class="rows">
          {#each g.tasks as t (t.id)}
            {@render taskRow(t)}
          {/each}
        </ul>
      {/each}
      {#if shown.done.length}
        <button class="done-tg" aria-expanded={tk.doneOpen} onclick={() => (tk.doneOpen = !tk.doneOpen)}>
          <Icon name={tk.doneOpen ? 'chevron-down' : 'chevron-right'} size={16} />Completate<span class="cnt">{shown.done.length}</span>
        </button>
        {#if tk.doneOpen}
          <ul class="rows">
            {#each shown.done as t (t.id)}
              {@render taskRow(t)}
            {/each}
          </ul>
        {/if}
      {/if}
    {/if}
  </div>
  <Toast />
</section>

{#if tk.selected}
  <section class="tk-detail" aria-label="Dettagli dell’attività">
    <TaskDetail task={tk.selected} />
  </section>
{/if}

{#if menu}
  <Menu anchor={menu.el} at={menu.x !== undefined && menu.y !== undefined ? { x: menu.x, y: menu.y } : undefined} items={menu.items} label={menu.label} onclose={() => (menu = null)} />
{/if}

{#snippet taskRow(t: Task)}
  {@const on = tk.sel === t.id}
  {@const today = dayKey(tk.now)}
  {@const due = t.due !== null ? dueInfo(t.due, tk.now) : null}
  <li class="trow" data-sel={on ? 'si' : 'no'} data-done={t.done ? '' : undefined} oncontextmenu={(e) => taskMenu(e, t)}>
    <button class="chk" role="checkbox" aria-checked={t.done} aria-label="Completata: {t.title}" title={t.done ? 'Segna come da fare' : 'Segna come completata'} onclick={() => tk.toggleDone(t)}>
      {#if t.done}<Icon name="circle-check-filled" size={20} class="c-brand" />{:else}<Icon name="circle" size={20} class="c-off" /><Icon name="circle-check" size={20} class="c-on" />{/if}
    </button>
    <button class="tmain" aria-current={on ? 'true' : undefined} onclick={() => (tk.sel = on ? null : t.id)} onkeydown={(e) => rowKey(e, t)}>
      <span class="tt">{t.title}</span>
      <span class="tm">
        {#if tk.isSmart && tk.view !== 'attivita' && t.list}<span class="mi">{tk.listName(t.list)}</span>{/if}
        {#if t.myDay === today && tk.view !== 'giornata'}<span class="mi"><Icon name="sun" size={14} />La mia giornata</span>{/if}
        {#if t.steps.length}<span class="mi num">{stepsText(t)}</span>{/if}
        {#if due && !t.done}<span class="mi due" data-state={due.state}><Icon name="calendar-due" size={14} />{due.text}</span>{/if}
        {#if t.reminder !== null && !t.done}<span class="mi" title="Promemoria: {reminderWord(t.reminder, tk.now)}"><Icon name="bell" size={14} />{reminderWord(t.reminder, tk.now)}</span>{/if}
        {#if t.repeat}<span class="mi" title={repeatLabel(t.repeat)}><Icon name="repeat" size={14} /></span>{/if}
        {#if t.notes.trim()}<span class="mi" title="Ha delle note"><Icon name="notes" size={14} /></span>{/if}
        {#if t.message !== null}<span class="mi" title="Nata da un messaggio"><Icon name="mail" size={14} /></span>{/if}
      </span>
    </button>
    <button class="star" aria-pressed={t.important} aria-label="Importante: {t.title}" title={t.important ? 'Togli importanza' : 'Segna come importante'} onclick={() => tk.toggleImportant(t)}>
      <Icon name={t.important ? 'star-filled' : 'star'} size={18} />
    </button>
  </li>
{/snippet}

<style>
  /* Colori delle icone degli elenchi, come in Outlook. */
  .tk-nav :global(svg.c-brand), .tk-main :global(svg.c-brand) { color: var(--brand-ink); }
  .tk-nav :global(svg.c-amber), .tk-main :global(svg.c-amber) { color: var(--amber); }
  .tk-nav :global(svg.c-red), .tk-main :global(svg.c-red) { color: var(--danger); }
  .tk-nav :global(svg.c-green), .tk-main :global(svg.c-green) { color: var(--green); }
  .tk-nav { width: 240px; flex: none; box-sizing: border-box; overflow: auto; padding: 4px 8px 20px 2px; background: var(--ground); display: flex; flex-direction: column; gap: 1px; }
  .nav-it { padding-left: 12px; }
  .nav-it .cnt { color: var(--ink-3); font-weight: 400; }
  .sep { height: 1px; margin: 8px 10px; background: var(--rule-2); }
  .add { width: 100%; height: 32px; margin-top: 6px; display: flex; align-items: center; gap: 10px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; text-align: left; }
  .add:hover { background: var(--hover); }
  .edit { height: 32px; display: flex; align-items: center; gap: 10px; padding: 0 4px 0 12px; color: var(--ink-2); }
  .edit input { flex: 1; min-width: 0; height: 28px; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); color: var(--ink); font: inherit; font-size: 14px; }
  .edit input:focus { outline: none; }
  .edit input[aria-invalid="true"] { box-shadow: inset 0 0 0 1px var(--danger), inset 0 -2px 0 var(--danger); }
  .err { margin: 2px 4px 6px 12px; font-size: 12.5px; line-height: 1.4; color: var(--danger); }

  .tk-main { position: relative; flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; margin: 0 10px 10px 0; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .head { display: flex; align-items: flex-start; gap: 8px; padding: 14px 12px 6px 20px; }
  .ht { flex: 1; min-width: 0; }
  h1 { margin: 0; display: flex; align-items: center; gap: 10px; font-size: 20px; font-weight: 600; line-height: 1.3; }
  h1 :global(svg) { flex: none; }
  .sub { margin: 2px 0 0 32px; font-size: 13px; color: var(--ink-2); }
  .addrow { margin: 8px 16px 6px; height: 44px; display: flex; align-items: center; gap: 12px; padding: 0 6px 0 12px; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), var(--lift); }
  .addrow:focus-within { box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand), var(--lift); }
  .addrow input { flex: 1; min-width: 0; height: 100%; border: 0; background: transparent; color: var(--ink); font: inherit; font-size: 14px; }
  .addrow input:focus { outline: none; }
  .addrow input::placeholder { color: var(--brand-ink); opacity: 1; }
  .addrow input:focus::placeholder { color: var(--ink-3); }
  .body { flex: 1; min-height: 0; overflow: auto; padding: 4px 10px 80px; }
  .rows { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 2px; }
  .trow { position: relative; display: flex; align-items: center; gap: 2px; min-height: 52px; padding: 0 6px 0 4px; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 -1px 0 var(--rule); transition: background-color .12s ease-out; }
  .trow:hover { background: var(--hover); }
  .trow[data-sel="si"] { background: var(--brand-sel); box-shadow: none; }
  .chk { width: 40px; height: 40px; flex: none; display: grid; place-items: center; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--ink-2); }
  .chk :global(.c-on) { display: none; color: var(--brand-ink); }
  .chk:hover :global(.c-on), .chk:focus-visible :global(.c-on) { display: block; }
  .chk:hover :global(.c-off), .chk:focus-visible :global(.c-off) { display: none; }
  .tmain { flex: 1; min-width: 0; align-self: stretch; display: flex; flex-direction: column; justify-content: center; gap: 2px; padding: 6px 6px; border: 0; background: transparent; color: var(--ink); text-align: left; }
  .tt { font-size: 14px; line-height: 1.35; overflow-wrap: anywhere; }
  .trow[data-done] .tt { color: var(--ink-2); text-decoration: line-through; text-decoration-color: var(--ink-3); }
  .tm { display: flex; flex-wrap: wrap; align-items: center; column-gap: 12px; row-gap: 2px; font-size: 12.5px; color: var(--ink-2); }
  .tm:empty { display: none; }
  .mi { display: inline-flex; align-items: center; gap: 4px; }
  .mi :global(svg) { color: var(--ink-3); }
  .num { font-variant-numeric: tabular-nums; }
  .due[data-state="scaduta"], .due[data-state="scaduta"] :global(svg) { color: var(--danger); }
  .due[data-state="oggi"], .due[data-state="oggi"] :global(svg) { color: var(--brand-ink); }
  .star { width: 36px; height: 36px; flex: none; display: grid; place-items: center; padding: 0; border: 0; border-radius: 4px; background: transparent; color: var(--ink-3); }
  .star:hover { background: var(--hover); color: var(--ink-2); }
  .star[aria-pressed="true"] { color: var(--brand-ink); }
  .trow :global(.fl) { margin: 0 10px 0 6px; flex: none; }
  .grp { margin: 14px 10px 6px; font-size: 13px; font-weight: 600; color: var(--ink-2); }
  .grp[data-late] { color: var(--danger); }
  .done-tg { margin: 14px 4px 6px; height: 30px; display: inline-flex; align-items: center; gap: 6px; padding: 0 10px 0 6px; border: 0; border-radius: 4px; background: var(--rail); color: var(--ink); font-size: 13.5px; font-weight: 600; }
  .done-tg:hover { background: var(--hover); }
  .done-tg .cnt { margin-left: 4px; font-weight: 400; color: var(--ink-2); font-variant-numeric: tabular-nums; }
  .empty { display: flex; flex-direction: column; align-items: center; text-align: center; padding: 64px 24px 0; }
  .empty .t { margin: 14px 0 4px; font-size: 16px; font-weight: 600; }
  .empty .d { margin: 0; max-width: 44ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .ghosts { padding: 8px 6px; display: flex; flex-direction: column; gap: 6px; }
  .ghost { height: 44px; display: flex; align-items: center; gap: 14px; padding: 0 10px; }
  .g-c { width: 18px; height: 18px; border-radius: 50%; background: var(--rail); }
  .g-l { height: 10px; border-radius: 5px; background: var(--rail); }

  .tk-detail { width: 380px; flex: none; min-height: 0; display: flex; margin: 0 10px 10px 0; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  @media (max-width: 1100px) {
    .tk-detail { width: 320px; }
    .tk-nav { width: 200px; }
  }
  @media (max-width: 760px) {
    .tk-nav { display: none; }
    .tk-main { margin-left: 10px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .trow { transition: none; }
  }
</style>
