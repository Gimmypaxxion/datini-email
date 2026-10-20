//! Invio con SMTP, direttamente al server del provider.

use std::time::Duration;

use lettre::address::Envelope as LEnvelope;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

use crate::db::Envelope;
use crate::error::{Error, Result};
use crate::model::{Security, ServerConfig};

fn transport(cfg: &ServerConfig, password: &str) -> Result<AsyncSmtpTransport<Tokio1Executor>> {
    crate::net::install_crypto();
    let tls = TlsParameters::new_rustls(cfg.host.clone()).map_err(|e| Error::Protocol(e.to_string()))?;
    let builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(cfg.host.clone())
        .port(cfg.port)
        .tls(match cfg.security {
            Security::Ssl => Tls::Wrapper(tls),
            Security::Starttls => Tls::Required(tls),
        })
        .credentials(Credentials::new(cfg.username.clone(), password.to_string()))
        .authentication(vec![Mechanism::Plain, Mechanism::Login])
        .timeout(Some(Duration::from_secs(60)));
    Ok(builder.build())
}

fn map_err(cfg: &ServerConfig, e: lettre::transport::smtp::Error) -> Error {
    match Error::from(e) {
        Error::Certificate { detail, .. } => Error::Certificate { host: cfg.host.clone(), detail },
        Error::Io(io) => {
            let t = io.to_string().to_ascii_lowercase();
            if t.contains("timed out") || t.contains("timeout") {
                Error::Timeout { host: cfg.host.clone() }
            } else {
                Error::Unreachable { host: cfg.host.clone() }
            }
        }
        other => other,
    }
}

/// Verifica server e credenziali senza spedire nulla.
pub async fn test(cfg: &ServerConfig, password: &str) -> Result<()> {
    let t = transport(cfg, password)?;
    match t.test_connection().await {
        Ok(true) => Ok(()),
        Ok(false) => Err(Error::Unreachable { host: cfg.host.clone() }),
        Err(e) => Err(map_err(cfg, e)),
    }
}

pub async fn send(cfg: &ServerConfig, password: &str, env: &Envelope, raw: &[u8]) -> Result<()> {
    let from = env.from.parse().map_err(|_| Error::user(format!("L'indirizzo del mittente \"{}\" non è valido.", env.from)))?;
    let mut to = Vec::new();
    for r in &env.to {
        to.push(r.parse().map_err(|_| Error::user(format!("L'indirizzo \"{r}\" non è valido.")))?);
    }
    let envelope = LEnvelope::new(Some(from), to).map_err(|_| Error::user("Il messaggio non ha destinatari."))?;
    let t = transport(cfg, password)?;
    t.send_raw(&envelope, raw).await.map_err(|e| map_err(cfg, e))?;
    Ok(())
}
