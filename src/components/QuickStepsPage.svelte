<script lang="ts">
  // Impostazioni › Azioni rapide: elenco (nell'ordine della barra), editor di
  // un'azione rapida con i suoi passaggi, icona e scorciatoia.
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import { store, folderLabel } from '../lib/store.svelte';
  import { QUICK_ICONS, QUICK_KINDS, checkQuickStep, defaultQuickSteps, describeQuickStep, emptyQuickStep, kindInfo } from '../lib/quicksteps';
  import { keysText } from '../lib/platform';
  import { iconOr } from '../lib/icons';
  import { parseRecipient, personLabel } from '../lib/format';
  import type { MenuItem } from '../lib/menus';
  import type { QuickAction, QuickActionKind, QuickStep } from '../lib/types';

  const draft = $derived(store.quickDraft);
  const mailAccounts = $derived(store.accounts.filter((a) => a.kind !== 'local'));
  const folderName = (id: number) => {
    const f = store.folders.find((x) => x.id === id);
    return f ? folderLabel(f) : null;
  };

  // ---------- Editor ----------
  /** Destinatari degli inoltri come li ha scritti l'utente, per posizione. */
  let toText = $state<string[]>([]);
  let formErr = $state('');
  let loaded: QuickStep | null = null;

  $effect(() => {
    const d = draft;
    if (!d || d === loaded) return;
    loaded = d;
    toText = d.actions.map((a) => (a.to ?? []).map((p) => (p.name && p.email ? `${p.name} <${p.email}>` : personLabel(p))).join('; '));
    formErr = '';
  });

  /** Cartelle in cui spostare, per account. */
  const targetGroups = $derived(
    store.accounts.map((a) => ({
      label: a.kind === 'local' ? a.name : a.email,
      folders: store.foldersOf(a.id).filter((f) => f.selectable && f.role !== 'drafts' && f.role !== 'sent' && f.role !== 'deleted'),
    })),
  );

  function edit(q: QuickStep | null) {
    store.quickDraft = q ? (JSON.parse(JSON.stringify(q)) as QuickStep) : emptyQuickStep();
  }

  function fresh(kind: QuickActionKind): QuickAction {
    if (kind === 'sposta') {
      const fs = targetGroups.flatMap((g) => g.folders);
      return { kind, folder: (fs.find((f) => !f.role) ?? fs[0])?.id ?? null };
    }
    if (kind === 'categoria') return { kind, category: store.categories[0]?.id ?? null };
    return { kind };
  }

  function setKind(i: number, kind: QuickActionKind) {
    if (!draft) return;
    draft.actions[i] = fresh(kind);
    toText[i] = '';
    formErr = '';
  }

  function removeAction(i: number) {
    if (!draft) return;
    draft.actions.splice(i, 1);
    toText.splice(i, 1);
    formErr = '';
  }

  let addAnchor = $state<HTMLElement | null>(null);
  const usedPhases = $derived(new Set((draft?.actions ?? []).map((a) => kindInfo(a.kind).phase)));
  const addItems = $derived<MenuItem[]>([
    { kind: 'head', label: 'Sul messaggio' },
    ...QUICK_KINDS.filter((k) => k.phase === 'stato').map((k) => ({ label: k.label, icon: k.icon, disabled: k.kind === 'categoria' && !store.categories.length, onclick: () => add(k.kind) })),
    { kind: 'head', label: 'Rispondi o inoltra (al massimo una)' },
    ...QUICK_KINDS.filter((k) => k.phase === 'scrivi').map((k) => ({ label: k.label, icon: k.icon, disabled: usedPhases.has('scrivi'), onclick: () => add(k.kind) })),
    { kind: 'head', label: 'Sposta (al massimo una)' },
    ...QUICK_KINDS.filter((k) => k.phase === 'posto').map((k) => ({ label: k.label, icon: k.icon, disabled: usedPhases.has('posto'), onclick: () => add(k.kind) })),
  ]);

  function add(kind: QuickActionKind) {
    if (!draft) return;
    draft.actions.push(fresh(kind));
    toText.push('');
    formErr = '';
  }

  /** Destinatari scritti a mano: separati da punto e virgola o virgola. */
  function parseTo(text: string): { list: QuickAction['to']; bad: string | null } {
    const parts = text.split(/[;,]/).map((s) => s.trim()).filter(Boolean);
    const list = [];
    for (const p of parts) {
      const r = parseRecipient(p);
      if (!r) return { list: [], bad: p };
      list.push(r);
    }
    return { list, bad: null };
  }

  /** Scorciatoie già prese da altre azioni rapide. */
  const takenBy = $derived(new Map(store.quickSteps.filter((q) => q.id !== draft?.id).flatMap((q) => (q.shortcut ? [[q.shortcut, q.name] as const] : []))));

  async function saveDraft() {
    const d = draft;
    if (!d) return;
    const q: QuickStep = JSON.parse(JSON.stringify(d));
    q.name = q.name.trim();
    for (let i = 0; i < q.actions.length; i++) {
      const a = q.actions[i];
      if (a.kind === 'inoltra' || a.kind === 'inoltra-allegato') {
        const { list, bad } = parseTo(toText[i] ?? '');
        if (bad) {
          formErr = `“${bad}” non è un indirizzo email valido.`;
          return;
        }
        a.to = list;
      } else {
        delete a.to;
      }
      if (a.text !== undefined && !a.text?.trim()) a.text = null;
    }
    const bad = checkQuickStep(q);
    if (bad) {
      formErr = bad;
      return;
    }
    const list = [...store.quickSteps];
    const i = list.findIndex((x) => x.id && x.id === q.id);
    if (i >= 0) list[i] = q;
    else list.push(q);
    const err = await store.saveQuickSteps(list);
    if (err) formErr = err;
    else {
      store.quickDraft = null;
      status = i >= 0 ? `«${q.name}» salvata.` : `«${q.name}» creata: la trovi in Home › Azioni rapide e nel clic destro sui messaggi.`;
    }
  }

  // ---------- Elenco ----------
  let status = $state('');
  let menu = $state<{ el: HTMLElement; items: MenuItem[] } | null>(null);
  let removing = $state<string | null>(null);

  async function apply(list: QuickStep[], ok = '') {
    const err = await store.saveQuickSteps(list);
    status = err || ok;
  }

  async function move(i: number, by: number) {
    const list = [...store.quickSteps];
    const [q] = list.splice(i, 1);
    list.splice(i + by, 0, q);
    await apply(list);
  }

  async function remove(q: QuickStep) {
    removing = null;
    await apply(
      store.quickSteps.filter((x) => x.id !== q.id),
      `«${q.name}» eliminata.`,
    );
  }

  function openMenu(e: MouseEvent, q: QuickStep, i: number) {
    const el = e.currentTarget as HTMLElement;
    if (menu?.el === el) {
      menu = null;
      return;
    }
    menu = {
      el,
      items: [
        { label: 'Modifica', icon: 'pencil', onclick: () => edit(q) },
        { label: 'Esegui sul messaggio selezionato', icon: 'bolt', disabled: store.sel === null, onclick: () => { store.settings = false; void store.runQuickStep(q); } },
        { kind: 'sep', label: '' },
        { label: 'Sposta su', icon: 'chevron-up', disabled: i === 0, onclick: () => move(i, -1) },
        { label: 'Sposta giù', icon: 'chevron-down', disabled: i === store.quickSteps.length - 1, onclick: () => move(i, 1) },
        { kind: 'sep', label: '' },
        { label: 'Elimina', icon: 'trash', onclick: () => (removing = q.id) },
      ],
    };
  }

  const shortcutText = (n: number) => keysText(`Ctrl+Maiusc+${n}`);
