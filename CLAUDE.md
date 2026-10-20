# Datini: note per chi lavora sul codice

Client di posta desktop open source, Tauri 2 (Rust) + Svelte 5 + TypeScript. L'utente di riferimento è Gimmy (Paxxion): non è uno sviluppatore, prova gli installer prodotti da GitHub Actions e decide. Scrivigli in italiano, in modo semplice.

Leggi prima `docs/progetto.md` (decisioni prese e piano) e `docs/PRODUCT.md`.

## Regole di prodotto da rispettare

- Interfaccia in **italiano**.
- Posizioni dei comandi e scorciatoie come **Outlook**, nelle due varianti (classica e nuova). Mai nomi, loghi o icone Microsoft.
- L'interfaccia di lavoro non mostra né il nome né il logo di Datini. Il logo compare solo nel primo avvio e nella pagina "Informazioni su Datini".
- Colori solo tramite le variabili CSS di `src/app.css` (palette Fluent 2), in chiaro e scuro.
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

Prima di consegnare una modifica devono passare `npm run check`, `npm test` e `npm run build`. Se tocchi Rust, anche `cargo check --manifest-path src-tauri/Cargo.toml`.

Per le schermate di prova si può aprire l'interfaccia nel browser con i parametri `?stile=…&view=…&tema=…&avvio=0`, descritti nel README.

## Stato

- **Fase 1 completata.** Interfaccia con dati di esempio (`src/lib/mock.ts`), stato in `src/lib/store.svelte.ts`.
- **Prossima fase: 2.** Motore IMAP/SMTP in Rust, esposto all'interfaccia con comandi Tauri. Crate suggeriti in `docs/progetto.md`.
- **Rinviati per scelta dell'utente:**
  - accesso OAuth Microsoft 365 e Gmail;
  - firma del codice;
  - licenza;
  - componenti COM e macro VBA (da decidere).
