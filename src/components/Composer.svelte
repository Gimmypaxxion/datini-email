<script lang="ts">
  // Campi di un messaggio in scrittura: Invia, mittente (con più account),
  // destinatari A, Cc e Ccn con suggerimenti, oggetto, allegati, testo e piede
  // con lo stato della bozza. Usato nel riquadro di lettura, nella finestra
  // interna e nella finestra separata del sistema.
  import Icon from '../lib/Icon.svelte';
  import RichEditor from './RichEditor.svelte';
  import LinkDialog from './LinkDialog.svelte';
  import SchedulePicker from './SchedulePicker.svelte';
  import { store, type RecipientField } from '../lib/store.svelte';
  import { composeEditor, fontStack } from '../lib/editor.svelte';
  import { textToHtml } from '../lib/compose-html';
  import { fileSize, initials, personLabel, validEmail } from '../lib/format';
  import type { Person } from '../lib/types';

  let { inset = 0, onescape = undefined }: { inset?: number; onescape?: () => void } = $props();

  const c = $derived(store.c!);
  const sendPos = $derived(store.prefs.ribbon === 'estesa' ? 'sinistra' : 'alto');
  let schedBtn: HTMLButtonElement | undefined = $state();
  let schedOpen = $state(false);
  const senders = $derived(store.accounts.filter((a) => store.canSend(a)));
  const uid = Math.random().toString(36).slice(2, 8);

  let toEl: HTMLInputElement | undefined = $state();
  let subjEl: HTMLInputElement | undefined = $state();
  let editorAnchor: HTMLDivElement | undefined = $state();
  let sugg = $state<{ key: RecipientField; items: Person[]; idx: number } | null>(null);
  let suggSeq = 0;

  // Come in Outlook: il cursore va nel campo A per un messaggio nuovo o
  // inoltrato, all'inizio del testo per una risposta (lo fa l'editor). Solo all'apertura.
  let focusedKey = '';
  $effect(() => {
    const key = store.c?.key ?? '';
    if (!key || key === focusedKey) return;
    focusedKey = key;
    if (store.c?.to.length) return;
    const t = setTimeout(() => toEl?.focus(), 0);
    return () => clearTimeout(t);
  });

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      void store.send();
    }
  }

  async function onInput(key: RecipientField, value: string) {
    store.recipientInput(key, value);
    const seq = ++suggSeq;
    const items = await store.contacts(value);
    if (seq !== suggSeq) return;
    const chosen = new Set((store.c?.[key] ?? []).map((p) => p.email.toLowerCase()));
    const fresh = items.filter((p) => !chosen.has(p.email.toLowerCase()));
    sugg = fresh.length && value.trim().length >= 2 ? { key, items: fresh, idx: 0 } : null;
  }

  function pick(key: RecipientField, p: Person) {
    store.addRecipient(key, p);
    sugg = null;
    suggSeq++;
  }

  function onFieldKey(key: RecipientField, e: KeyboardEvent) {
    if (sugg && sugg.key === key) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const n = sugg.items.length;
        sugg.idx = (sugg.idx + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
        return;
      }
      if (e.key === 'Enter' || e.key === 'Tab') {
        if (e.key === 'Enter') e.preventDefault();
        pick(key, sugg.items[sugg.idx]);
        return;
      }
      if (e.key === 'Escape') {
        e.stopPropagation();
        sugg = null;
        return;
      }
    }
    store.recipientKey(key, e);
  }

  const inputValue = (key: RecipientField) => (key === 'to' ? store.toInput : key === 'cc' ? store.ccInput : store.bccInput);
  const LABELS: Record<RecipientField, string> = { to: 'A', cc: 'Cc', bcc: 'Ccn' };
</script>

