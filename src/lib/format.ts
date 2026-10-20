// Piccole funzioni di formattazione condivise dall'interfaccia.

/** Iniziali per l'avatar: prima lettera delle prime due parole. */
export function initials(name: string): string {
  const words = String(name || '')
    .replace(/[^A-Za-zÀ-ÿ ]/g, ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (!words.length) return '?';
  return ((words[0][0] || '') + (words.length > 1 ? words[1][0] : '')).toUpperCase();
}

/**
 * Tinta dell'avatar (1-5) scelta con un hash del nome, così la stessa persona
 * ha sempre lo stesso colore. Il moltiplicatore 37 distribuisce bene i nomi italiani.
 */
export function tintIndex(name: string): number {
  const s = String(name || '');
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 37 + s.charCodeAt(i)) % 9973;
  return (h % 5) + 1;
}

export function tint(name: string): { bg: string; fg: string } {
  const k = tintIndex(name);
  return { bg: `var(--av${k}-bg)`, fg: `var(--av${k}-fg)` };
}

const DAYS = ['dom', 'lun', 'mar', 'mer', 'gio', 'ven', 'sab'];
const pad = (n: number) => (n < 10 ? '0' : '') + n;

/** Ora corrente nei formati usati nell'elenco (08:42) e nell'intestazione (ven 16/10/2026 08:42). */
export function clock(d = new Date()): { hhmm: string; long: string; stamp: number } {
  const hhmm = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  const long = `${DAYS[d.getDay()]} ${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()} ${hhmm}`;
  return { hhmm, long, stamp: d.getTime() };
}

/** Toglie i prefissi Re:, I:, R:, Fw:, Fwd: ripetuti dall'oggetto. */
export function cleanSubject(subject: string): string {
  return subject.replace(/^((re|i|r|fw|fwd):\s*)+/i, '');
}
