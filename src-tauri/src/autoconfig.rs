//! Configurazione automatica: dall'indirizzo email ai server da usare.
//! Ordine: provider noti, configurazione pubblicata dal dominio, archivio di
//! Thunderbird (ISPDB), record MX del dominio, nomi più comuni.

use std::time::Duration;

use crate::model::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub kind: AccountKind,
    pub incoming: ServerConfig,
    pub outgoing: ServerConfig,
    pub source: &'static str,
}

/// Provider per cui l'accesso con password non funziona più.
pub fn unsupported(domain: &str) -> Option<&'static str> {
    const MS: [&str; 8] = ["outlook.com", "outlook.it", "hotmail.com", "hotmail.it", "live.com", "live.it", "msn.com", "office365.com"];
    if MS.contains(&domain) {
        return Some("Gli account Microsoft (Outlook.com, Hotmail, Microsoft 365) richiedono l'accesso con Microsoft, che arriverà in una prossima versione di Better-Look.");
    }
    None
}

fn srv(host: &str, port: u16, security: Security, user: &str) -> ServerConfig {
    ServerConfig { host: host.into(), port, security, username: user.into() }
}

/// Provider italiani e internazionali più diffusi.
fn known(domain: &str, email: &str) -> Option<(ServerConfig, ServerConfig, Option<ServerConfig>)> {
    use Security::*;
    let d = domain;
    let r = match d {
        "gmail.com" | "googlemail.com" => (srv("imap.gmail.com", 993, Ssl, email), srv("smtp.gmail.com", 465, Ssl, email), Some(srv("pop.gmail.com", 995, Ssl, email))),
        "libero.it" | "inwind.it" | "iol.it" | "blu.it" | "giallo.it" => {
            (srv("imapmail.libero.it", 993, Ssl, email), srv("smtp.libero.it", 465, Ssl, email), Some(srv("popmail.libero.it", 995, Ssl, email)))
        }
        "virgilio.it" => (srv("in.virgilio.it", 993, Ssl, email), srv("out.virgilio.it", 465, Ssl, email), Some(srv("in.virgilio.it", 995, Ssl, email))),
        "tiscali.it" => (srv("imap.tiscali.it", 993, Ssl, email), srv("smtp.tiscali.it", 465, Ssl, email), Some(srv("pop.tiscali.it", 995, Ssl, email))),
        "aruba.it" => (srv("imaps.aruba.it", 993, Ssl, email), srv("smtps.aruba.it", 465, Ssl, email), Some(srv("pop3s.aruba.it", 995, Ssl, email))),
        "pec.it" => (srv("imaps.pec.aruba.it", 993, Ssl, email), srv("smtps.pec.aruba.it", 465, Ssl, email), Some(srv("pop3s.pec.aruba.it", 995, Ssl, email))),
        "yahoo.com" | "yahoo.it" => (srv("imap.mail.yahoo.com", 993, Ssl, email), srv("smtp.mail.yahoo.com", 465, Ssl, email), Some(srv("pop.mail.yahoo.com", 995, Ssl, email))),
        "icloud.com" | "me.com" | "mac.com" => (srv("imap.mail.me.com", 993, Ssl, email), srv("smtp.mail.me.com", 587, Starttls, email), None),
        _ => return None,
    };
    Some(r)
}

/// Domini di posta ospitati da un provider noto, riconosciuti dal record MX.
fn known_by_mx(mx: &str, email: &str) -> Option<(ServerConfig, ServerConfig, Option<ServerConfig>)> {
    let mx = mx.trim_end_matches('.').to_ascii_lowercase();
    if mx.ends_with(".aruba.it") || mx == "aruba.it" {
        return known("aruba.it", email);
    }
    if mx.ends_with(".google.com") || mx.ends_with(".googlemail.com") {
        return known("gmail.com", email);
    }
    None
}

