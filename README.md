# Datini

Datini è un client di posta desktop **libero e open source** per Windows, macOS e Linux. È pensato per chi usa Outlook ogni giorno: i comandi stanno dove li mette Outlook, sia nella **vista classica** sia nella **nuova**, così si lavora dal primo giorno senza riabituarsi.

Porta il nome di Francesco Datini, mercante di Prato del Trecento. Il suo archivio conserva circa 143.000 lettere d'affari.

> Datini non è affiliato a Microsoft. "Outlook" è un marchio di Microsoft Corporation e qui è citato solo per descrivere la disposizione dei comandi e la compatibilità con i file PST.

## A che punto siamo

**Versione di anteprima, da provare con caselle di prova.** Datini si collega a caselle vere con IMAP o POP3 e invia con SMTP.

Cosa funziona:
- **Account:**
  - aggiunta guidata con email e password, con riconoscimento automatico dei server (provider italiani noti, configurazione automatica, record MX);
  - impostazioni manuali se il riconoscimento non riesce;
  - IMAP e POP3, anche più account insieme.
- **Posta:**
  - cartelle e ruoli (arrivo, inviata, bozze, eliminata, indesiderata, archivio), anche con nomi italiani;
  - schede Prioritaria e Altre: newsletter e notifiche automatiche vanno in Altre;
  - lettura, risposta, risposta a tutti, inoltro;
  - sposta, elimina, archivia, contrassegna, letto e da leggere.
- **Offline e ricerca:**
  - archivio sul computer con ricerca nel testo;
  - uso offline: le modifiche partono quando torna la connessione.
- **Scrittura:**
  - editor di testo formattato con lo stesso motore della versione web di Outlook: carattere, dimensione, grassetto, colori, evidenziazione, elenchi (anche automatici con `* ` o `1. `), rientri, allineamento, interlinea, collegamenti, immagini e tabelle;
  - schede Messaggio, Inserisci e Formato testo con i comandi dove li mette Outlook; formato HTML o Testo normale;
  - incolla da Word, Excel e dai siti mantenendo la formattazione;
  - firme come in Outlook: più firme con un nome, una predefinita per i nuovi messaggi e una per le risposte di ogni account, pulsante Firma per cambiarla; la prima parte già scritta con nome e indirizzo;
  - nuovo messaggio in una vera finestra del sistema o nel riquadro di lettura;
  - suggerimento dei contatti, Cc e Ccn, allegati;
  - bozze salvate da sole;
  - conferma se manca l'oggetto;
  - 7 secondi per annullare l'invio;
  - posta in uscita che aspetta la connessione.
- **Archivi:**
  - apertura dei file PST di Outlook (solo la posta), con avanzamento e resoconto finale;
  - importazione di file MBOX;
  - esportazione di un messaggio in EML e di una cartella in MBOX.
- **Interfaccia:**
  - primo avvio con la scelta tra Outlook classico e nuovo Outlook;
  - barra estesa o compatta, menu File o ingranaggio, tema chiaro o scuro;
  - [scorciatoie da tastiera](docs/scorciatoie.md) di Outlook.

Cosa manca ancora:
- **Microsoft 365, Outlook.com e Gmail con accesso moderno (OAuth).** Su queste caselle l'accesso IMAP con la sola password è già disattivato o lo sarà presto. Gmail per ora si può usare solo con una "password per le app", se Google la consente sull'account.
- Regole, risposte automatiche, invio programmato.
- Calendario, contatti e attività.
- File `.ost` (copie locali di Exchange): non si aprono, va esportato un `.pst`.
- Firma del codice e aggiornamenti automatici.

## Sicurezza e privacy

- La posta va **direttamente** dal computer al server del provider. Non ci sono server di Datini in mezzo e niente tracciamento.
- Le password stanno nel **portachiavi del sistema** (Gestione credenziali di Windows, Portachiavi di macOS, Secret Service su Linux), mai in chiaro su disco.
- Le connessioni sono sempre cifrate (SSL/TLS o STARTTLS) e i certificati vengono verificati.
- L'HTML delle email viene ripulito e mostrato in un riquadro isolato, senza script.
- Le **immagini esterne restano bloccate** finché non scegli di scaricarle: servono spesso a sapere quando e dove apri un messaggio.
- Datini non apre gli allegati eseguibili (`.exe`, `.bat`, `.js` e simili). I link si aprono nel browser.

## Scaricare e provare

A ogni modifica GitHub prepara gli installer da solo:

