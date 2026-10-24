<script lang="ts">
  // Impostazioni › Regole e avvisi: elenco delle regole (in ordine di
  // applicazione), editor di una regola, avviso sul desktop.
  import Icon from '../lib/Icon.svelte';
  import Menu from './Menu.svelte';
  import { store, folderLabel } from '../lib/store.svelte';
  import { describeActions, describeConditions, emptyRule, hasActions, hasConditions, joinWords, splitWords } from '../lib/rules';
  import type { MenuItem } from '../lib/menus';
  import type { Rule } from '../lib/types';

  const mailAccounts = $derived(store.accounts.filter((a) => a.kind !== 'local'));
  const accountName = (id: string | null) => {
    const a = store.accounts.find((x) => x.id === id);
    return a ? (a.kind === 'local' ? a.name : a.email) : '';
  };

  // ---------- Editor ----------
  const draft = $derived(store.ruleDraft);
  let fromText = $state('');
  let toText = $state('');
  let subjectText = $state('');
  let wordsText = $state('');
  let formErr = $state('');
  let loaded: Rule | null = null;

  // Quando si apre una regola, i campi di testo ripartono da lei.
  $effect(() => {
    const d = draft;
    if (!d || d === loaded) return;
    loaded = d;
    fromText = joinWords(d.when.from);
    toText = joinWords(d.when.to);
    subjectText = joinWords(d.when.subject);
    wordsText = joinWords(d.when.words);
    formErr = '';
  });

  /** Cartelle in cui la regola può spostare: quelle dell'account scelto, o di tutti. */
  const targetGroups = $derived(
    (draft?.accountId ? store.accounts.filter((a) => a.id === draft.accountId) : mailAccounts).map((a) => ({
      label: a.email,
      folders: store.foldersOf(a.id).filter((f) => f.selectable && f.role !== 'drafts' && f.role !== 'sent'),
    })),
  );

  function edit(r: Rule | null) {
    store.ruleDraft = r ? (JSON.parse(JSON.stringify(r)) as Rule) : emptyRule();
  }

  function collect(): Rule | null {
    const d = draft;
    if (!d) return null;
    const r: Rule = JSON.parse(JSON.stringify(d));
    r.name = r.name.trim();
    r.when.from = splitWords(fromText);
    r.when.to = splitWords(toText);
    r.when.subject = splitWords(subjectText);
    r.when.words = splitWords(wordsText);
    if (!r.name) {
      formErr = 'Dai un nome alla regola.';
      return null;
    }
    if (!hasActions(r)) {
      formErr = 'Scegli almeno un’azione: per esempio Sposta nella cartella.';
      return null;
    }
    if (r.then.moveTo !== null && !r.then.delete && targetMissing) {
      formErr = 'La cartella scelta non c’è più: scegline un’altra.';
      return null;
    }
    formErr = '';
    return r;
  }

  const targetMissing = $derived(!!draft && draft.then.moveTo !== null && !store.folders.some((f) => f.id === draft.then.moveTo));

  async function saveDraft() {
    const r = collect();
    if (!r) return;
    const list = [...store.rules];
    const i = list.findIndex((x) => x.id && x.id === r.id);
    if (i >= 0) list[i] = r;
    else list.push(r);
    const err = await store.saveRules(list);
    if (err) formErr = err;
    else {
      store.ruleDraft = null;
      status = r.id ? 'Regola salvata.' : 'Regola creata: vale per i messaggi che arrivano da adesso. Per quelli già arrivati usa Esegui ora dal menu della regola.';
    }
  }

  /** Esito dell'ultima azione sull'elenco (salvataggio, Esegui ora, errori). */
  let status = $state('');
  async function apply(list: Rule[]) {
    const err = await store.saveRules(list);
    status = err ?? '';
  }

  // ---------- Elenco ----------
  let menu = $state<{ el: HTMLElement; items: MenuItem[] } | null>(null);

  async function toggle(r: Rule) {
    await apply(store.rules.map((x) => (x.id === r.id ? { ...x, enabled: !x.enabled } : x)));
  }

  async function move(i: number, by: number) {
    const list = [...store.rules];
    const [r] = list.splice(i, 1);
    list.splice(i + by, 0, r);
    await apply(list);
  }

  let removing = $state<string | null>(null);

  async function remove(r: Rule) {
    removing = null;
    await apply(store.rules.filter((x) => x.id !== r.id));
  }

  /** Esegui ora: nella posta in arrivo (dell'account della regola o di tutti) o nella cartella aperta. */
  function runItems(r: Rule): MenuItem[] {
    const inboxes = store.folders.filter((f) => f.role === 'inbox' && (!r.accountId || f.accountId === r.accountId));
    const items: MenuItem[] = inboxes.map((f) => ({
      label: mailAccounts.length > 1 ? `Esegui ora in Posta in arrivo (${accountName(f.accountId)})` : 'Esegui ora in Posta in arrivo',
      icon: 'inbox',
      onclick: async () => (status = await store.runRulesNow([r.id], f.id)),
    }));
    const cur = store.currentFolder;
    if (cur && cur.role !== 'inbox' && (!r.accountId || cur.accountId === r.accountId)) {
      items.push({ label: `Esegui ora in ${folderLabel(cur)}`, icon: 'folder', onclick: async () => (status = await store.runRulesNow([r.id], cur.id)) });
    }
    return items;
  }

  function openMenu(e: MouseEvent, r: Rule, i: number) {
    const el = e.currentTarget as HTMLElement;
    if (menu?.el === el) {
      menu = null;
      return;
    }
    menu = {
      el,
      items: [
        { label: 'Modifica', icon: 'pencil', onclick: () => edit(r) },
        ...runItems(r),
        { kind: 'sep', label: '' },
        { label: 'Sposta su', icon: 'chevron-up', disabled: i === 0, onclick: () => move(i, -1) },
        { label: 'Sposta giù', icon: 'chevron-down', disabled: i === store.rules.length - 1, onclick: () => move(i, 1) },
        { kind: 'sep', label: '' },
        { label: 'Elimina', icon: 'trash', onclick: () => (removing = r.id) },
      ],
    };
  }

  let tested = $state('');
  async function testAlert() {
    const ok = await store.backend.notify('Better-Look', 'Così appare l’avviso quando arriva un nuovo messaggio.');
    tested = ok ? 'Avviso inviato: se non lo vedi, controlla le notifiche del sistema per Better-Look.' : 'Il sistema non permette gli avvisi a Better-Look: abilitali nelle impostazioni delle notifiche del computer.';
  }
