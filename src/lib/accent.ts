// Colore di accento scelto dall'utente. Da un solo colore si ricavano le
// variabili CSS legate al blu di Fluent 2 (pulsanti, barra in alto, selezione,
// collegamenti), separatamente per il tema chiaro e per quello scuro.
// I conti si fanno in OKLCH, così il colore resta riconoscibile mentre si
// regola la luminosità; ogni coppia testo e sfondo rispetta il contrasto
// WCAG AA (4,5:1).

/** Blu predefinito (quello di Outlook): con questo non si cambia nulla. */
export const DEFAULT_ACCENT = '#0F6CBD';

/** Colori proposti in Impostazioni › Aspetto, tutti dalla palette Fluent 2. */
export const ACCENT_SWATCHES = [
  { name: 'Blu', hex: '#0F6CBD' },
  { name: 'Verde acqua', hex: '#038387' },
  { name: 'Verde', hex: '#107C10' },
  { name: 'Viola', hex: '#7160E8' },
  { name: 'Rosa', hex: '#C239B3' },
  { name: 'Rosso', hex: '#C4314B' },
  { name: 'Arancione', hex: '#CA5010' },
  { name: 'Grafite', hex: '#5D5A58' },
] as const;

/** Variabili CSS che dipendono dal colore di accento. */
export const ACCENT_VARS = ['--brand', '--brand-ink', '--on-brand', '--brand-soft', '--brand-sel', '--head', '--on-head', '--head-hover'] as const;
export type AccentVar = (typeof ACCENT_VARS)[number];

const WHITE = '#FFFFFF';
const DARK_TEXT = '#1A1A1A';
/** Sfondi su cui si leggono i colori di accento (vedi app.css). */
const LIGHT_GROUND = '#F0F0F0';
const DARK_SHEET = '#292929';
const AA = 4.5;

interface Lch {
  l: number;
  c: number;
  h: number;
}

