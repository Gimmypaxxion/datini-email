<script lang="ts">
  // Riquadro di lettura: il messaggio selezionato (testo semplice o HTML
  // isolato), gli allegati, l'avviso sulle immagini bloccate; oppure la
  // scrittura nel riquadro (vista nuova).
  import Icon from '../lib/Icon.svelte';
  import type { IconName } from '../lib/icons';
  import Composer from './Composer.svelte';
  import MailFrame from './MailFrame.svelte';
  import Menu, { type MenuItem } from './Menu.svelte';
  import { store } from '../lib/store.svelte';
  import { fileSize, initials, linkify, longDate, peopleText, personLabel, tint } from '../lib/format';
  import type { AttachmentInfo } from '../lib/types';

  const m = $derived(store.detail && store.detail.id === store.sel ? store.detail : null);
  const name = $derived(m ? personLabel(m.from) || 'Mittente sconosciuto' : '');
  const t = $derived(tint(name));
  const files = $derived(m ? m.attachments.filter((a) => !a.inline) : []);
  const paragraphs = $derived(m && !m.html ? m.text.replace(/\r\n/g, '\n').split(/\n\s*\n/).filter((p) => p.trim()) : []);

  let moreBtn: HTMLButtonElement | undefined = $state();
  let moreOpen = $state(false);

  function fileIcon(a: AttachmentInfo): { icon: IconName; color: string } {
    const n = a.name.toLowerCase();
    if (a.mime === 'application/pdf' || n.endsWith('.pdf')) return { icon: 'file-type-pdf', color: 'var(--danger)' };
    if (/\.(zip|rar|7z)$/.test(n)) return { icon: 'file-zip', color: 'var(--amber)' };
    if (/\.(docx?|odt|rtf)$/.test(n)) return { icon: 'file-type-doc', color: 'var(--brand-ink)' };
    if (/\.(xlsx?|ods|csv)$/.test(n)) return { icon: 'file-type-xls', color: 'var(--green)' };
    if (/\.(pptx?|odp)$/.test(n)) return { icon: 'file-type-ppt', color: 'var(--amber)' };
    if (a.mime.startsWith('image/')) return { icon: 'file-type-jpg', color: 'var(--brand-ink)' };
    return { icon: 'file', color: 'var(--ink-2)' };
  }

  const moreItems = $derived<MenuItem[]>(
    m
      ? [
          { label: 'Segna come non letto', icon: 'mail-opened', onclick: () => store.setRead(m.id, false) },
          { label: m.flagged ? 'Rimuovi contrassegno' : 'Contrassegna', icon: 'flag', onclick: () => store.toggleFlag(m.id) },
          { kind: 'sep', label: '' },
          { label: 'Salva con nome', icon: 'download', onclick: () => store.saveMessageAs(), disabled: !store.backend.real },
          { label: 'Segnala come posta indesiderata', icon: 'ban', onclick: () => store.moveMsg(m.id, 'junk') },
          { label: 'Elimina', icon: 'trash', onclick: () => store.deleteMsg(m.id) },
        ]
      : [],
  );
</script>

