<script lang="ts">
  // Impostazioni › Modelli di messaggio: testi da riusare, con oggetto
  // facoltativo. Si inseriscono dal pulsante Modelli mentre si scrive.
  import Icon from '../lib/Icon.svelte';
  import MiniEditor from './MiniEditor.svelte';
  import { store } from '../lib/store.svelte';
  import { htmlToText } from '../lib/compose-html';
  import type { MessageTemplate } from '../lib/types';

  let sel = $state<string | null>(null);
  let name = $state('');
  let subject = $state('');
  let html = $state('');
  let dirty = $state(false);
  let note = $state('');
  let err = $state('');
  let version = $state(0);
  let confirmDel = $state(false);

  function pick(t: MessageTemplate | null) {
    sel = t?.id ?? null;
    name = t?.name ?? '';
    subject = t?.subject ?? '';
    html = t?.html ?? '';
    dirty = false;
    note = '';
    err = '';
    confirmDel = false;
    version++;
  }

  // All'apertura: il primo modello, oppure uno nuovo da scrivere.
  let init = false;
  $effect(() => {
    if (store.settings && store.setPage === 'modelli') {
      if (!init) {
        init = true;
        void store.reloadTemplates().then(() => pick(store.templates[0] ?? null));
      }
    } else {
      init = false;
    }
  });

  const touched = () => {
    dirty = true;
    note = '';
    err = '';
  };

  async function save() {
    const n = name.trim() || subject.trim();
    if (!n) {
      err = 'Dai un nome al modello, per riconoscerlo nell’elenco.';
      return;
    }
    if (!htmlToText(html).trim() && !/<img/i.test(html)) {
      err = 'Scrivi il testo del modello.';
      return;
    }
    const same = store.templates.find((t) => t.id !== sel && t.name.trim().toLowerCase() === n.toLowerCase());
    if (same) {
      err = `Esiste già un modello che si chiama «${same.name}»: scegli un altro nome.`;
      return;
    }
    const before = new Set(store.templates.map((t) => t.id));
    const e = await store.saveTemplate({ id: sel ?? '', name: n, subject: subject.trim(), html });
    if (e) {
      err = e;
      return;
    }
    const saved = store.templates.find((t) => t.id === sel) ?? store.templates.find((t) => !before.has(t.id));
    sel = saved?.id ?? sel;
    name = n;
    dirty = false;
    note = 'Modello salvato.';
  }

  async function remove() {
    if (!sel) return;
    await store.deleteTemplate(sel);
    pick(store.templates[0] ?? null);
  }

  const preview = (t: MessageTemplate) => htmlToText(t.html).replace(/\s+/g, ' ').trim();
</script>

<h3>Modelli di messaggio</h3>
<p class="intro">Testi che scrivi spesso, pronti da inserire mentre scrivi un messaggio: li trovi nel pulsante Modelli, nelle schede Messaggio e Inserisci.</p>

<div class="grid">
  <div class="list" role="listbox" aria-label="Modelli">
    {#each store.templates as t (t.id)}
      {@const on = sel === t.id}
      <button class="it" role="option" aria-selected={on} data-sel={on ? 'si' : 'no'} onclick={() => pick(t)}>
        <span class="n">{t.name}</span>
        <span class="p">{preview(t) || 'Senza testo'}</span>
      </button>
    {/each}
    {#if !store.templates.length}<p class="empty">Non hai ancora modelli. Scrivi qui il primo, oppure salva come modello un messaggio che stai scrivendo.</p>{/if}
    <button class="btn add" onclick={() => pick(null)} aria-pressed={sel === null}><Icon name="plus" size={16} />Nuovo modello</button>
  </div>

  <div class="edit">
    <div class="two">
      <label class="tf">
        <span>Nome <span class="hint">serve solo a te</span></span>
        <input bind:value={name} oninput={touched} placeholder="Per esempio: Conferma d’ordine" maxlength="80" />
      </label>
      <label class="tf">
        <span>Oggetto <span class="hint">facoltativo</span></span>
        <input bind:value={subject} oninput={touched} placeholder="Usato se il messaggio non ne ha uno" maxlength="300" />
      </label>
    </div>
    <MiniEditor docKey="modello-{version}" {html} label="Testo del modello" placeholder="Scrivi il testo da riusare" height={260} onchange={(h, t) => { html = t || /<img/i.test(h) ? h : ''; touched(); }} />
    {#if err}<p class="bad" role="alert">{err}</p>{/if}
    <div class="acts">
      <button class="primary-btn" onclick={save} disabled={!dirty && !!sel}><Icon name="device-floppy" size={17} />Salva</button>
      {#if sel}
        {#if confirmDel}
          <span class="q">Eliminare «{name}»?</span>
          <button class="sec-btn danger-btn" onclick={remove}>Elimina</button>
          <button class="sec-btn" onclick={() => (confirmDel = false)}>Annulla</button>
        {:else}
          <button class="sec-btn" onclick={() => (confirmDel = true)}><Icon name="trash" size={16} />Elimina</button>
        {/if}
      {/if}
      <span class="note" role="status">{note || (dirty ? 'Modifiche non ancora salvate' : '')}</span>
    </div>
  </div>
</div>

<style>
  h3 { margin: 0; font-size: 22px; font-weight: 600; }
  .intro { margin: 6px 0 0; max-width: 64ch; font-size: 14.5px; line-height: 1.5; color: var(--ink-2); }
  .grid { margin-top: 24px; display: grid; grid-template-columns: 220px minmax(0, 1fr); gap: 20px; align-items: start; }
  .list { display: flex; flex-direction: column; gap: 2px; }
  .it { display: flex; flex-direction: column; gap: 1px; min-width: 0; padding: 7px 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); text-align: left; }
  .it:hover { background: var(--hover); }
  .it[data-sel="si"] { background: var(--brand-soft); }
  .n { font-size: 14px; font-weight: 600; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .p { font-size: 12.5px; color: var(--ink-2); overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  .empty { margin: 0 0 6px; padding: 0 10px; font-size: 13px; line-height: 1.45; color: var(--ink-2); }
  .add { height: 32px; margin-top: 6px; display: flex; align-items: center; gap: 8px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--brand-ink); font-size: 14px; }
  .add[aria-pressed="true"] { background: var(--brand-soft); }
  .edit { min-width: 0; display: flex; flex-direction: column; gap: 14px; }
  .two { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px 16px; max-width: 680px; }
  .tf .hint { margin-left: 4px; }
  .bad { margin: 0; font-size: 13.5px; color: var(--danger); }
  .acts { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .q { font-size: 13.5px; }
  .note { margin-left: 6px; font-size: 13px; color: var(--ink-3); }
  @media (max-width: 900px) {
    .grid { grid-template-columns: minmax(0, 1fr); }
    .two { grid-template-columns: minmax(0, 1fr); }
  }
</style>
