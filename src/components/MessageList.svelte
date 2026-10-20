<script lang="ts">
  // Elenco messaggi: schede Prioritaria/Altre, gruppi per data, azioni rapide
  // al passaggio del mouse e avvisi (offline, invio, bozza salvata).
  import Icon from '../lib/Icon.svelte';
  import { store, GROUPS } from '../lib/store.svelte';
  import { initials, tint } from '../lib/format';
  import type { FolderKey } from '../lib/mock';

  const EMPTY: Partial<Record<FolderKey, [string, string]>> = {
    inbox: ['Niente di nuovo qui', 'Non ci sono messaggi in questa scheda.'],
    ordini: ['Niente di nuovo qui', 'Non ci sono messaggi nuovi per ordini@valliarredi.it.'],
    sent: ['Nessun messaggio inviato', 'I messaggi che invii compaiono qui.'],
    drafts: ['Nessuna bozza', 'I messaggi che inizi e non invii restano qui.'],
    archive: ['L’archivio è vuoto', 'Archivia un messaggio per toglierlo dalla posta in arrivo senza eliminarlo.'],
    junk: ['Nessun messaggio indesiderato', 'Qui arrivano i messaggi che segnali come indesiderati.'],
    deleted: ['Nessun elemento eliminato', 'I messaggi eliminati restano qui finché non svuoti la cartella.'],
  };

  const q = $derived(store.q.trim());
  const groups = $derived(
    GROUPS.map((g) => ({ label: g, items: store.list.filter((m) => m.group === g) })).filter((g) => g.items.length),
  );
  const empty = $derived<[string, string]>(
    q
      ? ['Nessun risultato', `Nessun messaggio contiene “${q}”. Prova con un nome o con una parola dell’oggetto.`]
      : (EMPTY[store.folder] ?? ['Cartella vuota', 'Usa Sposta in per archiviare qui i messaggi.']),
  );
</script>

