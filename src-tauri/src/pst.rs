//! Importazione di archivi: file di dati di Outlook (PST) e MBOX.
//! Ogni archivio diventa un "file di dati" nel riquadro cartelle, come in Outlook.

use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::State;

use crate::db::NewMessage;
use crate::engine::Engine;
use crate::error::{Error, Result};
use crate::mime;
use crate::model::*;

fn file_stem(path: &Path) -> String {
    path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Archivio".into())
}

/// Inserisce un messaggio completo in una cartella di un file di dati.
pub fn insert_raw(engine: &Engine, account_id: &str, folder_id: i64, raw: Vec<u8>, fallback_date: i64, unread: bool) -> Result<()> {
    let mut nm = match mime::parse(&raw) {
        Some(p) => p.to_new_message(fallback_date, true, raw.len() as i64),
        None => NewMessage { date: fallback_date, ..Default::default() },
    };
    nm.unread = unread;
    nm.focused = true;
    nm.raw = Some(raw);
    engine.db.insert_message(account_id, folder_id, &nm)?;
    Ok(())
}

pub fn import_mbox_file(engine: &Engine, path: &Path) -> Result<AccountInfo> {
    let file = std::fs::File::open(path).map_err(|_| Error::user("Impossibile aprire il file MBOX."))?;
    let name = file_stem(path);
    let acct = engine.create_local_account(&name, Some(path.to_string_lossy().to_string()))?;
    let folder = engine.db.upsert_folder(&acct.id, &name, &name, None, Some("/"), true)?;
    let mut n = 0;
    for m in mail_parser::mailbox::mbox::MessageIterator::new(BufReader::new(file)) {
        let Ok(m) = m else { continue };
        let date = m.internal_date() as i64;
        insert_raw(engine, &acct.id, folder, m.unwrap_contents(), date, false)?;
        n += 1;
    }
    if n == 0 {
        engine.remove_account(&acct.id)?;
        return Err(Error::user("Il file non contiene messaggi in formato MBOX."));
    }
    Ok(AccountInfo { status: engine.status(&acct.id), config: acct })
}

#[tauri::command]
pub async fn import_mbox(e: State<'_, Arc<Engine>>, path: PathBuf) -> Result<AccountInfo> {
    let engine = e.inner().clone();
    tokio::task::spawn_blocking(move || import_mbox_file(&engine, &path)).await.map_err(|e| Error::user(e.to_string()))?
}

#[tauri::command]
pub async fn import_pst(e: State<'_, Arc<Engine>>, path: PathBuf) -> Result<AccountInfo> {
    let _ = (e, path);
    Err(Error::user("L'importazione dei file PST non è ancora pronta."))
}
