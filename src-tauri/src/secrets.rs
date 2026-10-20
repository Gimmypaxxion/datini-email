//! Password degli account: stanno nel portachiavi del sistema operativo
//! (Portachiavi di macOS, Gestione credenziali di Windows, Secret Service su
//! Linux), mai in chiaro su disco.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::{Error, Result};

pub trait Secrets: Send + Sync {
    fn get(&self, account_id: &str) -> Result<Option<String>>;
    fn set(&self, account_id: &str, password: &str) -> Result<()>;
    fn delete(&self, account_id: &str) -> Result<()>;
}

const SERVICE: &str = "Datini";

pub struct Keyring;

impl Secrets for Keyring {
    fn get(&self, account_id: &str) -> Result<Option<String>> {
        let entry = keyring::Entry::new(SERVICE, account_id).map_err(|e| Error::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(Error::Keyring(e.to_string())),
        }
    }

    fn set(&self, account_id: &str, password: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, account_id).map_err(|e| Error::Keyring(e.to_string()))?;
        entry.set_password(password).map_err(|e| Error::Keyring(e.to_string()))
    }

    fn delete(&self, account_id: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE, account_id).map_err(|e| Error::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(Error::Keyring(e.to_string())),
        }
    }
}

/// Solo per i test e per le prove in ambienti senza portachiavi
/// (variabile d'ambiente DATINI_PORTACHIAVI=memoria): le password si perdono
/// alla chiusura del programma.
#[derive(Default)]
pub struct Memory(Mutex<HashMap<String, String>>);

impl Secrets for Memory {
    fn get(&self, account_id: &str) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap_or_else(|e| e.into_inner()).get(account_id).cloned())
    }
    fn set(&self, account_id: &str, password: &str) -> Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).insert(account_id.into(), password.into());
        Ok(())
    }
    fn delete(&self, account_id: &str) -> Result<()> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).remove(account_id);
        Ok(())
    }
}
