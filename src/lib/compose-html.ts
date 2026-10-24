// Testo dei messaggi in scrittura come HTML: modello del nuovo messaggio,
// citazione nelle risposte, blocco della firma. Funzioni pure, senza DOM,
// così valgono anche nei test e nelle finestre senza editor.

import type { MessageDetail } from './types';
import { longDate, peopleText, personFull } from './format';

/** Contenitore con il carattere predefinito (versioni precedenti dell'editor). */
export const BODY_CLASS = 'better-look-corpo';
/** Firma aggiunta da Better-Look: si sostituisce cambiando account o firma. È un
 *  id e non una classe perché l'editor conserva gli id dei blocchi. */
export const SIG_ID = 'better-look-firma';
/** Messaggio originale citato sotto una risposta o un inoltro. */
export const QUOTE_ID = 'better-look-citazione';

export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

/** Testo semplice come HTML: una riga per riga, righe vuote comprese. */
export function textToHtml(text: string): string {
  const lines = text.replace(/\r\n/g, '\n').replace(/\s+$/, '').split('\n');
  return lines.map((l) => (l.trim() ? `<div>${escapeHtml(l)}</div>` : '<div><br></div>')).join('');
}

const ENTITIES: Record<string, string> = { nbsp: ' ', amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", '#39': "'" };

function decode(t: string): string {
  return t.replace(/&(#x[0-9a-f]+|#\d+|[a-z]+|#39);/gi, (m, e: string) => {
    if (e[0] === '#') {
      const n = e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return Number.isFinite(n) ? String.fromCodePoint(n) : m;
    }
    return ENTITIES[e.toLowerCase()] ?? m;
  });
}

const BLOCKS = new Set(['div', 'p', 'tr', 'table', 'blockquote', 'pre', 'hr', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'section', 'article', 'header', 'footer']);

/** Testo semplice da HTML, per la versione senza formattazione e i controlli.
 *  Gli elenchi tengono il punto o il numero, come nel testo normale di Outlook.
 *  `keepLeading`: le righe vuote all'inizio restano (lo spazio per scrivere). */
export function htmlToText(html: string, keepLeading = false): string {
  const src = html.replace(/<(style|script|title)[^>]*>[\s\S]*?<\/\1>/gi, '').replace(/<!--[\s\S]*?-->/g, '');
  let out = '';
  let lineStart = true;
  const lists: { ordered: boolean; n: number }[] = [];
  const nl = () => {
    if (!lineStart) {
      out += '\n';
      lineStart = true;
    }
  };
  const re = /<(\/?)([a-z][a-z0-9]*)\b[^>]*>|([^<]+)/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(src))) {
    if (m[3] !== undefined) {
      let t = decode(m[3]).replace(/[\r\n\t ]+/g, ' ');
      if (lineStart) t = t.replace(/^ +/, '');
      if (t) {
        out += t;
        lineStart = false;
      }
      continue;
    }
    const close = m[1] === '/';
    const tag = m[2].toLowerCase();
    if (tag === 'br') {
      out += '\n';
      lineStart = true;
    } else if (tag === 'ul' || tag === 'ol') {
      nl();
      if (close) lists.pop();
      else lists.push({ ordered: tag === 'ol', n: 0 });
    } else if (tag === 'li') {
      nl();
      if (!close) {
        const l = lists[lists.length - 1];
        out += '    '.repeat(Math.max(0, lists.length - 1)) + (l?.ordered ? `${++l.n}. ` : '• ');
      }
    } else if (tag === 'td' || tag === 'th') {
      if (close && !lineStart) out += '\t';
    } else if (BLOCKS.has(tag)) {
      nl();
    }
  }
  const text = out
    .replace(/\u00a0/g, ' ')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .replace(/\s+$/, '');
  return keepLeading ? text.replace(/^[ \t]+/, '') : text.replace(/^\s+/, '');
}

/** Posizione di un blocco `<div id="...">` (o con quella classe, nelle bozze
 *  più vecchie) con il suo contenuto, contando i div annidati. */
export function findBlock(html: string, name: string, before = html.length): { start: number; innerStart: number; innerEnd: number; end: number } | null {
  const open = new RegExp(`<div\\b[^>]*\\b(?:id="${name}"|class="[^"]*\\b${name}\\b[^"]*")[^>]*>`, 'i');
  const m = open.exec(html);
  if (m && m.index >= before) return null;
  if (!m) return null;
  const innerStart = m.index + m[0].length;
  const tag = /<(\/?)div\b[^>]*>/gi;
  tag.lastIndex = innerStart;
  let depth = 1;
  let t: RegExpExecArray | null;
  while ((t = tag.exec(html))) {
    depth += t[1] ? -1 : 1;
    if (depth === 0) return { start: m.index, innerStart, innerEnd: t.index, end: t.index + t[0].length };
  }
  return null;
}

export function signatureBlock(sig: string | null | undefined): string {
  return sig ? `<div id="${SIG_ID}">${sig}</div>` : '';
}

