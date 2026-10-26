//! Invio con SMTP, direttamente al server del provider.

use std::time::Duration;

use lettre::address::Envelope as LEnvelope;
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::client::{AsyncSmtpConnection, Tls, TlsParameters};
use lettre::transport::smtp::commands::{Data, Ehlo, Mail, Rcpt};
use lettre::transport::smtp::extension::{ClientId, MailBodyParameter, MailParameter, RcptParameter};
use lettre::transport::smtp::response::Response;
use lettre::transport::smtp::Error as SmtpError;
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

/// Spedisce il messaggio. Con `delivery_receipt` chiede al server la conferma
/// di recapito (DSN, RFC 3461), se il server la offre.
pub async fn send(cfg: &ServerConfig, password: &str, env: &Envelope, raw: &[u8], delivery_receipt: bool) -> Result<()> {
    let from = env.from.parse().map_err(|_| Error::user(format!("L'indirizzo del mittente \"{}\" non è valido.", env.from)))?;
    let mut to = Vec::new();
    for r in &env.to {
        to.push(r.parse().map_err(|_| Error::user(format!("L'indirizzo \"{r}\" non è valido.")))?);
    }
    let envelope = LEnvelope::new(Some(from), to).map_err(|_| Error::user("Il messaggio non ha destinatari."))?;
    if delivery_receipt {
        crate::net::install_crypto();
        let dsn = send_with_dsn(cfg, password, &envelope, raw).await.map_err(|e| map_err(cfg, e))?;
        if !dsn {
            log::info!("{}: il server non offre le conferme di recapito (DSN)", cfg.host);
        }
        return Ok(());
    }
    let t = transport(cfg, password)?;
    t.send_raw(&envelope, raw).await.map_err(|e| map_err(cfg, e))?;
    Ok(())
}

/// Le parole chiave annunciate dal server in risposta a EHLO ("DSN", "SIZE"...).
fn ehlo_has(resp: &Response, keyword: &str) -> bool {
    resp.message().any(|l| l.split_whitespace().next().is_some_and(|k| k.eq_ignore_ascii_case(keyword)))
}

/// Invio a basso livello, per aggiungere RET e NOTIFY a MAIL e RCPT.
/// Restituisce se il server ha accettato la richiesta di conferma.
async fn send_with_dsn(cfg: &ServerConfig, password: &str, env: &LEnvelope, raw: &[u8]) -> std::result::Result<bool, SmtpError> {
    let hello = ClientId::default();
    let tls = TlsParameters::new_rustls(cfg.host.clone())?;
    let timeout = Some(Duration::from_secs(60));
    let addr = (cfg.host.as_str(), cfg.port);
    let mut conn = match cfg.security {
        Security::Ssl => AsyncSmtpConnection::connect_tokio1(addr, timeout, &hello, Some(tls), None).await?,
        Security::Starttls => {
            let mut c = AsyncSmtpConnection::connect_tokio1(addr, timeout, &hello, None, None).await?;
            c.starttls(tls, &hello).await?;
            c
        }
    };
    // ServerInfo di lettre non conosce DSN: lo si legge da un nuovo EHLO, prima dell'accesso.
    let ehlo = conn.command(Ehlo::new(hello.clone())).await?;
    let dsn = ehlo_has(&ehlo, "DSN");
    let eight_bit = ehlo_has(&ehlo, "8BITMIME");
    conn.auth(&[Mechanism::Plain, Mechanism::Login], &Credentials::new(cfg.username.clone(), password.to_string())).await?;
    let mut mail = Vec::new();
    if !raw.is_ascii() && eight_bit {
        mail.push(MailParameter::Body(MailBodyParameter::EightBitMime));
    }
    if dsn {
        mail.push(MailParameter::Other { keyword: "RET".into(), value: Some("HDRS".into()) });
    }
    conn.command(Mail::new(env.from().cloned(), mail)).await?;
    for to in env.to() {
        let params = if dsn { vec![RcptParameter::Other { keyword: "NOTIFY".into(), value: Some("SUCCESS,FAILURE".into()) }] } else { Vec::new() };
        conn.command(Rcpt::new(to.clone(), params)).await?;
    }
    conn.command(Data).await?;
    conn.message(raw).await?;
    let _ = conn.quit().await;
    Ok(dsn)
}
