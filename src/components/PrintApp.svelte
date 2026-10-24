<script lang="ts">
  // Finestra di stampa di un messaggio: intestazione come in Outlook (Da,
  // Inviato, A, Cc, Oggetto, Allegati) e testo. La finestra di dialogo del
  // sistema si apre da sola; la barra in alto non viene stampata.
  import Icon from '../lib/Icon.svelte';
  import { store } from '../lib/store.svelte';
  import { fileSize, longDate, personFull } from '../lib/format';
  import { errorMessage, type MessageDetail, type Person } from '../lib/types';

  let { id, remote }: { id: number; remote: boolean } = $props();

  let m = $state<MessageDetail | null>(null);
  let error = $state('');
  let printed = false;

  const me = $derived(store.accounts.find((a) => a.id === m?.accountId));
  const files = $derived(m ? m.attachments.filter((a) => !a.inline) : []);
  const paragraphs = $derived(m && !m.html ? m.text.replace(/\r\n/g, '\n').split(/\n\s*\n/).filter((p) => p.trim()) : []);

  const people = (list: Person[]) => list.map(personFull).join('; ');

  $effect(() => {
    document.documentElement.dataset.theme = 'chiaro';
    document.body.classList.add('print-page');
    void (async () => {
      try {
        await store.reloadAccounts().catch(() => {});
        m = await store.backend.message(id, remote);
        document.title = `${m.subject || 'Messaggio'} - Stampa`;
      } catch (e) {
        error = errorMessage(e);
      }
    })();
  });

  /** Il testo HTML dentro un'area isolata: i suoi stili non toccano la pagina. */
  function shadow(host: HTMLElement, html: string) {
    const root = host.attachShadow({ mode: 'open' });
    const set = (h: string) => {
      root.innerHTML = `<style>:host{display:block;font-family:Aptos,Calibri,Arial,Helvetica,sans-serif;font-size:11pt;line-height:1.45;overflow-wrap:anywhere}img{max-width:100%;height:auto}table{max-width:100%}a{color:inherit}</style>${h}`;
    };
    set(html);
    // Quando il testo è pronto (immagini comprese) si apre la stampa.
    const waits = [...root.querySelectorAll('img')]
      .filter((img) => !img.complete)
      .map(
        (img) =>
          new Promise<void>((r) => {
            img.addEventListener('load', () => r(), { once: true });
            img.addEventListener('error', () => r(), { once: true });
          }),
      );
    void Promise.race([Promise.all(waits), new Promise((r) => setTimeout(r, 3000))]).then(() => autoPrint());
    return { update: set };
  }

  function autoPrint() {
    if (printed) return;
    printed = true;
    setTimeout(() => void print(), 250);
  }

  async function print() {
    try {
      await store.backend.printPage();
    } catch (e) {
      error = errorMessage(e);
    }
  }

  async function close() {
    if (store.backend.real) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().destroy();
    } else {
      globalThis.close?.();
    }
  }

  $effect(() => {
    if (m && !m.html) autoPrint();
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape') void close();
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      void print();
    }
  }}
/>

<div class="bar">
  <span class="t">Stampa</span>
  <button class="primary-btn" onclick={print} disabled={!m}><Icon name="printer" size={17} />Stampa</button>
  <button class="sec-btn" onclick={close}>Chiudi</button>
</div>

<div class="paper">
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if !m}
    <p class="wait">Preparazione della stampa…</p>
  {:else}
    {#if me}<p class="owner">{me.name || me.email}</p>{/if}
    <table class="head">
      <tbody>
        <tr><th>Da:</th><td>{personFull(m.from)}</td></tr>
        <tr><th>Inviato:</th><td>{longDate(m.date)}</td></tr>
        {#if m.to.length}<tr><th>A:</th><td>{people(m.to)}</td></tr>{/if}
        {#if m.cc.length}<tr><th>Cc:</th><td>{people(m.cc)}</td></tr>{/if}
        <tr><th>Oggetto:</th><td>{m.subject || '(nessun oggetto)'}</td></tr>
        {#if files.length}<tr><th>Allegati:</th><td>{files.map((a) => `${a.name} (${fileSize(a.size)})`).join('; ')}</td></tr>{/if}
      </tbody>
    </table>
    <div class="body">
      {#if m.html}
        <div use:shadow={m.html}></div>
      {:else}
        {#each paragraphs as p, i (i)}<p class="tp">{p}</p>{/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  :global(body.print-page) { overflow: auto; background: var(--ground); }
  .bar { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; gap: 8px; padding: 10px 16px; background: var(--sheet); box-shadow: inset 0 -1px 0 var(--rule); }
  .bar .t { flex: 1; font-size: 15px; font-weight: 600; }
  .paper { box-sizing: border-box; width: min(794px, calc(100% - 32px)); min-height: 600px; margin: 16px auto 32px; padding: 48px 56px; background: var(--sheet); box-shadow: var(--sheet-shadow); color: var(--ink); }
  .owner { margin: 0 0 6px; padding-bottom: 6px; font-size: 15pt; font-weight: 700; border-bottom: 3px solid var(--ink); }
  .head { margin-top: 10px; border-collapse: collapse; font-size: 10.5pt; line-height: 1.4; }
  .head th { padding: 1px 16px 1px 0; text-align: left; vertical-align: top; font-weight: 700; white-space: nowrap; }
  .head td { padding: 1px 0; overflow-wrap: anywhere; }
  .body { margin-top: 22px; }
  .tp { margin: 0 0 1em; white-space: pre-line; font-size: 11pt; line-height: 1.45; overflow-wrap: anywhere; }
  .wait, .err { margin: 0; font-size: 14px; color: var(--ink-2); }
  .err { color: var(--danger); }
  @media print {
    .bar { display: none; }
    :global(body.print-page) { background: none; }
    .paper { width: auto; min-height: 0; margin: 0; padding: 0; box-shadow: none; }
  }
</style>