<section class="pane-list" aria-label="Elenco messaggi">
  {#if store.offline}
    <div role="status" class="offline">
      <Icon name="wifi-off" style="color:var(--warn)" />
      <span>Sei offline. I messaggi in uscita partono appena torna la connessione.</span>
      <button class="btn retry">Riprova</button>
    </div>
  {/if}

  <div class="head">
    {#if !store.showFolders}
      <button class="ibtn btn" style="margin-left:-6px" onclick={() => store.toggleFolders()} aria-label="Mostra riquadro cartelle" title="Mostra riquadro cartelle"><Icon name="menu-2" size={20} /></button>
    {/if}
    <h1>{store.listTitle}</h1>
    <button class="ibtn btn" aria-label="Seleziona messaggi" title="Seleziona"><Icon name="checkbox" /></button>
    <button class="ibtn btn" aria-label="Filtra" title="Filtra"><Icon name="filter" /></button>
  </div>

  {#if store.folder === 'inbox' && !q}
    <div class="tabs" role="tablist" aria-label="Schede della posta in arrivo">
      {#each [['prio', 'Prioritaria'], ['altre', 'Altre']] as [id, label] (id)}
        {@const on = store.tab === id}
        <button role="tab" aria-selected={on} class:on onclick={() => store.pickTab(id as 'prio' | 'altre')}>{label}</button>
      {/each}
    </div>
  {/if}

  <div class="scroll body">
    {#if !store.list.length}
      <div class="empty">
        <Icon name="inbox" size={56} stroke={1.2} style="color:var(--brand-ink)" />
        <p class="t">{empty[0]}</p>
        <p class="d">{empty[1]}</p>
      </div>
    {/if}

    {#each groups as g (g.label)}
      <h2 class="group">{g.label}</h2>
      {#each g.items as m (m.id)}
        {@const on = m.id === store.sel}
        {@const t = tint(m.from)}
        {@const strong = m.unread && !on}
        <div class="row" data-sel={on ? 'si' : 'no'}>
          <button
            class="row-main"
            onclick={() => store.pickMsg(m.id)}
            aria-current={on ? 'true' : undefined}
            aria-label={`${m.unread ? 'Non letto. ' : ''}${m.draft ? 'Bozza. ' : ''}${m.from}, ${m.subject}, ${m.time}`}
          >
            <span class="bar" aria-hidden="true" style:background={m.unread ? 'var(--brand)' : 'transparent'}></span>
            <span class="avatar" aria-hidden="true" style="width:36px;height:36px;font-size:13px;background:{t.bg};color:{t.fg}">{initials(m.from)}</span>
            <span class="txt">
              <span class="l1">
                <span class="from" style:font-weight={m.unread ? 600 : 400}>{m.from}</span>
                <span class="time" style:font-weight={m.unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink-3)'}>{m.time}</span>
              </span>
              <span class="l2">
                <span class="subj" style:font-weight={m.unread ? 600 : 400} style:color={strong ? 'var(--brand-ink)' : 'var(--ink)'}>{m.subject}</span>
                {#if m.atts.length}<Icon name="paperclip" size={15} stroke={1.7} style="color:var(--ink-3)" />{/if}
                {#if m.flagged}<Icon name="flag-filled" size={15} style="color:var(--danger)" />{/if}
              </span>
              <span class="prev">{#if m.draft}<span class="draft">[Bozza] </span>{/if}{m.preview}</span>
            </span>
          </button>
          <div class="row-acts">
            <button class="ibtn btn" onclick={() => store.moveMsg(m.id, 'deleted')} aria-label="Elimina" title="Elimina"><Icon name="trash" size={16} /></button>
            <button class="ibtn btn" style="color:var(--danger)" onclick={() => store.toggleFlag(m.id)} aria-label={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'} title={m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna'}><Icon name="flag" size={16} /></button>
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.toggleRead(m.id)} aria-label={m.unread ? 'Segna come letto' : 'Segna come non letto'} title={m.unread ? 'Segna come letto' : 'Segna come non letto'}><Icon name="mail-opened" size={16} /></button>
          </div>
        </div>
      {/each}
    {/each}
  </div>

  {#if store.toast}
    <div role="status" class="toast rise">
      <Icon name={store.toast.kind === 'sent' ? 'send' : 'file-pencil'} />
      <span class="msg">{store.toast.kind === 'sent' ? 'Messaggio inviato' : 'Bozza salvata in Bozze'}</span>
      {#if store.toast.kind === 'sent'}
        <button class="undo" onclick={() => store.undoSend()}>Annulla</button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .pane-list { position: relative; width: 420px; flex: none; min-height: 0; display: flex; flex-direction: column; margin: 0 0 10px; border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); overflow: hidden; }
  .offline { display: flex; align-items: center; gap: 10px; padding: 8px 8px 8px 14px; background: var(--warn-soft); color: var(--ink); font-size: 13.5px; line-height: 1.35; }
  .offline span { flex: 1; }
  .retry { height: 30px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; font-size: 13.5px; font-weight: 600; flex: none; }
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
  .l1 { display: flex; align-items: baseline; gap: 10px; }
  .from { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 14px; }
  .time { flex: none; font-size: 12.5px; font-variant-numeric: tabular-nums; }
  .l2 { display: flex; align-items: center; gap: 6px; }
  .subj { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13.5px; }
  .prev { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--ink-3); }
  .draft { color: var(--danger); font-weight: 600; }
  .toast { position: absolute; left: 14px; right: 14px; bottom: 14px; z-index: 5; display: flex; align-items: center; gap: 12px; min-height: 44px; box-sizing: border-box; padding: 6px 6px 6px 14px; border-radius: 4px; background: var(--ink); color: var(--sheet); box-shadow: var(--lift); font-size: 14px; }
  .toast .msg { flex: 1; }
  .undo { height: 32px; padding: 0 12px; border: 0; border-radius: 4px; background: transparent; color: var(--sheet); font-size: 14px; font-weight: 600; text-decoration: underline; text-underline-offset: 3px; }
  .undo:focus-visible { outline-color: var(--sheet); }
  @media (max-width: 760px) { .pane-list { width: auto; flex: 1 1 auto; margin-right: 10px; } }
</style>
