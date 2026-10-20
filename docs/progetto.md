# Datini: documento di passaggio

Questo file raccoglie le decisioni di progetto e lo stato dello sviluppo. Le sezioni da 1 a 7 vengono dalla fase di progettazione dell'interfaccia (19 ottobre 2026); la sezione 0 dice a che punto è il codice.

**Per una nuova chat:** leggi prima questo file, poi `PRODUCT.md` e `brief-interfaccia.md`, poi `CLAUDE.md` nella cartella principale.

---

## 0. Stato dello sviluppo (20 ottobre 2026)

Repository: [Gimmypaxxion/datini-email](https://github.com/Gimmypaxxion/datini-email), personale e pubblico. È un progetto personale e sperimentale di Gimmy.

**Fasi da 1 a 6 completate.** Il motore in Rust è collegato all'interfaccia con comandi Tauri.

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
| Editor di testo | [RoosterJS](https://github.com/microsoft/roosterjs) (MIT), lo stesso motore della versione web e del nuovo Outlook, dentro un iframe con una Content-Security-Policy che vieta script e immagini esterne: l'HTML citato non può cambiare l'aspetto di Datini né scaricare immagini di tracciamento. Schede Messaggio, Inserisci e Formato testo come in Outlook; formato HTML o Testo normale. Le immagini nel testo partono come parti incorporate (cid), non come dati. |
| Firme | Più firme con un nome, salvate nell'archivio. Ogni account sceglie la firma per i nuovi messaggi e quella per risposte e inoltri; il pulsante Firma la cambia mentre si scrive. La prima firma parte da nome e indirizzo dell'account. Nel testo la firma è il blocco `id="datini-firma"`, la citazione `id="datini-citazione"`: nei messaggi citati questi segni si tolgono. |

**Prove automatiche.** A ogni modifica GitHub Actions esegue i test dell'interfaccia (vitest), i test del motore e una prova completa contro un server di posta vero (Dovecot e un server SMTP di prova, in `tools/posta-di-prova`). Poi prepara gli installer per Windows, macOS (Apple Silicon e Intel) e Linux.

**Limiti noti:**
- Il PST non si modifica: "Apri PST" ne fa una copia nell'archivio di Datini, come file di dati separato. Spostare o eliminare messaggi lì non tocca il file originale.
- I file `.ost` non si aprono (la libreria non legge il formato di Exchange 2013 e successivi).
- Alcuni allegati di PST generati da programmi diversi da Outlook non sono leggibili dalla libreria: vengono contati e segnalati alla fine dell'importazione.
- Mancano ancora regole, risposte automatiche, invio programmato e modelli.
- Calendario, Contatti e Attività sono solo icone.

---

## 1. Il progetto

**Datini** è un client di posta desktop **gratuito e open source**, pensato per le PMI italiane che usano Outlook ogni giorno.

L'obiettivo è che un utente di Outlook trovi ogni comando dove se lo aspetta, sia che venga da **Outlook classico** sia dal **nuovo Outlook**, senza doversi riabituare. Si riprendono posizioni dei comandi, scorciatoie da tastiera e sensazione cromatica di Outlook, ma **non** nomi, loghi o icone Microsoft.

- **Committente e tester:** Gimmy.
- **Sviluppo:** il codice lo scrive Claude.
- **Piattaforme:** Windows, macOS e Linux.
- **Lingua dell'interfaccia:** italiano.

### Il nome

Il nome viene da **Francesco Datini**, mercante di Prato del Trecento. Il suo archivio conserva circa **143.000 lettere d'affari** scambiate con quasi trecento città. Gimmy ha scelto il nome perché la sua storia si sposa con i valori dell'open source.

Prima di scegliere Datini sono stati scartati alcuni nomi per conflitti nel settore della posta:
- Missiva (troppo vicino a Missive);
- Brevia (troppo vicino a Brevo, ex Sendinblue);
- Pigeon Mail (esiste già);
- Sigillo (troppo vicino a Sigilium);
- Envelo (app delle Poste polacche).

Una ricerca veloce non ha trovato software di posta che si chiamano Datini. A Prato esiste la Fondazione Datini (Istituto Internazionale di Storia Economica): non è un concorrente, ma potrebbe valere la pena avvisarla del progetto.

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
| Account | **Ogni account si sincronizza direttamente con il proprio server**, senza server intermedi di Datini. |
| Posta offline | La sceglie l'utente. **IMAP:** quanta posta tenere sul computer (ultimo mese, 3 mesi, ultimo anno, tutta) e allegati (scarica subito, quando li apro). **POP:** per quanto lasciare una copia sul server (mai, 14 giorni, sempre), con l'avviso di includere la cartella di Datini nei backup. |
| PST | Supporto richiesto: **importa** da PST e **apri** un PST senza importarlo. In più c'è l'export in formati aperti (MBOX, EML). *Realizzato come importazione in un file di dati separato, che si comporta come un PST aperto in Outlook ma non modifica l'originale (vedi sezione 0).* |
| Componenti aggiuntivi COM e macro VBA | **Da decidere.** Gimmy li aveva chiesti. Claude ha sconsigliato entrambi, perché funzionano solo su Windows e sono un rischio di sicurezza, e ha proposto in alternativa regole, azioni rapide e un sistema di estensioni in JavaScript. La scelta non è stata confermata. |

### Aspetto

- **Colori come Outlook**, sia in vista classica sia nuova, in tema chiaro e scuro. Si usa la palette **Fluent 2**: il design system di Microsoft è pubblicato con licenza MIT, i colori si possono richiamare.
- **Carattere di sistema:** "Segoe UI Variable Text", "Segoe UI", -apple-system, system-ui. Non si distribuiscono font.
- **Niente metafora postale:** niente timbri, annulli postali, strisce di posta aerea o animazioni di timbratura. Erano nella prima versione e sono stati tolti.
- **Integrato nel sistema operativo:** l'interfaccia di lavoro **non mostra né il nome né il logo**. La barra blu in alto contiene solo ricerca, guida, account e (vista nuova) ingranaggio.
- **Il logo compare solo in due punti:**
  - in alto nella finestra del **primo avvio** (scritta completa DATINI);
  - nella pagina **"Informazioni su Datini"**, che si apre dal menu File nella vista classica e da Impostazioni nella vista nuova.
- La "D" da sola potrà servire come **icona del programma** (desktop, barra delle applicazioni).

### Rinviato a dopo

- **Accesso Microsoft 365 e Gmail con OAuth.** Arriverà dopo la registrazione dell'app presso Microsoft (Entra) e Google; Google richiede anche una verifica dell'app per accedere a Gmail. In Exchange Online IMAP e POP con la sola password sono già disattivati. L'invio con password (SMTP AUTH) sarà disattivato di default da fine dicembre 2026, e la data di rimozione definitiva verrà annunciata nel 2027 (comunicazione Microsoft MC786329).
- **Firma del codice** (certificati Windows e notarizzazione Apple). Si fa quando il programma sarà pronto.
- **Licenza.** Da scegliere prima di rendere pubblico il codice. Le candidate sono **GPL-3.0** (chi modifica e distribuisce deve rilasciare le modifiche) e **MPL-2.0** (la licenza di Thunderbird, più flessibile).
- **Domini.** Da verificare su un registrar: datini.it, datini.app. breviaemail.com risultava già registrato, ma è irrilevante dopo la scelta del nome.

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
7. **Quando sarà il momento.** OAuth Microsoft 365 e Gmail, firma del codice, aggiornamenti automatici, icona con la "D", licenza e pubblicazione.

**Cosa serve da Gimmy per partire:**
- account GitHub collegato a Claude, con un repository vuoto (per esempio `datini`), all'inizio privato;
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
| `logo/datini-logo.svg` | Logo completo (scritta DATINI), originale fornito da Gimmy. |
| `logo/datini-d.svg` | La sola "D", originale fornito da Gimmy. |
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
- **Menu File:** Informazioni (account, risposte automatiche, pulizia, regole), Apri ed esporta (PST, MBOX/EML), Informazioni su Datini, Opzioni.
- **Impostazioni:**
  - Aspetto: preset, barra, accesso, nuovi messaggi, tema;
  - Account e sincronizzazione: dati del server e opzioni IMAP/POP;
  - Informazioni su Datini.
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
5. Schermate non ancora progettate: Calendario, Contatti, Attività, regole, risposte automatiche, firma.
