<script lang="ts">
  // Contatti (Ctrl+3): elenco in ordine alfabetico a sinistra, scheda del
  // contatto a destra, come la rubrica del nuovo Outlook. La scheda diventa
  // un modulo per creare o modificare.
  import Icon from '../lib/Icon.svelte';
  import Toast from './Toast.svelte';
  import { store } from '../lib/store.svelte';
  import { birthdayText, contactLetter, contactName, EXPORT_HELP, phoneLabel } from '../lib/contacts';
  import { initials, listTime, tint } from '../lib/format';
  import { PHONE_KINDS, type Contact, type MessageSummary, type PhoneKind } from '../lib/types';

  const list = $derived(store.filteredContacts);
  const groups = $derived.by(() => {
    const out: { letter: string; items: Contact[] }[] = [];
    for (const c of list) {
      const l = contactLetter(c);
      const g = out[out.length - 1];
      if (g && g.letter === l) g.items.push(c);
      else out.push({ letter: l, items: [c] });
    }
    return out;
  });
  const sel = $derived(store.selectedContact);
  const edit = $derived(store.contactEdit);

  // ---------- Messaggi recenti del contatto ----------
  let recent = $state<MessageSummary[]>([]);
  let recentFor = '';
  $effect(() => {
    const c = sel;
    const email = c?.emails[0] ?? '';
    if (email === recentFor) return;
    recentFor = email;
    recent = [];
    if (!email) return;
    void store.backend
      .search(email, null)
      .then((r) => {
        if (recentFor === email) recent = r.filter((m) => !m.draft).slice(0, 5);
      })
      .catch(() => {});
  });

  // ---------- Modulo ----------
  let formErr = $state('');
  let saving = $state(false);
  let firstField: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (edit) {
      formErr = '';
      setTimeout(() => firstField?.focus(), 0);
    }
  });

  function addEmail() {
    edit?.emails.push('');
  }
  function addPhone() {
    edit?.phones.push({ kind: edit.phones.length ? 'lavoro' : 'cellulare', number: '' });
  }

  async function save() {
    if (!edit) return;
    saving = true;
    formErr = (await store.saveContactEdit()) ?? '';
    saving = false;
  }

  const fullName = (c: Contact) => contactName(c) || 'Senza nome';
  const sub = (c: Contact) => [c.jobTitle, c.company].filter(Boolean).join(', ');
</script>

