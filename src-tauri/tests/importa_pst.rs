//! Importazione di un file PST vero nel motore completo.
//! Si attiva indicando il file: DATINI_PST=/percorso/file.pst cargo test --test importa_pst -- --nocapture

use std::sync::{Arc, Mutex};

use datini_lib::db::Db;
use datini_lib::engine::Engine;
use datini_lib::model::*;
use datini_lib::secrets::Memory;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn importa_pst() {
    let Ok(path) = std::env::var("DATINI_PST") else { return };
    let dir = std::env::temp_dir().join(format!("datini-pst-{}", datini_lib::mime::random_id()));
    std::fs::create_dir_all(&dir).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let ev = events.clone();
    let engine = Engine::new(Db::open(&dir.join("datini.db")).unwrap(), Arc::new(Memory::default()), dir, Arc::new(move |e| ev.lock().unwrap().push(e)));

    let e2 = engine.clone();
    let r = tokio::task::spawn_blocking(move || datini_lib::pst::import_pst_file(&e2, std::path::Path::new(&path))).await.unwrap().expect("importazione riuscita");
    println!("file di dati: {} ({} messaggi e {} allegati non letti)", r.account.config.name, r.skipped, r.lost_attachments);

    let folders: Vec<FolderInfo> = engine.folders().unwrap().into_iter().filter(|f| f.account_id == r.account.config.id).collect();
    assert!(!folders.is_empty());
    let mut total = 0;
    for f in &folders {
        println!("{}{} [{:?}] {} messaggi, {} da leggere", "  ".repeat(f.depth as usize), f.name, f.role, f.total, f.unread);
        total += f.total;
    }
    assert!(total > 0);
    assert!(!folders.iter().any(|f| ["Calendar", "Contacts", "Tasks", "Notes", "Journal"].contains(&f.name.as_str())), "solo cartelle di posta");

    // Ogni messaggio si apre: testo, mittente, allegati.
    let mut opened = 0;
    for f in folders.iter().filter(|f| f.total > 0) {
        for m in engine.messages(f.id, None, 0, 500).unwrap() {
            let d = engine.detail(m.id, false).await.expect("messaggio leggibile");
            if opened < 6 {
                println!("  - {} | {} <{}> | {} allegati | html {} caratteri | testo {} caratteri", d.summary.subject, d.summary.from.name, d.summary.from.email, d.attachments.len(), d.html.as_deref().map(str::len).unwrap_or(0), d.text.len());
            }
            opened += 1;
        }
    }
    println!("messaggi aperti: {opened}");
    assert_eq!(opened, total);
    let progress = events.lock().unwrap().iter().filter(|e| matches!(e, Event::Import { .. })).count();
    assert!(progress >= 2, "avanzamento segnalato");
}