{#snippet field(key: RecipientField)}
  <div class="field">
    <label for="{key}-{uid}">{LABELS[key]}</label>
    <div class="chips">
      {#each c[key] as p, i (p.email + p.name + i)}
        {@const bad = !validEmail(p.email)}
        <span class="chip" class:bad title={p.email || 'Indirizzo mancante'}>
          <span class="avatar" aria-hidden="true">{initials(personLabel(p))}</span>{personLabel(p)}{#if bad}<span class="sr"> (indirizzo non valido)</span>{/if}<button class="ibtn btn" onclick={() => store.removeRecipient(key, i)} aria-label="Rimuovi {personLabel(p)}"><Icon name="x" size={13} stroke={1.8} /></button>
        </span>
      {/each}
      <input
        bind:this={() => (key === 'to' ? toEl : undefined), (el) => { if (key === 'to') toEl = el; }}
        id="{key}-{uid}"
        class="ph bare grow"
        value={inputValue(key)}
        oninput={(e) => onInput(key, e.currentTarget.value)}
        onkeydown={(e) => onFieldKey(key, e)}
        onblur={() => setTimeout(() => { if (sugg?.key === key) sugg = null; }, 150)}
        placeholder={key === 'to' && !c.to.length ? 'Aggiungi destinatari' : ''}
        autocomplete="off"
        role="combobox"
        aria-expanded={sugg?.key === key}
        aria-controls="sugg-{key}-{uid}"
        aria-autocomplete="list"
      />
    </div>
    {#if key === 'cc' && !store.showBcc}
      <button class="lbtn ccn" onclick={() => (store.showBcc = true)}>Ccn</button>
    {/if}
    {#if sugg?.key === key}
      <ul class="sugg" id="sugg-{key}-{uid}" role="listbox" aria-label="Indirizzi suggeriti">
        {#each sugg.items as p, i (p.email)}
          <li role="option" aria-selected={i === sugg.idx}>
            <button class="sg" class:on={i === sugg.idx} onmousedown={(e) => { e.preventDefault(); pick(key, p); }}>
              <span class="avatar" aria-hidden="true">{initials(personLabel(p))}</span>
              <span class="sgt"><span class="sgn">{personLabel(p)}</span>{#if p.name}<span class="sge">{p.email}</span>{/if}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="composer" onkeydown={onKey}>
  <div class="cmp-h" data-send={sendPos} style:padding-right={inset ? `${inset}px` : undefined}>
    {#if sendPos === 'alto'}
      <div class="cmp-split">
        <button class="primary cmp-send" onclick={() => store.send()} disabled={store.sending} title="Invia (Ctrl+Invio)"><Icon name="send" size={20} />Invia</button>
        <button class="primary cmp-send-x" bind:this={schedBtn} onclick={() => (schedOpen = !schedOpen)} disabled={store.sending} aria-label="Pianifica invio" title="Pianifica invio" aria-haspopup="dialog" aria-expanded={schedOpen}><Icon name="chevron-down" size={16} /></button>
      </div>
      {#if schedOpen && schedBtn}<SchedulePicker anchor={schedBtn} onclose={() => (schedOpen = false)} />{/if}
    {:else}
      <button class="primary cmp-send" onclick={() => store.send()} disabled={store.sending} title="Invia (Ctrl+Invio)"><Icon name="send" size={20} />Invia</button>
    {/if}
    <div style="min-width:0">
      {#if senders.length > 1}
        <div class="field">
          <label for="from-{uid}">Da</label>
          <select id="from-{uid}" class="bare from" value={c.accountId} onchange={(e) => store.setAccount(e.currentTarget.value)}>
            {#each senders as a (a.id)}<option value={a.id}>{a.email}</option>{/each}
          </select>
        </div>
      {/if}
      {@render field('to')}
      {#if store.err}
        <p role="alert" class="err"><Icon name="alert-circle" size={15} stroke={1.7} />{store.err}</p>
      {/if}
      {@render field('cc')}
      {#if store.showBcc}{@render field('bcc')}{/if}
    </div>
  </div>
  {#if store.askNoSubject}
    <div class="note-bar warn ask" role="alertdialog" aria-label="Messaggio senza oggetto">
      <span class="t">Questo messaggio non ha un oggetto. Vuoi inviarlo comunque?</span>
      <button class="act" onclick={() => store.send(true)}>Invia comunque</button>
      <button class="act" onclick={() => { store.askNoSubject = false; subjEl?.focus(); }}>Aggiungi oggetto</button>
    </div>
  {/if}
  {#if store.askPlain}
    <div class="note-bar warn ask" role="alertdialog" aria-label="Passa a testo normale">
      <span class="t">In testo normale si perdono formattazione, immagini e tabelle del messaggio. Continuare?</span>
      <button class="act" onclick={() => store.setFormat('testo', true)}>Continua</button>
      <button class="act" onclick={() => (store.askPlain = false)}>Annulla</button>
    </div>
  {/if}
  <div class="pad">
    <div class="field subj">
      <label for="subj-{uid}" class="sr">Oggetto</label>
      <input bind:this={subjEl} id="subj-{uid}" class="ph bare" value={c.subject} oninput={(e) => store.setSubject(e.currentTarget.value)} placeholder="Aggiungi un oggetto" autocomplete="off" />
    </div>
  </div>
  {#if c.attachments.length}
    <ul class="atts" aria-label="Allegati">
      {#each c.attachments as a, i (a.name + i)}
        <li class="att">
          <Icon name="paperclip" size={16} style="color:var(--brand-ink)" />
          <span class="an">{a.name}</span>
          {#if a.size}<span class="sz">{fileSize(a.size)}</span>{/if}
          <button class="ibtn btn rm" onclick={() => store.removeAttachment(i)} aria-label="Togli {a.name}" title="Togli allegato"><Icon name="x" size={14} /></button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="body">
    <div class="anchor" bind:this={editorAnchor}></div>
    {#if c.plain}
      <label for="testo-{uid}" class="sr">Testo del messaggio</label>
      <textarea
        id="testo-{uid}"
        class="plain scroll"
        value={c.body}
        oninput={(e) => store.setBody(e.currentTarget.value)}
        onkeydown={(e) => {
          if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); void store.send(); }
          else if (e.key === 'Escape') { e.preventDefault(); onescape?.(); }
        }}
        placeholder="Scrivi il messaggio"
        style:font-family={fontStack(store.font.family)}
        style:font-size="{store.font.size}pt"
      ></textarea>
    {:else}
    <RichEditor
      ctl={composeEditor}
      docKey={c.key}
      html={c.html ?? textToHtml(c.body)}
      font={store.font}
      focusOnLoad={c.to.length ? 'start' : 'none'}
      onchange={(h, t) => store.setContent(h, t)}
      onsubmit={() => void store.send()}
      onescape={() => onescape?.()}
      onlink={() => store.openLinkDialog(null)}
      fetchImage={(u) => store.backend.fetchImage(u)}
    />
    {/if}
  </div>
  {#if store.linkOpen && (store.linkAnchor ?? editorAnchor)}
    <LinkDialog ctl={composeEditor} anchor={(store.linkAnchor ?? editorAnchor)!} onclose={() => store.closeLinkDialog()} />
  {/if}
  <div class="foot">
    <button class="btn attach" onclick={() => store.addFiles()}><Icon name="paperclip" size={17} style="color:var(--brand-ink)" />Allega file</button>
    <span class="status" class:bad={!!store.saveError} aria-live="polite" title={store.saveError || undefined}>
      {store.saveError ? 'Bozza non salvata: riprovo alla prossima modifica' : store.savedAt ? `Bozza salvata alle ${store.savedAt}` : 'Bozza non ancora salvata'}
    </span>
    <button class="btn discard" onclick={() => store.discard()}><Icon name="trash" size={17} style="color:var(--ink-2)" />Elimina bozza</button>
  </div>
</div>

<style>
  .composer { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .field { position: relative; }
  .grow { flex: 1; min-width: 140px; height: 28px; font-size: 14px; outline: none; }
  .from { flex: 1; min-width: 0; height: 30px; font-size: 14px; cursor: pointer; }
  .chip.bad { background: color-mix(in srgb, var(--danger) 14%, transparent); box-shadow: inset 0 -1px 0 var(--danger); }
  .err { margin: 6px 0 0 38px; display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--danger); }
  .ccn { flex: none; height: 28px; padding: 0 4px; font-size: 13.5px; }
  .sugg { position: absolute; z-index: 6; left: 38px; top: calc(100% - 2px); min-width: 300px; max-width: 420px; margin: 0; padding: 4px; list-style: none; border-radius: 6px; background: var(--sheet); box-shadow: var(--win-shadow), 0 0 0 1px var(--rule-2); }
  .sg { width: 100%; display: flex; align-items: center; gap: 10px; padding: 6px 10px; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .sg.on, .sg:hover { background: var(--hover); }
  .sg .avatar { width: 28px; height: 28px; font-size: 11px; background: var(--av1-bg); color: var(--av1-fg); }
  .sgt { min-width: 0; display: flex; flex-direction: column; }
  .sgn { font-size: 14px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sge { font-size: 12.5px; color: var(--ink-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pad { padding: 0 24px; }
  .ask { margin: 10px 24px 0; border-radius: 4px; }
  .subj { min-height: 44px; }
  .subj input { flex: 1; min-width: 0; height: 32px; font-size: 15px; font-weight: 600; outline: none; }
  .atts { margin: 0; padding: 10px 24px 0; list-style: none; display: flex; flex-wrap: wrap; gap: 8px; }
  .att { height: 32px; max-width: 320px; display: inline-flex; align-items: center; gap: 8px; padding: 0 2px 0 10px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--rule-2); font-size: 13px; }
  .an { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sz { color: var(--ink-3); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .rm { width: 26px; height: 26px; }
  .body { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; margin-top: 6px; background: #ffffff; }
  :global([data-theme="scuro"]) .body { margin: 8px 12px 0; border-radius: 6px; overflow: hidden; }
  .plain { flex: 1; min-height: 0; width: 100%; box-sizing: border-box; padding: 14px 24px 28px; border: 0; resize: none; outline: none; background: #ffffff; color: #000000; line-height: 1.4; }
  .plain::placeholder { color: #707070; }
  .anchor { position: absolute; left: 24px; top: 6px; width: 1px; height: 1px; pointer-events: none; }
  .foot { display: flex; align-items: center; gap: 12px; padding: 8px 14px 10px 18px; box-shadow: inset 0 1px 0 var(--rule); }
  .attach, .discard { height: 32px; display: flex; align-items: center; gap: 6px; padding: 0 12px 0 10px; border: 0; border-radius: 4px; background: transparent; font-size: 13.5px; }
  .status { flex: 1; font-size: 12.5px; color: var(--ink-3); text-align: right; }
  .status.bad { color: var(--danger); }
</style>
