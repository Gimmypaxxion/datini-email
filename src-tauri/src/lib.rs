//! Better-Look: punto d'ingresso dell'applicazione desktop.
//!
//! Il motore di posta (IMAP, POP3, SMTP, archivio locale) sta in `engine` e
//! nei moduli che usa; l'interfaccia lo chiama con i comandi di `commands` e
//! riceve le notifiche sull'evento "better-look://evento".

pub mod autoconfig;
pub mod commands;
pub mod db;
pub mod engine;
pub mod error;
pub mod imap;
pub mod legacy;
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

/// Nome del file dell'archivio nella cartella dei dati.
const DB_FILE: &str = "better-look.db";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    net::install_crypto();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = match std::env::var_os("BETTERLOOK_DATI") {
                Some(d) => std::path::PathBuf::from(d),
                None => {
                    let dir = app.path().app_data_dir()?;
                    legacy::migrate_data_dir(&dir, DB_FILE);
                    dir
                }
            };
            std::fs::create_dir_all(&dir)?;
            let db = Db::open(&dir.join(DB_FILE))?;
            let secrets: Arc<dyn Secrets> = if std::env::var("BETTERLOOK_PORTACHIAVI").as_deref() == Ok("memoria") {
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
                    let _ = handle.emit("better-look://evento", e);
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
        .expect("errore durante l'avvio di Better-Look");
}
