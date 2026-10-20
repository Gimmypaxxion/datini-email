<script lang="ts">
  // Campi di un messaggio in scrittura: Invia, destinatari A e Cc, oggetto,
  // allegati, testo e piede con stato della bozza. Usato nel riquadro di
  // lettura e nella finestra separata.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { initials } from '../lib/format';

  let { inset = 0 }: { inset?: number } = $props();

  const c = $derived(store.c!);
  const sendPos = $derived(store.prefs.ribbon === 'estesa' ? 'sinistra' : 'alto');
  const uid = Math.random().toString(36).slice(2, 8);

  let toEl: HTMLInputElement | undefined = $state();
  let bodyEl: HTMLTextAreaElement | undefined = $state();

  // Come in Outlook: il cursore va nel campo A per un messaggio nuovo o
  // inoltrato, all'inizio del testo per una risposta.
  $effect(() => {
    const t = setTimeout(() => {
      if (!store.c) return;
      if (store.c.to.length && bodyEl) {
        bodyEl.focus();
        bodyEl.setSelectionRange(0, 0);
        bodyEl.scrollTop = 0;
      } else {
        toEl?.focus();
      }
    }, 0);
    return () => clearTimeout(t);
  });

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      store.send();
    }
  }
</script>

{#snippet chips(key: 'to' | 'cc')}
  {#each c[key] as name, i (name + i)}
    <span class="chip">
      <span class="avatar" aria-hidden="true">{initials(name)}</span>{name}<button class="ibtn btn" onclick={() => store.removeRecipient(key, i)} aria-label="Rimuovi {name}"><Icon name="x" size={13} stroke={1.8} /></button>
    </span>
  {/each}
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="composer" onkeydown={onKey}>
  <div class="cmp-h" data-send={sendPos} style:padding-right={inset ? `${inset}px` : undefined}>
    <button class="primary cmp-send" onclick={() => store.send()} title="Invia (Ctrl+Invio)"><Icon name="send" size={20} />Invia</button>
    <div style="min-width:0">
      <div class="field">
        <label for="to-{uid}">A</label>
        <div class="chips">
          {@render chips('to')}
          <input
            bind:this={toEl}
            id="to-{uid}"
            class="ph bare grow"
            value={store.toInput}
            oninput={(e) => store.recipientInput('to', e.currentTarget.value)}
            onkeydown={(e) => store.recipientKey('to', e)}
            placeholder={c.to.length ? '' : 'Aggiungi destinatari'}
            autocomplete="off"
          />
        </div>
      </div>
      {#if store.err}
        <p role="alert" class="err"><Icon name="alert-circle" size={15} stroke={1.7} />{store.err}</p>
      {/if}
      <div class="field">
        <label for="cc-{uid}">Cc</label>
        <div class="chips">
          {@render chips('cc')}
          <input
            id="cc-{uid}"
            class="ph bare grow"
            value={store.ccInput}
            oninput={(e) => store.recipientInput('cc', e.currentTarget.value)}
            onkeydown={(e) => store.recipientKey('cc', e)}
            autocomplete="off"
          />
        </div>
        <button class="lbtn ccn">Ccn</button>
      </div>
    </div>
  </div>
  <div class="pad">
    <div class="field subj">
      <label for="subj-{uid}" class="sr">Oggetto</label>
      <input id="subj-{uid}" class="ph bare" value={c.subj} oninput={(e) => store.setSubject(e.currentTarget.value)} placeholder="Aggiungi un oggetto" autocomplete="off" />
    </div>
  </div>
  {#if c.atts.length}
    <div class="atts">
      {#each c.atts as a (a.name)}
        <span class="att"><Icon name="paperclip" size={16} style="color:var(--brand-ink)" />{a.name}<span class="sz">{a.size}</span></span>
      {/each}
    </div>
  {/if}
  <label for="body-{uid}" class="sr">Testo del messaggio</label>
  <textarea bind:this={bodyEl} id="body-{uid}" class="ph bare scroll body" value={c.body} oninput={(e) => store.setBody(e.currentTarget.value)} placeholder="Scrivi il messaggio"></textarea>
  <div class="foot">
    <span class="status">{c.saved ? `Bozza salvata alle ${c.saved}` : 'Bozza non ancora salvata'}</span>
    <button class="btn discard" onclick={() => store.discard()}><Icon name="trash" size={17} style="color:var(--ink-2)" />Elimina bozza</button>
  </div>
</div>

<style>
  .composer { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .grow { flex: 1; min-width: 140px; height: 28px; font-size: 14px; }
  .err { margin: 6px 0 0 38px; display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--danger); }
  .ccn { flex: none; height: 28px; padding: 0 4px; font-size: 13.5px; }
  .pad { padding: 0 24px; }
  .subj { min-height: 44px; }
  .subj input { flex: 1; min-width: 0; height: 32px; font-size: 15px; font-weight: 600; }
  .atts { padding: 10px 24px 0; display: flex; flex-wrap: wrap; gap: 8px; }
  .att { height: 30px; display: inline-flex; align-items: center; gap: 8px; padding: 0 12px 0 8px; border: 1px solid var(--rule-2); border-radius: 4px; font-size: 13px; }
  .sz { color: var(--ink-3); }
  .body { flex: 1; min-height: 0; margin: 0 24px; padding: 16px 0; font-size: 15px; line-height: 1.55; resize: none; outline: none; }
  .foot { display: flex; align-items: center; gap: 12px; padding: 8px 14px 10px 24px; box-shadow: inset 0 1px 0 var(--rule); }
  .status { flex: 1; font-size: 12.5px; color: var(--ink-3); }
  .discard { height: 32px; display: flex; align-items: center; gap: 6px; padding: 0 12px 0 10px; border: 0; border-radius: 4px; background: transparent; font-size: 13.5px; }
</style>
