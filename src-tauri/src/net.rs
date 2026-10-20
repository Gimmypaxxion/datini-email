//! Connessioni cifrate verso i server di posta. Il certificato del server viene
//! verificato con i certificati del sistema operativo (come fa il browser);
//! le connessioni in chiaro non sono ammesse.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use rustls::pki_types::ServerName;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;

use crate::error::{Error, Result};

pub type Tls = TlsStream<TcpStream>;

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(90);

static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();

/// Sceglie la libreria crittografica (ring) per tutto il programma.
pub fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn tls_config() -> Result<Arc<rustls::ClientConfig>> {
    if let Some(c) = CONFIG.get() {
        return Ok(c.clone());
    }
    install_crypto();
    use rustls_platform_verifier::ConfigVerifierExt;
    let cfg = rustls::ClientConfig::with_platform_verifier().map_err(|e| Error::Protocol(e.to_string()))?;
    let cfg = Arc::new(cfg);
    let _ = CONFIG.set(cfg.clone());
    Ok(cfg)
}

pub async fn tcp(host: &str, port: u16) -> Result<TcpStream> {
    let host_s = host.to_string();
    match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port))).await {
        Err(_) => Err(Error::Timeout { host: host_s }),
        Ok(Err(_)) => Err(Error::Unreachable { host: host_s }),
        Ok(Ok(s)) => {
            let _ = s.set_nodelay(true);
            Ok(s)
        }
    }
}

fn describe_cert_error(e: &rustls::CertificateError) -> String {
    use rustls::CertificateError as C;
    match e {
        C::Expired | C::ExpiredContext { .. } => "il certificato è scaduto".into(),
        C::NotValidYet | C::NotValidYetContext { .. } => "il certificato non è ancora valido".into(),
        C::NotValidForName | C::NotValidForNameContext { .. } => {
            "il certificato è intestato a un altro nome: prova con il nome di server indicato dal tuo provider".into()
        }
        C::UnknownIssuer => "il certificato non è stato emesso da un'autorità riconosciuta (potrebbe essere autofirmato)".into(),
        C::Revoked => "il certificato è stato revocato".into(),
        other => format!("{other:?}"),
    }
}

pub async fn tls_wrap(tcp: TcpStream, host: &str) -> Result<Tls> {
    let connector = TlsConnector::from(tls_config()?);
    let name = ServerName::try_from(host.to_string()).map_err(|_| Error::user(format!("Il nome del server \"{host}\" non è valido.")))?;
    match tokio::time::timeout(CONNECT_TIMEOUT, connector.connect(name, tcp)).await {
        Err(_) => Err(Error::Timeout { host: host.into() }),
        Ok(Ok(s)) => Ok(s),
        Ok(Err(e)) => {
            let inner = e.get_ref().and_then(|i| i.downcast_ref::<rustls::Error>());
            match inner {
                Some(rustls::Error::InvalidCertificate(c)) => Err(Error::Certificate { host: host.into(), detail: describe_cert_error(c) }),
                Some(other) => Err(Error::Certificate { host: host.into(), detail: other.to_string() }),
                None => Err(Error::Unreachable { host: host.into() }),
            }
        }
    }
}

pub async fn tls(host: &str, port: u16) -> Result<Tls> {
    let s = tcp(host, port).await?;
    tls_wrap(s, host).await
}

/// Legge una riga terminata da CRLF, un byte alla volta: prima del passaggio
/// a TLS non si deve leggere oltre la risposta del server.
pub async fn read_line(s: &mut TcpStream) -> Result<String> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = tokio::time::timeout(CONNECT_TIMEOUT, s.read(&mut b))
            .await
            .map_err(|_| Error::Timeout { host: String::new() })??;
        if n == 0 {
            return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "connessione chiusa dal server")));
        }
        buf.push(b[0]);
        if buf.ends_with(b"\r\n") || buf.len() > 8192 {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).trim_end().to_string())
}

pub async fn write_line(s: &mut TcpStream, line: &str) -> Result<()> {
    s.write_all(line.as_bytes()).await?;
    s.write_all(b"\r\n").await?;
    s.flush().await?;
    Ok(())
}

/// IMAP con STARTTLS: saluto del server, comando STARTTLS, poi cifratura.
pub async fn imap_starttls(host: &str, port: u16) -> Result<Tls> {
    let mut s = tcp(host, port).await?;
    let greet = read_line(&mut s).await?;
    if !greet.starts_with("* OK") && !greet.starts_with("* PREAUTH") {
        return Err(Error::Protocol(greet));
    }
    write_line(&mut s, "a0 STARTTLS").await?;
    loop {
        let line = read_line(&mut s).await?;
        if line.starts_with("a0 ") {
            if line.to_ascii_uppercase().starts_with("A0 OK") {
                break;
            }
            return Err(Error::user(format!("Il server {host} non accetta connessioni cifrate con STARTTLS sulla porta {port}.")));
        }
    }
    tls_wrap(s, host).await
}

/// POP3 con STARTTLS (comando STLS).
pub async fn pop3_starttls(host: &str, port: u16) -> Result<Tls> {
    let mut s = tcp(host, port).await?;
    let greet = read_line(&mut s).await?;
    if !greet.starts_with("+OK") {
        return Err(Error::Protocol(greet));
    }
    write_line(&mut s, "STLS").await?;
    let line = read_line(&mut s).await?;
    if !line.starts_with("+OK") {
        return Err(Error::user(format!("Il server {host} non accetta connessioni cifrate con STARTTLS sulla porta {port}.")));
    }
    tls_wrap(s, host).await
}

/// Esegue un'operazione di rete con un tempo massimo.
pub async fn with_timeout<T>(host: &str, d: Duration, f: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    match tokio::time::timeout(d, f).await {
        Ok(r) => r,
        Err(_) => Err(Error::Timeout { host: host.into() }),
    }
}
