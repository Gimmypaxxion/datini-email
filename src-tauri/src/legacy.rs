//! Dati delle versioni di anteprima uscite con il nome precedente del
//! programma. Al primo avvio cartella, archivio e password passano a
//! Better-Look; poi qui non c'è più niente da fare. Il modulo si può togliere
//! quando nessuno usa più quelle versioni.

use std::path::Path;

/// Identificativi usati in passato per la cartella dei dati, dal più recente.
const OLD_IDS: [&str; 2] = ["app.datini", "com.paxxion.datini"];
/// Nome precedente del file dell'archivio.
const OLD_DB: &str = "datini.db";
/// Nome precedente del servizio nel portachiavi del sistema.
pub const OLD_KEYRING_SERVICE: &str = "Datini";

/// Porta in `dir` i dati di una versione precedente, se ci sono e se
/// l'archivio `db_name` non esiste ancora. Non sovrascrive mai nulla.
pub fn migrate_data_dir(dir: &Path, db_name: &str) {
    if dir.join(db_name).exists() {
        return;
    }
    if !dir.join(OLD_DB).exists() {
        if let Some(old) = dir.parent().and_then(|p| OLD_IDS.iter().map(|id| p.join(id)).find(|o| o != dir && o.join(OLD_DB).exists())) {
            // La cartella nuova può esistere già (il sistema ci mette i dati
            // della finestra): si spostano i file uno per uno, senza toccare
            // quelli presenti.
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
    }
    // L'archivio SQLite può avere accanto i file del registro (-wal, -shm).
    for suffix in ["", "-wal", "-shm"] {
        let from = dir.join(format!("{OLD_DB}{suffix}"));
        let to = dir.join(format!("{db_name}{suffix}"));
        if from.exists() && !to.exists() {
            let _ = std::fs::rename(from, to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("better-look-migra-{}", crate::mime::random_id()))
    }

    #[test]
    fn data_of_the_previous_name_moves_to_the_new_folder() {
        let base = base();
        let old = base.join(OLD_IDS[0]);
        let new = base.join("app.betterlook");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join(OLD_DB), b"archivio").unwrap();
        std::fs::write(old.join(format!("{OLD_DB}-wal")), b"registro").unwrap();
        std::fs::write(old.join("allegati.dat"), b"a").unwrap();
        std::fs::write(new.join("finestra.dat"), b"x").unwrap();
        migrate_data_dir(&new, "better-look.db");
        assert_eq!(std::fs::read(new.join("better-look.db")).unwrap(), b"archivio");
        assert_eq!(std::fs::read(new.join("better-look.db-wal")).unwrap(), b"registro");
        assert!(new.join("allegati.dat").exists());
        assert!(new.join("finestra.dat").exists());
        assert!(!new.join(OLD_DB).exists());
        assert!(!old.exists());
        // Una seconda volta non fa nulla.
        migrate_data_dir(&new, "better-look.db");
        assert!(new.join("better-look.db").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_oldest_folder_is_used_when_it_is_the_only_one() {
        let base = base();
        let old = base.join(OLD_IDS[1]);
        let new = base.join("app.betterlook");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join(OLD_DB), b"archivio").unwrap();
        migrate_data_dir(&new, "better-look.db");
        assert_eq!(std::fs::read(new.join("better-look.db")).unwrap(), b"archivio");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_existing_archive_is_never_replaced() {
        let base = base();
        let old = base.join(OLD_IDS[0]);
        let new = base.join("app.betterlook");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join(OLD_DB), b"vecchio").unwrap();
        std::fs::write(new.join("better-look.db"), b"nuovo").unwrap();
        migrate_data_dir(&new, "better-look.db");
        assert_eq!(std::fs::read(new.join("better-look.db")).unwrap(), b"nuovo");
        assert!(old.join(OLD_DB).exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