</script>

{#if draft}
  <button class="back lbtn" onclick={() => (store.ruleDraft = null)}><Icon name="arrow-left" size={16} />Regole e avvisi</button>
  <h3>{draft.id ? 'Modifica regola' : 'Nuova regola'}</h3>

  <div class="form">
    <label class="tf" style="max-width:420px">
      <span>Nome</span>
      <input value={draft.name} oninput={(e) => (draft.name = e.currentTarget.value)} placeholder="Per esempio: Fatture dei fornitori" />
    </label>
    {#if mailAccounts.length > 1}
      <label class="tf" style="max-width:420px">
        <span>Per i messaggi in arrivo in</span>
        <select value={draft.accountId ?? ''} onchange={(e) => (draft.accountId = e.currentTarget.value || null)}>
          <option value="">Tutti gli account</option>
          {#each mailAccounts as a (a.id)}<option value={a.id}>{a.email}</option>{/each}
        </select>
      </label>
    {/if}

    <section aria-labelledby="r-when">
      <h4 id="r-when">Quando arriva un messaggio in cui</h4>
      <p class="hint">Valgono tutte le condizioni compilate. In ogni campo puoi scrivere più parole separate da punto e virgola: basta una.</p>
      <div class="conds">
        <label class="tf"><span>Il mittente contiene</span><input bind:value={fromText} placeholder="nome o indirizzo, per esempio studiobassi.it" /></label>
        <label class="tf"><span>I destinatari contengono</span><input bind:value={toText} placeholder="per esempio ordini@valliarredi.it" /></label>
        <label class="tf"><span>L’oggetto contiene</span><input bind:value={subjectText} placeholder="per esempio fattura; preventivo" /></label>
        <label class="tf"><span>L’oggetto o il testo contengono</span><input bind:value={wordsText} placeholder="per esempio IBAN" /></label>
      </div>
      <label class="check"><input type="checkbox" checked={draft.when.hasAttachments} onchange={(e) => (draft.when.hasAttachments = e.currentTarget.checked)} />Ci sono allegati</label>
      {#if !hasConditions({ ...draft.when, from: splitWords(fromText), to: splitWords(toText), subject: splitWords(subjectText), words: splitWords(wordsText) })}
        <p class="warn-t"><Icon name="alert-triangle" size={16} style="color:var(--warn)" />Senza condizioni la regola vale per tutti i messaggi in arrivo.</p>
      {/if}
    </section>

    <section aria-labelledby="r-then">
      <h4 id="r-then">Fai questo</h4>
      <div class="acts-list">
        <div class="act-row">
          <label class="check"><input type="checkbox" checked={draft.then.moveTo !== null} disabled={draft.then.delete} onchange={(e) => (draft.then.moveTo = e.currentTarget.checked ? (targetGroups[0]?.folders.find((f) => !f.role)?.id ?? targetGroups[0]?.folders[0]?.id ?? null) : null)} />Sposta nella cartella</label>
          {#if draft.then.moveTo !== null && !draft.then.delete}
            <select class="sel" aria-label="Cartella" value={String(draft.then.moveTo)} onchange={(e) => (draft.then.moveTo = Number(e.currentTarget.value))}>
              {#if targetMissing}<option value={String(draft.then.moveTo)}>Cartella non più presente</option>{/if}
              {#each targetGroups as g (g.label)}
                <optgroup label={g.label}>
                  {#each g.folders as f (f.id)}<option value={String(f.id)}>{'  '.repeat(f.role ? 0 : Math.max(0, f.depth))}{folderLabel(f)}</option>{/each}
                </optgroup>
              {/each}
            </select>
          {/if}
        </div>
        <label class="check"><input type="checkbox" checked={draft.then.delete} onchange={(e) => (draft.then.delete = e.currentTarget.checked)} />Elimina (va in Posta eliminata)</label>
        <label class="check"><input type="checkbox" checked={draft.then.markRead} onchange={(e) => (draft.then.markRead = e.currentTarget.checked)} />Segna come letto</label>
        <label class="check"><input type="checkbox" checked={draft.then.flag} onchange={(e) => (draft.then.flag = e.currentTarget.checked)} />Contrassegna</label>
        <div class="act-row">
          <label class="check"><input type="checkbox" checked={!!draft.then.category} disabled={!store.categories.length} onchange={(e) => (draft.then.category = e.currentTarget.checked ? (store.categories[0]?.id ?? null) : null)} />Assegna la categoria</label>
          {#if draft.then.category}
            <select class="sel" aria-label="Categoria" value={draft.then.category} onchange={(e) => (draft.then.category = e.currentTarget.value)}>
              {#each store.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
            </select>
          {/if}
        </div>
        <label class="check"><input type="checkbox" checked={draft.then.alert} onchange={(e) => (draft.then.alert = e.currentTarget.checked)} />Mostra un avviso sul desktop</label>
      </div>
      <label class="check stop"><input type="checkbox" checked={draft.then.stop} onchange={(e) => (draft.then.stop = e.currentTarget.checked)} />Non applicare le regole successive</label>
    </section>

    {#if formErr}<p class="bad" role="alert">{formErr}</p>{/if}
    <div class="form-acts">
      <button class="primary-btn" onclick={saveDraft}><Icon name="device-floppy" size={17} />Salva</button>
      <button class="sec-btn" onclick={() => (store.ruleDraft = null)}>Annulla</button>
    </div>
  </div>
{:else}
  <h3>Regole e avvisi</h3>
  <p class="intro">Le regole spostano, contrassegnano o eliminano i messaggi appena arrivano, in ordine dall’alto. Funzionano mentre Better-Look è aperto; i messaggi arrivati a programma chiuso passano dalle regole alla riapertura.</p>

  <div class="group">
    <label class="check" style="margin:0">
      <input type="checkbox" checked={store.notifyDesktop} onchange={(e) => store.setNotify(e.currentTarget.checked)} />
      Mostra un avviso sul desktop quando arriva un nuovo messaggio
    </label>
    <p class="hint sub">Vale per la posta Prioritaria. Le regole possono chiedere l’avviso anche per altri messaggi.</p>
    <div class="test">
      <button class="sec-btn" onclick={testAlert} disabled={!store.backend.real}>Prova l’avviso</button>
      {#if tested}<span class="hint" role="status">{tested}</span>{/if}
    </div>
  </div>

  <div class="group">
    <div class="list-head">
      <h4 style="margin:0">Regole</h4>
      <button class="primary-btn" onclick={() => edit(null)}><Icon name="plus" size={16} />Nuova regola</button>
    </div>
    {#if !store.rules.length}
      <p class="hint" style="margin-top:12px">Non hai ancora regole. Puoi crearne una anche dal clic destro su un messaggio, con Crea regola.</p>
    {:else}
      <ol class="rules">
        {#each store.rules as r, i (r.id)}
          <li class="rule" data-off={r.enabled ? undefined : ''}>
            <input type="checkbox" class="on" checked={r.enabled} aria-label="Regola “{r.name}” attiva" title={r.enabled ? 'Attiva' : 'Spenta'} onchange={() => toggle(r)} />
            <button class="rt" onclick={() => edit(r)}>
              <span class="rn">{r.name}{#if r.accountId && mailAccounts.length > 1}<span class="ra">{accountName(r.accountId)}</span>{/if}</span>
              <span class="rd">{describeConditions(r.when)}: {describeActions(r, store.folders, store.categories)}.</span>
            </button>
            {#if removing === r.id}
              <span class="ask">Eliminare?</span>
              <button class="sec-btn danger-btn" onclick={() => remove(r)}>Elimina</button>
              <button class="sec-btn" onclick={() => (removing = null)}>Annulla</button>
            {:else}
              <button class="ibtn btn" aria-label="Altre azioni per “{r.name}”" title="Altre azioni" aria-haspopup="menu" onclick={(e) => openMenu(e, r, i)}><Icon name="dots" /></button>
            {/if}
          </li>
        {/each}
      </ol>
    {/if}
    {#if status}<p class="status" role="status">{status}</p>{/if}
  </div>
{/if}

{#if menu}
  <Menu anchor={menu.el} items={menu.items} label="Azioni della regola" onclose={() => (menu = null)} />
{/if}

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  h4 { margin: 0 0 6px; font-size: 15px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 64ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .hint { margin: 0 0 10px; max-width: 62ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .hint.sub { margin: 6px 0 0 28px; }
  .group { margin-top: 26px; padding-top: 22px; box-shadow: inset 0 1px 0 var(--rule); }
  .check { margin-top: 10px; display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .check input { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); flex: none; }
  .test { margin: 12px 0 0 28px; display: flex; align-items: center; flex-wrap: wrap; gap: 12px; }
  .test .hint { margin: 0; }
  .list-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; max-width: 680px; }
  .rules { margin: 12px 0 0; padding: 0; list-style: none; max-width: 680px; display: flex; flex-direction: column; gap: 6px; }
  .rule { display: flex; align-items: center; gap: 10px; padding: 8px 8px 8px 12px; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); }
  .rule[data-off] .rt { opacity: .6; }
  .on { width: 18px; height: 18px; margin: 0; accent-color: var(--brand); flex: none; }
  .rt { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; padding: 4px 6px; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .rt:hover { background: var(--hover); }
  .rn { display: flex; align-items: baseline; gap: 8px; font-size: 14px; font-weight: 600; }
  .ra { font-size: 12.5px; font-weight: 400; color: var(--ink-3); }
  .rd { font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .ask { font-size: 13.5px; }
  .status { margin: 12px 0 0; max-width: 680px; font-size: 13.5px; color: var(--ink-2); }
  .back { margin: -4px 0 10px -6px; height: 30px; display: inline-flex; align-items: center; gap: 6px; padding: 0 8px; font-size: 13.5px; }
  .form { margin-top: 20px; display: flex; flex-direction: column; gap: 16px; max-width: 680px; }
  section { padding-top: 18px; box-shadow: inset 0 1px 0 var(--rule); }
  .conds { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px 16px; }
  .acts-list { display: flex; flex-direction: column; }
  .act-row { display: flex; align-items: center; flex-wrap: wrap; gap: 10px 14px; }
  .act-row .check { min-width: 200px; }
  .sel { margin-top: 10px; height: 32px; min-width: 220px; max-width: 100%; box-sizing: border-box; padding: 0 8px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); color: var(--ink); font: inherit; font-size: 13.5px; }
  .stop { margin-top: 16px; }
  .warn-t { margin: 12px 0 0; display: flex; align-items: center; gap: 8px; font-size: 13.5px; }
  .bad { margin: 0; font-size: 13.5px; color: var(--danger); }
  .form-acts { display: flex; gap: 8px; }
  @media (max-width: 760px) {
    .conds { grid-template-columns: minmax(0, 1fr); }
  }
</style>
