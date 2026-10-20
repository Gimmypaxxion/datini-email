<script lang="ts">
  // Riquadro di lettura: messaggio selezionato, stato vuoto, oppure
  // scrittura nel riquadro (vista nuova).
  import Icon from '../lib/Icon.svelte';
  import Composer from './Composer.svelte';
  import { store } from '../lib/store.svelte';
  import { initials, tint } from '../lib/format';
  import { ME } from '../lib/mock';

  const m = $derived(store.selected);
  const t = $derived(m ? (m.sent ? { bg: 'var(--av1-bg)', fg: 'var(--av1-fg)' } : tint(m.from)) : null);
</script>

<main class="pane-read">
  {#if store.paneCompose}
    <section class="sheet compose rise" aria-label={store.c?.subj || 'Nuovo messaggio'}>
      <button class="ibtn btn popout" onclick={() => store.popOut()} aria-label="Apri in una finestra separata" title="Apri in una finestra separata"><Icon name="external-link" /></button>
      <Composer inset={60} />
    </section>
  {:else if m && t}
    <article class="sheet scroll msg">
      <div class="title">
        <h2>{m.subject}</h2>
        {#if m.cat}<span class="cat"><Icon name="tag" size={13} stroke={1.8} />{m.cat}</span>{/if}
      </div>
      <div class="hdr">
        <span class="avatar" aria-hidden="true" style="width:44px;height:44px;font-size:15px;background:{t.bg};color:{t.fg}">{m.sent ? ME.initials : initials(m.from)}</span>
        <div class="who">
          <p class="from"><strong>{m.sent ? ME.name : m.from}</strong> <span class="addr">{m.sent ? `<${ME.email}>` : m.email ? `<${m.email}>` : ''}</span></p>
          <p class="to">A: {m.to || ME.name}</p>
          {#if m.cc}<p class="to cc">Cc: {m.cc}</p>{/if}
        </div>
        <div class="acts">
          <div class="row-btns">
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('reply')} aria-label="Rispondi" title="Rispondi (Ctrl+R)"><Icon name="arrow-back-up" size={19} /></button>
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('replyAll')} aria-label="Rispondi a tutti" title="Rispondi a tutti (Ctrl+Maiusc+R)"><Icon name="arrow-back-up-double" size={19} /></button>
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('forward')} aria-label="Inoltra" title="Inoltra (Ctrl+F)"><Icon name="arrow-forward-up" size={19} /></button>
            <button class="ibtn btn" aria-label="Altre azioni" title="Altre azioni"><Icon name="dots" size={19} /></button>
          </div>
          <span class="date">{m.dateLong}</span>
        </div>
      </div>
      {#if m.atts.length}
        <div class="atts">
          {#each m.atts as a (a.name)}
            <button class="btn att" aria-label="Scarica {a.name}, {a.size}">
              <span class="ic" aria-hidden="true" style:color={a.kind === 'zip' ? 'var(--amber)' : 'var(--danger)'}>
                <Icon name={a.kind === 'zip' ? 'file-zip' : 'file-type-pdf'} />
              </span>
              <span class="meta"><span class="n">{a.name}</span><span class="s">{a.size}</span></span>
            </button>
          {/each}
        </div>
      {/if}
      <div class="text">
        {#each m.body as p, i (i)}<p>{p}</p>{/each}
        {#if m.sig}<p class="sig">{m.sig}</p>{/if}
      </div>
      <div class="foot">
        <button class="btn obtn" onclick={() => store.openCompose('reply')}><Icon name="arrow-back-up" size={17} style="color:var(--brand-ink)" />Rispondi</button>
        <button class="btn obtn" onclick={() => store.openCompose('replyAll')}><Icon name="arrow-back-up-double" size={17} style="color:var(--brand-ink)" />Rispondi a tutti</button>
        <button class="btn obtn" onclick={() => store.openCompose('forward')}><Icon name="arrow-forward-up" size={17} style="color:var(--brand-ink)" />Inoltra</button>
      </div>
    </article>
  {:else}
    <div class="sheet none">
      <Icon name="mail-opened" size={64} stroke={1.1} style="color:var(--brand-ink)" />
      <p class="t">Nessun messaggio selezionato</p>
      <p class="d">Scegli un messaggio dall’elenco per leggerlo qui.</p>
    </div>
  {/if}
</main>

<style>
  .pane-read { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; box-sizing: border-box; padding: 0 10px 10px 8px; }
  .sheet { border-radius: 8px; background: var(--sheet); box-shadow: var(--sheet-shadow); }
  .compose { position: relative; flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
  .popout { position: absolute; top: 12px; right: 12px; z-index: 1; }
  .msg { flex: 1; min-height: 0; overflow: auto; box-sizing: border-box; padding: 20px 28px 32px; }
  .title { display: flex; align-items: flex-start; gap: 12px; }
  h2 { margin: 0; flex: 1; min-width: 0; font-size: 20px; font-weight: 600; line-height: 1.3; text-wrap: balance; }
  .cat { flex: none; margin-top: 2px; height: 22px; display: inline-flex; align-items: center; gap: 6px; padding: 0 8px; border-radius: 4px; background: var(--brand-soft); color: var(--brand-ink); font-size: 12.5px; font-weight: 600; }
  .hdr { margin-top: 16px; display: grid; grid-template-columns: 44px minmax(0, 1fr) auto; column-gap: 14px; align-items: start; }
  .who { min-width: 0; padding-top: 1px; }
  .from { margin: 0; font-size: 15px; line-height: 1.35; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .from strong { font-weight: 600; }
  .addr { color: var(--ink-3); font-size: 13.5px; }
  .to { margin: 3px 0 0; font-size: 13.5px; color: var(--ink-2); }
  .to.cc { margin-top: 1px; }
  .acts { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
  .row-btns { display: flex; gap: 2px; }
  .date { font-size: 12.5px; color: var(--ink-3); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .atts { margin: 14px 0 0 58px; display: flex; flex-wrap: wrap; gap: 8px; }
  .att { max-width: 280px; height: 50px; display: flex; align-items: center; gap: 10px; padding: 0 14px 0 8px; border: 1px solid var(--rule-2); border-radius: 4px; background: transparent; text-align: left; }
  .att .ic { width: 32px; height: 32px; flex: none; border-radius: 4px; display: grid; place-items: center; background: var(--rail); }
  .meta { min-width: 0; display: flex; flex-direction: column; }
  .meta .n { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta .s { font-size: 12.5px; color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .text { margin: 22px 0 0 58px; max-width: 68ch; font-size: 15px; line-height: 1.55; }
  .text p { margin: 0 0 14px; white-space: pre-line; }
  .text .sig { margin: 18px 0 0; padding-top: 12px; box-shadow: inset 0 1px 0 var(--rule); font-size: 13.5px; color: var(--ink-2); }
  .foot { margin: 26px 0 0 58px; display: flex; flex-wrap: wrap; gap: 8px; }
  .none { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 0 32px; }
  .none .t { margin: 16px 0 4px; font-size: 15px; font-weight: 600; }
  .none .d { margin: 0; font-size: 13.5px; color: var(--ink-2); }
  @media (max-width: 760px) { .pane-read { display: none; } }
</style>