fn expand(template: &str, email: &str) -> String {
    let (local, domain) = email.split_once('@').unwrap_or((email, ""));
    template.replace("%EMAILADDRESS%", email).replace("%EMAILLOCALPART%", local).replace("%EMAILDOMAIN%", domain)
}

/// Legge il formato di configurazione di Thunderbird (config-v1.1.xml).
pub fn parse_autoconfig(xml: &str, email: &str) -> Vec<Candidate> {
    let Ok(doc) = roxmltree::Document::parse(xml) else { return Vec::new() };
    let read = |n: roxmltree::Node| -> Option<ServerConfig> {
        let text = |tag: &str| n.children().find(|c| c.has_tag_name(tag)).and_then(|c| c.text()).map(|t| t.trim().to_string());
        let host = text("hostname")?;
        let port: u16 = text("port")?.parse().ok()?;
        let security = match text("socketType")?.to_ascii_uppercase().as_str() {
            "SSL" => Security::Ssl,
            "STARTTLS" => Security::Starttls,
            _ => return None,
        };
        let user = expand(&text("username").unwrap_or_else(|| "%EMAILADDRESS%".into()), email);
        Some(ServerConfig { host: expand(&host, email), port, security, username: user })
    };
    let mut imap = Vec::new();
    let mut pop = Vec::new();
    let mut smtp = Vec::new();
    for n in doc.descendants() {
        if n.has_tag_name("incomingServer") {
            match n.attribute("type") {
                Some("imap") => imap.extend(read(n)),
                Some("pop3") => pop.extend(read(n)),
                _ => {}
            }
        } else if n.has_tag_name("outgoingServer") && n.attribute("type") == Some("smtp") {
            smtp.extend(read(n));
        }
    }
    let Some(out) = smtp.into_iter().next() else { return Vec::new() };
    let mut res = Vec::new();
    for i in imap {
        res.push(Candidate { kind: AccountKind::Imap, incoming: i, outgoing: out.clone(), source: "autoconfig" });
    }
    for p in pop {
        res.push(Candidate { kind: AccountKind::Pop3, incoming: p, outgoing: out.clone(), source: "autoconfig" });
    }
    res
}

async fn fetch(url: &str) -> Option<String> {
    crate::net::install_crypto();
    let client = reqwest::Client::builder().timeout(Duration::from_secs(8)).build().ok()?;
    let r = client.get(url).send().await.ok()?;
    if !r.status().is_success() {
        return None;
    }
    let text = r.text().await.ok()?;
    if text.contains("clientConfig") {
        Some(text)
    } else {
        None
    }
}

async fn mx(domain: &str) -> Vec<String> {
    let Ok(b) = hickory_resolver::TokioResolver::builder_tokio() else { return Vec::new() };
    let resolver = b.build();
    match tokio::time::timeout(Duration::from_secs(6), resolver.mx_lookup(domain)).await {
        Ok(Ok(l)) => {
            let mut v: Vec<(u16, String)> = l.iter().map(|m| (m.preference(), m.exchange().to_utf8())).collect();
            v.sort();
            v.into_iter().map(|(_, h)| h.trim_end_matches('.').to_string()).collect()
        }
        _ => Vec::new(),
    }
}

fn base_domain(host: &str) -> String {
    let parts: Vec<&str> = host.trim_end_matches('.').split('.').collect();
    if parts.len() >= 2 {
        format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        host.to_string()
    }
}

fn push_known(out: &mut Vec<Candidate>, k: (ServerConfig, ServerConfig, Option<ServerConfig>), source: &'static str) {
    out.push(Candidate { kind: AccountKind::Imap, incoming: k.0, outgoing: k.1.clone(), source });
    if let Some(p) = k.2 {
        out.push(Candidate { kind: AccountKind::Pop3, incoming: p, outgoing: k.1, source });
    }
}

