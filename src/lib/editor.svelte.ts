// Editor del testo formattato, come quello di Outlook: grassetto, colori,
// elenchi, allineamento, collegamenti, immagini, tabelle e firma.
//
// Il motore è RoosterJS (licenza MIT), lo stesso che Microsoft usa nella
// versione web e nel nuovo Outlook. Il testo si scrive in un iframe isolato
// (niente script, risorse esterne bloccate dalla Content-Security-Policy),
// così l'HTML citato da un altro messaggio non può cambiare l'aspetto di
// Better-Look né scaricare immagini di tracciamento.

import { Editor, createModelFromHtml, exportContent, paste, redo, undo } from 'roosterjs-content-model-core';
import {
  adjustLinkSelection,
  changeCapitalization,
  changeFontSize,
  clearFormat,
  getFormatState,
  insertLink,
  insertTable,
  removeLink,
  setAlignment,
  setBackgroundColor,
  setFontName,
  setFontSize,
  setIndentation,
  setSpacing,
  setTextColor,
  toggleBold,
  toggleBullet,
  toggleItalic,
  toggleNumbering,
  toggleStrikethrough,
  toggleSubscript,
  toggleSuperscript,
  toggleUnderline,
} from 'roosterjs-content-model-api';
import {
  AutoFormatPlugin,
  EditPlugin,
  ImageEditPlugin,
  PastePlugin,
  ShortcutBold,
  ShortcutBullet,
  ShortcutClearFormat,
  ShortcutDecreaseFont,
  ShortcutIncreaseFont,
  ShortcutIndentList,
  ShortcutItalic,
  ShortcutNumbering,
  ShortcutOutdentList,
  ShortcutPlugin,
  ShortcutRedo,
  ShortcutRedoAlt,
  ShortcutUnderline,
  ShortcutUndo,
  ShortcutUndo2,
  TableEditPlugin,
  WatermarkPlugin,
  type AutoFormatOptions,
  type ShortcutCommand,
} from 'roosterjs-content-model-plugins';
import type { ContentModelFormatState, EditorPlugin, PluginEvent } from 'roosterjs-content-model-types';
import { BODY_CLASS, QUOTE_ID, SIG_ID, escapeHtml, unwrapBody } from './compose-html';

export interface FontChoice {
  name: string;
  stack: string;
}

/** Caratteri comuni a Windows, macOS e Linux (con i sostituti più simili). */
export const FONTS: FontChoice[] = [
  { name: 'Aptos', stack: "Aptos, Calibri, Carlito, Arial, Helvetica, sans-serif" },
  { name: 'Arial', stack: "Arial, Helvetica, 'Liberation Sans', sans-serif" },
  { name: 'Calibri', stack: "Calibri, Carlito, Arial, sans-serif" },
  { name: 'Cambria', stack: "Cambria, Caladea, Georgia, serif" },
  { name: 'Century Gothic', stack: "'Century Gothic', 'URW Gothic', Futura, sans-serif" },
  { name: 'Courier New', stack: "'Courier New', Courier, 'Liberation Mono', monospace" },
  { name: 'Garamond', stack: "Garamond, 'EB Garamond', Georgia, serif" },
  { name: 'Georgia', stack: "Georgia, 'DejaVu Serif', serif" },
  { name: 'Helvetica', stack: "Helvetica, Arial, sans-serif" },
  { name: 'Segoe UI', stack: "'Segoe UI', Tahoma, Arial, sans-serif" },
  { name: 'Tahoma', stack: "Tahoma, Verdana, 'DejaVu Sans', sans-serif" },
  { name: 'Times New Roman', stack: "'Times New Roman', Times, 'Liberation Serif', serif" },
  { name: 'Trebuchet MS', stack: "'Trebuchet MS', Arial, sans-serif" },
  { name: 'Verdana', stack: "Verdana, Geneva, 'DejaVu Sans', sans-serif" },
];

