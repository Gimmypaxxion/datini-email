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

Prova completa del motore con un server di posta vero: vedi "Server di posta di prova" nel README (`tools/posta-di-prova`, poi `BETTERLOOK_SERVER_PROVA=1 cargo test --test server_prova`). Per i PST: `BETTERLOOK_PST=file.pst cargo test --test importa_pst`.

Variabili utili per le prove dell'app: `BETTERLOOK_DATI` cambia la cartella dei dati; `BETTERLOOK_PORTACHIAVI=memoria` tiene le password in memoria (solo per le prove automatiche).

Mai scrivere password vere nel codice, nei test o nei commit.

Per le schermate di prova si può aprire l'interfaccia nel browser con i parametri `?stile=…&view=…&tema=…&avvio=0`, descritti nel README.

## Stato

- **Fasi da 1 a 6 completate**: motore IMAP, POP3 e SMTP con archivio locale e uso offline, scrittura in finestre del sistema con testo formattato e firme, più account, apertura dei PST, MBOX ed EML. Riepilogo e limiti noti in `docs/progetto.md`, sezione 0.
- L'interfaccia parla con il motore tramite `src/lib/backend.ts`. Nel browser e nei test usa `src/lib/mockBackend.ts`, con i messaggi di esempio di `src/lib/mock.ts`.
- Editor di testo: RoosterJS (`src/lib/editor.svelte.ts`) in un iframe isolato; firme con predefinite per account; formato HTML o Testo normale.
- **Prossimi passi possibili:** regole, risposte automatiche, invio programmato, calendario e contatti.
- **Rinviati per scelta dell'utente:**
  - accesso OAuth Microsoft 365 e Gmail;
  - firma del codice;
  - licenza;
  - componenti COM e macro VBA (da decidere).
