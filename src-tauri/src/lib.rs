//! Better-Look: punto d'ingresso dell'applicazione desktop.
//!
//! Il motore di posta (IMAP, POP3, SMTP, archivio locale) sta in `engine` e
//! nei moduli che usa; l'interfaccia lo chiama con i comandi di `commands` e
//! riceve le notifiche sull'evento "better-look://evento".

pub mod autoconfig;
pub mod calendar;
pub mod commands;
pub mod contacts;
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
pub mod rules;
pub mod sanitize;
pub mod secrets;
pub mod smtp;
pub mod utf7;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::db::Db;
use crate::engine::Engine;
use crate::secrets::{Keyring, Memory, Secrets};

/// Nome del file dell'archivio nella cartella dei dati.
const DB_FILE: &str = "better-look.db";

/// Argomento con cui il sistema avvia Better-Look all'accensione: parte in sottofondo, senza finestra.
pub const BACKGROUND_ARG: &str = "--sottofondo";

/// Identificativo dell'icona vicino all'orologio.
pub const TRAY_ID: &str = "principale";

/// Chiudendo la finestra principale Better-Look resta attivo in sottofondo
/// (regole, risposte automatiche, invio programmato e promemoria continuano).
pub struct Background(pub AtomicBool);

/// L'icona vicino all'orologio c'è: senza, la finestra chiusa si riduce a icona invece di sparire.
pub struct TrayReady(pub AtomicBool);

/// Su Linux l'icona richiede la libreria AppIndicator: se manca, il sistema
/// interromperebbe il programma, quindi prima si controlla che ci sia.
fn tray_supported() -> bool {
    if !cfg!(target_os = "linux") {
        return true;
    }
    let names = ["libayatana-appindicator3.so.1", "libappindicator3.so.1"];
    let mut dirs: Vec<std::path::PathBuf> = ["/usr/lib", "/usr/lib64", "/usr/lib/x86_64-linux-gnu", "/usr/lib/aarch64-linux-gnu", "/lib/x86_64-linux-gnu", "/usr/local/lib"].iter().map(std::path::PathBuf::from).collect();
    for var in ["LD_LIBRARY_PATH", "APPDIR"] {
        if let Some(v) = std::env::var_os(var) {
            for d in std::env::split_paths(&v) {
                dirs.push(d.join("usr/lib"));
                dirs.push(d);
            }
        }
    }
    dirs.iter().any(|d| names.iter().any(|n| d.join(n).exists()))
}

impl Default for Background {
    fn default() -> Self {
        Background(AtomicBool::new(true))
    }
}

/// Mostra la finestra principale (dall'icona, dal Dock o da un secondo avvio).
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Icona vicino all'orologio (sul Mac nella barra dei menu) con il suo menu.
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "apri", "Apri Better-Look", true, None::<&str>)?;
    let new = MenuItem::with_id(app, "nuovo", "Nuovo messaggio", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "esci", "Esci da Better-Look", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &new, &sep, &quit])?;
    // Sul Mac un'icona monocromatica, che il sistema colora come le altre della barra.
    let icon = if cfg!(target_os = "macos") {
        Image::from_bytes(include_bytes!("../icons/tray-template.png"))?
    } else {
        match app.default_window_icon() {
            Some(i) => i.clone(),
            None => Image::from_bytes(include_bytes!("../icons/32x32.png"))?,
        }
    };
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Better-Look")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "apri" => show_main(app),
            "nuovo" => {
                show_main(app);
                let _ = app.emit("better-look://comando", "nuovo-messaggio");
            }
            "esci" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    net::install_crypto();
    tauri::Builder::default()
        // Un secondo avvio (dal menu Start, dal Dock, dall'avvio automatico)
        // non apre un altro programma sullo stesso archivio: mostra la finestra.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![BACKGROUND_ARG])))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
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
            app.manage(commands::Speech::default());
            app.manage(Background::default());
            tauri::async_runtime::spawn(async move { engine.start() });
            let tray = tray_supported() && build_tray(app).map_err(|e| log::warn!("icona vicino all'orologio non disponibile: {e}")).is_ok();
            app.manage(TrayReady(AtomicBool::new(tray)));
            // All'accensione del computer parte senza finestra; altrimenti la mostra.
            if !std::env::args().any(|a| a == BACKGROUND_ARG) {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let keep = app.state::<Background>().0.load(Ordering::Relaxed);
                if window.label() == "main" && keep {
                    api.prevent_close();
                    if app.state::<TrayReady>().0.load(Ordering::Relaxed) {
                        let _ = window.hide();
                        let _ = app.emit("better-look://comando", "nascosta");
                    } else {
                        // Senza icona la finestra resta nella barra delle applicazioni.
                        let _ = window.minimize();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::quit_app,
            commands::set_background,
            commands::autostart,
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
            commands::send_read_receipt,
            commands::skip_read_receipt,
            commands::signatures,
            commands::save_signature,
            commands::delete_signature,
            commands::fetch_image,
            commands::set_flags,
            commands::move_messages,
            commands::move_to_role,
            commands::delete_messages,
            commands::categories,
            commands::save_categories,
            commands::set_category,
            commands::clear_categories,
            commands::thread,
            commands::create_folder,
            commands::rename_folder,
            commands::delete_folder,
            commands::mark_folder_read,
            commands::empty_folder,
            commands::save_draft,
            commands::close_draft,
            commands::discard_draft,
            commands::open_draft,
            commands::send,
            commands::cancel_send,
            commands::send_later,
            commands::rules,
            commands::save_rules,
            commands::run_rules,
            commands::set_auto_reply,
            commands::address_book,
            commands::save_contact,
            commands::delete_contacts,
            commands::import_contacts,
            commands::export_contacts,
            commands::ignore_thread,
            commands::thread_ignored,
            commands::clean_up,
            commands::quick_steps,
            commands::save_quick_steps,
            commands::templates,
            commands::save_template,
            commands::delete_template,
            commands::task_lists,
            commands::save_task_list,
            commands::delete_task_list,
            commands::tasks,
            commands::save_task,
            commands::delete_task,
            commands::flagged_messages,
            commands::events,
            commands::save_event,
            commands::delete_event,
            commands::respond_invite,
            commands::respond_event,
            commands::import_calendar,
            commands::export_calendar,
            commands::mailbox_usage,
            commands::large_messages,
            commands::older_messages,
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
            commands::open_print_window,
            commands::print_page,
            commands::speak,
            commands::speaking,
            commands::stop_speaking,
            pst::import_pst,
            pst::import_mbox,
        ])
        .build(tauri::generate_context!())
        .expect("errore durante l'avvio di Better-Look")
        .run(|_app, _event| {
            // Sul Mac il clic sull'icona nel Dock riapre la finestra nascosta.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_main(_app);
            }
        });
}
