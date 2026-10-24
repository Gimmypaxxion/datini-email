<script lang="ts">
  // Schede "Messaggio", "Inserisci" e "Formato testo" del messaggio in
  // scrittura, usate sia nella barra principale (scrittura nel riquadro) sia
  // nelle finestre del messaggio. Gruppi e posizioni come in Outlook: Appunti,
  // Testo base, Includi; nella scheda Inserisci tabelle, immagini,
  // collegamenti; in Formato testo il formato del messaggio, Carattere e Paragrafo.
  // I pulsanti di formato non prendono il fuoco: il cursore resta nel testo.
  import RB from './RB.svelte';
  import Icon from '../lib/Icon.svelte';
  import Menu, { type MenuItem } from './Menu.svelte';
  import Popover from './Popover.svelte';
  import SchedulePicker from './SchedulePicker.svelte';
  import type { IconName } from '../lib/icons';
  import { store } from '../lib/store.svelte';
  import { FONTS, HIGHLIGHTS, SIZES, STANDARD_COLORS, THEME_COLORS, composeEditor as ed, fontStack } from '../lib/editor.svelte';

  let { tab }: { tab: 'messaggio' | 'inserisci' | 'formato' } = $props();

  type PopKind = 'font' | 'size' | 'color' | 'hl' | 'table' | 'sig' | 'case' | 'spacing';
  let pop = $state<{ kind: PopKind; anchor: HTMLElement } | null>(null);
  let schedOpen = $state(false);
  let schedEl = $state<HTMLElement | null>(null);
  let hover = $state({ r: 0, c: 0 });
  let imgInput: HTMLInputElement | undefined = $state();
  let colorInput: HTMLInputElement | undefined = $state();

  const fontLabel = $derived(ed.font || store.font.family);
  const sizeLabel = $derived(String(ed.size ?? store.font.size).replace('.', ','));

  function open(kind: PopKind, e: MouseEvent) {
    const a = e.currentTarget as HTMLElement;
    pop = pop?.kind === kind ? null : { kind, anchor: a };
    if (kind === 'sig') void store.reloadSignatures();
  }
  const close = () => (pop = null);
  const keep = (e: MouseEvent) => e.preventDefault();

  const sigItems = $derived<MenuItem[]>([
    ...(store.signatures.length
      ? store.signatures.map((s) => ({ label: s.name, icon: 'signature' as IconName, onclick: () => store.insertSignature(s.id) }))
      : [{ label: 'Nessuna firma', disabled: true }]),
    { kind: 'sep', label: '' },
    { label: store.signatures.length ? 'Firme…' : 'Crea una firma…', icon: 'settings', onclick: () => store.openSettings('composizione') },
  ]);

  const caseItems: MenuItem[] = [
    { label: 'Tipo frase.', onclick: () => ed.setCase('sentence') },
    { label: 'minuscole', onclick: () => ed.setCase('lowerCase') },
    { label: 'MAIUSCOLE', onclick: () => ed.setCase('upperCase') },
    { label: 'Iniziali Maiuscole', onclick: () => ed.setCase('capitalize') },
  ];
  const spacingItems: MenuItem[] = [1, 1.15, 1.5, 2, 2.5, 3].map((n) => ({
    label: n.toFixed(n === 1.15 ? 2 : 1).replace('.', ','),
    onclick: () => ed.setSpacing(n),
  }));
  const plain = $derived(!!store.c?.plain);

  async function paste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) ed.insertHtml(text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/\r?\n/g, '<br>'));
    } catch {
      store.hint('Per incollare premi Ctrl+V nel testo del messaggio.');
    }
  }

  function clip(cmd: 'cut' | 'copy') {
    ed.cmd(cmd);
  }

  function pickImages(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = [...(input.files ?? [])];
    input.value = '';
    if (files.length) void ed.insertImageFiles(files);
  }
</script>