<main class="pane-read">
  {#if store.paneCompose}
    <section class="sheet compose rise" aria-label={store.c?.subject || 'Nuovo messaggio'}>
      <button class="ibtn btn popout" onclick={() => store.popOut()} aria-label="Apri in una finestra separata" title="Apri in una finestra separata"><Icon name="external-link" /></button>
      <Composer inset={60} />
    </section>
  {:else if m}
    <article class="sheet scroll msg" aria-label={m.subject || 'Messaggio senza oggetto'}>
      <div class="title">
        <h2>{m.subject || '(nessun oggetto)'}</h2>
        {#if m.flagged}<span class="flag" title="Contrassegnato"><Icon name="flag-filled" size={16} style="color:var(--danger)" /></span>{/if}
      </div>
      <div class="hdr">
        <span class="avatar" aria-hidden="true" style="width:44px;height:44px;font-size:15px;background:{t.bg};color:{t.fg}">{initials(name)}</span>
        <div class="who">
          <p class="from"><strong>{name}</strong> {#if m.from.email && m.from.email !== name}<span class="addr">&lt;{m.from.email}&gt;</span>{/if}</p>
          {#if m.to.length}<p class="to">A: {peopleText(m.to)}</p>{/if}
          {#if m.cc.length}<p class="to cc">Cc: {peopleText(m.cc)}</p>{/if}
          {#if m.bcc.length}<p class="to cc">Ccn: {peopleText(m.bcc)}</p>{/if}
        </div>
        <div class="acts">
          <div class="row-btns">
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('reply')} aria-label="Rispondi" title="Rispondi (Ctrl+R)"><Icon name="arrow-back-up" size={19} /></button>
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('replyAll')} aria-label="Rispondi a tutti" title="Rispondi a tutti (Ctrl+Maiusc+R)"><Icon name="arrow-back-up-double" size={19} /></button>
            <button class="ibtn btn" style="color:var(--brand-ink)" onclick={() => store.openCompose('forward')} aria-label="Inoltra" title="Inoltra (Ctrl+F)"><Icon name="arrow-forward-up" size={19} /></button>
            <button class="ibtn btn" bind:this={moreBtn} onclick={() => (moreOpen = !moreOpen)} aria-label="Altre azioni" aria-haspopup="menu" aria-expanded={moreOpen} title="Altre azioni"><Icon name="dots" size={19} /></button>
          </div>
          <span class="date">{longDate(m.date)}</span>
        </div>
      </div>

      {#if m.remoteImages > 0 && !m.remoteAllowed}
        <div class="note-bar info privacy" role="note">
          <Icon name="photo-off" style="color:var(--brand-ink)" />
          <span class="t">Per proteggere la tua privacy, Datini non ha scaricato le immagini esterne di questo messaggio.</span>
          <button class="act" onclick={() => store.showRemoteImages()}>Scarica immagini</button>
        </div>
      {/if}

      {#if files.length}
        <ul class="atts" aria-label="Allegati">
          {#each files as a (a.index)}
            {@const fi = fileIcon(a)}
            <li class="att">
              <button class="btn open" onclick={() => store.openAttachment(a.index)} title="Apri {a.name}">
                <span class="ic" aria-hidden="true" style:color={fi.color}><Icon name={fi.icon} /></span>
                <span class="meta"><span class="n">{a.name}</span><span class="s">{fileSize(a.size)}</span></span>
              </button>
              {#if store.backend.real}
                <button class="ibtn btn save" onclick={() => store.saveAttachment(a.index, a.name)} aria-label="Salva {a.name}" title="Salva con nome"><Icon name="download" size={16} /></button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}

      <div class="body">
        {#if m.html}
          <MailFrame html={m.html} onlink={(u) => store.openLink(u)} />
        {:else}
          <div class="text">
            {#each paragraphs as p, i (i)}
              <p>{#each linkify(p) as part, j (j)}{#if part.href}<a href={part.href} onclick={(e) => { e.preventDefault(); store.openLink(part.href!); }}>{part.text}</a>{:else}{part.text}{/if}{/each}</p>
            {/each}
          </div>
        {/if}
      </div>
      <div class="foot">
        <button class="btn obtn" onclick={() => store.openCompose('reply')}><Icon name="arrow-back-up" size={17} style="color:var(--brand-ink)" />Rispondi</button>
        <button class="btn obtn" onclick={() => store.openCompose('replyAll')}><Icon name="arrow-back-up-double" size={17} style="color:var(--brand-ink)" />Rispondi a tutti</button>
        <button class="btn obtn" onclick={() => store.openCompose('forward')}><Icon name="arrow-forward-up" size={17} style="color:var(--brand-ink)" />Inoltra</button>
      </div>
    </article>
    {#if moreOpen && moreBtn}
      <Menu anchor={moreBtn} items={moreItems} label="Altre azioni" onclose={() => (moreOpen = false)} />
    {/if}
  {:else if store.sel !== null && store.detailError}
    <div class="sheet none" role="alert">
      <Icon name="alert-circle" size={56} stroke={1.2} style="color:var(--danger)" />
      <p class="t">Non riesco ad aprire il messaggio</p>
      <p class="d">{store.detailError}</p>
      <button class="sec-btn" style="margin-top:16px" onclick={() => store.loadDetail(store.sel!)}><Icon name="refresh" size={16} />Riprova</button>
    </div>
  {:else if store.sel !== null}
    <div class="sheet msg" aria-busy="true" aria-label="Caricamento del messaggio">
      <span class="skel" style="display:block;width:58%;height:22px"></span>
      <div class="hdr" style="margin-top:18px">
        <span class="skel" style="width:44px;height:44px;border-radius:50%"></span>
        <span style="display:flex;flex-direction:column;gap:8px"><span class="skel" style="width:220px;height:13px"></span><span class="skel" style="width:160px;height:12px"></span></span>
      </div>
      <div style="margin:28px 0 0 58px;display:flex;flex-direction:column;gap:10px;max-width:60ch">
        <span class="skel" style="height:12px"></span><span class="skel" style="height:12px;width:92%"></span><span class="skel" style="height:12px;width:80%"></span>
      </div>
    </div>
  {:else}
    <div class="sheet none">
      <Icon name="mail-opened" size={64} stroke={1.1} style="color:var(--brand-ink)" />
      {#if store.ready && !store.accounts.length}
        <p class="t">Benvenuto in Datini</p>
        <p class="d">Aggiungi il tuo account di posta: Datini trova da solo le impostazioni dei provider più diffusi.</p>
        <button class="primary-btn" style="margin-top:18px" onclick={() => store.openWizard(false)}><Icon name="user-plus" size={17} />Aggiungi account</button>
      {:else}
        <p class="t">Nessun messaggio selezionato</p>
        <p class="d">Scegli un messaggio dall’elenco per leggerlo qui.</p>
      {/if}
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
  h2 { margin: 0; flex: 1; min-width: 0; font-size: 20px; font-weight: 600; line-height: 1.3; text-wrap: balance; overflow-wrap: anywhere; }
  .flag { margin-top: 4px; }
  .hdr { margin-top: 16px; display: grid; grid-template-columns: 44px minmax(0, 1fr) auto; column-gap: 14px; align-items: start; }
  .who { min-width: 0; padding-top: 1px; }
  .from { margin: 0; font-size: 15px; line-height: 1.35; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .from strong { font-weight: 600; }
  .addr { color: var(--ink-3); font-size: 13.5px; }
  .to { margin: 3px 0 0; font-size: 13.5px; color: var(--ink-2); overflow-wrap: anywhere; }
  .to.cc { margin-top: 1px; }
  .acts { display: flex; flex-direction: column; align-items: flex-end; gap: 4px; }
  .row-btns { display: flex; gap: 2px; }
  .date { font-size: 12.5px; color: var(--ink-3); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .privacy { margin: 16px 0 0 58px; border-radius: 4px; }
  .atts { margin: 14px 0 0 58px; padding: 0; list-style: none; display: flex; flex-wrap: wrap; gap: 8px; }
  .att { position: relative; display: flex; align-items: center; max-width: 300px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--rule-2); }
  .open { min-width: 0; height: 50px; display: flex; align-items: center; gap: 10px; padding: 0 14px 0 8px; border: 0; border-radius: 4px; background: transparent; text-align: left; }
  .save { width: 30px; height: 30px; margin-right: 6px; flex: none; }
  .ic { width: 32px; height: 32px; flex: none; border-radius: 4px; display: grid; place-items: center; background: var(--rail); }
  .meta { min-width: 0; display: flex; flex-direction: column; }
  .meta .n { font-size: 13.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta .s { font-size: 12.5px; color: var(--ink-3); font-variant-numeric: tabular-nums; }
  .body { margin: 22px 0 0 58px; }
  .text { max-width: 72ch; font-size: 15px; line-height: 1.55; overflow-wrap: anywhere; }
  .text p { margin: 0 0 14px; white-space: pre-line; }
  .foot { margin: 26px 0 0 58px; display: flex; flex-wrap: wrap; gap: 8px; }
  .none { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; padding: 0 32px; }
  .none .t { margin: 16px 0 4px; font-size: 15px; font-weight: 600; }
  .none .d { margin: 0; max-width: 46ch; font-size: 13.5px; line-height: 1.5; color: var(--ink-2); }
  @media (max-width: 1200px) { .body, .atts, .foot, .privacy { margin-left: 0; } }
  @media (max-width: 760px) { .pane-read { display: none; } }
</style>