1. Apri [Actions › Installer](https://github.com/Gimmypaxxion/datini-email/actions/workflows/installer.yml) e scegli l'esecuzione più recente con il segno verde.
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
npm test               # test dell'interfaccia
npm run check          # controllo dei tipi
cargo test --manifest-path src-tauri/Cargo.toml --lib   # test del motore
npm run tauri build    # installer per il sistema in uso
```

### Server di posta di prova

Per collaudare il motore senza toccare caselle vere c'è un server di prova (Ubuntu o Debian), che i controlli automatici usano a ogni modifica:

```bash
sudo apt-get install dovecot-imapd dovecot-pop3d
pip install aiosmtpd
sudo tools/posta-di-prova/avvia.sh          # IMAP, POP3 e SMTP su mail.datini.test
python3 tools/posta-di-prova/popola.py      # 54 messaggi di esempio
DATINI_SERVER_PROVA=1 cargo test --manifest-path src-tauri/Cargo.toml --test server_prova -- --test-threads=1
```

Le caselle sono `prova@datini.test` (password `prova-123`) e `altro@datini.test` (password `altro-456`).

Per provare l'apertura di un file PST: `DATINI_PST=/percorso/file.pst cargo test --manifest-path src-tauri/Cargo.toml --test importa_pst -- --nocapture`.

### Interfaccia nel browser

Per lavorare solo sull'interfaccia basta `npm run dev`, poi aprire http://localhost:1420. Nel browser Datini usa messaggi di esempio, senza collegarsi a nessuna casella.

Gli stati si aprono aggiungendo parametri all'indirizzo, per esempio `?stile=nuovo&view=composizione&tema=scuro&avvio=0`:
- `stile`: `classico`, `nuovo`;
- `view`: `composizione`, `inviato`, `file`, `impostazioni`, `account`, `informazioni`, `aggiungi`, `newsletter`, `uscita`;
- `tema`: `chiaro`, `scuro`, `sistema`;
- `avvio`: `1` per mostrare il primo avvio, `0` per saltarlo;
- `offline`: `1` per mostrare l'avviso di connessione assente.

## Struttura

```
src/                     interfaccia (Svelte 5 + TypeScript)
  App.svelte             disposizione generale della finestra
  app.css                colori (palette Fluent 2) e stili comuni
  components/            barra in alto, comandi, cartelle, elenco, lettura, scrittura (anche in finestra separata), menu File, impostazioni
  lib/store.svelte.ts    stato e azioni dell'interfaccia
  lib/backend.ts         collegamento al motore (comandi Tauri)
  lib/mockBackend.ts     motore finto con i messaggi di esempio, per il browser e i test
  lib/shortcuts.ts       scorciatoie da tastiera
  lib/editor.svelte.ts   editor di testo formattato (RoosterJS in un iframe isolato)
  lib/compose-html.ts    nuovo messaggio, citazione, firma e testo normale
src-tauri/src/           motore (Rust)
  engine.rs              account, sincronizzazione, posta in uscita, bozze
  imap.rs, pop3.rs, smtp.rs   protocolli di posta
  autoconfig.rs          riconoscimento automatico dei server
  db.rs                  archivio locale SQLite con ricerca
  mime.rs                lettura e scrittura dei messaggi
  sanitize.rs            pulizia dell'HTML delle email
  pst.rs, rtf.rs         apertura dei file PST e importazione MBOX
  secrets.rs             password nel portachiavi del sistema
tests/                   test dell'interfaccia
src-tauri/tests/         prove del motore con un server vero e con file PST
tools/posta-di-prova/    server di posta per le prove
docs/                    decisioni di progetto, contesto di prodotto, scorciatoie
assets/                  logo e icona del programma
```

## Piano

1. **Fondamenta:** progetto, interfaccia, installer automatici. *(fatto)*
2. **Primo account reale** IMAP/SMTP con password: cartelle, messaggi, lettura sicura dell'HTML, invio. *(fatto)*
3. **Archivio locale**, uso offline, ricerca. *(fatto)*
4. **Scrittura completa:** bozze, allegati, finestre separate vere, testo formattato, firme. *(fatto)*
5. **POP3 e più account.** *(fatto)*
6. **Archivi:** apertura dei file PST, import MBOX, export MBOX ed EML. *(fatto)*
7. Accesso Microsoft 365 e Gmail (OAuth), firma del codice, aggiornamenti automatici, licenza.

I dettagli e le decisioni prese sono in [`docs/progetto.md`](docs/progetto.md).

## Licenza

Da definire prima della pubblicazione (in valutazione GPL-3.0 o MPL-2.0).

## Riconoscimenti

- [Tauri](https://tauri.app) e [Svelte](https://svelte.dev).
- Posta: [async-imap](https://github.com/chatmail/async-imap), [lettre](https://lettre.rs), [mail-parser e mail-builder](https://github.com/stalwartlabs), [ammonia](https://github.com/rust-ammonia/ammonia), [outlook-pst](https://github.com/microsoft/outlook-pst-rs).
- Editor di testo: [RoosterJS](https://github.com/microsoft/roosterjs), licenza MIT.
- Archivio: [SQLite](https://sqlite.org) con [rusqlite](https://github.com/rusqlite/rusqlite).
- Icone [Tabler Icons](https://tabler.io/icons), licenza MIT.
- Colori ispirati alla palette [Fluent 2](https://fluent2.microsoft.design) di Microsoft, pubblicata con licenza MIT.
