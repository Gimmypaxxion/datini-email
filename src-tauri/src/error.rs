//! Errori del motore di posta, con messaggi in italiano pensati per l'utente.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Messaggio già scritto per l'utente.
    #[error("{0}")]
    User(String),

    #[error("Impossibile raggiungere {host}. Controlla la connessione a internet e il nome del server.")]
    Unreachable { host: String },

    #[error("Il server {host} non ha risposto in tempo.")]
    Timeout { host: String },

    #[error("Il server ha rifiutato nome utente o password.")]
    Auth,

    #[error("Il certificato di sicurezza di {host} non è valido, quindi la connessione è stata interrotta per proteggerti. ({detail})")]
    Certificate { host: String, detail: String },

    #[error("Il server ha risposto con un errore: {0}")]
    Protocol(String),

    #[error("Il portachiavi del sistema non è disponibile: {0}")]
    Keyring(String),

    #[error("Errore dell'archivio locale: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Errore di lettura o scrittura: {0}")]
    Io(#[from] std::io::Error),

    #[error("Non trovato.")]
    NotFound,

    /// Le impostazioni del server vanno inserite a mano.
    #[error("{0}")]
    Manual(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn user(msg: impl Into<String>) -> Self {
        Error::User(msg.into())
    }

    /// Vero per i problemi di rete: l'operazione va ritentata più tardi.
    pub fn is_network(&self) -> bool {
        matches!(self, Error::Unreachable { .. } | Error::Timeout { .. } | Error::Io(_))
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Error::User(_) => "utente",
            Error::Unreachable { .. } | Error::Timeout { .. } | Error::Io(_) => "rete",
            Error::Auth => "credenziali",
            Error::Certificate { .. } => "certificato",
            Error::Protocol(_) => "server",
            Error::Keyring(_) => "portachiavi",
            Error::Db(_) => "archivio",
            Error::NotFound => "non-trovato",
            Error::Manual(_) => "manuale",
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::User(format!("Dati non validi: {e}"))
    }
}

impl From<async_imap::error::Error> for Error {
    fn from(e: async_imap::error::Error) -> Self {
        use async_imap::error::Error as E;
        match e {
            E::Io(io) => Error::Io(io),
            E::No(msg) | E::Bad(msg) => {
                let low = msg.to_ascii_lowercase();
                if low.contains("authenticat") || low.contains("login") || low.contains("password") || low.contains("credentials") {
                    Error::Auth
                } else {
                    Error::Protocol(msg)
                }
            }
            E::ConnectionLost => Error::Io(std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connessione interrotta")),
            other => Error::Protocol(other.to_string()),
        }
    }
}

impl From<lettre::transport::smtp::Error> for Error {
    fn from(e: lettre::transport::smtp::Error) -> Self {
        let text = e.to_string();
        if let Some(code) = e.status() {
            let c = code.to_string();
            if c.starts_with("535") || c.starts_with("534") || c.starts_with("530") {
                return Error::Auth;
            }
            return Error::Protocol(text);
        }
        if e.is_permanent() {
            return Error::Protocol(text);
        }
        let low = text.to_ascii_lowercase();
        if low.contains("certificate") || low.contains("invalid peer") {
            return Error::Certificate { host: String::new(), detail: text };
        }
        Error::Io(std::io::Error::other(text))
    }
}
