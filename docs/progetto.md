# Better-Look: documento di passaggio

Questo file raccoglie le decisioni di progetto e lo stato dello sviluppo. Le sezioni da 1 a 7 vengono dalla fase di progettazione dell'interfaccia (19 ottobre 2026); la sezione 0 dice a che punto è il codice.

**Per una nuova chat:** leggi prima questo file, poi `PRODUCT.md` e `brief-interfaccia.md`, poi `CLAUDE.md` nella cartella principale.

---

## 0. Stato dello sviluppo (24 ottobre 2026)

Repository: [Gimmypaxxion/datini-email](https://github.com/Gimmypaxxion/datini-email), personale e pubblico. È un progetto personale e sperimentale di Gimmy. Va rinominato per il nuovo nome; GitHub reindirizza i vecchi indirizzi, poi si aggiornano `REPO_URL` in `src/lib/platform.ts` e i collegamenti nel README.

**Nome (23 ottobre 2026).** Il programma si chiama Better-Look: identificativo `app.betterlook`, archivio `better-look.db`, servizio del portachiavi `Better-Look`, variabili d'ambiente `BETTERLOOK_…`. Al primo avvio `legacy.rs` sposta cartella dei dati, archivio e password delle anteprime uscite con il nome precedente. Le preferenze dell'interfaccia non si spostano (il sistema le tiene per identificativo): ricompare la scelta tra Outlook classico e nuovo. Su Windows e macOS il nuovo programma si installa accanto a quello vecchio, che va disinstallato a mano dopo il primo avvio.

**Fasi da 1 a 8 completate** (versione 0.2.0). Il motore in Rust è collegato all'interfaccia con comandi Tauri.

| Parte | Come è fatta |
|---|---|
| Archivio | SQLite in un file nella cartella dati dell'app: account, cartelle, messaggi, testo completo per la ricerca (FTS5), coda delle modifiche offline, posta in uscita. |
| IMAP | `async-imap` con TLS (`rustls` e verifica dei certificati del sistema). Un lavoratore per account sincronizza ogni 2 minuti e subito dopo ogni modifica. I ruoli delle cartelle vengono da SPECIAL-USE o dai nomi (anche italiani); se mancano, Inviata, Bozze ed Eliminata vengono create. |
| POP3 | Implementazione propria (USER/PASS, UIDL, RETR, DELE). Copia sul server tenuta secondo l'opzione scelta (mai, 14 giorni, sempre). |
| SMTP | `lettre`. Invio dalla coda di uscita dopo 7 secondi, per poterlo annullare. Copia in Posta inviata se il server non la salva da solo. |
| Offline | Le modifiche (letto, contrassegno, sposta, elimina, bozze) si applicano subito sul computer e partono verso il server quando c'è connessione. |
| Messaggi | `mail-parser` (con tutte le codifiche dei caratteri) e `mail-builder`/`lettre` per scriverli. HTML ripulito con `ammonia` e mostrato in un iframe isolato, con CSP e immagini esterne bloccate. |
| Riconoscimento dei server | Tabella dei provider noti (Aruba, Libero, Virgilio, Tiscali, Gmail...), configurazione automatica in stile Thunderbird, record MX, poi tentativi con `imap.` e `mail.`. I domini Microsoft vengono riconosciuti e rimandati a quando ci sarà OAuth. |
| Password | Portachiavi del sistema (`keyring`). |
| Finestre | Il nuovo messaggio si apre in una vera finestra del sistema (vista classica) o nel riquadro di lettura (vista nuova). |
| PST | Libreria `outlook-pst` (Rust, MIT, pubblicata da Microsoft). Ogni messaggio viene ricostruito come email completa: intestazioni originali quando ci sono, testo, HTML (anche quello salvato solo come RTF), allegati e immagini. Si importano solo le cartelle di posta. |
| MBOX ed EML | Import MBOX in un file di dati; export di una cartella in MBOX e di un messaggio in EML. |
| Editor di testo | [RoosterJS](https://github.com/microsoft/roosterjs) (MIT), lo stesso motore della versione web e del nuovo Outlook, dentro un iframe con una Content-Security-Policy che vieta script e immagini esterne: l'HTML citato non può cambiare l'aspetto di Better-Look né scaricare immagini di tracciamento. Schede Messaggio, Inserisci e Formato testo come in Outlook; formato HTML o Testo normale. Le immagini nel testo partono come parti incorporate (cid), non come dati. |
| Regole e avvisi | Ogni messaggio nuovo (IMAP: UID oltre il precedente UIDNEXT; POP: dopo il primo scaricamento) passa dalle regole, poi dalla risposta automatica, poi dall'avviso sul desktop. Le regole stanno nell'archivio (`rules.rs`); "Esegui ora" le applica a una cartella. |
| Risposte automatiche | Per account, con periodo facoltativo. Una risposta per mittente per ogni attivazione; niente risposte a newsletter, liste, risposte automatiche, indirizzi "noreply" o al proprio indirizzo. Intestazioni `Auto-Submitted: auto-replied` e `X-Auto-Response-Suppress: All` (RFC 3834). Funzionano solo con il programma aperto. |
| Invio programmato | Il messaggio aspetta nella posta in uscita fino all'ora scelta; parte appena il programma è aperto. |
| Rubrica | Contatti nell'archivio. Import vCard 2.1/3.0/4.0, CSV (Outlook in italiano e inglese, Google, Thunderbird), LDIF, con riconoscimento della codifica; i contatti già presenti (stessa email) vengono completati, non duplicati. Export vCard. I suggerimenti degli indirizzi usano prima la rubrica, poi la posta. |
| Calendario | Eventi nell'archivio (tabella `events`), un evento per serie più le occorrenze modificate a parte (stesso UID, `RECURRENCE-ID`). Le ripetizioni si espandono nell'interfaccia (`src/lib/calendar.ts`) in ora locale. File `.ics` letti e scritti in `calendar.rs`: zone orarie dai blocchi VTIMEZONE (anche i nomi di Windows di Outlook), serie scritte con la zona del computer ricavata dal sistema. Promemoria e avvisi dall'interfaccia, solo con il programma aperto. |
| Inviti | Parte `text/calendar` o allegato `.ics` (iTIP, RFC 5546). REQUEST: Accetta, Provvisorio, Rifiuta con risposta REPLY all'organizzatore; CANCEL: togli dal calendario; REPLY: aggiorna i partecipanti delle riunioni organizzate. Le riunioni create in Better-Look mandano REQUEST, aggiornamenti con SEQUENCE crescente e CANCEL, come messaggi con testo e parte `text/calendar; method=…`. |
| Conversazioni | Ignora conversazione: la conversazione (account + Message-ID del primo messaggio) va in un elenco nell'archivio; i suoi messaggi vanno in Posta eliminata e quelli nuovi li seguono appena arrivano, dopo le regole. Pulisci conversazione o cartella: un messaggio letto, senza contrassegno né allegati, va in Posta eliminata se il suo testo (senza la parte citata) compare per intero in un messaggio più recente della stessa conversazione (`engine/conversations.rs`). |
| Azioni rapide e modelli | Salvati nell'archivio (`engine/quick.rs`), con gli stessi controlli nell'interfaccia (`src/lib/quicksteps.ts`): al massimo un'azione che apre un messaggio da scrivere e una che sposta. Le esegue l'interfaccia con i comandi di sempre: prima lo stato (letto, contrassegno, categorie), poi prepara la risposta, poi sposta, e alla fine apre la risposta. I modelli sono HTML ripulito; "Salva come modello" toglie firma e messaggio citato. |
| Attività | Tabella `tasks` dell'archivio, una riga per attività con i dati in JSON; elenchi dell'utente in `meta`. Gli elenchi intelligenti (La mia giornata, Importanti, Pianificate) si calcolano nell'interfaccia (`src/lib/tasks.ts`). Completando un'attività che si ripete il motore crea subito la successiva (`calendar::next_task_date`). Posta contrassegnata: i messaggi con il contrassegno di tutti gli account; completarli toglie il contrassegno. Promemoria nella stessa finestra di quelli del calendario. |
| Firme | Più firme con un nome, salvate nell'archivio. Ogni account sceglie la firma per i nuovi messaggi e quella per risposte e inoltri; il pulsante Firma la cambia mentre si scrive. La prima firma parte da nome e indirizzo dell'account. Nel testo la firma è il blocco `id="better-look-firma"`, la citazione `id="better-look-citazione"`: nei messaggi citati questi segni si tolgono. |

**Prove automatiche.** A ogni modifica GitHub Actions esegue i test dell'interfaccia (vitest), i test del motore e una prova completa contro un server di posta vero (Dovecot e un server SMTP di prova, in `tools/posta-di-prova`). Poi prepara gli installer per Windows, macOS (Apple Silicon e Intel) e Linux.

**Limiti noti:**
- Il PST non si modifica: "Apri PST" ne fa una copia nell'archivio di Better-Look, come file di dati separato. Spostare o eliminare messaggi lì non tocca il file originale.
- I file `.ost` non si aprono (la libreria non legge il formato di Exchange 2013 e successivi).
- Alcuni allegati di PST generati da programmi diversi da Outlook non sono leggibili dalla libreria: vengono contati e segnalati alla fine dell'importazione.
- Risposte automatiche, regole sui messaggi in arrivo, Ignora conversazione, invio programmato e promemoria lavorano sul computer: funzionano solo mentre Better-Look è aperto.
- Il calendario è solo sul computer: niente Google Calendar, Microsoft 365 o CalDAV, niente calendari condivisi. "Proponi nuovo orario" non c'è ancora.
- Dai PST si importa solo la posta (non contatti e calendario).
- Traduci rimanda ai futuri servizi di intelligenza artificiale (prodotto separato).
- Le attività restano sul computer: non si sincronizzano con Microsoft To Do, Google Tasks o CalDAV.
- Estensioni: il pulsante c'è, ma il modo di farle è ancora da decidere (sezione 7).

---

## 1. Il progetto

**Better-Look** è un client di posta desktop **gratuito e open source**, pensato per le PMI italiane che usano Outlook ogni giorno.

L'obiettivo è che un utente di Outlook trovi ogni comando dove se lo aspetta, sia che venga da **Outlook classico** sia dal **nuovo Outlook**, senza doversi riabituare. Si riprendono posizioni dei comandi, scorciatoie da tastiera e sensazione cromatica di Outlook, ma **non** nomi, loghi o icone Microsoft.

- **Committente e tester:** Gimmy.
- **Sviluppo:** il codice lo scrive Claude.
- **Piattaforme:** Windows, macOS e Linux.
- **Lingua dell'interfaccia:** italiano.

### Il nome

**Better-Look**, con il trattino. Il nome dice l'obiettivo: un'alternativa migliore per chi arriva da Outlook. Prima della pubblicazione conviene una verifica del marchio, perché il nome richiama Outlook.

### Gratuito per sempre

Decisione di Gimmy (ottobre 2026): Better-Look sarà **sempre gratuito, libero e open source**. Eventuali servizi di intelligenza artificiale potranno arrivare come prodotto separato in abbonamento, offerto accanto al programma; il programma resta completo senza.

---

## 2. Decisioni prese

### Interfaccia e comportamento

| Tema | Decisione |
|---|---|
| Vista classica o nuova | L'utente sceglie. Al **primo avvio** compare la domanda "Da quale Outlook arrivi?" con due schede, Outlook classico e Nuovo Outlook. Tutto si cambia poi da Impostazioni › Aspetto, anche combinando le opzioni (stato "personalizzato"). |
| Barra dei comandi | **Estesa**, cioè icone grandi con etichetta e gruppi con nome, come in Outlook classico. Oppure **compatta**, una sola riga come nel nuovo Outlook. La sceglie l'utente. |
| Accesso a impostazioni e account | **Menu File** (pagina a tutto schermo come in Outlook classico) oppure **ingranaggio** in alto a destra (nuovo Outlook). |
| Nuovo messaggio | In una **finestra separata** oppure **nel riquadro di lettura**. Con la finestra separata c'è l'opzione "Apri in una finestra anche risposte e inoltri". |
| Preset | **Classico** = barra estesa, menu File, finestra separata. **Nuovo** = barra compatta, ingranaggio, riquadro di lettura. |
| Pulsante Invia | Con la barra estesa sta a sinistra dei destinatari (come in Outlook classico). Con la barra compatta sta sopra i destinatari (come nel nuovo Outlook). |
| Invio | Immediato. Compare l'avviso "Messaggio inviato" con il pulsante **Annulla**, che riapre il messaggio. |
| Chiusura della finestra di scrittura | Salva in Bozze e lo segnala con un avviso. |
| Account | **Ogni account si sincronizza direttamente con il proprio server**, senza server intermedi di Better-Look. |
| Posta offline | La sceglie l'utente. **IMAP:** quanta posta tenere sul computer (ultimo mese, 3 mesi, ultimo anno, tutta) e allegati (scarica subito, quando li apro). **POP:** per quanto lasciare una copia sul server (mai, 14 giorni, sempre), con l'avviso di includere la cartella di Better-Look nei backup. |
| PST | Supporto richiesto: **importa** da PST e **apri** un PST senza importarlo. In più c'è l'export in formati aperti (MBOX, EML). *Realizzato come importazione in un file di dati separato, che si comporta come un PST aperto in Outlook ma non modifica l'originale (vedi sezione 0).* |
| Componenti aggiuntivi COM e macro VBA | **Da decidere.** Gimmy li aveva chiesti. Claude ha sconsigliato entrambi, perché funzionano solo su Windows e sono un rischio di sicurezza, e ha proposto in alternativa regole, azioni rapide e un sistema di estensioni in JavaScript. La scelta non è stata confermata. |

### Aspetto

- **Colori come Outlook**, sia in vista classica sia nuova, in tema chiaro e scuro. Si usa la palette **Fluent 2**: il design system di Microsoft è pubblicato con licenza MIT, i colori si possono richiamare.
- **Colore di accento a scelta** (ottobre 2026): in Impostazioni › Aspetto, oltre al tema chiaro, scuro o come il sistema, si sceglie un colore qualsiasi (8 proposte, selettore del sistema o codice esadecimale). Il blu di Outlook resta il predefinito. Dal colore scelto si ricavano barra in alto, pulsanti, selezione e collegamenti per ciascun tema, regolando la luminosità in OKLCH finché ogni testo raggiunge il contrasto 4,5:1 (`src/lib/accent.ts`).
- **Carattere di sistema:** "Segoe UI Variable Text", "Segoe UI", -apple-system, system-ui. Non si distribuiscono font.
- **Niente metafora postale:** niente timbri, annulli postali, strisce di posta aerea o animazioni di timbratura. Erano nella prima versione e sono stati tolti.
- **Integrato nel sistema operativo:** l'interfaccia di lavoro **non mostra né il nome né il logo**. La barra blu in alto contiene solo ricerca, guida, account e (vista nuova) ingranaggio.
- **Il logo compare solo in due punti:**
  - in alto nella finestra del **primo avvio** (scritta completa Better-Look);
  - nella pagina **"Informazioni su Better-Look"**, che si apre dal menu File nella vista classica e da Impostazioni nella vista nuova.
- Logo e icona sono **provvisori**: la scritta Better-Look con il trattino nel colore di accento, e come icona un quadrato blu con la "B" e il trattino.

### Rinviato a dopo

- **Accesso Microsoft 365 e Gmail con OAuth.** Arriverà dopo la registrazione dell'app presso Microsoft (Entra) e Google; Google richiede anche una verifica dell'app per accedere a Gmail. In Exchange Online IMAP e POP con la sola password sono già disattivati. L'invio con password (SMTP AUTH) sarà disattivato di default da fine dicembre 2026, e la data di rimozione definitiva verrà annunciata nel 2027 (comunicazione Microsoft MC786329).
- **Firma del codice** (certificati Windows e notarizzazione Apple). Si fa quando il programma sarà pronto.
- **Licenza.** Non ancora scelta. Deve essere una licenza open source, visto che il programma resterà libero per sempre.
- **Domini e marchio.** Da verificare per il nome Better-Look.

---

## 3. Architettura consigliata

**Tauri 2**, con il motore in **Rust** e l'interfaccia in **HTML, CSS e TypeScript**. Le ragioni:

- **Leggerezza.** Usa il motore web del sistema, quindi installer piccolo e poca memoria. Conta per un programma aperto tutto il giorno sui PC non recenti delle PMI.
- **Sicurezza.** Rust è più robusto nel leggere email non affidabili, che sono il principale punto d'attacco di un client di posta.
- **Riuso del prototipo.** L'interfaccia è già HTML e CSS, quindi si riusa quasi così com'è.

Il limite principale di Tauri: i motori web sono tre diversi (WebView2 su Windows, WebKit su macOS, WebKitGTK su Linux), quindi interfaccia e resa delle email vanno testate su tutti e tre. Linux è il più delicato.

Se un giorno il codice dovesse essere mantenuto solo da sviluppatori JavaScript, **Electron** resta un'alternativa valida, ma pesa di più.

### Componenti suggeriti (da verificare alla data di inizio)

- **IMAP:** `async-imap`.
- **SMTP:** `lettre`.
- **Lettura e scrittura dei messaggi:** `mail-parser` e `mail-builder`.
- **POP3:** implementazione propria (scelta fatta: il protocollo è semplice).
- **Archivio locale:** SQLite (`rusqlite`), con FTS5 per la ricerca nel testo.
- **Password:** nel portachiavi del sistema operativo (`keyring`), mai in chiaro su disco.
- **HTML delle email:**
  - pulito con `ammonia`;
  - mostrato in un iframe isolato con una Content Security Policy restrittiva;
  - **immagini remote bloccate di default**, perché sono usate per tracciare l'apertura.
- **PST:** scelta `outlook-pst` (Rust, MIT), che implementa la specifica pubblica Microsoft [MS-PST]. Niente librerie C da collegare.
- **Build:** GitHub Actions con `tauri-action`, che a ogni modifica produce gli installer per Windows, macOS e Linux. Gimmy scarica e prova senza installare strumenti di sviluppo.

---

## 4. Piano di lavoro

1. **Fondamenta.** Repository GitHub, progetto Tauri, interfaccia del prototipo portata nell'app con dati finti, installer automatici per i tre sistemi.
2. **Primo account reale.** Un account IMAP con password, che basta per Aruba, Register e la gran parte dei provider italiani: cartelle, elenco messaggi, lettura sicura dell'HTML, invio SMTP.
3. **Archivio locale.** Posta salvata sul computer, uso offline secondo le opzioni scelte dall'utente, ricerca.
4. **Scrittura completa.** Bozze, allegati, risposte e inoltri, salvataggio automatico, Annulla invio.
5. **Più account.** POP3, più account insieme e opzioni di sincronizzazione.
6. **PST.** Import, apertura e export MBOX/EML.
7. **Organizzazione.** Cartelle, categorie, conversazioni, stampa, lettura ad alta voce, regole e avvisi, risposte automatiche, invio programmato, pulizia, rubrica, calendario e inviti. *(fatto, 24 ottobre 2026)*
8. **Produttività.** Attività, azioni rapide, modelli di messaggio, Ignora e Pulisci conversazione, Inoltra come allegato, riquadro di lettura a destra, in basso o nascosto, densità dell'elenco; Cmd+Q e scorciatoie del Mac; rifinitura dell'interfaccia. *(fatto, versione 0.2.0, 24 ottobre 2026)*
9. **Quando sarà il momento.** OAuth Microsoft 365 e Gmail, calendari online (CalDAV), firma del codice, aggiornamenti automatici, logo definitivo, licenza.

**Cosa serve da Gimmy per partire:**
- account GitHub collegato a Claude, con un repository vuoto, all'inizio privato;
- una casella di posta **di prova** con IMAP e SMTP attivi; la password si inserisce solo nell'app, mai in chat;
- il sistema operativo su cui farà le prove.

---

## 5. Il prototipo

Il prototipo online, con tutte le tavole, è il canvas di design su claude.ai: https://claude.ai/artifact/UHbnvoJTxqRrbSDrD45GHY (privato, visibile all'account di Gimmy).

### Formato dei file

I file `.dc.html` sono nel formato dei canvas di design di Claude:
- il markup usa segnaposto `{{nome}}`;
- le condizioni sono `<sc-if>`, i cicli `<sc-for>`;
- `<dc-import>` richiama un altro file;
- `<helmet>` contiene gli stili.

Tutta la logica sta in JavaScript semplice, nella classe `Component` e nel suo metodo `renderVals()`, in fondo a `Main.dc.html`. Fuori dal canvas i file non si aprono direttamente nel browser, perché manca il runtime. Vanno usati come **specifica**: markup, CSS, token e comportamento da riportare nell'app Tauri.

### File

| File | Contenuto |
|---|---|
| `prototipo/Main.dc.html` | **Il prototipo completo.** Tutta l'interfaccia, i token di colore, la logica e i dati di esempio. Opzioni (props): `theme` (chiaro/scuro), `stile` (classico/nuovo), `avvio` (mostra il primo avvio), `view` (lettura, composizione, inviato, file, impostazioni, account), `connection` (online/offline). |
| `prototipo/Classico.dc.html` | Vista classica, lettura. |
| `prototipo/Nuovo.dc.html` | Vista nuova, lettura. |
| `prototipo/File.dc.html` | Menu File (vista classica). |
| `prototipo/FinestraMessaggio.dc.html` | Nuovo messaggio in finestra separata (classica). |
| `prototipo/Composizione.dc.html` | Risposta nel riquadro di lettura (nuova). |
| `prototipo/Impostazioni.dc.html` | Impostazioni › Aspetto. |
| `prototipo/Account.dc.html` | Impostazioni › Account e sincronizzazione. |
| `prototipo/Inviato.dc.html` | Stato dopo l'invio, con Annulla. |
| `prototipo/Scuro.dc.html` | Tema scuro. |
| `prototipo/Telefono.dc.html` | Posta in arrivo su telefono (schermata statica, 390×844). |
| `prototipo/canvas.json` | Disposizione delle tavole nel canvas. |
| `../assets/better-look-logo.svg` | Scritta Better-Look provvisoria (Plus Jakarta Sans ExtraBold, licenza OFL, convertita in tracciati). |
| `../assets/app-icon.svg` | Icona provvisoria del programma. |
| `design/brief-interfaccia.md` | Brief di design aggiornato (direzione visiva). |
| `PRODUCT.md` | Contesto di prodotto aggiornato. |

**Tutti i dati nel prototipo sono finti.** Il riferimento è l'azienda inventata Valli Arredi, con l'utente Chiara Valli e l'account POP ordini@valliarredi.it.

### Cosa funziona già nel prototipo

- **Primo avvio:** scelta tra classico e nuovo, che applica il preset.
- **Comandi:**
  - barra estesa o compatta, con un pulsante in fondo alla barra per passare dall'una all'altra;
  - schede Home, Visualizza, Aiuto, più Messaggio e Inserisci quando si scrive nel riquadro;
  - nella barra estesa i gruppi meno usati si nascondono sotto 1380 e 1260 pixel di larghezza.
- **Cartelle:**
  - Preferiti, cartelle dell'account principale e secondo account POP richiudibile;
  - contatori dei non letti;
  - riquadro cartelle richiudibile.
- **Elenco messaggi:**
  - schede Prioritaria e Altre, raggruppamento Oggi, Ieri, Questa settimana;
  - messaggio non letto con barra e oggetto in blu;
  - azioni al passaggio del mouse: elimina, contrassegna, letto/non letto;
  - ricerca;
  - stati vuoti per ogni cartella e per la ricerca senza risultati.
- **Lettura:**
  - intestazione con mittente, destinatari e Cc, categoria, allegati;
  - Rispondi, Rispondi a tutti e Inoltra, con il testo originale citato.
- **Scrittura:**
  - destinatari come "chip", aggiunti con Invio, virgola o punto e virgola, e rimossi con Backspace;
  - errore se manca il destinatario;
  - nella finestra separata: riduci, ingrandisci, chiudi salvando in Bozze;
  - nel riquadro: pulsante per staccarlo in una finestra;
  - le bozze si riaprono dalla cartella Bozze.
- **Menu File:** Informazioni (account, risposte automatiche, pulizia, regole), Apri ed esporta (PST, MBOX/EML), Informazioni su Better-Look, Opzioni.
- **Impostazioni:**
  - Aspetto: preset, barra, accesso, nuovi messaggi, tema;
  - Account e sincronizzazione: dati del server e opzioni IMAP/POP;
  - Informazioni su Better-Look.
- **Altro:** tema scuro e avviso offline.

### Scorciatoie da tastiera indicate nell'interfaccia

| Azione | Scorciatoia |
|---|---|
| Nuovo messaggio | Ctrl+N |
| Rispondi | Ctrl+R |
| Rispondi a tutti | Ctrl+Maiusc+R |
| Inoltra | Ctrl+F |
| Elimina | Canc |
| Archivia | Backspace |
| Sposta in | Ctrl+Maiusc+V |
| Segna come letto / non letto | Ctrl+Q / Ctrl+U |
| Contrassegna | Ins |
| Invia/Ricevi | F9 |
| Cerca | Ctrl+E |
| Posta, Calendario, Contatti, Attività | Ctrl+1 … Ctrl+4 |
| Invia il messaggio | Ctrl+Invio |
| Chiudi finestre e pannelli | Esc |
| Grassetto, Corsivo, Sottolineato (Office italiano) | Ctrl+G, Ctrl+I, Ctrl+S |

---

## 6. Sistema visivo (token)

### Tema chiaro

| Token | Valore | Uso |
|---|---|---|
| `--ground` | #F0F0F0 | fondo della finestra |
| `--sheet` | #FFFFFF | pannelli (elenco, lettura, barra comandi) |
| `--rail` | #F5F5F5 | fondi secondari |
| `--rail-2` | #E0E0E0 | |
| `--ink` | #242424 | testo |
| `--ink-2` | #424242 | |
| `--ink-3` | #616161 | |
| `--rule` | #E0E0E0 | linee |
| `--rule-2` | #D1D1D1 | |
| `--brand` / `--brand-ink` | #0F6CBD | azioni principali, non letti, link |
| `--brand-soft` | #EBF3FC | cartella selezionata, chip destinatari |
| `--brand-sel` | #CFE4FA | messaggio selezionato |
| `--head` | #0F6CBD | barra in alto, barra del titolo della finestra di scrittura |
| `--danger` | #C50F1F | contrassegni, errori, [Bozza], posta indesiderata |
| `--warn-soft` / `--warn` | #FFF4CE / #8A6200 | avvisi (offline, POP) |
| `--green` | #107C10 | archivia |
| `--amber` | #A06A00 | azioni rapide, categorie |

### Tema scuro

| Token | Valore |
|---|---|
| `--ground` | #141414 |
| `--sheet` | #292929 |
| `--rail` | #1F1F1F |
| `--rail-2` | #3D3D3D |
| `--ink` | #FFFFFF |
| `--ink-2` | #D6D6D6 |
| `--ink-3` | #ADADAD |
| `--rule` | #3D3D3D |
| `--rule-2` | #525252 |
| `--brand` | #115EA3 |
| `--brand-ink` | #479EF5 |
| `--brand-soft` | #082338 |
| `--brand-sel` | #0A3354 |
| `--head` | #262626 |
| `--danger` | #E37D80 |
| `--warn-soft` / `--warn` | #463100 / #FFC83D |
| `--green` | #54B054 |
| `--amber` | #FFC83D |

**Avatar:** cinque tinte tenui (blu, verde acqua, viola, zucca, verde), assegnate con un hash del nome del mittente.

| Tinta | Chiaro (sfondo, testo) | Scuro (sfondo, testo) |
|---|---|---|
| 1 blu | #DEECF9, #0F548C | #0F3A5E, #9CCBF3 |
| 2 verde acqua | #C7EBE8, #00666D | #0B3D40, #7FD3CC |
| 3 viola | #E7DDF4, #5C2E91 | #3A2556, #C9B3EA |
| 4 zucca | #FCE0D4, #8E3C0E | #4A2412, #F2B597 |
| 5 verde | #DFF0D6, #2E6A0F | #203A12, #A9D98F |

**Forme e misure:**
- angoli: 4 px per i controlli, 8 px per pannelli e finestre;
- ombre leggere stile Fluent;
- barra in alto: 48 px;
- colonna delle app: 52 px;
- cartelle: 240 px;
- elenco messaggi: 420 px;
- riquadro di lettura: largo quanto resta;
- barra comandi: 108 px se estesa, 46 px se compatta.

**Icone:**
- set Tabler Icons (MIT), a tratto, spessore 1.6;
- nella barra comandi sono colorate per funzione: blu per rispondere e spostare, verde per archiviare, rosso per contrassegno e posta indesiderata, ambra per azioni rapide e categorie.

**Ordine dei gruppi nella scheda Home** (barra estesa): Nuovo, Aggiorna, Elimina, Rispondi, Azioni rapide, Sposta, Tag, Trova, Comandi vocali, Lingua, Estensioni.

---

## 7. Decisioni ancora aperte

1. Licenza: GPL-3.0 o MPL-2.0.
2. Componenti COM e macro VBA, oppure solo regole, azioni rapide ed estensioni JavaScript.
3. Domini.
4. ~~Repository GitHub e casella di prova.~~ Fatto: repository personale di Gimmy e server di prova automatico.
5. ~~Schermate non ancora progettate: Attività.~~ Fatto nella versione 0.2.0.
