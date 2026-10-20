//! Datini: punto d'ingresso dell'applicazione desktop.
//!
//! Il motore di posta (IMAP, POP3, SMTP, archivio locale) sta in `engine` e
//! nei moduli che usa; l'interfaccia lo chiama con i comandi di `commands` e
//! riceve le notifiche sull'evento "datini://evento".

pub mod autoconfig;
pub mod commands;
pub mod db;
pub mod engine;
pub mod error;
pub mod imap;
pub mod mime;
pub mod model;
pub mod net;
pub mod pop3;
pub mod pst;
pub mod rtf;
pub mod sanitize;
pub mod secrets;
pub mod smtp;
pub mod utf7;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use crate::db::Db;
use crate::engine::Engine;
use crate::secrets::{Keyring, Memory, Secrets};

/// Le prime versioni salvavano i dati in una cartella con un altro
/// identificativo dell'app: se c'è ancora, diventa la cartella di Datini.
fn migrate_old_data_dir(dir: &std::path::Path) {
    const OLD_ID: &str = "com.paxxion.datini";
    let Some(parent) = dir.parent() else { return };
    let old = parent.join(OLD_ID);
    if old == dir || !old.join("datini.db").exists() || dir.join("datini.db").exists() {
        return;
    }
    // La cartella nuova può esistere già (il sistema ci mette i dati della
    // finestra): si spostano i file uno per uno, senza toccare quelli presenti.
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(&old) else { return };
    for e in entries.flatten() {
        let dest = dir.join(e.file_name());
        if !dest.exists() {
            let _ = std::fs::rename(e.path(), dest);
        }
    }
    let _ = std::fs::remove_dir(&old);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    net::install_crypto();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = match std::env::var_os("DATINI_DATI") {
                Some(d) => std::path::PathBuf::from(d),
                None => {
                    let dir = app.path().app_data_dir()?;
                    migrate_old_data_dir(&dir);
                    dir
                }
            };
            std::fs::create_dir_all(&dir)?;
            let db = Db::open(&dir.join("datini.db"))?;
            let secrets: Arc<dyn Secrets> = if std::env::var("DATINI_PORTACHIAVI").as_deref() == Ok("memoria") {
                Arc::new(Memory::default())
            } else {
                Arc::new(Keyring)
            };
            let handle = app.handle().clone();
            let engine = Engine::new(
                db,
                secrets,
                dir,
                Arc::new(move |e| {
                    let _ = handle.emit("datini://evento", e);
                }),
            );
            app.manage(engine.clone());
            app.manage(commands::ComposeWindows::default());
            tauri::async_runtime::spawn(async move { engine.start() });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts,
            commands::add_account,
            commands::update_account,
            commands::set_password,
            commands::remove_account,
            commands::folders,
            commands::messages,
            commands::search,
            commands::message,
            commands::quote_html,
            commands::signatures,
            commands::save_signature,
            commands::delete_signature,
            commands::fetch_image,
            commands::set_flags,
            commands::move_messages,
            commands::move_to_role,
            commands::delete_messages,
            commands::save_draft,
            commands::close_draft,
            commands::discard_draft,
            commands::open_draft,
            commands::send,
            commands::cancel_send,
            commands::outbox,
            commands::outbox_take,
            commands::sync_now,
            commands::contacts,
            commands::open_attachment,
            commands::save_attachment,
            commands::export_eml,
            commands::export_mbox,
            commands::open_link,
            commands::open_compose_window,
            commands::take_compose_draft,
            pst::import_pst,
            pst::import_mbox,
        ])
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di Datini");
}

#[cfg(test)]
mod tests {
    #[test]
    fn old_data_folder_moves_to_the_new_one() {
        let base = std::env::temp_dir().join(format!("datini-migra-{}", crate::mime::random_id()));
        let old = base.join("com.paxxion.datini");
        let new = base.join("app.datini");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("datini.db"), b"archivio").unwrap();
        std::fs::write(new.join("finestra.dat"), b"x").unwrap();
        super::migrate_old_data_dir(&new);
        assert_eq!(std::fs::read(new.join("datini.db")).unwrap(), b"archivio");
        assert!(new.join("finestra.dat").exists());
        assert!(!old.exists());
        // Una seconda volta non fa nulla.
        super::migrate_old_data_dir(&new);
        assert!(new.join("datini.db").exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