/// Elenco ordinato di configurazioni possibili. Vanno provate una alla volta.
pub async fn candidates(email: &str) -> Vec<Candidate> {
    let email = email.trim();
    let domain = email.rsplit('@').next().unwrap_or("").to_ascii_lowercase();
    let mut out = Vec::new();
    if let Some(k) = known(&domain, email) {
        push_known(&mut out, k, "provider");
    }
    let urls = [
        format!("https://autoconfig.{domain}/mail/config-v1.1.xml?emailaddress={email}"),
        format!("https://{domain}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={email}"),
        format!("https://autoconfig.thunderbird.net/v1.1/{domain}"),
    ];
    for u in &urls {
        if let Some(xml) = fetch(u).await {
            out.extend(parse_autoconfig(&xml, email));
            break;
        }
    }
    let mxs = mx(&domain).await;
    if let Some(first) = mxs.first() {
        if let Some(k) = known_by_mx(first, email) {
            push_known(&mut out, k, "mx");
        } else {
            let base = base_domain(first);
            if base != domain {
                if let Some(xml) = fetch(&format!("https://autoconfig.thunderbird.net/v1.1/{base}")).await {
                    out.extend(parse_autoconfig(&xml, email));
                }
            }
        }
    }
    use Security::*;
    for host in [format!("imap.{domain}"), format!("mail.{domain}")] {
        let smtp_host = if host.starts_with("mail.") { host.clone() } else { format!("smtp.{domain}") };
        out.push(Candidate { kind: AccountKind::Imap, incoming: srv(&host, 993, Ssl, email), outgoing: srv(&smtp_host, 465, Ssl, email), source: "nome" });
        out.push(Candidate { kind: AccountKind::Imap, incoming: srv(&host, 993, Ssl, email), outgoing: srv(&smtp_host, 587, Starttls, email), source: "nome" });
    }
    for host in [format!("pop.{domain}"), format!("mail.{domain}")] {
        let smtp_host = if host.starts_with("mail.") { host.clone() } else { format!("smtp.{domain}") };
        out.push(Candidate { kind: AccountKind::Pop3, incoming: srv(&host, 995, Ssl, email), outgoing: srv(&smtp_host, 465, Ssl, email), source: "nome" });
    }
    let mut seen = Vec::new();
    out.retain(|c| {
        let k = (c.kind, c.incoming.clone(), c.outgoing.clone());
        if seen.contains(&k) {
            false
        } else {
            seen.push(k);
            true
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_thunderbird_format() {
        let xml = r#"<?xml version="1.0"?>
<clientConfig version="1.1"><emailProvider id="returnio.eu">
<incomingServer type="imap"><hostname>mail.%EMAILDOMAIN%</hostname><port>993</port><socketType>SSL</socketType><username>%EMAILADDRESS%</username></incomingServer>
<incomingServer type="pop3"><hostname>mail.returnio.eu</hostname><port>995</port><socketType>SSL</socketType><username>%EMAILADDRESS%</username></incomingServer>
<outgoingServer type="smtp"><hostname>mail.returnio.eu</hostname><port>465</port><socketType>SSL</socketType><username>%EMAILADDRESS%</username></outgoingServer>
</emailProvider></clientConfig>"#;
        let c = parse_autoconfig(xml, "info@returnio.eu");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].kind, AccountKind::Imap);
        assert_eq!(c[0].incoming.host, "mail.returnio.eu");
        assert_eq!(c[0].incoming.username, "info@returnio.eu");
        assert_eq!(c[0].outgoing.port, 465);
        assert_eq!(c[1].kind, AccountKind::Pop3);
    }

    #[test]
    fn known_and_unsupported_providers() {
        assert!(unsupported("hotmail.it").is_some());
        let k = known("libero.it", "a@libero.it").unwrap();
        assert_eq!(k.0.host, "imapmail.libero.it");
        assert_eq!(known_by_mx("mx.aruba.it.", "a@azienda.it").unwrap().0.host, "imaps.aruba.it");
    }
}
