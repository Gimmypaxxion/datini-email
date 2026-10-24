//! Collaudo completo contro il server di posta di prova (tools/posta-di-prova).
//! Parte solo se il server è attivo e si imposta BETTERLOOK_SERVER_PROVA=1:
//!
//!   sudo tools/posta-di-prova/avvia.sh
//!   BETTERLOOK_SERVER_PROVA=1 cargo test --test server_prova -- --nocapture

use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use better_look_lib::db::Db;
use better_look_lib::engine::{Engine, NewAccount};
use better_look_lib::imap;
use better_look_lib::model::*;
use better_look_lib::secrets::Memory;
use futures::StreamExt;

fn enabled() -> bool {
    std::env::var("BETTERLOOK_SERVER_PROVA").is_ok()
}

fn tools() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tools/posta-di-prova")
}

fn reseed() {
    let ok = std::process::Command::new("python3").arg(tools().join("popola.py")).status().expect("python3").success();
    assert!(ok, "impossibile riempire la casella di prova");
}

async fn wait_for<F, Fut>(what: &str, secs: u64, mut f: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        if f().await {
            return;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("tempo scaduto aspettando: {what}");
}

fn server() -> ServerConfig {
    ServerConfig { host: "mail.better-look.test".into(), port: 993, security: Security::Ssl, username: "prova@better-look.test".into() }
}

/// Messaggi in una cartella del server, con un filtro opzionale.
async fn server_count(folder: &str, search: &str) -> usize {
    let mut s = imap::login(&server(), "prova-123").await.unwrap();
    s.select(folder).await.unwrap();
    let n = s.uid_search(search).await.unwrap().len();
    let _ = s.logout().await;
    n
}

async fn server_flags(folder: &str, subject: &str) -> String {
    let mut s = imap::login(&server(), "prova-123").await.unwrap();
    s.select(folder).await.unwrap();
    let uids: Vec<u32> = s.uid_search(format!("SUBJECT \"{subject}\"")).await.unwrap().into_iter().collect();
    let mut out = String::new();
    {
        let mut st = s.uid_fetch(uids[0].to_string(), "(FLAGS)").await.unwrap();
        while let Some(f) = st.next().await {
            out = format!("{:?}", f.unwrap().flags().collect::<Vec<_>>());
        }
    }
    let _ = s.logout().await;
    out
}

fn folder(eng: &Engine, acct: &str, role: &str) -> FolderInfo {
    eng.folders().unwrap().into_iter().find(|f| f.account_id == acct && f.role.as_deref() == Some(role)).unwrap()
}

fn find(eng: &Engine, folder_id: i64, subject: &str) -> Option<MessageSummary> {
    eng.messages(folder_id, None, 0, 1000).unwrap().into_iter().find(|m| m.subject == subject)
}

fn engine() -> (Arc<Engine>, Arc<Mutex<Vec<Event>>>) {
    let dir = std::env::temp_dir().join(format!("better-look-prova-{}", better_look_lib::mime::random_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    let eng = Engine::new(Db::open(&dir.join("better-look.db")).unwrap(), Arc::new(Memory::default()), dir, Arc::new(move |e| ev.lock().unwrap().push(e)));
    eng.start();
    (eng, events)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ciclo_completo_imap() {
    if !enabled() {
        eprintln!("server di prova non richiesto: test saltato");
        return;
    }
    reseed();
    let (eng, events) = engine();

    // Password sbagliata: errore chiaro, nessun account creato.
    let err = eng
        .add_account(NewAccount { name: "Chiara Valli".into(), email: "prova@better-look.test".into(), password: "sbagliata".into(), kind: None, incoming: None, outgoing: None })
        .await
        .unwrap_err();
    assert_eq!(err.kind(), "credenziali", "{err}");
    assert!(eng.accounts().unwrap().is_empty());

    // Configurazione automatica dai nomi più comuni (imap.dominio, smtp.dominio).
    let info = eng
        .add_account(NewAccount { name: "Chiara Valli".into(), email: "prova@better-look.test".into(), password: "prova-123".into(), kind: None, incoming: None, outgoing: None })
        .await
        .unwrap();
    let a = info.config.id.clone();
    assert_eq!(info.config.kind, AccountKind::Imap);
    assert_eq!(info.config.incoming.as_ref().unwrap().host, "imap.better-look.test");

    wait_for("prima sincronizzazione", 60, || {
        let eng = eng.clone();
        let a = a.clone();
        async move { eng.status(&a).state == "pronto" && eng.folders().unwrap().iter().any(|f| f.role.as_deref() == Some("inbox") && f.total == 48) }
    })
    .await;
    let inbox = folder(&eng, &a, "inbox");
    assert_eq!(inbox.name, "INBOX");
    let names: Vec<String> = eng.folders().unwrap().iter().map(|f| f.path.clone()).collect();
    for p in ["INBOX", "Sent", "Drafts", "Trash", "Junk", "Archive", "Clienti"] {
        assert!(names.contains(&p.to_string()), "manca {p}: {names:?}");
    }

    // Schede Prioritaria e Altre.
    let altre = eng.messages(inbox.id, Some("altre"), 0, 100).unwrap();
    let altre_subj: Vec<&str> = altre.iter().map(|m| m.subject.as_str()).collect();
    assert_eq!(altre.len(), 3, "{altre_subj:?}");
    assert_eq!(eng.messages(inbox.id, Some("prio"), 0, 100).unwrap().len(), 45);

    // Lettura sicura dell'HTML.
    let news = find(&eng, inbox.id, "Uffici 2027: materiali, colori e acustica").unwrap();
    let d = eng.detail(news.id, false).await.unwrap();
    let html = d.html.unwrap();
    assert!(d.remote_images >= 2, "immagini esterne: {}", d.remote_images);
    assert!(!html.contains("<script"));
    assert!(!html.contains("traccia.arredoecontract"));
    let d = eng.detail(news.id, true).await.unwrap();
    assert!(d.html.unwrap().contains("traccia.arredoecontract"));

    let lucia = find(&eng, inbox.id, "Re: Campionario tessuti per lo showroom").unwrap();
    let d = eng.detail(lucia.id, false).await.unwrap();
    assert!(d.html.as_ref().unwrap().contains("data:image/png;base64,"));
    assert_eq!(d.attachments.iter().filter(|a| !a.inline).count(), 1);
    assert_eq!(d.remote_images, 0);
    let (name, mime, bytes) = eng.attachment(lucia.id, d.attachments.iter().find(|a| !a.inline).unwrap().index).await.unwrap();
    assert_eq!(name, "Cartella_colori_2027.pdf");
    assert_eq!(mime, "application/pdf");
    assert!(bytes.starts_with(b"%PDF"));

    // Ricerca: ignora maiuscole e accenti, trova anche l'inizio delle parole.
    let r = eng.search("boucle", Some(&a)).unwrap();
    assert!(r.iter().any(|m| m.subject.contains("Campionario")), "{:?}", r.iter().map(|m| &m.subject).collect::<Vec<_>>());
    assert!(!eng.search("fattur 418", None).unwrap().is_empty());

    // Letto e contrassegno arrivano al server.
    let davide = find(&eng, inbox.id, "Preventivo uffici Bassano, revisione 3").unwrap();
    assert!(davide.unread);
    eng.set_flags(&[davide.id], Some(true), Some(true), None).unwrap();
    wait_for("flag sul server", 30, || async { let f = server_flags("INBOX", "revisione 3").await; f.contains("Seen") && f.contains("Flagged") }).await;

    // Archivia ed elimina: spostamenti sul server, senza doppioni sul computer.
    eng.move_to_role(&[davide.id], "archive").await.unwrap();
    assert!(find(&eng, inbox.id, "Preventivo uffici Bassano, revisione 3").is_none());
    let archive = folder(&eng, &a, "archive");
    wait_for("archiviato sul server", 30, || async { server_count("Archive", "ALL").await == 2 }).await;
    wait_for("archivio sincronizzato", 30, || {
        let eng = eng.clone();
        async move {
            let l = eng.messages(archive.id, None, 0, 100).unwrap();
            l.len() == 2 && l.iter().all(|m| !m.subject.is_empty())
        }
    })
    .await;
    let moved = find(&eng, archive.id, "Preventivo uffici Bassano, revisione 3").unwrap();
    assert!(!moved.unread && moved.flagged);

    let sara = find(&eng, inbox.id, "Ferie di Natale: calendario da compilare").unwrap();
    eng.delete_messages(&[sara.id]).await.unwrap();
    wait_for("cestino sul server", 30, || async { server_count("Trash", "ALL").await == 1 }).await;
    let trash = folder(&eng, &a, "deleted");
    wait_for("cestino sincronizzato", 30, || {
        let eng = eng.clone();
        async move { eng.messages(trash.id, None, 0, 10).unwrap().len() == 1 }
    })
    .await;
    // Eliminare dal cestino cancella del tutto.
    let in_trash = eng.messages(trash.id, None, 0, 10).unwrap();
    eng.delete_messages(&[in_trash[0].id]).await.unwrap();
    wait_for("cestino vuoto sul server", 30, || async { server_count("Trash", "ALL").await == 0 }).await;

    // Bozze: salvataggio sul server, sostituzione della versione precedente, eliminazione.
    let mut draft = Draft {
        key: better_look_lib::mime::random_id(),
        account_id: a.clone(),
        to: vec![Person::new("Davide Orlandi", "d.orlandi@studiorlandi.it")],
        cc: vec![],
        bcc: vec![],
        subject: "Campioni del rovere".into(),
        body: "Ciao Davide,\n\nriesci a portarli giovedì?\n\nChiara".into(),
        html: None,
        plain: false,
        attachments: vec![],
        in_reply_to: None,
        references: vec![],
        reply_to_message: None,
    };
    eng.close_draft(&draft).await.unwrap();
    wait_for("bozza sul server", 30, || async { server_count("Drafts", "SUBJECT \"Campioni del rovere\"").await == 1 }).await;
    draft.subject = "Campioni del rovere per Bassano".into();
    eng.close_draft(&draft).await.unwrap();
    wait_for("bozza sostituita", 30, || async {
        server_count("Drafts", "SUBJECT \"Campioni del rovere\"").await == 1 && server_count("Drafts", "SUBJECT \"per Bassano\"").await == 1
    })
    .await;
    let drafts = folder(&eng, &a, "drafts");
    wait_for("bozze sincronizzate senza doppioni", 30, || {
        let eng = eng.clone();
        async move {
            let l = eng.messages(drafts.id, None, 0, 10).unwrap();
            l.len() == 2 && l.iter().filter(|m| m.subject.starts_with("Campioni")).count() == 1
        }
    })
    .await;
    eng.discard_draft(&a, &draft.key).unwrap();
    wait_for("bozza eliminata dal server", 30, || async { server_count("Drafts", "ALL").await == 1 }).await;

    // Bozza di un altro programma, riaperta e salvata: sostituisce l'originale.
    let lamp = find(&eng, drafts.id, "Lampade sala riunioni").unwrap();
    let mut reopened = eng.open_draft(lamp.id).await.unwrap();
    assert_eq!(reopened.to[0].email, "giorgio@valliarredi.it");
    reopened.body.push_str("\nTe li mostro lunedì.");
    eng.close_draft(&reopened).await.unwrap();
    wait_for("bozza esterna sostituita", 30, || async {
        server_count("Drafts", "ALL").await == 1 && server_count("Drafts", "BODY \"mostro luned\"").await == 1
    })
    .await;

    // Invio annullato: non parte nulla.
    let mut msg = draft.clone();
    msg.key = better_look_lib::mime::random_id();
    msg.to = vec![Person::new("Chiara Valli", "prova@better-look.test")];
    msg.subject = "Messaggio annullato".into();
    let out = eng.send(&msg).await.unwrap();
    let back = eng.cancel_send(out).unwrap().unwrap();
    assert_eq!(back.subject, "Messaggio annullato");

    // Invio vero a se stessi: arriva in Posta in arrivo, la copia in Posta inviata.
    msg.subject = "Prova di invio da Better-Look".into();
    msg.attachments = vec![DraftAttachment {
        name: "Contratto.pdf".into(),
        size: 0,
        mime: None,
        path: None,
        source: Some(AttachmentRef { message_id: lucia.id, index: d.attachments.iter().find(|a| !a.inline).unwrap().index }),
    }];
    let reply_target = lucia.id;
    msg.reply_to_message = Some(reply_target);
    // Testo formattato con un'immagine incollata: viaggia come parte incorporata.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
    msg.html = Some(format!(
        "<div class=\"better-look-corpo\" style=\"font-family: Aptos; font-size: 11pt;\"><div>Ciao Davide, riesci a portarli <b>giovedì</b>?</div><img src=\"data:image/png;base64,{PNG}\" width=\"1\"><div class=\"better-look-firma\"><div>Chiara Valli</div></div></div>"
    ));
    eng.send(&msg).await.unwrap();
    wait_for("inviato", 40, || {
        let events = events.clone();
        async move { events.lock().unwrap().iter().any(|e| matches!(e, Event::Outbox { state, .. } if state == "inviato")) }
    })
    .await;
    wait_for("arrivato in posta in arrivo", 40, || async { server_count("INBOX", "SUBJECT \"Prova di invio da Better-Look\"").await == 1 }).await;
    wait_for("copia in posta inviata sul server", 40, || async { server_count("Sent", "SUBJECT \"Prova di invio da Better-Look\"").await == 1 }).await;
    eng.sync_now(None);
    let sent = folder(&eng, &a, "sent");
    wait_for("posta inviata e in arrivo sincronizzate", 40, || {
        let eng = eng.clone();
        async move {
            let s = eng.messages(sent.id, None, 0, 10).unwrap();
            let got = find(&eng, inbox.id, "Prova di invio da Better-Look");
            s.iter().filter(|m| m.subject == "Prova di invio da Better-Look").count() == 1 && got.map(|m| m.has_attachments).unwrap_or(false)
        }
    })
    .await;
    let got = find(&eng, inbox.id, "Prova di invio da Better-Look").unwrap();
    let d = eng.detail(got.id, false).await.unwrap();
    let h = d.html.expect("parte HTML");
    assert!(h.contains("<b>giovedì</b>"), "formattazione conservata");
    assert!(h.contains("data:image/png;base64,"), "immagine incorporata visibile");
    assert!(d.attachments.iter().any(|x| x.inline), "immagine come parte incorporata");
    assert!(d.attachments.iter().any(|x| !x.inline && x.name.ends_with(".pdf")), "allegato vero conservato");
    assert!(d.text.contains("riesci a portarli"), "versione in testo semplice");
    wait_for("risposto sul server", 30, || async { server_flags("INBOX", "Campionario tessuti").await.contains("Answered") }).await;

    // Cambiamenti fatti da un altro dispositivo arrivano qui.
    {
        let mut s = imap::login(&server(), "prova-123").await.unwrap();
        s.select("INBOX").await.unwrap();
        let uids: Vec<u32> = s.uid_search("SUBJECT \"Riunione commerciale\"").await.unwrap().into_iter().collect();
        {
            let mut st = s.uid_store(uids[0].to_string(), "-FLAGS (\\Seen)").await.unwrap();
            while st.next().await.is_some() {}
        }
        let _ = s.logout().await;
    }
    eng.sync_now(Some(&a));
    wait_for("non letto da un altro dispositivo", 30, || {
        let eng = eng.clone();
        async move { find(&eng, inbox.id, "Riunione commerciale di lunedì").map(|m| m.unread).unwrap_or(false) }
    })
    .await;

    eng.remove_account(&a).unwrap();
    assert!(eng.folders().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn account_pop3() {
    if !enabled() {
        return;
    }
    // Un messaggio nella casella "altro" tramite SMTP.
    let (eng, _) = engine();
    let info = eng
        .add_account(NewAccount { name: "Altro".into(), email: "altro@better-look.test".into(), password: "altro-456".into(), kind: Some(AccountKind::Pop3), incoming: None, outgoing: None })
        .await
        .unwrap();
    let a = info.config.id.clone();
    assert_eq!(info.config.kind, AccountKind::Pop3);
    let inbox = folder(&eng, &a, "inbox");
    let draft = Draft {
        key: "pop".into(),
        account_id: a.clone(),
        to: vec![Person::new("", "altro@better-look.test")],
        cc: vec![],
        bcc: vec![],
        subject: format!("POP {}", better_look_lib::mime::random_id()),
        body: "Prova POP".into(),
        html: None,
        plain: false,
        attachments: vec![],
        in_reply_to: None,
        references: vec![],
        reply_to_message: None,
    };
    eng.send(&draft).await.unwrap();
    let subject = draft.subject.clone();
    wait_for("scaricato con POP", 60, || {
        let eng = eng.clone();
        let subject = subject.clone();
        let a = a.clone();
        async move {
            eng.sync_now(Some(&a));
            find(&eng, inbox.id, &subject).is_some()
        }
    })
    .await;
    let sent = folder(&eng, &a, "sent");
    assert!(find(&eng, sent.id, &subject).is_some(), "copia locale in Posta inviata");
    let m = find(&eng, inbox.id, &subject).unwrap();
    assert!(eng.detail(m.id, false).await.unwrap().text.contains("Prova POP"));
    eng.remove_account(&a).unwrap();
}

/// Regole e risposta automatica con posta vera: un messaggio da "altro"
/// arriva a "prova", la regola lo sposta in Clienti e "altro" riceve la
/// risposta automatica, una sola volta.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn regole_e_risposta_automatica() {
    if !enabled() {
        return;
    }
    reseed();
    let (eng, events) = engine();
    let prova = eng
        .add_account(NewAccount { name: "Chiara Valli".into(), email: "prova@better-look.test".into(), password: "prova-123".into(), kind: None, incoming: None, outgoing: None })
        .await
        .unwrap()
        .config
        .id;
    wait_for("prima sincronizzazione", 60, || {
        let eng = eng.clone();
        let prova = prova.clone();
        async move { eng.status(&prova).state == "pronto" && eng.folders().unwrap().iter().any(|f| f.account_id == prova && f.role.as_deref() == Some("inbox") && f.total == 48) }
    })
    .await;
    let clienti = eng.folders().unwrap().into_iter().find(|f| f.account_id == prova && f.path == "Clienti").unwrap();
    let tag = better_look_lib::mime::random_id()[..8].to_string();
    eng.save_rules(vec![Rule {
        id: String::new(),
        name: "Preventivi in Clienti".into(),
        enabled: true,
        account_id: Some(prova.clone()),
        when: RuleConditions { subject: vec![format!("Regola-{tag}")], ..Default::default() },
        then: RuleActions { move_to: Some(clienti.id), flag: true, ..Default::default() },
    }])
    .unwrap();
    eng.set_auto_reply(&prova, AutoReply { enabled: true, html: "<div>Sono in ferie.</div>".into(), text: "Sono in ferie.".into(), ..Default::default() }).unwrap();

    let altro = eng
        .add_account(NewAccount { name: "Altro".into(), email: "altro@better-look.test".into(), password: "altro-456".into(), kind: Some(AccountKind::Pop3), incoming: None, outgoing: None })
        .await
        .unwrap()
        .config
        .id;
    let send = |subject: String| {
        let eng = eng.clone();
        let altro = altro.clone();
        async move {
            eng.send(&Draft {
                key: better_look_lib::mime::random_id(),
                account_id: altro,
                to: vec![Person::new("Chiara Valli", "prova@better-look.test")],
                cc: vec![],
                bcc: vec![],
                subject,
                body: "Buongiorno, mi mandate il preventivo?".into(),
                html: None,
                plain: false,
                attachments: vec![],
                in_reply_to: None,
                references: vec![],
                reply_to_message: None,
            })
            .await
            .unwrap();
        }
    };
    let subject = format!("Regola-{tag} preventivo");
    send(subject.clone()).await;

    // La regola sposta il messaggio in Clienti, sul computer e sul server.
    wait_for("messaggio spostato dalla regola", 90, || {
        let eng = eng.clone();
        let prova = prova.clone();
        let subject = subject.clone();
        async move {
            eng.sync_now(Some(&prova));
            find(&eng, clienti.id, &subject).is_some_and(|m| m.flagged)
        }
    })
    .await;
    wait_for("spostato anche sul server", 60, || {
        let eng = eng.clone();
        let prova = prova.clone();
        let subject = subject.clone();
        async move {
            eng.sync_now(Some(&prova));
            server_count("Clienti", &format!("SUBJECT \"{subject}\"")).await == 1
        }
    })
    .await;
    assert_eq!(server_count("INBOX", &format!("SUBJECT \"{subject}\"")).await, 0);

    // Risposta automatica ad "altro", con le intestazioni giuste.
    let reply_subject = format!("Risposta automatica: {subject}");
    let altro_inbox = folder(&eng, &altro, "inbox");
    wait_for("risposta automatica arrivata", 90, || {
        let eng = eng.clone();
        let altro = altro.clone();
        let reply_subject = reply_subject.clone();
        async move {
            eng.sync_now(Some(&altro));
            find(&eng, altro_inbox.id, &reply_subject).is_some()
        }
    })
    .await;
    let r = find(&eng, altro_inbox.id, &reply_subject).unwrap();
    assert!(eng.detail(r.id, false).await.unwrap().text.contains("Sono in ferie."));

    // Un secondo messaggio dallo stesso mittente: nessuna seconda risposta.
    let second = format!("Regola-{tag} di nuovo");
    send(second.clone()).await;
    wait_for("secondo messaggio spostato", 90, || {
        let eng = eng.clone();
        let prova = prova.clone();
        let second = second.clone();
        async move {
            eng.sync_now(Some(&prova));
            find(&eng, clienti.id, &second).is_some()
        }
    })
    .await;
    tokio::time::sleep(Duration::from_secs(3)).await;
    eng.sync_now(Some(&altro));
    tokio::time::sleep(Duration::from_secs(3)).await;
    // Solo le risposte di questa prova: la casella di "altro" non si svuota tra una prova e l'altra.
    let replies = eng.messages(altro_inbox.id, None, 0, 1000).unwrap().into_iter().filter(|m| m.subject.starts_with(&format!("Risposta automatica: Regola-{tag}"))).count();
    assert_eq!(replies, 1);
    // Il messaggio spostato dalla regola non fa scattare l'avviso.
    assert!(!events.lock().unwrap().iter().any(|e| matches!(e, Event::NewMail { messages, .. } if messages.iter().any(|m| m.subject.starts_with("Regola-")))));

    eng.remove_account(&altro).unwrap();
    eng.remove_account(&prova).unwrap();
}

/// Riunione vera tra due computer: invito da Chiara (IMAP) ad "altro" (POP),
/// risposta Accetto che torna all'organizzatore, poi annullamento.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn invito_risposta_e_annullamento() {
    if !enabled() {
        return;
    }
    reseed();
    let (org, _) = engine();
    let (ospite, _) = engine();
    let prova = org
        .add_account(NewAccount { name: "Chiara Valli".into(), email: "prova@better-look.test".into(), password: "prova-123".into(), kind: None, incoming: None, outgoing: None })
        .await
        .unwrap()
        .config
        .id;
    let altro = ospite
        .add_account(NewAccount { name: "Altro".into(), email: "altro@better-look.test".into(), password: "altro-456".into(), kind: Some(AccountKind::Pop3), incoming: None, outgoing: None })
        .await
        .unwrap()
        .config
        .id;
    wait_for("prima sincronizzazione", 60, || {
        let org = org.clone();
        let prova = prova.clone();
        async move { org.status(&prova).state == "pronto" }
    })
    .await;

    let tag = better_look_lib::mime::random_id()[..8].to_string();
    let title = format!("Riunione {tag}");
    let start = (chrono::Utc::now() + chrono::Duration::days(3)).timestamp_millis() / 60_000 * 60_000;
    let ev = org
        .save_event(
            CalEvent {
                title: title.clone(),
                location: "Sala riunioni".into(),
                start,
                end: start + 3_600_000,
                attendees: vec![Attendee { name: "Altro".into(), email: "altro@better-look.test".into(), ..Default::default() }],
                ..Default::default()
            },
            true,
        )
        .unwrap();
    assert_eq!(ev.organizer.as_ref().map(|o| o.email.as_str()), Some("prova@better-look.test"));

    // L'invito arriva all'ospite, che lo accetta.
    let ospite_inbox = folder(&ospite, &altro, "inbox");
    wait_for("invito arrivato", 90, || {
        let ospite = ospite.clone();
        let altro = altro.clone();
        let title = title.clone();
        async move {
            ospite.sync_now(Some(&altro));
            find(&ospite, ospite_inbox.id, &title).is_some()
        }
    })
    .await;
    let msg = find(&ospite, ospite_inbox.id, &title).unwrap();
    let inv = ospite.detail(msg.id, false).await.unwrap().invite.expect("invito nel messaggio");
    assert_eq!(inv.method, "request");
    assert!(inv.for_me && inv.local.is_none());
    assert_eq!((inv.event.start, inv.event.end, inv.event.uid.clone()), (ev.start, ev.end, ev.uid.clone()));
    let accepted = ospite.respond_invite(msg.id, better_look_lib::engine::InviteAction::Accepted, true).await.unwrap().unwrap();
    assert_eq!(accepted.response, Some(PartStat::Accepted));

    // La risposta torna all'organizzatore e aggiorna i partecipanti.
    let org_inbox = folder(&org, &prova, "inbox");
    let reply_subject = format!("Accettato: {title}");
    wait_for("risposta arrivata", 90, || {
        let org = org.clone();
        let prova = prova.clone();
        let reply_subject = reply_subject.clone();
        async move {
            org.sync_now(Some(&prova));
            find(&org, org_inbox.id, &reply_subject).is_some()
        }
    })
    .await;
    let reply = find(&org, org_inbox.id, &reply_subject).unwrap();
    let inv = org.detail(reply.id, false).await.unwrap().invite.unwrap();
    assert_eq!(inv.method, "reply");
    assert_eq!(inv.replier.unwrap().status, PartStat::Accepted);
    assert_eq!(org.db.event(&ev.id).unwrap().unwrap().attendees[0].status, PartStat::Accepted);

    // L'organizzatore annulla: l'ospite toglie l'evento dal calendario.
    org.delete_event(&ev.id, None, true).unwrap();
    let cancel_subject = format!("Annullato: {title}");
    wait_for("annullamento arrivato", 90, || {
        let ospite = ospite.clone();
        let altro = altro.clone();
        let cancel_subject = cancel_subject.clone();
        async move {
            ospite.sync_now(Some(&altro));
            find(&ospite, ospite_inbox.id, &cancel_subject).is_some()
        }
    })
    .await;
    let c = find(&ospite, ospite_inbox.id, &cancel_subject).unwrap();
    let inv = ospite.detail(c.id, false).await.unwrap().invite.unwrap();
    assert_eq!(inv.method, "cancel");
    assert!(inv.local.is_some());
    ospite.respond_invite(c.id, better_look_lib::engine::InviteAction::Remove, false).await.unwrap();
    assert!(ospite.db.events_by_uid(&ev.uid).unwrap().is_empty());
}
