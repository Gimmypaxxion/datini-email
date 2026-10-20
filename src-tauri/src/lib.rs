//! Datini: punto d'ingresso dell'applicazione desktop.
//!
//! Fase 1: l'interfaccia gira con dati di esempio. Il motore di posta
//! (IMAP, SMTP, POP3, archivio locale) arriva nelle fasi successive e
//! verrà esposto all'interfaccia tramite comandi Tauri definiti qui.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("errore durante l'avvio di Datini");
}
