<script lang="ts">
  // Nuova cartella di ricerca (Ctrl+Maiusc+P), come in Outlook: si sceglie
  // che cosa cercare fra i tipi proposti, si completano le condizioni e il
  // nome. La cartella mostra i messaggi dell'account che le rispettano, in
  // tutte le cartelle tranne Bozze, Posta eliminata e Posta indesiderata.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { DAY_CHOICES, SEARCH_FOLDER_PRESETS, SIZE_CHOICES, describeCriteria, presetOf } from '../lib/searchfolders';

  const e = $derived(store.sfEdit);
  const preset = $derived(e ? presetOf(e.kind) : null);
  const groups = $derived([...new Set(SEARCH_FOLDER_PRESETS.map((p) => p.group))]);
  const mailAccounts = $derived(store.accounts.filter((a) => a.kind !== 'local'));
  const summary = $derived(e ? describeCriteria({ ...e.criteria, people: e.peopleText.split(/[;,\n]/).map((s) => s.trim()).filter(Boolean) }, (id) => store.category(id)?.name ?? null) : '');

  function close() {
    store.sfEdit = null;
  }

  function submit(ev: SubmitEvent) {
    ev.preventDefault();
    void store.saveSearchFolderDialog();
  }

  /** Categoria: nessun filtro, una qualsiasi, oppure quella scelta. */
  function categoryValue(): string {
    const c = e!.criteria;
    return c.category ?? (c.categorized ? '*' : '');
  }

  function setCategory(v: string) {
    const c = e!.criteria;
    c.categorized = v !== '';
    c.category = v && v !== '*' ? v : null;
  }

  function focusFirst(node: HTMLElement) {
    const sel = node.querySelector<HTMLInputElement>('input[type="radio"]:checked');
    (sel ?? node.querySelector<HTMLElement>('input, button'))?.focus();
  }
</script>

