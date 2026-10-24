# Better-Look: note per chi lavora sul codice

Client di posta desktop open source, Tauri 2 (Rust) + Svelte 5 + TypeScript. L'utente di riferimento è Gimmy: non è uno sviluppatore, prova gli installer prodotti da GitHub Actions e decide. Scrivigli in italiano, in modo semplice.

Leggi prima `docs/progetto.md` (decisioni prese e piano) e `docs/PRODUCT.md`.

## Regole di prodotto da rispettare

- Interfaccia in **italiano**.
- Posizioni dei comandi e scorciatoie come **Outlook**, nelle due varianti (classica e nuova). Mai nomi, loghi o icone Microsoft.
- L'interfaccia di lavoro non mostra né il nome né il logo di Better-Look. Il logo compare solo nel primo avvio e nella pagina "Informazioni su Better-Look".
- Il nome dell'app è **Better-Look**, con il trattino. Nessun riferimento al nome precedente (resta solo in `src-tauri/src/legacy.rs`, per spostare i dati delle anteprime) né a Paxxion nell'app, negli installer o nei documenti. Il repository è quello personale di Gimmy, oggi `Gimmypaxxion/datini-email` (da rinominare), non un'organizzazione aziendale.
- Better-Look è e resterà gratuito, libero e open source. Eventuali servizi di IA saranno un prodotto separato.
- Colori solo tramite le variabili CSS di `src/app.css` (palette Fluent 2), in chiaro e scuro. Il colore di accento scelto dall'utente sostituisce le variabili del blu (`--brand`, `--brand-ink`, `--on-brand`, `--brand-soft`, `--brand-sel`, `--head`): usa sempre quelle, mai un blu scritto a mano. Le regole sono in `src/lib/accent.ts`.
- Icone solo da Tabler, registrate in `src/lib/icons.ts`.
- La posta va **direttamente** dal computer al server del provider: niente server intermedi, niente tracciamento.
- Sicurezza di default:
  - HTML delle email ripulito e isolato;
  - immagini remote bloccate finché l'utente non le consente;
  - password nel portachiavi del sistema.

## Comandi

```bash
npm test              # test della logica (vitest)
npm run check         # tipi (svelte-check)
npm run build         # build dell'interfaccia
npm run tauri dev     # app desktop in sviluppo
npm run tauri build   # installer locale
```

Prima di consegnare una modifica devono passare `npm run check`, `npm test` e `npm run build`. Se tocchi Rust, anche `cargo test --manifest-path src-tauri/Cargo.toml --lib`.

Prova completa del motore con un server di posta vero: vedi "Server di posta di prova" nel README (`tools/posta-di-prova`, poi `BETTERLOOK_SERVER_PROVA=1 cargo test --test server_prova -- --test-threads=1`). Per i PST: `BETTERLOOK_PST=file.pst cargo test --test importa_pst`.

Variabili utili per le prove dell'app: `BETTERLOOK_DATI` cambia la cartella dei dati; `BETTERLOOK_PORTACHIAVI=memoria` tiene le password in memoria (solo per le prove automatiche).

Mai scrivere password vere nel codice, nei test o nei commit.

Per le schermate di prova si può aprire l'interfaccia nel browser con i parametri `?stile=…&view=…&tema=…&avvio=0`, descritti nel README.

## Stato

- **Fasi da 1 a 8 completate** (versione 0.2.0): motore IMAP, POP3 e SMTP con archivio locale e uso offline, scrittura in finestre del sistema con testo formattato e firme, più account, apertura dei PST, MBOX ed EML; cartelle, categorie, conversazioni (Ignora, Pulisci), stampa, lettura ad alta voce, regole e avvisi, risposte automatiche, invio programmato, pulizia, rubrica, calendario con inviti; attività, azioni rapide, modelli di messaggio, posizione del riquadro di lettura e densità. Riepilogo e limiti noti in `docs/progetto.md`, sezione 0.
- L'interfaccia parla con il motore tramite `src/lib/backend.ts`. Nel browser e nei test usa `src/lib/mockBackend.ts`, con i messaggi e gli appuntamenti di esempio di `src/lib/mock.ts`.
- Editor di testo: RoosterJS (`src/lib/editor.svelte.ts`) in un iframe isolato; firme con predefinite per account; formato HTML o Testo normale.
- Calendario: file `.ics` e inviti in `src-tauri/src/calendar.rs`, operazioni in `src-tauri/src/engine/agenda.rs`; ripetizioni espanse nell'interfaccia in `src/lib/calendar.ts`, stato in `src/lib/calendar.svelte.ts`.
- Attività: motore in `src-tauri/src/engine/tasks.rs` (tabella `tasks`), elenchi intelligenti in `src/lib/tasks.ts`, stato in `src/lib/tasks.svelte.ts`. Azioni rapide e modelli: `src-tauri/src/engine/quick.rs` e `src/lib/quicksteps.ts`.
- Sul Mac Cmd+Q, Cmd+W, Cmd+H e Cmd+M non vanno mai intercettati in `src/lib/shortcuts.ts` (WKWebView li perderebbe); i testi delle scorciatoie passano da `keysText` (`src/lib/platform.ts`), che mostra ⌘ ⇧ ⌥ sul Mac.
- Classi CSS globali in `src/app.css` (`.opt`, `.field`, `.row`, `.two`...): nei componenti nuovi usa nomi diversi, altrimenti gli stili si sommano.
- Le release si pubblicano con un tag `vX.Y.Z` (installer nella pagina Release, non in bozza).
- Le prove con il server di posta vero vanno eseguite una alla volta (`-- --test-threads=1`): ripopolano la stessa casella.
- **Prossimi passi possibili:** calendari online (CalDAV), sincronizzazione delle attività, estensioni (da decidere).
- **Rinviati per scelta dell'utente:**
  - accesso OAuth Microsoft 365 e Gmail;
  - traduzione (con i futuri servizi IA, prodotto separato);
  - firma del codice;
  - licenza;
  - componenti COM e macro VBA (da decidere).
