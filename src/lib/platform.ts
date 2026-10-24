// Collegamento con Tauri. Se l'interfaccia gira in un normale browser
// (sviluppo, test, anteprime), le funzioni non fanno nulla e non si rompono.

/** Repository pubblico del progetto. */
export const REPO_URL = 'https://github.com/Gimmypaxxion/datini-email';

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Il programma gira su un Mac: Cmd al posto di Ctrl, Cmd+Q per uscire. */
export const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/i.test(navigator.platform || navigator.userAgent || '');

/** Scorciatoie scritte come le mostra il sistema: "Ctrl+Maiusc+R" diventa "⇧⌘R" sul Mac. */
export function keysText(s: string, mac = isMac): string {
  if (!mac) return s;
  return s.replace(/((?:Ctrl|Maiusc|Alt)\+)+([^\s),]+)/g, (all, _mods: string, key: string) => {
    const mods = all.slice(0, all.length - key.length);
    const sym = `${mods.includes('Alt+') ? '⌥' : ''}${mods.includes('Maiusc+') ? '⇧' : ''}${mods.includes('Ctrl+') ? '⌘' : ''}`;
    return `${sym}${key}`;
  });
}

/** Esce dal programma (Cmd+Q sul Mac). */
export async function quitApp(): Promise<void> {
  if (!inTauri) return;
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('quit_app');
  } catch {
    /* ci pensa il menu del sistema */
  }
}

export async function appVersion(): Promise<string> {
  if (!inTauri) return '';
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return '';
  }
}

/** Titolo della finestra del sistema, come in Outlook: "Posta in arrivo - Better-Look". */
export async function setWindowTitle(title: string): Promise<void> {
  if (typeof document !== 'undefined') document.title = title;
  if (!inTauri) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setTitle(title);
  } catch {
    /* il titolo resta quello del documento */
  }
}