/** Nuovo messaggio: righe vuote per scrivere, poi la firma. */
export function newMessageHtml(sig: string | null | undefined): string {
  return `<div><br></div><div><br></div>${signatureBlock(sig)}`;
}

/** Intestazione sopra il messaggio citato, come in Outlook. */
export function quoteHeader(m: MessageDetail): string {
  const rows = [`<b>Da:</b> ${escapeHtml(personFull(m.from))}`, `<b>Inviato:</b> ${escapeHtml(longDate(m.date))}`];
  if (m.to.length) rows.push(`<b>A:</b> ${escapeHtml(peopleText(m.to))}`);
  if (m.cc.length) rows.push(`<b>Cc:</b> ${escapeHtml(peopleText(m.cc))}`);
  rows.push(`<b>Oggetto:</b> ${escapeHtml(m.subject)}`);
  return `<div style="border:none;border-top:solid #E1E1E1 1pt;padding:3pt 0 0 0"><div>${rows.join('<br>')}</div></div><div><br></div>`;
}

/** Risposta o inoltro: spazio per scrivere, firma, poi il messaggio originale. */
export function replyHtml(sig: string | null | undefined, m: MessageDetail, quote: string): string {
  const head = sig ? `<div><br></div><div><br></div>${signatureBlock(sig)}<div><br></div>` : '<div><br></div><div><br></div>';
  return `${head}<div id="${QUOTE_ID}">${quoteHeader(m)}${quote}</div>`;
}

/** Mette, cambia o toglie la firma aggiunta da Better-Look. */
export function replaceSignature(html: string, sig: string | null): string {
  // Solo la firma sopra il messaggio citato: quella dentro è di chi ha scritto.
  const q = findBlock(html, QUOTE_ID);
  const b = findBlock(html, SIG_ID, q?.start);
  if (b) {
    if (!sig) return html.slice(0, b.start) + html.slice(b.end);
    return html.slice(0, b.innerStart) + sig + html.slice(b.innerEnd);
  }
  if (!sig) return html;
  if (q) return `${html.slice(0, q.start)}${signatureBlock(sig)}<div><br></div>${html.slice(q.start)}`;
  return `${html}<div><br></div>${signatureBlock(sig)}`;
}

/** Il testo dentro il contenitore del carattere predefinito, se c'è. */
export function unwrapBody(html: string): string {
  const s = html.trim();
  const b = findBlock(s, BODY_CLASS);
  if (b && b.start === 0 && b.end === s.length) return s.slice(b.innerStart, b.innerEnd);
  return s;
}

/** Solo il testo scritto: senza firma e senza messaggio citato (per i modelli). */
export function ownHtml(html: string): string {
  let s = unwrapBody(html);
  const q = findBlock(s, QUOTE_ID);
  if (q) s = s.slice(0, q.start);
  s = replaceSignature(s, null);
  return s.replace(/^(\s*<div><br\s*\/?><\/div>)+/i, '').replace(/(<div><br\s*\/?><\/div>\s*)+$/i, '').trim();
}

export function hasQuote(html: string | null | undefined): boolean {
  return !!html && findBlock(html, QUOTE_ID) !== null;
}

/** Riga che separa la risposta dal messaggio citato nel testo normale, come in Outlook. */
export const PLAIN_RULE = '________________________________';

/** Intestazione del messaggio citato in testo normale. */
export function plainQuote(m: MessageDetail, text: string): string {
  const lines = [`Da: ${personFull(m.from)}`, `Inviato: ${longDate(m.date)}`];
  if (m.to.length) lines.push(`A: ${peopleText(m.to)}`);
  if (m.cc.length) lines.push(`Cc: ${peopleText(m.cc)}`);
  lines.push(`Oggetto: ${m.subject}`);
  return `${PLAIN_RULE}\n${lines.join('\n')}\n\n${text.trim()}`;
}

/** Risposta in testo normale: spazio per scrivere, firma, poi il messaggio citato. */
export function plainReply(sig: string | null | undefined, m: MessageDetail, text: string): string {
  const s = sig ? htmlToText(sig) : '';
  return `\n\n${s ? `${s}\n\n` : ''}${plainQuote(m, text)}`;
}

/** Da testo formattato a testo normale, con la citazione separata dalla riga. */
export function plainFromHtml(html: string): string {
  const q = findBlock(html, QUOTE_ID);
  if (!q) return htmlToText(html, true);
  const before = htmlToText(html.slice(0, q.start), true);
  const quoted = htmlToText(html.slice(q.innerStart, q.innerEnd));
  return `${before}\n\n${PLAIN_RULE}\n${quoted}`;
}

/** Firma in un testo normale: sopra il messaggio citato, oppure in fondo. */
export function addPlainSignature(text: string, sig: string): string {
  const s = htmlToText(sig);
  if (!s) return text;
  const at = text.indexOf(PLAIN_RULE);
  if (at < 0) return `${text.replace(/\s+$/, '')}\n\n${s}`;
  return `${text.slice(0, at).replace(/\s+$/, '')}\n\n${s}\n\n${text.slice(at)}`;
}