{#if e && preset}
  <div class="scrim fade-in sfd-scrim">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="sfd rise"
      role="dialog"
      aria-modal="true"
      aria-labelledby="sfd-title"
      tabindex="-1"
      use:focusFirst
      onkeydown={(ev) => {
        if (ev.key === 'Escape') {
          ev.stopPropagation();
          close();
        }
      }}
    >
      <header>
        <h2 id="sfd-title">{e.id ? 'Modifica cartella di ricerca' : 'Nuova cartella di ricerca'}</h2>
        <button class="dlg-x" onclick={close} aria-label="Chiudi" title="Chiudi (Esc)"><Icon name="x" size={18} /></button>
      </header>
      <form class="scroll sfd-body" onsubmit={submit} novalidate>
        <p class="sfd-intro">Una ricerca salvata che si apre come una cartella. I messaggi restano dove sono: la cartella li mostra tutti insieme e si aggiorna da sola.</p>

        <fieldset class="sfd-kinds">
          <legend class="sfd-h">Che cosa cercare</legend>
          {#each groups as g (g)}
            <p class="sfd-g">{g}</p>
            <div class="sfd-cols">
              {#each SEARCH_FOLDER_PRESETS.filter((p) => p.group === g) as p (p.kind)}
                {@const on = e.kind === p.kind}
                <label class="sfd-kind" data-on={on ? 'si' : 'no'}>
                  <input type="radio" name="sfd-kind" checked={on} onchange={() => store.pickSearchFolderKind(p.kind)} />
                  <Icon name={p.icon} size={18} />
                  <span>{p.label}</span>
                </label>
              {/each}
            </div>
          {/each}
        </fieldset>

        {#if preset.need}
          <div class="sfd-more">
            {#if preset.need === 'persone'}
              <label class="tf">
                <span>Persone <span class="hint">nomi o indirizzi, separati da punto e virgola</span></span>
                <input value={e.peopleText} oninput={(ev) => store.setSearchFolderPeople(ev.currentTarget.value)} placeholder="Per esempio: Davide Orlandi; studiobassi.it" />
              </label>
            {:else if preset.need === 'parole'}
              <label class="tf">
                <span>Parole <span class="hint">nel testo, nell’oggetto o nei nomi</span></span>
                <input value={e.criteria.text} oninput={(ev) => store.setSearchFolderText(ev.currentTarget.value)} placeholder="Per esempio: preventivo Bassano" />
              </label>
            {:else if preset.need === 'dimensione'}
              <label class="tf sfd-narrow">
                <span>Messaggi</span>
                <select value={String(e.criteria.minKb ?? 1024)} onchange={(ev) => (e.criteria.minKb = Number(ev.currentTarget.value))}>
                  {#each SIZE_CHOICES as [kb, label] (kb)}<option value={String(kb)}>{label}</option>{/each}
                </select>
              </label>
            {:else if preset.need === 'categoria'}
              <label class="tf sfd-narrow">
                <span>Categoria</span>
                <select value={categoryValue()} onchange={(ev) => setCategory(ev.currentTarget.value || '*')}>
                  <option value="*">Qualsiasi categoria</option>
                  {#each store.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
                </select>
              </label>
            {:else}
              <div class="sfd-checks" role="group" aria-label="Condizioni">
                <label class="sfd-chk"><input type="checkbox" bind:checked={e.criteria.unread} />Da leggere</label>
                <label class="sfd-chk"><input type="checkbox" bind:checked={e.criteria.flagged} />Contrassegnati</label>
                <label class="sfd-chk"><input type="checkbox" bind:checked={e.criteria.important} />Con priorità alta</label>
                <label class="sfd-chk"><input type="checkbox" bind:checked={e.criteria.attachments} />Con allegati</label>
              </div>
              <div class="sfd-grid">
                <label class="tf">
                  <span>Contengono le parole</span>
                  <input bind:value={e.criteria.text} placeholder="Qualsiasi testo" />
                </label>
                <label class="tf">
                  <span>Da o verso</span>
                  <input bind:value={e.peopleText} placeholder="Nomi o indirizzi" />
                </label>
                <label class="tf">
                  <span>Categoria</span>
                  <select value={categoryValue()} onchange={(ev) => setCategory(ev.currentTarget.value)}>
                    <option value="">Qualsiasi, anche nessuna</option>
                    <option value="*">Con una categoria qualsiasi</option>
                    {#each store.categories as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
                  </select>
                </label>
                <label class="tf">
                  <span>Dimensione</span>
                  <select value={String(e.criteria.minKb ?? '')} onchange={(ev) => (e.criteria.minKb = ev.currentTarget.value ? Number(ev.currentTarget.value) : null)}>
                    <option value="">Qualsiasi</option>
                    {#each SIZE_CHOICES as [kb, label] (kb)}<option value={String(kb)}>{label}</option>{/each}
                  </select>
                </label>
                <label class="tf">
                  <span>Arrivati</span>
                  <select value={String(e.criteria.days ?? '')} onchange={(ev) => (e.criteria.days = ev.currentTarget.value ? Number(ev.currentTarget.value) : null)}>
                    <option value="">In qualsiasi momento</option>
                    {#each DAY_CHOICES as [d, label] (d)}<option value={String(d)}>{label}</option>{/each}
                  </select>
                </label>
              </div>
            {/if}
          </div>
        {/if}

        <div class="sfd-grid sfd-last">
          <label class="tf">
            <span>Nome della cartella</span>
            <input value={e.name} oninput={(ev) => { e.name = ev.currentTarget.value; e.nameTouched = true; e.error = ''; }} maxlength="80" />
          </label>
          {#if mailAccounts.length > 1 || e.id}
            <label class="tf">
              <span>Cerca la posta in</span>
              <select value={e.accountId} onchange={(ev) => (e.accountId = ev.currentTarget.value)} disabled={!!e.id}>
                {#each mailAccounts as a (a.id)}<option value={a.id}>{a.email}</option>{/each}
              </select>
            </label>
          {/if}
        </div>
        <p class="sfd-sum"><Icon name="folder-search" size={16} />{summary}, in tutte le cartelle tranne Bozze, Posta eliminata e Posta indesiderata.</p>

        {#if e.error}
          <p class="sfd-err" role="alert"><Icon name="alert-circle" size={17} />{e.error}</p>
        {/if}
        <div class="sfd-acts">
          <button type="submit" class="primary-btn" disabled={e.busy || !e.name.trim()}>{e.id ? 'Salva' : 'Crea cartella'}</button>
          <button type="button" class="sec-btn" onclick={close}>Annulla</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .sfd-scrim { z-index: 60; position: fixed; }
  .sfd { width: min(680px, 100%); max-height: 100%; display: flex; flex-direction: column; border-radius: 8px; background: var(--sheet); box-shadow: var(--win-shadow); overflow: hidden; outline: none; }
  header { display: flex; align-items: center; gap: 12px; padding: 20px 16px 0 28px; }
  h2 { margin: 0; flex: 1; font-size: 20px; font-weight: 600; }
  .sfd-body { overflow: auto; padding: 6px 28px 22px; }
  .sfd-intro { margin: 0 0 18px; max-width: 60ch; font-size: 14px; line-height: 1.5; color: var(--ink-2); }
  .sfd-kinds { margin: 0; padding: 0; border: 0; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .sfd-h { padding: 0; margin-bottom: 4px; font-size: 14px; font-weight: 600; }
  .sfd-g { margin: 10px 0 4px; font-size: 12.5px; font-weight: 600; color: var(--ink-3); }
  .sfd-cols { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 2px 8px; }
  .sfd-kind { min-height: 34px; display: flex; align-items: center; gap: 10px; padding: 4px 10px; box-sizing: border-box; line-height: 1.3; border-radius: 4px; font-size: 14px; cursor: pointer; transition: background-color .12s ease-out; }
  .sfd-kind:hover { background: var(--hover); }
  .sfd-kind[data-on="si"] { background: var(--brand-soft); }
  .sfd-kind input { position: absolute; opacity: 0; pointer-events: none; }
  .sfd-kind:has(input:focus-visible) { outline: 2px solid var(--brand-ink); outline-offset: -2px; }
  .sfd-kind :global(svg) { flex: none; color: var(--ink-2); }
  .sfd-kind[data-on="si"] :global(svg) { color: var(--brand-ink); }
  .sfd-more { margin-top: 14px; padding: 16px; border-radius: 6px; background: var(--rail); display: flex; flex-direction: column; gap: 14px; }
  .sfd-narrow { max-width: 260px; }
  .sfd-checks { display: flex; flex-wrap: wrap; gap: 8px 22px; }
  .sfd-chk { display: flex; align-items: center; gap: 8px; font-size: 14px; }
  .sfd-chk input { width: 17px; height: 17px; margin: 0; accent-color: var(--brand); }
  .sfd-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 14px 16px; }
  .sfd-last { margin-top: 20px; }
  .sfd-sum { margin: 14px 0 0; display: flex; gap: 8px; align-items: flex-start; font-size: 13px; line-height: 1.5; color: var(--ink-2); }
  .sfd-sum :global(svg) { flex: none; margin-top: 2px; color: var(--ink-3); }
  .sfd-err { margin: 14px 0 0; display: flex; gap: 8px; align-items: flex-start; font-size: 14px; line-height: 1.45; color: var(--danger); }
  .sfd-err :global(svg) { flex: none; margin-top: 1px; }
  .sfd-acts { margin-top: 20px; display: flex; justify-content: flex-end; gap: 8px; }
  @media (max-width: 560px) { .sfd-grid, .sfd-cols { grid-template-columns: 1fr; } }
</style>