<section class="pane-list" aria-label="Elenco contatti">
  <div class="head">
    <h1>Contatti</h1>
    <span class="count">{store.book.length || ''}</span>
  </div>
  <div class="scroll body">
    {#if !store.book.length}
      <div class="empty">
        <Icon name="address-book" size={52} stroke={1.2} style="color:var(--brand-ink)" />
        <p class="t">La rubrica è vuota</p>
        <p class="d">Aggiungi un contatto, oppure importa la rubrica da Outlook, Gmail, Apple o Thunderbird.</p>
        <div class="empty-acts">
          <button class="primary-btn" onclick={() => store.newContact()}><Icon name="user-plus" size={17} />Nuovo contatto</button>
          <button class="sec-btn" onclick={() => store.importContacts()}><Icon name="file-import" size={17} />Importa</button>
        </div>
      </div>
    {:else if !list.length}
      <p class="none">Nessun contatto corrisponde alla ricerca.</p>
    {/if}
    {#each groups as g (g.letter)}
      <h2 class="letter" aria-label="Lettera {g.letter}">{g.letter}</h2>
      {#each g.items as c (c.id)}
        {@const t = tint(fullName(c))}
        <button class="crow" data-sel={sel?.id === c.id && !edit ? 'si' : 'no'} onclick={() => { store.contactEdit = null; store.contactSel = c.id; }}>
          <span class="avatar" aria-hidden="true" style:background={t.bg} style:color={t.fg}>{initials(fullName(c))}</span>
          <span class="ct"><span class="cn">{fullName(c)}</span><span class="cs">{c.emails[0] || sub(c) || c.phones[0]?.number || ''}</span></span>
        </button>
      {/each}
    {/each}
  </div>
  <Toast />
</section>

<section class="pane-read" aria-label={edit ? 'Modifica contatto' : 'Scheda del contatto'}>
  {#if edit}
    <div class="scroll card">
      <h2 class="ftitle">{edit.id ? 'Modifica contatto' : 'Nuovo contatto'}</h2>
      <form class="form" onsubmit={(e) => { e.preventDefault(); void save(); }}>
        <div class="two-col">
          <label class="tf"><span>Nome</span><input bind:this={firstField} bind:value={edit.firstName} autocomplete="off" /></label>
          <label class="tf"><span>Cognome</span><input bind:value={edit.lastName} autocomplete="off" /></label>
          <label class="tf"><span>Azienda</span><input bind:value={edit.company} autocomplete="off" /></label>
          <label class="tf"><span>Posizione</span><input bind:value={edit.jobTitle} autocomplete="off" /></label>
        </div>

        <fieldset class="multi">
          <legend>Email</legend>
          {#each edit.emails as _, i (i)}
            <div class="mrow">
              <input class="minput" type="email" aria-label="Indirizzo email {i + 1}" bind:value={edit.emails[i]} autocomplete="off" spellcheck="false" />
              <button type="button" class="ibtn btn" aria-label="Togli l'indirizzo" title="Togli" onclick={() => edit.emails.splice(i, 1)}><Icon name="x" size={16} /></button>
            </div>
          {/each}
          <button type="button" class="lbtn more" onclick={addEmail}><Icon name="plus" size={15} />Aggiungi email</button>
        </fieldset>

        <fieldset class="multi">
          <legend>Telefono</legend>
          {#each edit.phones as p, i (i)}
            <div class="mrow">
              <select class="mkind" aria-label="Tipo di numero {i + 1}" value={p.kind} onchange={(e) => (p.kind = e.currentTarget.value as PhoneKind)}>
                {#each PHONE_KINDS as [k, label] (k)}<option value={k}>{label}</option>{/each}
              </select>
              <input class="minput" type="tel" aria-label="Numero {i + 1}" bind:value={p.number} autocomplete="off" />
              <button type="button" class="ibtn btn" aria-label="Togli il numero" title="Togli" onclick={() => edit.phones.splice(i, 1)}><Icon name="x" size={16} /></button>
            </div>
          {/each}
          <button type="button" class="lbtn more" onclick={addPhone}><Icon name="plus" size={15} />Aggiungi telefono</button>
        </fieldset>

        <label class="tf"><span>Indirizzo</span><textarea rows="3" bind:value={edit.address}></textarea></label>
        <div class="two-col">
          <label class="tf"><span>Sito web</span><input bind:value={edit.website} autocomplete="off" spellcheck="false" placeholder="https://" /></label>
          <label class="tf">
            <span>Compleanno</span>
            {#if edit.birthday.startsWith('--')}
              <input value={birthdayText(edit.birthday)} readonly title="Data senza anno, arrivata da un'altra rubrica" />
            {:else}
              <input type="date" bind:value={edit.birthday} />
            {/if}
          </label>
        </div>
        <label class="tf"><span>Note</span><textarea rows="4" bind:value={edit.notes}></textarea></label>

        {#if formErr}<p class="bad" role="alert">{formErr}</p>{/if}
        <div class="form-acts">
          <button class="primary-btn" type="submit" disabled={saving}><Icon name="device-floppy" size={17} />Salva</button>
          <button class="sec-btn" type="button" onclick={() => (store.contactEdit = null)}>Annulla</button>
        </div>
      </form>
    </div>
  {:else if sel}
    {@const t = tint(fullName(sel))}
    <div class="scroll card">
      <header class="who">
        <span class="avatar big" aria-hidden="true" style:background={t.bg} style:color={t.fg}>{initials(fullName(sel))}</span>
        <div class="who-t">
          <h2>{fullName(sel)}</h2>
          {#if sub(sel)}<p class="role">{sub(sel)}</p>{/if}
          <div class="acts">
            {#if sel.emails.length}
              <button class="primary-btn" onclick={() => store.composeTo(sel.emails[0], fullName(sel))}><Icon name="mail" size={17} />Invia un messaggio</button>
            {/if}
            <button class="sec-btn" onclick={() => store.editContact(sel)}><Icon name="pencil" size={16} />Modifica</button>
            <button class="sec-btn" onclick={() => store.deleteContacts([sel.id])}><Icon name="trash" size={16} />Elimina</button>
          </div>
        </div>
      </header>

      <dl class="facts">
        {#each sel.emails as e (e)}
          <dt><Icon name="mail" size={17} />Email</dt>
          <dd><button class="lbtn link" onclick={() => store.composeTo(e, fullName(sel))} title="Scrivi a {e}">{e}</button></dd>
        {/each}
        {#each sel.phones as p, i (i)}
          <dt><Icon name="phone" size={17} />{phoneLabel(p.kind)}</dt>
          <dd class="num">{p.number}</dd>
        {/each}
        {#if sel.address}
          <dt><Icon name="map-pin" size={17} />Indirizzo</dt>
          <dd class="pre">{sel.address}</dd>
        {/if}
        {#if sel.website}
          <dt><Icon name="world" size={17} />Sito web</dt>
          <dd><button class="lbtn link" onclick={() => store.openLink(/^https?:/i.test(sel.website) ? sel.website : `https://${sel.website}`)}>{sel.website}</button></dd>
        {/if}
        {#if sel.birthday}
          <dt><Icon name="cake" size={17} />Compleanno</dt>
          <dd>{birthdayText(sel.birthday)}</dd>
        {/if}
        {#if sel.notes}
          <dt><Icon name="notes" size={17} />Note</dt>
          <dd class="pre">{sel.notes}</dd>
        {/if}
      </dl>

      {#if sel.emails.length}
        <section class="recent" aria-labelledby="ct-recent">
          <h3 id="ct-recent">Messaggi recenti</h3>
          {#if !recent.length}
            <p class="muted">Nessun messaggio con {sel.emails[0]} su questo computer.</p>
          {:else}
            <ul>
              {#each recent as m (m.id)}
                <li>
                  <button class="msg" onclick={() => store.showMailFrom(sel.emails[0], m.id)}>
                    <span class="ms">{m.subject || '(nessun oggetto)'}</span>
                    <span class="md">{listTime(m.date)}</span>
                  </button>
                </li>
              {/each}
            </ul>
            <button class="lbtn more" onclick={() => store.showMailFrom(sel.emails[0])}>Mostra tutti i messaggi</button>
          {/if}
        </section>
      {/if}
    </div>
  {:else}
    <div class="scroll card help">
      <Icon name="users" size={48} stroke={1.2} style="color:var(--brand-ink)" />
      <p class="t">{store.book.length ? 'Scegli un contatto' : 'Porta qui la tua rubrica'}</p>
      <p class="d">Better-Look importa i file vCard (.vcf), CSV e LDIF. I contatti già presenti vengono completati, non duplicati.</p>
      <h3>Come esportare la rubrica</h3>
      <dl class="howto">
        {#each EXPORT_HELP as h (h.app)}
          <dt>{h.app}</dt>
          <dd>{h.how}</dd>
        {/each}
      </dl>
      <button class="sec-btn" onclick={() => store.importContacts()} disabled={!store.backend.real && store.book.length > 8}><Icon name="file-import" size={17} />Importa contatti</button>
    </div>
  {/if}
</section>

<style>
  .pane-list { position: relative; width: 360px; flex: none; min-height: 0; display: flex; flex-direction: column; margin: 0 0 10px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .head { display: flex; align-items: baseline; gap: 10px; padding: 14px 16px 8px; }
  h1 { margin: 0; font-size: 20px; font-weight: 600; }
  .count { font-size: 13px; color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .body { flex: 1; min-height: 0; overflow: auto; padding: 0 6px 70px; }
  .letter { position: sticky; top: 0; z-index: 1; margin: 0; padding: 8px 12px 4px; background: var(--sheet); font-size: 13px; font-weight: 600; color: var(--brand-ink); }
  .crow { width: 100%; display: flex; align-items: center; gap: 12px; padding: 8px 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); text-align: left; }
  .crow:hover { background: var(--hover); }
  .crow[data-sel="si"] { background: var(--brand-sel); }
  .avatar { width: 36px; height: 36px; flex: none; border-radius: 50%; display: grid; place-items: center; font-size: 13px; font-weight: 600; }
  .avatar.big { width: 72px; height: 72px; font-size: 26px; }
  .ct { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .cn { font-size: 14px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cs { font-size: 12.5px; color: var(--ink-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .none { margin: 16px 12px; font-size: 13.5px; color: var(--ink-2); }
  .empty { display: flex; flex-direction: column; align-items: center; text-align: center; padding: 48px 20px 0; }
  .empty .t, .help .t { margin: 14px 0 4px; font-size: 16px; font-weight: 600; }
  .empty .d, .help .d { margin: 0; max-width: 44ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  .empty-acts { margin-top: 18px; display: flex; flex-wrap: wrap; justify-content: center; gap: 8px; }

  .pane-read { flex: 1; min-width: 0; min-height: 0; display: flex; margin: 0 10px 10px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .card { flex: 1; min-width: 0; overflow: auto; padding: 28px 36px 40px; }
  .who { display: flex; align-items: flex-start; gap: 20px; }
  .who-t { min-width: 0; }
  .who h2 { margin: 6px 0 0; font-size: 24px; font-weight: 600; overflow-wrap: anywhere; }
  .role { margin: 4px 0 0; font-size: 14px; color: var(--ink-2); }
  .acts { margin-top: 16px; display: flex; flex-wrap: wrap; gap: 8px; }
  .facts { margin: 28px 0 0; display: grid; grid-template-columns: 150px minmax(0, 1fr); column-gap: 16px; row-gap: 12px; max-width: 640px; font-size: 14px; }
  .facts dt { display: flex; align-items: flex-start; gap: 8px; color: var(--ink-2); line-height: 1.5; }
  .facts dt :global(svg) { margin-top: 2px; }
  .facts dt :global(svg) { color: var(--ink-3); }
  .facts dd { margin: 0; min-width: 0; overflow-wrap: anywhere; line-height: 1.5; }
  .pre { white-space: pre-line; line-height: 1.5; }
  .num { font-variant-numeric: tabular-nums; }
  .link { padding: 0; font-size: 14px; text-align: left; }
  .link:hover { text-decoration: underline; text-underline-offset: 3px; }
  .recent { margin-top: 32px; padding-top: 20px; max-width: 640px; box-shadow: inset 0 1px 0 var(--rule); }
  .recent h3, .help h3 { margin: 0 0 10px; font-size: 15px; font-weight: 600; }
  .recent ul { margin: 0; padding: 0; list-style: none; }
  .msg { width: 100%; display: flex; align-items: baseline; justify-content: space-between; gap: 16px; padding: 7px 8px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font-size: 13.5px; text-align: left; }
  .msg:hover { background: var(--hover); }
  .ms { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .md { flex: none; font-size: 12.5px; color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .more { margin-top: 6px; height: 30px; display: inline-flex; align-items: center; gap: 6px; padding: 0 8px; font-size: 13.5px; }
  .muted { margin: 0; font-size: 13.5px; color: var(--ink-2); }

  .help { display: flex; flex-direction: column; align-items: center; text-align: center; }
  .help h3 { margin-top: 28px; }
  .howto { margin: 0 0 20px; max-width: 560px; text-align: left; font-size: 13.5px; line-height: 1.5; }
  .howto dt { margin-top: 10px; font-weight: 600; }
  .howto dd { margin: 2px 0 0; color: var(--ink-2); }

  .ftitle { margin: 0; font-size: 22px; font-weight: 600; }
  .form { margin-top: 20px; display: flex; flex-direction: column; gap: 16px; max-width: 620px; }
  .two-col { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px 16px; }
  .tf textarea { box-sizing: border-box; padding: 8px 10px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -1px 0 var(--ink-3); color: var(--ink); font: inherit; font-size: 14px; line-height: 1.45; resize: vertical; }
  .tf textarea:focus { outline: none; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .multi { margin: 0; padding: 0; border: 0; min-width: 0; display: flex; flex-direction: column; gap: 8px; }
  .multi legend { margin-bottom: 6px; padding: 0; font-size: 13.5px; font-weight: 600; }
  .mrow { display: flex; align-items: center; gap: 8px; }
  .minput, .mkind { height: 34px; box-sizing: border-box; padding: 0 10px; border: 0; border-radius: 4px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -1px 0 var(--ink-3); color: var(--ink); font: inherit; font-size: 14px; }
  .minput { flex: 1; min-width: 0; }
  .mkind { width: 128px; flex: none; padding: 0 6px; }
  .minput:focus, .mkind:focus { outline: none; box-shadow: inset 0 0 0 1px var(--rule-2), inset 0 -2px 0 var(--brand); }
  .multi .more { align-self: flex-start; margin: 0; }
  .bad { margin: 0; font-size: 13.5px; color: var(--danger); }
  .form-acts { display: flex; gap: 8px; }

  @media (max-width: 1100px) {
    .pane-list { width: 300px; }
    .facts { grid-template-columns: minmax(0, 1fr); row-gap: 4px; }
    .facts dd { margin-bottom: 10px; }
  }
</style>
