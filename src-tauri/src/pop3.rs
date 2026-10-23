//! Account POP3: la posta viene scaricata sul computer, che ne conserva
//! l'unica copia completa. Sul server resta una copia per il tempo scelto.

use std::collections::{HashMap, HashSet};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::db::Db;
use crate::error::{Error, Result};
use crate::mime;
use crate::model::*;
use crate::net::{self, with_timeout, Tls, COMMAND_TIMEOUT};

pub struct Pop3 {
    s: BufReader<Tls>,
    host: String,
}

impl Pop3 {
    pub async fn connect(cfg: &ServerConfig, password: &str) -> Result<Pop3> {
        let (stream, greeted) = match cfg.security {
            Security::Ssl => (net::tls(&cfg.host, cfg.port).await?, false),
            Security::Starttls => (net::pop3_starttls(&cfg.host, cfg.port).await?, true),
        };
        let mut p = Pop3 { s: BufReader::new(stream), host: cfg.host.clone() };
        if !greeted {
            let g = p.read_line().await?;
            if !g.starts_with("+OK") {
                return Err(Error::Protocol(g));
            }
        }
        p.cmd(&format!("USER {}", cfg.username)).await.map_err(|_| Error::Auth)?;
        p.cmd(&format!("PASS {password}")).await.map_err(|e| match e {
            Error::Protocol(_) => Error::Auth,
            other => other,
        })?;
        Ok(p)
    }

    async fn read_line(&mut self) -> Result<String> {
        let host = self.host.clone();
        let mut buf = Vec::new();
        let n = with_timeout(&host, COMMAND_TIMEOUT, async { Ok(self.s.read_until(b'\n', &mut buf).await?) }).await?;
        if n == 0 {
            return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "connessione chiusa dal server")));
        }
        Ok(String::from_utf8_lossy(&buf).trim_end().to_string())
    }

    async fn cmd(&mut self, line: &str) -> Result<String> {
        let w = self.s.get_mut();
        w.write_all(line.as_bytes()).await?;
        w.write_all(b"\r\n").await?;
        w.flush().await?;
        let resp = self.read_line().await?;
        if let Some(rest) = resp.strip_prefix("+OK") {
            Ok(rest.trim().to_string())
        } else {
            Err(Error::Protocol(resp.trim_start_matches("-ERR").trim().to_string()))
        }
    }

    /// Risposta su più righe, terminata da una riga con un solo punto.
    async fn multiline(&mut self) -> Result<Vec<u8>> {
        let host = self.host.clone();
        let mut out = Vec::new();
        loop {
            let mut line = Vec::new();
            let n = with_timeout(&host, COMMAND_TIMEOUT, async { Ok(self.s.read_until(b'\n', &mut line).await?) }).await?;
            if n == 0 {
                return Err(Error::Io(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "connessione chiusa dal server")));
            }
            if line == b".\r\n" || line == b".\n" {
                break;
            }
            if line.starts_with(b"..") {
                out.extend_from_slice(&line[1..]);
            } else {
                out.extend_from_slice(&line);
            }
        }
        Ok(out)
    }

    pub async fn uidl(&mut self) -> Result<Vec<(u32, String)>> {
        self.cmd("UIDL").await.map_err(|_| Error::user("Il server POP non supporta UIDL: Better-Look non può sapere quali messaggi ha già scaricato."))?;
        let body = self.multiline().await?;
        Ok(String::from_utf8_lossy(&body)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                Some((it.next()?.parse().ok()?, it.next()?.to_string()))
            })
            .collect())
    }

    pub async fn retr(&mut self, n: u32) -> Result<Vec<u8>> {
        self.cmd(&format!("RETR {n}")).await?;
        self.multiline().await
    }

    pub async fn dele(&mut self, n: u32) -> Result<()> {
        self.cmd(&format!("DELE {n}")).await.map(|_| ())
    }

    pub async fn quit(mut self) -> Result<()> {
        self.cmd("QUIT").await.map(|_| ())
    }
}

/// Scarica i messaggi nuovi nella Posta in arrivo e applica la scelta sulla
/// copia da lasciare sul server. `progress` viene chiamata ogni tanto, per
/// mostrare i messaggi mentre arrivano.
pub async fn sync(db: &Db, acct: &AccountConfig, password: &str, progress: &(dyn Fn() + Send + Sync)) -> Result<bool> {
    let cfg = acct.incoming.as_ref().ok_or_else(|| Error::user("Mancano i dati del server POP."))?;
    let inbox = db.folder_by_role(&acct.id, "inbox")?.ok_or(Error::NotFound)?;
    let mut p = Pop3::connect(cfg, password).await?;
    let list = p.uidl().await?;
    let seen: HashMap<String, i64> = db.pop_seen(&acct.id)?.into_iter().collect();
    let mut changed = false;
    let mut count = 0;
    for (n, uidl) in &list {
        if seen.contains_key(uidl) {
            continue;
        }
        let raw = p.retr(*n).await?;
        let mut nm = match mime::parse(&raw) {
            Some(parsed) => parsed.to_new_message(now_secs(), true, raw.len() as i64),
            None => NewMessage::default(),
        };
        nm.pop_uidl = Some(uidl.clone());
        nm.unread = true;
        nm.raw = Some(raw);
        db.insert_message(&acct.id, inbox.id, &nm)?;
        db.pop_mark_seen(&acct.id, uidl)?;
        changed = true;
        count += 1;
        if count % 25 == 0 {
            progress();
        }
    }
    let now = now_secs();
    let downloaded: HashMap<String, i64> = db.pop_seen(&acct.id)?.into_iter().collect();
    let mut deleted = Vec::new();
    for (n, uidl) in &list {
        let Some(at) = downloaded.get(uidl) else { continue };
        let remove = match acct.sync.pop_keep {
            PopKeep::Never => true,
            PopKeep::Days14 => now - at >= 14 * 86_400,
            PopKeep::Always => false,
        };
        if remove {
            p.dele(*n).await?;
            deleted.push(uidl.clone());
        }
    }
    p.quit().await?;
    // Dimentica i messaggi che non sono più sul server.
    let on_server: HashSet<&String> = list.iter().map(|(_, u)| u).collect();
    for uidl in downloaded.keys() {
        if !on_server.contains(uidl) || deleted.contains(uidl) {
            db.pop_forget(&acct.id, uidl)?;
        }
    }
    Ok(changed)
}

use crate::db::NewMessage;
