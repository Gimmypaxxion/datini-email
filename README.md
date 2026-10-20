# Datini

Datini è un client di posta desktop **libero e open source** per Windows, macOS e Linux. È pensato per chi usa Outlook ogni giorno: i comandi stanno dove li mette Outlook, sia nella **vista classica** sia nella **nuova**, così si lavora dal primo giorno senza riabituarsi.

Porta il nome di Francesco Datini, mercante di Prato del Trecento. Il suo archivio conserva circa 143.000 lettere d'affari.

> Datini non è affiliato a Microsoft. "Outlook" è un marchio di Microsoft Corporation e qui è citato solo per descrivere la disposizione dei comandi.

## A che punto siamo

**Fase 1, versione di anteprima.** L'interfaccia completa funziona con **messaggi di esempio**: non si collega ancora a nessuna casella di posta. Il motore di posta arriva dalla fase 2.

Cosa si può già provare:
- primo avvio con la scelta tra Outlook classico e nuovo Outlook;
- barra dei comandi estesa o compatta, menu File o ingranaggio;
- nuovo messaggio in finestra separata o nel riquadro di lettura;
- cartelle, schede Prioritaria e Altre, ricerca;
- lettura, risposta, risposta a tutti, inoltro;
- invio con Annulla, bozze;
- tema chiaro, scuro o come il sistema;
- scorciatoie da tastiera di Outlook.

## Scaricare e provare

A ogni modifica GitHub prepara gli installer da solo:

1. Apri la scheda **Actions** del repository e scegli l'ultima esecuzione di **Installer** con il segno verde.
2. In fondo, nella sezione **Artifacts**, scarica il file del tuo sistema:
   - `datini-windows`: installer `.exe` e `.msi`;
   - `datini-macos-apple-silicon` o `datini-macos-intel`: `.dmg`;
   - `datini-linux`: `.deb`, `.rpm`, `.AppImage`.

Gli installer **non sono ancora firmati**, quindi al primo avvio compare un avviso:
- **Windows:** nella finestra "Windows ha protetto il PC", clicca *Ulteriori informazioni* e poi *Esegui comunque*.
- **macOS:** apri Datini dalla cartella Applicazioni con il tasto destro, poi scegli *Apri*.

Per pubblicare una versione basta creare un tag, per esempio `v0.1.0`: GitHub prepara una release in bozza con tutti gli installer.

## Sviluppo

Requisiti:
- [Node.js](https://nodejs.org) 22 o successivo;
- [Rust](https://rustup.rs) stabile;
- le [librerie di sistema richieste da Tauri](https://tauri.app/start/prerequisites/). Su Windows 11 e macOS sono già quasi tutte presenti; su Linux servono i pacchetti di WebKitGTK.

```bash
npm install
npm run tauri dev      # avvia l'app con ricaricamento automatico
npm test               # test della logica dell'interfaccia
npm run check          # controllo dei tipi
npm run tauri build    # installer per il sistema in uso
```

Per lavorare solo sull'interfaccia nel browser basta `npm run dev`, poi aprire http://localhost:1420.

Gli stati del prototipo si aprono aggiungendo parametri all'indirizzo, per esempio `?stile=nuovo&view=composizione&tema=scuro&avvio=0`:
- `stile`: `classico`, `nuovo`;
- `view`: `composizione`, `inviato`, `file`, `impostazioni`, `account`, `informazioni`;
- `tema`: `chiaro`, `scuro`, `sistema`;
- `avvio`: `1` per mostrare il primo avvio, `0` per saltarlo;
- `offline`: `1` per mostrare l'avviso di connessione assente.

## Struttura

```
src/                 interfaccia (Svelte 5 + TypeScript)
  App.svelte         disposizione generale della finestra
  app.css            colori (palette Fluent 2) e stili comuni
  components/        barra in alto, comandi, cartelle, elenco, lettura, scrittura, menu File, impostazioni, primo avvio
  lib/store.svelte.ts  stato e azioni dell'interfaccia
  lib/mock.ts        messaggi di esempio (fase 1)
  lib/shortcuts.ts   scorciatoie da tastiera
src-tauri/           applicazione desktop (Rust + Tauri 2)
tests/               test della logica
docs/                decisioni di progetto, contesto di prodotto e prototipo originale
assets/              logo e icona del programma
```

## Piano

1. **Fondamenta:** progetto, interfaccia, installer automatici. *(fatto)*
2. Primo account reale IMAP/SMTP con password: cartelle, messaggi, lettura sicura dell'HTML, invio.
3. Archivio locale, uso offline, ricerca.
4. Scrittura completa: bozze, allegati, finestre separate vere.
5. POP3 e più account.
6. Import e apertura dei file PST, export MBOX/EML.
7. Accesso Microsoft 365 e Gmail (OAuth), firma del codice, aggiornamenti automatici.

I dettagli e le decisioni prese sono in [`docs/progetto.md`](docs/progetto.md).

## Licenza

Da definire prima della pubblicazione (in valutazione GPL-3.0 o MPL-2.0).

## Riconoscimenti

- [Tauri](https://tauri.app) e [Svelte](https://svelte.dev).
- Icone [Tabler Icons](https://tabler.io/icons), licenza MIT.
- Colori ispirati alla palette [Fluent 2](https://fluent2.microsoft.design) di Microsoft, pubblicata con licenza MIT.