</script>

{#if draft}
  <button class="back lbtn" onclick={() => (store.quickDraft = null)}><Icon name="arrow-left" size={16} />Azioni rapide</button>
  <h3>{draft.id ? 'Modifica azione rapida' : 'Nuova azione rapida'}</h3>

  <div class="form">
    <label class="tf" style="max-width:420px">
      <span>Nome</span>
      <input value={draft.name} oninput={(e) => { draft.name = e.currentTarget.value; formErr = ''; }} placeholder="Per esempio: Al commercialista" maxlength="60" />
    </label>

    <div class="qs-field">
      <span class="lab" id="qs-ico">Icona</span>
      <div class="icons" role="radiogroup" aria-labelledby="qs-ico">
        {#each QUICK_ICONS as [ic, label] (ic)}
          <button class="ico-pick" role="radio" aria-checked={draft.icon === ic} aria-label={label} title={label} onclick={() => (draft.icon = ic)}><Icon name={ic} size={18} /></button>
        {/each}
      </div>
    </div>

    <section aria-labelledby="qs-acts">
      <h4 id="qs-acts">Azioni</h4>
      <p class="hint">Prima si fanno le azioni sul messaggio, poi lo spostamento. La risposta o l’inoltro si apre alla fine, pronta da completare.</p>
      {#if draft.actions.length}
        <ol class="steps">
          {#each draft.actions as a, i (i)}
            {@const info = kindInfo(a.kind)}
            <li class="step">
              <span class="step-ico" aria-hidden="true"><Icon name={info.icon} size={16} /></span>
              <div class="step-body">
                <div class="step-row">
                  <select class="sel kind" aria-label="Azione {i + 1}" value={a.kind} onchange={(e) => setKind(i, e.currentTarget.value as QuickActionKind)}>
                    <optgroup label="Sul messaggio">
                      {#each QUICK_KINDS.filter((k) => k.phase === 'stato') as k (k.kind)}<option value={k.kind}>{k.label}</option>{/each}
                    </optgroup>
                    <optgroup label="Rispondi">
                      {#each QUICK_KINDS.filter((k) => k.phase === 'scrivi') as k (k.kind)}<option value={k.kind}>{k.label}</option>{/each}
                    </optgroup>
                    <optgroup label="Sposta">
                      {#each QUICK_KINDS.filter((k) => k.phase === 'posto') as k (k.kind)}<option value={k.kind}>{k.label}</option>{/each}
                    </optgroup>
                  </select>
                  {#if a.kind === 'sposta'}
                    <select class="sel" aria-label="Cartella" value={String(a.folder ?? '')} onchange={(e) => (a.folder = Number(e.currentTarget.value))}>
                      {#if a.folder !== null && a.folder !== undefined && !folderName(a.folder)}<option value={String(a.folder)}>Cartella non più presente</option>{/if}
                      {#each targetGroups as g (g.label)}
                        <optgroup label={g.label}>
                          {#each g.folders as f (f.id)}<option value={String(f.id)}>{'  '.repeat(f.role ? 0 : Math.max(0, f.depth))}{folderLabel(f)}</option>{/each}
                        </optgroup>
                      {/each}
                    </select>
                  {:else if a.kind === 'categoria'}
                    <select class="sel" aria-label="Categoria" value={a.category ?? ''} onchange={(e) => (a.category = e.currentTarget.value)}>
                      {#each store.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
                    </select>
                  {/if}
                </div>
                {#if a.kind === 'inoltra' || a.kind === 'inoltra-allegato'}
                  <label class="tf sub">
                    <span>A <span class="hint">facoltativo; più indirizzi separati da punto e virgola</span></span>
                    <input value={toText[i] ?? ''} oninput={(e) => { toText[i] = e.currentTarget.value; formErr = ''; }} placeholder="per esempio studio@commercialista.it" />
                  </label>
                {/if}
                {#if info.phase === 'scrivi'}
                  <label class="tf sub">
                    <span>Testo da aggiungere <span class="hint">facoltativo, in cima al messaggio</span></span>
                    <input value={a.text ?? ''} oninput={(e) => (a.text = e.currentTarget.value)} placeholder="per esempio: Ti giro questa fattura da registrare." />
                  </label>
                {/if}
              </div>
              <button class="ibtn btn rm" aria-label="Togli l’azione {info.label}" title="Togli" onclick={() => removeAction(i)}><Icon name="x" size={16} /></button>
            </li>
          {/each}
        </ol>
      {:else}
        <p class="none">Ancora nessuna azione. Aggiungine una, per esempio Segna come letto.</p>
      {/if}
      <button class="sec-btn add" aria-haspopup="menu" aria-expanded={!!addAnchor} onclick={(e) => (addAnchor = addAnchor ? null : (e.currentTarget as HTMLElement))}><Icon name="plus" size={16} />Aggiungi un’azione</button>
    </section>

    <section aria-labelledby="qs-keys">
      <h4 id="qs-keys">Scorciatoia</h4>
      <select class="sel" aria-labelledby="qs-keys" value={String(draft.shortcut ?? '')} onchange={(e) => (draft.shortcut = e.currentTarget.value ? Number(e.currentTarget.value) : null)}>
        <option value="">Nessuna</option>
        {#each [1, 2, 3, 4, 5, 6, 7, 8, 9] as n (n)}
          {@const other = takenBy.get(n)}
          <option value={String(n)} disabled={!!other}>{shortcutText(n)}{other ? ` (usata da «${other}»)` : ''}</option>
        {/each}
      </select>
    </section>

    {#if formErr}<p class="bad" role="alert">{formErr}</p>{/if}
    <div class="form-acts">
      <button class="primary-btn" onclick={saveDraft}><Icon name="device-floppy" size={17} />Salva</button>
      <button class="sec-btn" onclick={() => (store.quickDraft = null)}>Annulla</button>
    </div>
  </div>
  {#if addAnchor}
    <Menu anchor={addAnchor} items={addItems} label="Aggiungi un’azione" onclose={() => (addAnchor = null)} />
  {/if}
{:else}
  <h3>Azioni rapide</h3>
  <p class="intro">Più passaggi con un clic sul messaggio selezionato, per esempio segnarlo come letto e archiviarlo. Le trovi in Home › Azioni rapide, nel clic destro sui messaggi e con le scorciatoie da {shortcutText(1)} a {shortcutText(9)}.</p>

  <div class="group">
    <div class="list-head">
      <h4 style="margin:0">Le tue azioni rapide</h4>
      <button class="primary-btn" onclick={() => edit(null)}><Icon name="plus" size={16} />Nuova azione rapida</button>
    </div>
    {#if !store.quickSteps.length}
      <div class="empty">
        <p>Non hai azioni rapide. Creane una oppure riparti da quelle proposte: Fatto (segna come letto e archivia) e Rispondi ed elimina.</p>
        <button class="sec-btn" onclick={() => apply(defaultQuickSteps(), 'Azioni rapide proposte ripristinate.')}><Icon name="refresh" size={16} />Ripristina le azioni proposte</button>
      </div>
    {:else}
      <ol class="qs">
        {#each store.quickSteps as q, i (q.id)}
          <li class="q">
            <span class="q-ico" aria-hidden="true"><Icon name={iconOr(q.icon, 'bolt')} size={18} /></span>
            <button class="qt" onclick={() => edit(q)}>
              <span class="qn">{q.name}</span>
              <span class="qd">{describeQuickStep(q, folderName, store.categories)}.</span>
            </button>
            {#if q.shortcut}<kbd class="keys">{shortcutText(q.shortcut)}</kbd>{/if}
            {#if removing === q.id}
              <span class="ask">Eliminare?</span>
              <button class="sec-btn danger-btn" onclick={() => remove(q)}>Elimina</button>
              <button class="sec-btn" onclick={() => (removing = null)}>Annulla</button>
            {:else}
              <button class="ibtn btn" aria-label="Altre azioni per «{q.name}»" title="Altre azioni" aria-haspopup="menu" onclick={(e) => openMenu(e, q, i)}><Icon name="dots" /></button>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
    {#if status}<p class="status" role="status">{status}</p>{/if}
  </div>
  {#if !mailAccounts.length}
    <p class="hint" style="margin-top:16px">Rispondi e Inoltra servono un account di posta: aggiungilo in Account e sincronizzazione.</p>
  {/if}
{/if}

{#if menu}
  <Menu anchor={menu.el} items={menu.items} label="Azioni dell’azione rapida" onclose={() => (menu = null)} />
{/if}

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 6px; font-size: 15px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 64ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .hint { margin: 0 0 10px; max-width: 62ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .group { margin-top: 26px; padding-top: 22px; box-shadow: inset 0 1px 0 var(--rule); }
  .list-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; max-width: 680px; }
  .qs { margin: 12px 0 0; padding: 0; list-style: none; max-width: 680px; display: flex; flex-direction: column; gap: 6px; }
  .q { display: flex; align-items: center; gap: 10px; padding: 8px 8px 8px 10px; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); }
  .q-ico { width: 32px; height: 32px; flex: none; display: grid; place-items: center; border-radius: 4px; background: var(--brand-soft); color: var(--brand-ink); }
  .qt { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; padding: 4px 6px; border: 0; border-radius: 4px; background: transparent; text-align: left; color: var(--ink); }
  .qt:hover { background: var(--hover); }
  .qn { font-size: 14px; font-weight: 600; }
  .qd { font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .keys { flex: none; padding: 2px 7px; border-radius: 4px; background: var(--rail); color: var(--ink-2); font: inherit; font-size: 12.5px; font-variant-numeric: tabular-nums; white-space: nowrap; }
  .ask { font-size: 13.5px; }
  .status { margin: 12px 0 0; max-width: 680px; font-size: 13.5px; color: var(--ink-2); }
  .empty { margin-top: 14px; max-width: 680px; display: flex; flex-direction: column; align-items: flex-start; gap: 12px; }
  .empty p { margin: 0; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .back { margin: -4px 0 10px -6px; height: 30px; display: inline-flex; align-items: center; gap: 6px; padding: 0 8px; font-size: 13.5px; }
  .form { margin-top: 20px; display: flex; flex-direction: column; gap: 18px; max-width: 680px; }
  section { padding-top: 18px; box-shadow: inset 0 1px 0 var(--rule); }
  .qs-field { display: flex; flex-direction: column; gap: 8px; }
  .lab { font-size: 13.5px; font-weight: 600; }
  .icons { display: flex; flex-wrap: wrap; gap: 6px; }
  .ico-pick { width: 36px; height: 36px; display: grid; place-items: center; padding: 0; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink-2); }
  .ico-pick:hover { background: var(--hover); color: var(--ink); }
  .ico-pick[aria-checked="true"] { background: var(--brand-soft); color: var(--brand-ink); box-shadow: inset 0 0 0 2px var(--brand); }
  .steps { margin: 4px 0 12px; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 6px; }
  .step { display: flex; align-items: flex-start; gap: 10px; padding: 10px 6px 10px 12px; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); }
  .step-ico { width: 28px; height: 32px; flex: none; display: grid; place-items: center; color: var(--brand-ink); }
  .step-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 10px; }
  .step-row { display: flex; flex-wrap: wrap; gap: 8px; }
  .rm { width: 32px; height: 32px; flex: none; }
  .sel { height: 32px; min-width: 200px; max-width: 100%; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background-color: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .sel.kind { min-width: 230px; }
  .tf.sub > span { font-size: 13px; }
  .none { margin: 0 0 12px; font-size: 13.5px; color: var(--ink-2); }
  .add { align-self: flex-start; }
  .bad { margin: 0; font-size: 13.5px; color: var(--danger); }
  .form-acts { display: flex; gap: 8px; }
</style>