{#snippet fb(icon: IconName, label: string, title: string, on: boolean, act: () => void, cls = '')}
  <button class="rb ico {cls}" aria-label={label} {title} aria-pressed={on} onmousedown={keep} onclick={act} disabled={!ed.ready}>
    <Icon name={icon} />
  </button>
{/snippet}

{#snippet letter(ch: string, style: string, label: string, title: string, on: boolean, act: () => void)}
  <button class="rb ico lt" aria-label={label} {title} aria-pressed={on} onmousedown={keep} onclick={act} disabled={!ed.ready}>
    <span style={style}>{ch}</span>
  </button>
{/snippet}

{#snippet colorSplit(kind: 'color' | 'hl')}
  {@const hl = kind === 'hl'}
  <span class="split">
    <button
      class="rb ico cbtn"
      aria-label={hl ? 'Colore evidenziazione testo' : 'Colore carattere'}
      title={hl ? 'Colore evidenziazione testo' : 'Colore carattere'}
      onmousedown={keep}
      onclick={() => (hl ? ed.setHighlight(ed.lastHighlight) : ed.setColor(ed.lastColor))}
      disabled={!ed.ready}
    >
      <Icon name={hl ? 'highlight' : 'text-color'} />
      <span class="bar" style:background={hl ? ed.lastHighlight : ed.lastColor}></span>
    </button>
    <button class="rb arr" aria-label={hl ? 'Scegli il colore di evidenziazione' : 'Scegli il colore del carattere'} aria-expanded={pop?.kind === kind} onmousedown={keep} onclick={(e) => open(kind, e)} disabled={!ed.ready}>
      <Icon name="chevron-down" size={12} class="chev" />
    </button>
  </span>
{/snippet}

{#snippet pickers()}
  <button class="rb sel" style="width:136px" aria-label="Tipo di carattere: {fontLabel}" title="Tipo di carattere" aria-expanded={pop?.kind === 'font'} onmousedown={keep} onclick={(e) => open('font', e)} disabled={!ed.ready}>
    <span class="selv" style:font-family={fontStack(fontLabel)}>{fontLabel}</span><Icon name="chevron-down" size={14} class="chev" />
  </button>
  <button class="rb sel" style="width:58px" aria-label="Dimensione carattere: {sizeLabel}" title="Dimensione carattere" aria-expanded={pop?.kind === 'size'} onmousedown={keep} onclick={(e) => open('size', e)} disabled={!ed.ready}>
    <span class="selv">{sizeLabel}</span><Icon name="chevron-down" size={14} class="chev" />
  </button>
{/snippet}

{#snippet clipboardGroup()}
  <div class="rg"><div class="rgb">
    <RB big icon="clipboard" tone="c-amber" label="Incolla" title="Incolla (Ctrl+V)" onclick={paste} disabled={!ed.ready} />
    <div class="rs only-e">
      <button class="rb" onmousedown={keep} onclick={() => clip('cut')} disabled={!ed.ready} title="Taglia (Ctrl+X)"><Icon name="scissors" />Taglia</button>
      <button class="rb" onmousedown={keep} onclick={() => clip('copy')} disabled={!ed.ready} title="Copia (Ctrl+C)"><Icon name="copy" />Copia</button>
    </div>
  </div><span class="rgl">Appunti</span></div>
{/snippet}

{#if tab === 'messaggio'}
  {@render clipboardGroup()}

  <div class="rg"><div class="rgb"><div class="rs">
    <div class="rr">
      {@render pickers()}
      {@render fb('text-increase', 'Aumenta dimensione carattere', 'Aumenta dimensione carattere', false, () => ed.growFont(1), 'only-e')}
      {@render fb('text-decrease', 'Riduci dimensione carattere', 'Riduci dimensione carattere', false, () => ed.growFont(-1), 'only-e')}
      {@render fb('list', 'Elenco puntato', 'Elenco puntato', ed.ul, () => ed.bullets())}
      {@render fb('list-numbers', 'Elenco numerato', 'Elenco numerato', ed.ol, () => ed.numbers())}
      {@render fb('clear-formatting', 'Cancella formattazione', 'Cancella tutta la formattazione', false, () => ed.clearFormat())}
    </div>
    <div class="rr">
      {@render letter('G', 'font-weight:700', 'Grassetto', 'Grassetto (Ctrl+G)', ed.bold, () => ed.toggleBold())}
      {@render letter('C', 'font-style:italic;font-family:Georgia,serif', 'Corsivo', 'Corsivo (Ctrl+I)', ed.italic, () => ed.toggleItalic())}
      {@render letter('S', 'text-decoration:underline;text-underline-offset:2px', 'Sottolineato', 'Sottolineato (Ctrl+S)', ed.underline, () => ed.toggleUnderline())}
      {@render colorSplit('hl')}
      {@render colorSplit('color')}
      {@render fb('align-left', 'Allinea a sinistra', 'Allinea a sinistra', ed.align === 'left', () => ed.setAlign('left'), 'only-e')}
      {@render fb('align-center', 'Al centro', 'Al centro', ed.align === 'center', () => ed.setAlign('center'), 'only-e')}
      {@render fb('align-right', 'Allinea a destra', 'Allinea a destra', ed.align === 'right', () => ed.setAlign('right'), 'only-e')}
      {@render fb('indent-decrease', 'Riduci rientro', 'Riduci rientro', false, () => ed.outdent(), 'only-e')}
      {@render fb('indent-increase', 'Aumenta rientro', 'Aumenta rientro', false, () => ed.indent(), 'only-e')}
    </div>
  </div></div><span class="rgl">Testo base</span></div>

  <div class="rg"><div class="rgb">
    <RB big icon="paperclip" tone="c-brand" label="Allega file" title="Allega file" onclick={() => store.addFiles()} />
    <RB big icon="link" tone="c-brand" label="Collegamento" title="Inserisci collegamento (Ctrl+K)" onclick={(e) => store.openLinkDialog(e.currentTarget as HTMLElement)} disabled={!ed.ready} />
    <RB big menu icon="signature" tone="c-brand" label="Firma" title="Inserisci una firma" pressed={pop?.kind === 'sig'} onclick={(e) => open('sig', e)} />
  </div><span class="rgl">Includi</span></div>

  <div class="rg only-e"><div class="rgb">
    <RB big menu icon="clock" tone="c-brand" label="Invio programmato" title="Ritarda il recapito: scegli quando inviare" disabled={!store.c} pressed={schedOpen} onclick={(e) => { schedEl = e.currentTarget as HTMLElement; schedOpen = !schedOpen; }} />
  </div><span class="rgl">Invio</span></div>
{:else if tab === 'formato'}
  {@render clipboardGroup()}

  <div class="rg"><div class="rgb">
    <RB big icon="file-type-html" tone="c-brand" label="HTML" title="Testo formattato: caratteri, colori, immagini, tabelle" pressed={!plain} onclick={() => store.setFormat('html')} disabled={!store.c} />
    <RB big icon="txt" label="Testo normale" title="Solo testo, senza formattazione" pressed={plain} onclick={() => store.setFormat('testo')} disabled={!store.c} />
  </div><span class="rgl">Formato</span></div>

  <div class="rg"><div class="rgb"><div class="rs">
    <div class="rr">
      {@render pickers()}
      {@render fb('text-increase', 'Aumenta dimensione carattere', 'Aumenta dimensione carattere (Ctrl+Maiusc+punto)', false, () => ed.growFont(1))}
      {@render fb('text-decrease', 'Riduci dimensione carattere', 'Riduci dimensione carattere (Ctrl+Maiusc+virgola)', false, () => ed.growFont(-1))}
      <button class="rb ico" aria-label="Maiuscole/minuscole" title="Maiuscole/minuscole" aria-expanded={pop?.kind === 'case'} onmousedown={keep} onclick={(e) => open('case', e)} disabled={!ed.ready}><Icon name="letter-case" /></button>
      {@render fb('clear-formatting', 'Cancella formattazione', 'Cancella tutta la formattazione', false, () => ed.clearFormat())}
    </div>
    <div class="rr">
      {@render letter('G', 'font-weight:700', 'Grassetto', 'Grassetto (Ctrl+G)', ed.bold, () => ed.toggleBold())}
      {@render letter('C', 'font-style:italic;font-family:Georgia,serif', 'Corsivo', 'Corsivo (Ctrl+I)', ed.italic, () => ed.toggleItalic())}
      {@render letter('S', 'text-decoration:underline;text-underline-offset:2px', 'Sottolineato', 'Sottolineato (Ctrl+S)', ed.underline, () => ed.toggleUnderline())}
      {@render fb('strikethrough', 'Barrato', 'Barrato', ed.strike, () => ed.toggleStrike())}
      {@render fb('subscript', 'Pedice', 'Pedice', ed.sub, () => ed.toggleSub())}
      {@render fb('superscript', 'Apice', 'Apice', ed.sup, () => ed.toggleSup())}
      {@render colorSplit('hl')}
      {@render colorSplit('color')}
    </div>
  </div></div><span class="rgl">Carattere</span></div>

  <div class="rg"><div class="rgb"><div class="rs">
    <div class="rr">
      {@render fb('list', 'Elenco puntato', 'Elenco puntato', ed.ul, () => ed.bullets())}
      {@render fb('list-numbers', 'Elenco numerato', 'Elenco numerato', ed.ol, () => ed.numbers())}
      {@render fb('indent-decrease', 'Riduci rientro', 'Riduci rientro', false, () => ed.outdent())}
      {@render fb('indent-increase', 'Aumenta rientro', 'Aumenta rientro', false, () => ed.indent())}
    </div>
    <div class="rr">
      {@render fb('align-left', 'Allinea a sinistra', 'Allinea a sinistra', ed.align === 'left', () => ed.setAlign('left'))}
      {@render fb('align-center', 'Al centro', 'Al centro', ed.align === 'center', () => ed.setAlign('center'))}
      {@render fb('align-right', 'Allinea a destra', 'Allinea a destra', ed.align === 'right', () => ed.setAlign('right'))}
      {@render fb('align-justified', 'Giustifica', 'Giustifica', ed.align === 'justify', () => ed.setAlign('justify'))}
      <button class="rb ico" aria-label="Interlinea" title="Interlinea" aria-expanded={pop?.kind === 'spacing'} onmousedown={keep} onclick={(e) => open('spacing', e)} disabled={!ed.ready}><Icon name="line-height" /></button>
    </div>
  </div></div><span class="rgl">Paragrafo</span></div>
{:else}
  <div class="rg"><div class="rgb">
    <RB big icon="paperclip" tone="c-brand" label="Allega file" title="Allega file" onclick={() => store.addFiles()} />
    <RB big menu icon="signature" tone="c-brand" label="Firma" title="Inserisci una firma" pressed={pop?.kind === 'sig'} onclick={(e) => open('sig', e)} />
  </div><span class="rgl">Includi</span></div>
  <div class="rg"><div class="rgb">
    <RB big menu icon="table" tone="c-brand" label="Tabella" title="Inserisci tabella" pressed={pop?.kind === 'table'} onclick={(e) => open('table', e)} disabled={!ed.ready} />
  </div><span class="rgl">Tabelle</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="photo" tone="c-green" label="Immagini" title="Inserisci un'immagine dal computer" onclick={() => imgInput?.click()} disabled={!ed.ready} />
  </div><span class="rgl">Illustrazioni</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="link" tone="c-brand" label="Collegamento" title="Inserisci collegamento (Ctrl+K)" onclick={(e) => store.openLinkDialog(e.currentTarget as HTMLElement)} disabled={!ed.ready} />
  </div><span class="rgl">Collegamenti</span></div>
  <div class="rg"><div class="rgb">
    <RB big icon="separator-horizontal" label="Linea orizzontale" title="Inserisci una linea orizzontale" onclick={() => ed.insertLine()} disabled={!ed.ready} />
  </div><span class="rgl">Simboli</span></div>
{/if}

<input bind:this={imgInput} type="file" accept="image/png,image/jpeg,image/gif,image/webp,image/bmp" multiple hidden onchange={pickImages} />
<input bind:this={colorInput} type="color" hidden onchange={(e) => ed.setColor(e.currentTarget.value.toUpperCase())} />

{#if pop?.kind === 'sig'}
  <Menu anchor={pop.anchor} items={sigItems} label="Firme" onclose={close} />
{:else if pop?.kind === 'case'}
  <Menu anchor={pop.anchor} items={caseItems} label="Maiuscole/minuscole" onclose={close} />
{:else if pop?.kind === 'spacing'}
  <Menu anchor={pop.anchor} items={spacingItems} label="Interlinea" onclose={close} />
{:else if pop}
  <Popover anchor={pop.anchor} onclose={close} label={pop.kind === 'font' ? 'Tipo di carattere' : pop.kind === 'size' ? 'Dimensione carattere' : pop.kind === 'table' ? 'Inserisci tabella' : pop.kind === 'hl' ? 'Colore evidenziazione testo' : 'Colore carattere'}>
    {#if pop.kind === 'font'}
      <div class="list" role="listbox" aria-label="Tipo di carattere">
        {#each FONTS as f (f.name)}
          <button class="li" role="option" aria-selected={f.name === fontLabel} style:font-family={f.stack} onmousedown={keep} onclick={() => { ed.setFont(f.name); close(); }} data-first={f.name === fontLabel ? '' : undefined}>
            {f.name}{#if f.name === fontLabel}<Icon name="check" size={16} />{/if}
          </button>
        {/each}
      </div>
    {:else if pop.kind === 'size'}
      <div class="list sizes" role="listbox" aria-label="Dimensione carattere">
        {#each SIZES as s (s)}
          {@const lab = String(s).replace('.', ',')}
          <button class="li" role="option" aria-selected={lab === sizeLabel} onmousedown={keep} onclick={() => { ed.setSize(s); close(); }} data-first={lab === sizeLabel ? '' : undefined}>
            {lab}{#if lab === sizeLabel}<Icon name="check" size={16} />{/if}
          </button>
        {/each}
      </div>
    {:else if pop.kind === 'color'}
      <div class="colors">
        <button class="auto" onmousedown={keep} onclick={() => { ed.setColor('#000000'); close(); }}><span class="sw" style:background="#000000"></span>Automatico</button>
        <p class="ch">Colori tema</p>
        <div class="theme">
          {#each THEME_COLORS as col, ci (ci)}
            <div class="tcol">
              {#each col as hex, ri (ri)}
                <button class="sw" class:gap={ri === 0} style:background={hex} title={hex} aria-label="Colore {hex}" onmousedown={keep} onclick={() => { ed.setColor(hex); close(); }}></button>
              {/each}
            </div>
          {/each}
        </div>
        <p class="ch">Colori standard</p>
        <div class="row">
          {#each STANDARD_COLORS as c (c.hex)}
            <button class="sw" style:background={c.hex} title={c.name} aria-label={c.name} onmousedown={keep} onclick={() => { ed.setColor(c.hex); close(); }}></button>
          {/each}
        </div>
        <button class="auto more" onmousedown={keep} onclick={() => { close(); colorInput?.click(); }}><Icon name="palette" size={16} />Altri colori…</button>
      </div>
    {:else if pop.kind === 'hl'}
      <div class="colors">
        <div class="hl">
          {#each HIGHLIGHTS as c (c.hex)}
            <button class="sw big" style:background={c.hex} title={c.name} aria-label={c.name} onmousedown={keep} onclick={() => { ed.setHighlight(c.hex); close(); }}></button>
          {/each}
        </div>
        <button class="auto more" onmousedown={keep} onclick={() => { ed.setHighlight(null); close(); }}><Icon name="x" size={16} />Nessun colore</button>
      </div>
    {:else if pop.kind === 'table'}
      <div class="tbl">
        <p class="ch">{hover.r ? `Tabella ${hover.c}×${hover.r}` : 'Inserisci tabella'}</p>
        <div class="grid" role="grid" aria-label="Righe e colonne della tabella">
          {#each { length: 8 } as _, r (r)}
            {#each { length: 10 } as _, c (c)}
              <button
                class="cell"
                class:on={r < hover.r && c < hover.c}
                aria-label="{c + 1} colonne, {r + 1} righe"
                onmouseenter={() => (hover = { r: r + 1, c: c + 1 })}
                onfocus={() => (hover = { r: r + 1, c: c + 1 })}
                onmousedown={keep}
                onclick={() => { ed.insertTable(r + 1, c + 1); hover = { r: 0, c: 0 }; close(); }}
              ></button>
            {/each}
          {/each}
        </div>
      </div>
    {/if}
  </Popover>
{/if}

{#if schedOpen && schedEl}
  <SchedulePicker anchor={schedEl} onclose={() => (schedOpen = false)} />
{/if}

<style>
  .selv { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; text-align: left; }
  .lt span { font-size: 15px; line-height: 1; color: var(--ink); }
  .split { display: inline-flex; align-items: center; }
  .cbtn { position: relative; width: 30px !important; border-radius: 4px 0 0 4px; }
  .cbtn .bar { position: absolute; left: 7px; right: 7px; bottom: 4px; height: 3px; border-radius: 1px; box-shadow: 0 0 0 1px color-mix(in srgb, var(--ink) 18%, transparent); }
  :global(.rib[data-mode="estesa"]) .cbtn .bar { bottom: 2px; height: 3px; }
  .arr { width: 16px; padding: 0 !important; justify-content: center; border-radius: 0 4px 4px 0; }
  .arr :global(svg.chev) { margin: 0; }
  :global(.rib[data-mode="estesa"]) .arr { height: 25px; }
  .list { display: flex; flex-direction: column; min-width: 210px; }
  .list.sizes { min-width: 92px; }
  .li { height: 32px; display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 0 10px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font-size: 15px; text-align: left; }
  .li:hover, .li:focus-visible { background: var(--hover); outline: none; }
  .li[aria-selected="true"] { background: var(--brand-soft); }
  .li :global(svg) { color: var(--brand-ink); flex: none; }
  .colors { width: 238px; padding: 2px; }
  .ch { margin: 10px 2px 6px; font-size: 12.5px; font-weight: 600; color: var(--ink-2); }
  .theme { display: grid; grid-template-columns: repeat(10, 1fr); gap: 4px; }
  .tcol { display: flex; flex-direction: column; gap: 0; }
  .row { display: grid; grid-template-columns: repeat(10, 1fr); gap: 4px; }
  .hl { display: grid; grid-template-columns: repeat(5, 1fr); gap: 6px; padding: 4px 2px 2px; }
  .sw { width: 19px; height: 15px; padding: 0; border: 0; border-radius: 0; box-shadow: inset 0 0 0 1px rgba(0, 0, 0, .14); cursor: pointer; }
  .sw.gap { margin-bottom: 4px; }
  .sw.big { width: 100%; height: 24px; border-radius: 3px; }
  .sw:hover, .sw:focus-visible { outline: 2px solid var(--brand-ink); outline-offset: 1px; position: relative; z-index: 1; }
  .auto { width: 100%; height: 30px; display: flex; align-items: center; gap: 10px; padding: 0 8px; border: 0; border-radius: 4px; background: transparent; color: var(--ink); font-size: 13.5px; }
  .auto:hover { background: var(--hover); }
  .auto .sw { width: 16px; height: 16px; border-radius: 2px; }
  .more { margin-top: 8px; box-shadow: inset 0 1px 0 var(--rule); border-radius: 0 0 4px 4px; padding-top: 2px; }
  .tbl { padding: 0 4px 4px; }
  .grid { display: grid; grid-template-columns: repeat(10, 18px); gap: 3px; }
  .cell { width: 18px; height: 18px; padding: 0; border: 0; border-radius: 1px; background: var(--sheet); box-shadow: inset 0 0 0 1px var(--rule-2); }
  .cell.on { background: var(--brand-soft); box-shadow: inset 0 0 0 1px var(--brand-ink); }
</style>