/** Dimensioni in punti, come nell'elenco di Outlook. */
export const SIZES = [8, 9, 10, 10.5, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

export const DEFAULT_FONT = { family: 'Aptos', size: 11 };

export function fontStack(name: string): string {
  return FONTS.find((f) => f.name.toLowerCase() === name.toLowerCase())?.stack ?? `'${name.replace(/'/g, '')}', Arial, sans-serif`;
}

// Colori del testo: sono contenuto dei messaggi, non colori dell'interfaccia.
const THEME_BASE = ['#FFFFFF', '#000000', '#E7E6E6', '#44546A', '#4472C4', '#ED7D31', '#A5A5A5', '#FFC000', '#5B9BD5', '#70AD47'];

function mix(hex: string, target: number, amount: number): string {
  const n = parseInt(hex.slice(1), 16);
  const ch = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((c) => Math.round(c + (target - c) * amount));
  return `#${ch.map((c) => c.toString(16).padStart(2, '0')).join('').toUpperCase()}`;
}

/** Colori del tema: per ogni colore, le versioni più chiare e più scure. */
export const THEME_COLORS: string[][] = THEME_BASE.map((c, i) => {
  if (i === 0) return [c, '#F2F2F2', '#D9D9D9', '#BFBFBF', '#A6A6A6', '#808080'];
  if (i === 1) return [c, '#808080', '#595959', '#404040', '#262626', '#0D0D0D'];
  return [c, mix(c, 255, 0.8), mix(c, 255, 0.6), mix(c, 255, 0.4), mix(c, 0, 0.25), mix(c, 0, 0.5)];
});

export const STANDARD_COLORS = [
  { hex: '#C00000', name: 'Rosso scuro' },
  { hex: '#FF0000', name: 'Rosso' },
  { hex: '#FFC000', name: 'Arancione' },
  { hex: '#FFFF00', name: 'Giallo' },
  { hex: '#92D050', name: 'Verde chiaro' },
  { hex: '#00B050', name: 'Verde' },
  { hex: '#00B0F0', name: 'Azzurro' },
  { hex: '#0070C0', name: 'Blu' },
  { hex: '#002060', name: 'Blu scuro' },
  { hex: '#7030A0', name: 'Viola' },
];

export const HIGHLIGHTS = [
  { hex: '#FFFF00', name: 'Giallo' },
  { hex: '#00FF00', name: 'Verde brillante' },
  { hex: '#00FFFF', name: 'Turchese' },
  { hex: '#FF00FF', name: 'Rosa' },
  { hex: '#0000FF', name: 'Blu' },
  { hex: '#FF0000', name: 'Rosso' },
  { hex: '#000080', name: 'Blu scuro' },
  { hex: '#008080', name: 'Verde acqua' },
  { hex: '#008000', name: 'Verde' },
  { hex: '#800080', name: 'Viola' },
  { hex: '#800000', name: 'Rosso scuro' },
  { hex: '#808000', name: 'Giallo scuro' },
  { hex: '#808080', name: 'Grigio 50%' },
  { hex: '#C0C0C0', name: 'Grigio 25%' },
  { hex: '#000000', name: 'Nero' },
];

/** Documento dell'iframe: niente script, solo immagini come dati. */
export function editorDocument(font: { family: string; size: number }): string {
  const csp = "default-src 'none'; script-src 'none'; img-src data: blob:; style-src 'unsafe-inline'; font-src data:; base-uri 'none'; form-action 'none'";
  return `<!DOCTYPE html><html lang="it"><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${csp}"><meta name="color-scheme" content="light"><style>
html,body{margin:0;height:100%;background:#ffffff;color-scheme:light}
#testo{box-sizing:border-box;min-height:100%;padding:14px 24px 28px;font-family:${fontStack(font.family)};font-size:${font.size}pt;color:#000000;overflow-wrap:break-word;outline:none;caret-color:#000000}
p{margin:0}
img{max-width:100%;height:auto}
img:not([src^="data:"]):not([src^="blob:"]){background:#f0f0f0;min-width:24px;min-height:24px;outline:1px dashed #c8c8c8;outline-offset:-1px}
table{border-collapse:collapse}
td,th{min-width:24px}
a{color:#0563C1}
blockquote{margin:0 0 0 .8em;padding-left:.8em;border-left:2px solid #d1d1d1}
::selection{background:#b3d4fc}
</style></head><body><div id="testo"></div></body></html>`;
}

/** Il frammento HTML salvato, senza il vecchio contenitore del carattere. */
export function toEditorHtml(html: string): string {
  return unwrapBody(html || '');
}

function normalizeUrl(raw: string): string {
  const u = raw.trim();
  if (!u) return '';
  if (/^(https?:|mailto:|tel:)/i.test(u)) return u;
  if (/^[^\s@/]+@[^\s@/]+\.[^\s@/]+$/.test(u)) return `mailto:${u}`;
  return `https://${u.replace(/^\/+/, '')}`;
}

function readDataUrl(file: Blob): Promise<string> {
  return new Promise((ok, ko) => {
    const r = new FileReader();
    r.onload = () => ok(String(r.result));
    r.onerror = () => ko(r.error);
    r.readAsDataURL(file);
  });
}

/** Immagini molto grandi (foto del telefono) si rimpiccioliscono prima di metterle nel testo. */
async function fitImage(dataUrl: string, mime: string): Promise<{ src: string; width: number }> {
  const img = new Image();
  img.src = dataUrl;
  try {
    await img.decode();
  } catch {
    return { src: dataUrl, width: 0 };
  }
  const max = 1600;
  let src = dataUrl;
  let w = img.naturalWidth;
  if (w > max) {
    const h = Math.round((img.naturalHeight * max) / w);
    const canvas = document.createElement('canvas');
    canvas.width = max;
    canvas.height = h;
    canvas.getContext('2d')?.drawImage(img, 0, 0, max, h);
    src = canvas.toDataURL(mime === 'image/png' ? 'image/png' : 'image/jpeg', 0.88);
    w = max;
  }
  return { src, width: Math.min(w, 640) };
}

const KEY_G = 71;
const KEY_S = 83;

/** Scorciatoie: quelle di Office in italiano (Ctrl+G, Ctrl+I, Ctrl+S) più le internazionali. */
const SHORTCUTS: ShortcutCommand[] = [
  { shortcutKey: { modifierKey: 'ctrl', shiftKey: false, which: KEY_G }, onClick: (e) => toggleBold(e) },
  { shortcutKey: { modifierKey: 'ctrl', shiftKey: false, which: KEY_S }, onClick: (e) => toggleUnderline(e) },
  ShortcutBold,
  ShortcutItalic,
  ShortcutUnderline,
  ShortcutClearFormat,
  ShortcutUndo,
  ShortcutUndo2,
  ShortcutRedo,
  ShortcutRedoAlt,
  ShortcutBullet,
  ShortcutNumbering,
  ShortcutIncreaseFont,
  ShortcutDecreaseFont,
  ShortcutIndentList,
  ShortcutOutdentList,
];

export type Align = 'left' | 'center' | 'right' | 'justify';

export interface EditorHooks {
  onchange?: (html: string, text: string) => void;
  onsubmit?: () => void;
  onescape?: () => void;
  onlink?: () => void;
  fetchImage?: (url: string) => Promise<string | null>;
}

/**
 * Comandi della barra sull'editor di testo formattato. Il motore è RoosterJS,
 * lo stesso della versione web di Outlook: elenchi automatici, incolla da
 * Word ed Excel, tabelle, annulla e ripeti si comportano come lì.
 */
export class EditorCtl {
  // Stato del testo dove sta il cursore, per i pulsanti della barra.
  ready = $state(false);
  bold = $state(false);
  italic = $state(false);
  underline = $state(false);
  strike = $state(false);
  sub = $state(false);
  sup = $state(false);
  ul = $state(false);
  ol = $state(false);
  align = $state<Align>('left');
  font = $state('');
  size = $state<number | null>(null);
  link = $state(false);
  table = $state(false);
  canUndo = $state(false);
  canRedo = $state(false);
  /** Ultimi colori usati: il pulsante li applica con un clic, come in Outlook. */
  lastColor = $state('#C00000');
  lastHighlight = $state('#FFFF00');

  font0 = { ...DEFAULT_FONT };
  private editor: Editor | null = null;
  private div: HTMLDivElement | null = null;
  private frame: HTMLIFrameElement | null = null;
  private hooks: EditorHooks = {};
  private emitTimer: ReturnType<typeof setTimeout> | undefined;
  private refreshFrame = 0;
  /** Immagini incollate da una pagina web, da scaricare e incorporare. */
  private pastedUrls = new Set<string>();
  private cleanup: (() => void) | null = null;

  get attached(): boolean {
    return !!this.editor;
  }

  attach(frame: HTMLIFrameElement, html: string, font: { family: string; size: number }, hooks: EditorHooks, placeholder = 'Scrivi il messaggio', label = 'Testo del messaggio') {
    this.detach();
    const doc = frame.contentDocument;
    const div = doc?.getElementById('testo') as HTMLDivElement | null;
    if (!doc || !div) return;
    this.frame = frame;
    this.div = div;
    this.font0 = { ...font };
    this.hooks = hooks;
    div.setAttribute('role', 'textbox');
    div.setAttribute('aria-multiline', 'true');
    div.setAttribute('aria-label', label);
    div.spellcheck = true;

    const defaults = { fontFamily: fontStack(font.family), fontSize: `${font.size}pt`, textColor: '#000000' };
    const ctl = this;
    const bridge: EditorPlugin = {
      getName: () => 'Better-Look',
      initialize: () => {},
      dispose: () => {},
      onPluginEvent: (e: PluginEvent) => ctl.onEvent(e),
    };
    this.editor = new Editor(div, {
      plugins: [
        bridge,
        new ShortcutPlugin(SHORTCUTS),
        new EditPlugin(),
        new AutoFormatPlugin({ autoBullet: true, autoNumbering: true, autoLink: true, autoUnlink: false, autoHyphen: false, removeListMargins: false } as AutoFormatOptions),
        new PastePlugin(),
        new TableEditPlugin(),
        new ImageEditPlugin(),
        new WatermarkPlugin(placeholder, { textColor: '#707070' }),
      ],
      defaultSegmentFormat: defaults,
      initialModel: createModelFromHtml(toEditorHtml(html) || '<div><br></div>', undefined, undefined, defaults),
    });

    // Trascinare un'immagine nel testo la inserisce lì.
    const onDrop = (e: DragEvent) => {
      const files = [...(e.dataTransfer?.files ?? [])].filter((f) => f.type.startsWith('image/'));
      if (!files.length) return;
      e.preventDefault();
      void this.insertImageFiles(files);
    };
    // I collegamenti nel testo non si aprono mentre si scrive.
    const onClick = (e: MouseEvent) => {
      if ((e.target as Element | null)?.closest?.('a')) e.preventDefault();
    };
    div.addEventListener('drop', onDrop);
    div.addEventListener('click', onClick);
    this.cleanup = () => {
      div.removeEventListener('drop', onDrop);
      div.removeEventListener('click', onClick);
    };
    this.ready = true;
    this.refresh();
  }

  detach() {
    clearTimeout(this.emitTimer);
    cancelAnimationFrame(this.refreshFrame);
    this.cleanup?.();
    this.cleanup = null;
    try {
      this.editor?.dispose();
    } catch {
      /* l'iframe può essere già stato tolto */
    }
    this.editor = null;
    this.div = null;
    this.frame = null;
    this.pastedUrls.clear();
    this.ready = false;
  }

  private onEvent(e: PluginEvent) {
    switch (e.eventType) {
      case 'keyDown': {
        const k = e.rawEvent;
        const mod = k.ctrlKey || k.metaKey;
        if (mod && k.key === 'Enter') {
          k.preventDefault();
          this.flush();
          this.hooks.onsubmit?.();
        } else if (k.key === 'Escape') {
          k.preventDefault();
          this.flush();
          this.hooks.onescape?.();
        } else if (mod && !k.shiftKey && !k.altKey && k.key.toLowerCase() === 'k') {
          k.preventDefault();
          this.refresh();
          this.hooks.onlink?.();
        }
        break;
      }
      case 'beforePaste':
        // Le immagini di una pagina web si scaricano dopo, come fa Outlook.
        for (const img of e.fragment.querySelectorAll('img')) {
          const src = img.getAttribute('src') ?? '';
          if (/^https?:/i.test(src)) this.pastedUrls.add(src);
        }
        break;
      case 'contentChanged':
        this.scheduleEmit();
        this.scheduleRefresh();
        if (this.pastedUrls.size) void this.loadPasted();
        break;
      case 'input':
        this.scheduleEmit();
        this.scheduleRefresh();
        break;
      case 'selectionChanged':
      case 'keyUp':
      case 'mouseUp':
        this.scheduleRefresh();
        break;
    }
  }

  /** Cursore all'inizio del testo (risposte), oppure dov'era prima di usare la barra. */
  focus(where: 'start' | 'keep' = 'keep') {
    const ed = this.editor;
    const div = this.div;
    if (!ed || !div) return;
    this.frame?.contentWindow?.focus();
    ed.focus();
    if (where === 'start') {
      const doc = div.ownerDocument;
      const r = doc.createRange();
      let first: Node = div;
      while (first.firstChild && first.firstChild.nodeType === 1 && !['BR', 'IMG', 'TABLE', 'HR'].includes((first.firstChild as Element).tagName)) first = first.firstChild;
      r.setStart(first, 0);
      r.collapse(true);
      ed.setDOMSelection({ type: 'range', range: r, isReverted: false });
      doc.documentElement.scrollTop = 0;
    }
  }

  // ---------- Contenuto ----------

  /** HTML da salvare e spedire, dentro un contenitore con il carattere del
   *  messaggio: anche le righe senza stile (firma, intestazione della
   *  citazione) arrivano con lo stesso carattere, come in Outlook. */
  html(): string {
    if (!this.editor) return '';
    const f = this.font0;
    return `<div class="${BODY_CLASS}" style="font-family: ${fontStack(f.family)}; font-size: ${f.size}pt; color: #000000;">${exportContent(this.editor, 'HTML')}</div>`;
  }

  text(): string {
    if (!this.editor) return '';
    return exportContent(this.editor, 'PlainText').replace(/\r\n/g, '\n').replace(/ /g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  private scheduleEmit() {
    clearTimeout(this.emitTimer);
    this.emitTimer = setTimeout(() => this.emit(), 150);
  }

  private emit() {
    clearTimeout(this.emitTimer);
    if (this.editor) this.hooks.onchange?.(this.html(), this.text());
  }

  /** Salva subito l'ultimo testo scritto (prima di inviare o chiudere). */
  flush() {
    this.emit();
  }

  // ---------- Stato per la barra ----------

  private scheduleRefresh() {
    cancelAnimationFrame(this.refreshFrame);
    this.refreshFrame = requestAnimationFrame(() => this.refresh());
  }

  private refresh() {
    const ed = this.editor;
    if (!ed) return;
    let s: ContentModelFormatState;
    try {
      s = getFormatState(ed);
    } catch {
      return;
    }
    this.bold = !!s.isBold;
    this.italic = !!s.isItalic;
    this.underline = !!s.isUnderline;
    this.strike = !!s.isStrikeThrough;
    this.sub = !!s.isSubscript;
    this.sup = !!s.isSuperscript;
    this.ul = !!s.isBullet;
    this.ol = !!s.isNumbering;
    const a = (s.textAlign ?? 'left').toLowerCase();
    this.align = a === 'center' ? 'center' : a === 'right' || a === 'end' ? 'right' : a === 'justify' ? 'justify' : 'left';
    const first = (s.fontName ?? '').split(',')[0].trim().replace(/^['"]|['"]$/g, '');
    this.font = first ? FONTS.find((f) => f.name.toLowerCase() === first.toLowerCase())?.name ?? first : this.font0.family;
    this.size = parseSize(s.fontSize) ?? this.font0.size;
    this.link = !!s.canUnlink;
    this.table = !!s.isInTable;
    this.canUndo = !!s.canUndo;
    this.canRedo = !!s.canRedo;
  }

  // ---------- Comandi ----------

  private run(fn: (ed: Editor) => void) {
    const ed = this.editor;
    if (!ed) return;
    if (!ed.hasFocus()) this.focus('keep');
    fn(ed);
    this.refresh();
    this.scheduleEmit();
  }

  /** Comandi del browser: taglia e copia passano comunque da RoosterJS. */
  cmd(name: 'cut' | 'copy') {
    this.run(() => this.div?.ownerDocument.execCommand(name));
  }

  toggleBold() {
    this.run(toggleBold);
  }
  toggleItalic() {
    this.run(toggleItalic);
  }
  toggleUnderline() {
    this.run(toggleUnderline);
  }
  toggleStrike() {
    this.run(toggleStrikethrough);
  }
  toggleSub() {
    this.run(toggleSubscript);
  }
  toggleSup() {
    this.run(toggleSuperscript);
  }
  bullets() {
    this.run(toggleBullet);
  }
  numbers() {
    this.run(toggleNumbering);
  }
  indent() {
    this.run((ed) => setIndentation(ed, 'indent'));
  }
  outdent() {
    this.run((ed) => setIndentation(ed, 'outdent'));
  }
  setAlign(a: Align) {
    this.run((ed) => setAlignment(ed, a));
  }
  clearFormat() {
    this.run(clearFormat);
  }
  setFont(name: string) {
    this.run((ed) => setFontName(ed, fontStack(name)));
  }
  setSize(pt: number) {
    this.run((ed) => setFontSize(ed, `${pt}pt`));
  }
  growFont(step: 1 | -1) {
    this.run((ed) => changeFontSize(ed, step > 0 ? 'increase' : 'decrease', SIZES));
  }
  setColor(hex: string) {
    this.lastColor = hex;
    this.run((ed) => setTextColor(ed, hex));
  }
  setHighlight(hex: string | null) {
    if (hex) this.lastHighlight = hex;
    this.run((ed) => setBackgroundColor(ed, hex));
  }
  /** Maiuscole e minuscole, come "Maiuscole/minuscole" di Office. */
  setCase(kind: 'lowerCase' | 'upperCase' | 'capitalize' | 'sentence') {
    this.run((ed) => changeCapitalization(ed, kind, 'it-IT'));
  }
  /** Interlinea: 1, 1,15, 1,5, 2... */
  setSpacing(lines: number) {
    this.run((ed) => setSpacing(ed, lines));
  }
  undo() {
    this.run(undo);
  }
  redo() {
    this.run(redo);
  }

  // ---------- Collegamenti ----------

  /** Testo selezionato e indirizzo del collegamento sotto il cursore. */
  linkInfo(): { text: string; href: string } {
    const ed = this.editor;
    if (!ed) return { text: '', href: '' };
    if (getFormatState(ed).canUnlink) {
      const [text, href] = adjustLinkSelection(ed);
      return { text, href: href ?? '' };
    }
    const sel = ed.getDOMSelection();
    return { text: sel?.type === 'range' ? sel.range.toString() : '', href: '' };
  }

  setLink(text: string, href: string) {
    const url = normalizeUrl(href);
    if (!url) return;
    this.run((ed) => insertLink(ed, url, undefined, text || undefined));
  }

  removeLink() {
    this.run(removeLink);
  }

  // ---------- Inserisci ----------

  /** HTML al posto del cursore, ripulito come un incolla. */
  insertHtml(html: string) {
    this.run((ed) => paste(ed, { types: ['text/html'], text: '', rawHtml: html, image: null, files: [], customValues: {} }, 'normal'));
  }

  /** Testo semplice al posto del cursore. */
  insertText(text: string) {
    this.run((ed) => paste(ed, { types: ['text/plain'], text, rawHtml: null, image: null, files: [], customValues: {} }, 'asPlainText'));
  }

  insertTable(rows: number, cols: number) {
    // Come "Griglia tabella" di Outlook: bordi neri sottili, niente colori.
    this.run((ed) =>
      insertTable(ed, cols, rows, {
        topBorderColor: '#000000',
        bottomBorderColor: '#000000',
        verticalBorderColor: '#000000',
        hasHeaderRow: false,
        hasFirstColumn: false,
        hasBandedRows: false,
        hasBandedColumns: false,
        bgColorEven: null,
        bgColorOdd: null,
        headerRowColor: null,
        tableBorderFormat: 0,
      }, undefined, { paddingLeft: '5.4pt', paddingRight: '5.4pt', verticalAlign: 'top' }),
    );
  }

  insertLine() {
    this.insertHtml('<hr><div><br></div>');
  }

  async insertImageFiles(files: File[]) {
    for (const f of files) {
      if (!f.type.startsWith('image/') || f.type === 'image/svg+xml') continue;
      const data = await readDataUrl(f);
      const { src, width } = await fitImage(data, f.type);
      this.insertHtml(`<img src="${src}"${width ? ` width="${width}"` : ''} alt="${escapeHtml(f.name)}">`);
    }
  }

  // ---------- Firma ----------

  private sigBlock(): HTMLElement | null {
    const div = this.div;
    if (!div) return null;
    // Solo la firma di questo messaggio, non quella dentro il messaggio citato.
    for (const el of div.querySelectorAll<HTMLElement>(`#${SIG_ID}`)) {
      if (!el.closest(`#${QUOTE_ID}`)) return el;
    }
    return null;
  }

  /** Firma: sostituisce quella già nel testo, altrimenti va dove sta il cursore. */
  insertSignature(html: string) {
    if (this.sigBlock()) {
      this.setSignature(html);
      return;
    }
    this.insertHtml(`<div id="${SIG_ID}">${html}</div>`);
  }

  /** Firma automatica: cambia con l'account, sopra il messaggio citato. */
  setSignature(html: string | null) {
    const div = this.div;
    const ed = this.editor;
    if (!div || !ed) return;
    const cur = this.sigBlock();
    if (cur) {
      if (html) cur.innerHTML = html;
      else cur.remove();
    } else if (html) {
      const doc = div.ownerDocument;
      const block = doc.createElement('div');
      block.id = SIG_ID;
      block.innerHTML = html;
      const gap = doc.createElement('div');
      gap.innerHTML = '<br>';
      const quote = div.querySelector(`#${QUOTE_ID}`);
      if (quote) {
        quote.before(block);
        quote.before(gap);
      } else {
        div.append(gap, block);
      }
    }
    ed.takeSnapshot();
    this.emit();
  }

  /** Le immagini incollate da una pagina web si scaricano e diventano parte del messaggio. */
  private async loadPasted() {
    const div = this.div;
    const fetchImage = this.hooks.fetchImage;
    const urls = [...this.pastedUrls];
    this.pastedUrls.clear();
    if (!div || !fetchImage) return;
    let changed = false;
    for (const url of urls) {
      const data = await fetchImage(url).catch(() => null);
      if (!data || this.div !== div) continue;
      for (const img of div.querySelectorAll<HTMLImageElement>('img')) {
        if (img.getAttribute('src') === url) {
          img.setAttribute('src', data);
          changed = true;
        }
      }
    }
    if (changed && this.editor) {
      this.editor.takeSnapshot();
      this.emit();
    }
  }
}

function parseSize(v: string | undefined): number | null {
  if (!v) return null;
  const n = parseFloat(v);
  if (!Number.isFinite(n)) return null;
  const pt = v.endsWith('px') ? n * 0.75 : n;
  return Math.round(pt * 2) / 2;
}

/** Editor del messaggio in scrittura: uno per finestra, comandato dalla barra. */
export const composeEditor = new EditorCtl();
