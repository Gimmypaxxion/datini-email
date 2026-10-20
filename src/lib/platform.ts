// Collegamento con Tauri. Se l'interfaccia gira in un normale browser
// (sviluppo, test, anteprime), le funzioni non fanno nulla e non si rompono.

export const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function appVersion(): Promise<string> {
  if (!inTauri) return '';
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    return await getVersion();
  } catch {
    return '';
  }
}

/** Titolo della finestra del sistema, come in Outlook: "Posta in arrivo - Datini". */
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