/** "#abc", "abc", "#AABBCC" → "#AABBCC"; null se non è un colore. */
export function normalizeHex(input: string): string | null {
  let s = input.trim().replace(/^#/, '');
  if (/^[0-9a-f]{3}$/i.test(s)) s = s.replace(/./g, (ch) => ch + ch);
  return /^[0-9a-f]{6}$/i.test(s) ? `#${s.toUpperCase()}` : null;
}

function toLinear(v: number): number {
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function fromLinear(v: number): number {
  return v <= 0.0031308 ? v * 12.92 : 1.055 * v ** (1 / 2.4) - 0.055;
}

function hexToLinear(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [toLinear(((n >> 16) & 255) / 255), toLinear(((n >> 8) & 255) / 255), toLinear((n & 255) / 255)];
}

/** Luminanza relativa secondo WCAG. */
function luminance(hex: string): number {
  const [r, g, b] = hexToLinear(hex);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Rapporto di contrasto WCAG tra due colori (da 1 a 21). */
export function contrast(a: string, b: string): number {
  const la = luminance(a);
  const lb = luminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

function toLch(hex: string): Lch {
  const [r, g, b] = hexToLinear(hex);
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const L = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
  const A = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
  const B = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
  return { l: L, c: Math.hypot(A, B), h: Math.atan2(B, A) };
}

/** Da OKLCH a sRGB lineare, senza limitare i valori. */
function lchToLinear({ l, c, h }: Lch): [number, number, number] {
  const A = c * Math.cos(h);
  const B = c * Math.sin(h);
  const l_ = (l + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m_ = (l - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s_ = (l - 0.0894841775 * A - 1.291485548 * B) ** 3;
  return [
    4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_,
    -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_,
    -0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_,
  ];
}

const inGamut = (rgb: number[]) => rgb.every((v) => v >= -1e-4 && v <= 1 + 1e-4);

/** Colore OKLCH in esadecimale; se esce dai colori dello schermo si riduce la saturazione. */
function toHex(lch: Lch): string {
  let { c } = lch;
  if (!inGamut(lchToLinear(lch))) {
    let lo = 0;
    let hi = c;
    for (let i = 0; i < 24; i++) {
      const mid = (lo + hi) / 2;
      if (inGamut(lchToLinear({ ...lch, c: mid }))) lo = mid;
      else hi = mid;
    }
    c = lo;
  }
  const hex = lchToLinear({ ...lch, c })
    .map((v) => Math.round(Math.min(1, Math.max(0, fromLinear(v))) * 255))
    .map((v) => v.toString(16).padStart(2, '0'))
    .join('');
  return `#${hex.toUpperCase()}`;
}

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/**
 * Sposta la luminosità del colore nella direzione indicata finché il
 * contrasto con `against` arriva almeno a `min` (o finché si può).
 */
function reach(base: Lch, against: string, min: number, dir: -1 | 1): string {
  let lch = base;
  let hex = toHex(lch);
  for (let i = 0; i < 100 && contrast(hex, against) < min; i++) {
    const l = lch.l + dir * 0.01;
    if (l < 0 || l > 1) break;
    lch = { ...lch, l };
    hex = toHex(lch);
  }
  return hex;
}

/** Testo più leggibile sul colore: bianco o scuro. */
export function readableOn(hex: string): string {
  return contrast(hex, WHITE) >= contrast(hex, DARK_TEXT) ? WHITE : DARK_TEXT;
}

/** Riempimento dei pulsanti con il testo più leggibile, bianco o scuro. */
function fill(base: Lch): { fill: string; text: string } {
  const text = readableOn(toHex(base));
  return { fill: reach(base, text, AA, text === WHITE ? -1 : 1), text };
}

/** Variabili CSS per il colore di accento nel tema indicato. */
export function accentPalette(accent: string, theme: 'chiaro' | 'scuro'): Partial<Record<AccentVar, string>> {
  const hex = normalizeHex(accent) ?? DEFAULT_ACCENT;
  const a = toLch(hex);
  const tint = (l: number, k: number, max: number) => toHex({ l, c: Math.min(a.c * k, max), h: a.h });
  if (theme === 'chiaro') {
    // Né troppo chiaro (sparirebbe sul bianco) né troppo scuro.
    const btn = fill({ ...a, l: clamp(a.l, 0.3, 0.8) });
    return {
      '--brand': btn.fill,
      '--brand-ink': reach(a, LIGHT_GROUND, AA, -1),
      '--on-brand': btn.text,
      '--brand-soft': tint(0.962, 0.11, 0.03),
      '--brand-sel': tint(0.905, 0.3, 0.06),
      // In chiaro la barra in alto ha il colore di accento, come in Outlook.
      '--head': btn.fill,
      '--on-head': btn.text,
      '--head-hover': btn.text === WHITE ? 'rgba(255, 255, 255, .16)' : 'rgba(0, 0, 0, .08)',
    };
  }
  // In scuro i pulsanti sono un po' più scuri e i testi più chiari, come in Fluent.
  const btn = fill({ ...a, l: clamp(a.l - 0.04, 0.45, 0.85) });
  return {
    '--brand': btn.fill,
    '--brand-ink': reach({ ...a, l: Math.max(a.l, 0.5) }, DARK_SHEET, AA, 1),
    '--on-brand': btn.text,
    '--brand-soft': tint(0.255, 0.35, 0.05),
    '--brand-sel': tint(0.315, 0.5, 0.075),
  };
}

/** Quanto il colore usato si allontana da quello scelto (distanza in OKLab). */
export function accentShift(accent: string, used: string): number {
  const a = toLch(normalizeHex(accent) ?? DEFAULT_ACCENT);
  const b = toLch(used);
  const ab = (x: Lch) => [x.l, x.c * Math.cos(x.h), x.c * Math.sin(x.h)];
  const [p, q] = [ab(a), ab(b)];
  return Math.hypot(p[0] - q[0], p[1] - q[1], p[2] - q[2]);
}

/** Applica il colore di accento all'elemento (di solito <html>); null torna al blu. */
export function applyAccent(el: HTMLElement, accent: string | null, theme: 'chiaro' | 'scuro') {
  for (const v of ACCENT_VARS) el.style.removeProperty(v);
  if (!accent) return;
  for (const [k, v] of Object.entries(accentPalette(accent, theme))) el.style.setProperty(k, v);
}
