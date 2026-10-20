# Product

<!-- impeccable:product-schema 1 -->

> Aggiornato il 19 ottobre 2026 con le decisioni confermate da Gimmy durante la progettazione. Sostituisce la versione iniziale, che conteneva ipotesi poi superate (nome provvisorio, identità visiva originale, colori diversi da Outlook).

## Platform

desktop (Windows, macOS, Linux)

## Stack

Raccomandato: Tauri 2, motore in Rust, interfaccia in HTML/CSS/TypeScript. Il codice lo scrive Claude; Gimmy prova gli installer prodotti da GitHub Actions.

## Users

Primari: i clienti di Paxxion, PMI italiane. Sono impiegati che usano Outlook (classico o nuovo) tutti i giorni e devono passare a Datini senza riabituarsi. Secondari: chiunque voglia un client di posta desktop gratuito e open source.

## Product Purpose

Datini è un client di posta gratuito e open source. Ha successo quando un utente di Outlook, il primo giorno, trova ogni comando dove se lo aspetta e lavora senza formazione.

## Positioning

Posizioni dei comandi, scorciatoie e sensazione cromatica di Outlook, in vista classica o nuova a scelta. Codice aperto, posta che va direttamente dal computer al server del provider, nessun server intermedio.

## Operating Context

Lavoro d'ufficio quotidiano in italiano: smistare, leggere, rispondere, inoltrare, archiviare, contrassegnare, cercare. Sessioni lunghe alla scrivania, spesso su PC non recenti.

Account:
- IMAP/SMTP generici (Aruba, Register.it, hosting);
- POP;
- più avanti Microsoft 365 e Gmail con OAuth.

## Capabilities and Constraints

- Layout a tre riquadri: cartelle a sinistra, elenco al centro, lettura a destra, barra dei comandi sopra.
- Vista classica o nuova, scelta dall'utente al primo avvio e modificabile:
  - barra dei comandi estesa o compatta;
  - accesso alle impostazioni dal menu File o dall'ingranaggio;
  - nuovo messaggio in finestra separata o nel riquadro di lettura.
- Sincronizzazione diretta con il server di ogni account. Posta offline configurabile dall'utente per IMAP e POP.
- PST: import e apertura. Export MBOX/EML.
- Colori ispirati a Outlook (palette Fluent 2, licenza MIT).
- Mai usare nomi, loghi o icone Microsoft. Icone Tabler (MIT).
- Interfaccia in italiano.
- L'app deve sembrare integrata nel sistema operativo: nessun nome né logo nell'interfaccia di lavoro.

## Brand Commitments

- Nome: Datini, in onore di Francesco Datini, mercante di Prato.
- Logo: scritta DATINI e "D" fornite da Gimmy (cartella `logo/`). Si usano solo nel primo avvio e nella pagina "Informazioni su Datini"; la D in futuro come icona del programma.
- Branding volutamente discreto.

## Evidence on Hand

Nessuna ricerca utenti. Tutti i contenuti di posta nei mockup sono finti.

## Product Principles

1. Posizioni familiari: chi viene da Outlook non deve cercare nulla.
2. Prima il lavoro: la velocità di smistamento conta più della decorazione.
3. Aperto e onesto: niente server intermedi, niente tracciamento, permessi chiari.
4. Sicuro di default: HTML delle email ripulito, immagini remote bloccate finché l'utente non le consente, password nel portachiavi del sistema.

## Accessibility & Inclusion

Contrasto WCAG AA, uso completo da tastiera, focus sempre visibile, rispetto di "riduci movimento".
