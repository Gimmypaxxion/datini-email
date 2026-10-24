//! Account IMAP: connessione, elenco delle cartelle, sincronizzazione con
//! l'archivio locale e invio al server delle modifiche fatte sul computer.

use std::collections::HashSet;
use std::time::Duration;

use futures::StreamExt;

use crate::db::{Db, FolderRow, NewMessage, Op, Target};
use crate::error::{Error, Result};
use crate::mime;
use crate::model::*;
use crate::net::{self, with_timeout, Tls, COMMAND_TIMEOUT};
use crate::utf7;

pub type Session = async_imap::Session<Tls>;

const FETCH_TIMEOUT: Duration = Duration::from_secs(300);
const HEADER_BATCH: usize = 200;
const BODY_BATCH_BYTES: i64 = 8 * 1024 * 1024;

pub async fn login(cfg: &ServerConfig, password: &str) -> Result<Session> {
    let host = cfg.host.clone();
    let client = match cfg.security {
        Security::Ssl => {
            let tls = net::tls(&cfg.host, cfg.port).await?;
            let mut c = async_imap::Client::new(tls);
            let greet = with_timeout(&host, net::CONNECT_TIMEOUT, async { Ok(c.read_response().await?) }).await?;
            if greet.is_none() {
                return Err(Error::Protocol("il server ha chiuso la connessione".into()));
            }
            c
        }
        Security::Starttls => async_imap::Client::new(net::imap_starttls(&cfg.host, cfg.port).await?),
    };
    match tokio::time::timeout(net::CONNECT_TIMEOUT, client.login(&cfg.username, password)).await {
        Err(_) => Err(Error::Timeout { host }),
        Ok(Ok(s)) => Ok(s),
        Ok(Err((async_imap::error::Error::No(_), _))) | Ok(Err((async_imap::error::Error::Bad(_), _))) => Err(Error::Auth),
        Ok(Err((e, _))) => Err(e.into()),
    }
}

pub struct Caps {
    pub mv: bool,
    pub uidplus: bool,
}

pub async fn caps(s: &mut Session) -> Result<Caps> {
    let c = s.capabilities().await?;
    Ok(Caps { mv: c.has_str("MOVE"), uidplus: c.has_str("UIDPLUS") })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub path: String,
    pub name: String,
    pub delim: Option<String>,
    pub role: Option<&'static str>,
    pub selectable: bool,
}

/// Ruolo di una cartella dal nome, per i server senza SPECIAL-USE.
pub fn role_from_name(path: &str, delim: Option<&str>) -> Option<&'static str> {
    if path.eq_ignore_ascii_case("INBOX") {
        return Some("inbox");
    }
    let decoded = utf7::decode(path);
    let last = match delim {
        Some(d) if !d.is_empty() => decoded.rsplit(d).next().unwrap_or(&decoded).to_string(),
        _ => decoded.clone(),
    };
    let n = last.trim().to_lowercase();
    // Solo cartelle al primo livello, o dentro INBOX (server Courier e cPanel).
    let depth = delim.map(|d| decoded.matches(d).count()).unwrap_or(0);
    let under_inbox = delim.map(|d| decoded.to_ascii_uppercase().starts_with(&format!("INBOX{d}"))).unwrap_or(false);
    if depth > 1 || (depth == 1 && !under_inbox) {
        return None;
    }
    const SENT: [&str; 9] = ["sent", "sent items", "sent messages", "sent mail", "posta inviata", "inviata", "inviati", "elementi inviati", "inviate"];
    const DRAFTS: [&str; 3] = ["drafts", "draft", "bozze"];
    const TRASH: [&str; 9] = ["trash", "deleted", "deleted items", "deleted messages", "cestino", "posta eliminata", "elementi eliminati", "eliminati", "bin"];
    const JUNK: [&str; 6] = ["junk", "spam", "junk e-mail", "junk email", "posta indesiderata", "indesiderata"];
    const ARCHIVE: [&str; 3] = ["archive", "archives", "archivio"];
    if SENT.contains(&n.as_str()) {
        Some("sent")
    } else if DRAFTS.contains(&n.as_str()) {
        Some("drafts")
    } else if TRASH.contains(&n.as_str()) {
        Some("deleted")
    } else if JUNK.contains(&n.as_str()) {
        Some("junk")
    } else if ARCHIVE.contains(&n.as_str()) {
        Some("archive")
    } else {
        None
    }
}

pub fn display_name(path: &str, delim: Option<&str>) -> String {
    let decoded = utf7::decode(path);
    match delim {
        Some(d) if !d.is_empty() => decoded.rsplit(d).next().unwrap_or(&decoded).to_string(),
        _ => decoded,
    }
}

pub async fn list(s: &mut Session) -> Result<Vec<Listed>> {
    use async_imap::imap_proto::NameAttribute as A;
    let mut out = Vec::new();
    {
        let mut stream = s.list(Some(""), Some("*")).await?;
        while let Some(n) = stream.next().await {
            let n = n?;
            let attrs = n.attributes();
            let selectable = !attrs.iter().any(|a| matches!(a, A::NoSelect)) && !attrs.iter().any(|a| matches!(a, A::Extension(e) if e.eq_ignore_ascii_case("\\NonExistent")));
            let special = attrs.iter().find_map(|a| match a {
                A::Sent => Some("sent"),
                A::Drafts => Some("drafts"),
                A::Trash => Some("deleted"),
                A::Junk => Some("junk"),
                A::Archive => Some("archive"),
                _ => None,
            });
            let path = n.name().to_string();
            let delim = n.delimiter().map(|d| d.to_string());
            let role = if path.eq_ignore_ascii_case("INBOX") { Some("inbox") } else { special.or_else(|| role_from_name(&path, delim.as_deref())) };
            out.push(Listed { name: display_name(&path, delim.as_deref()), path, delim, role, selectable });
        }
    }
    // Un solo ruolo per cartella: se due cartelle reclamano lo stesso ruolo
    // vince quella indicata dal server con SPECIAL-USE, poi la prima.
    let mut seen = HashSet::new();
    for l in out.iter_mut() {
        if let Some(r) = l.role {
            if !seen.insert(r) {
                l.role = None;
            }
        }
    }
    Ok(out)
}

/// Crea sul server le cartelle che mancano (Posta inviata, Bozze, Cestino).
pub async fn ensure_special(s: &mut Session, listed: &[Listed]) -> Result<bool> {
    let delim = listed.iter().find_map(|l| l.delim.clone()).unwrap_or_else(|| "/".into());
    let others: Vec<&Listed> = listed.iter().filter(|l| !l.path.eq_ignore_ascii_case("INBOX")).collect();
    let prefix_inbox = !others.is_empty() && others.iter().all(|l| l.path.to_ascii_uppercase().starts_with(&format!("INBOX{delim}")));
    let mut created = false;
    for (role, name) in [("sent", "Sent"), ("drafts", "Drafts"), ("deleted", "Trash")] {
        if listed.iter().any(|l| l.role == Some(role)) {
            continue;
        }
        let path = if prefix_inbox { format!("INBOX{delim}{name}") } else { name.to_string() };
        if s.create(&path).await.is_ok() {
            let _ = s.subscribe(&path).await;
            created = true;
        }
    }
    Ok(created)
}

pub async fn create_folder(s: &mut Session, path: &str) -> Result<()> {
    s.create(path).await?;
    let _ = s.subscribe(path).await;
    Ok(())
}

/// Aggiorna le cartelle dell'archivio dall'elenco del server.
pub fn store_folders(db: &Db, account_id: &str, listed: &[Listed]) -> Result<Vec<i64>> {
    let mut ids = Vec::new();
    let existing = db.folders(account_id)?;
    for l in listed {
        let id = db.upsert_folder(account_id, &l.path, &l.name, l.role, l.delim.as_deref(), l.selectable)?;
        ids.push(id);
    }
    for f in existing {
        if !listed.iter().any(|l| l.path == f.path) {
            db.delete_folder(f.id)?;
        }
    }
    Ok(ids)
}

fn uid_set(uids: &[u32]) -> String {
    let mut v = uids.to_vec();
    v.sort_unstable();
    v.dedup();
    let mut parts = Vec::new();
    let mut i = 0;
    while i < v.len() {
        let start = v[i];
        let mut end = start;
        while i + 1 < v.len() && v[i + 1] == end + 1 {
            i += 1;
            end = v[i];
        }
        parts.push(if start == end { start.to_string() } else { format!("{start}:{end}") });
        i += 1;
    }
    parts.join(",")
}

fn flags_of(f: &async_imap::types::Fetch) -> (bool, bool, bool, bool) {
    use async_imap::types::Flag;
    let mut seen = false;
    let mut flagged = false;
    let mut answered = false;
    let mut draft = false;
    for fl in f.flags() {
        match fl {
            Flag::Seen => seen = true,
            Flag::Flagged => flagged = true,
            Flag::Answered => answered = true,
            Flag::Draft => draft = true,
            _ => {}
        }
    }
    (seen, flagged, answered, draft)
}

pub struct FolderSync {
    pub changed: bool,
}

/// Sincronizza una cartella: messaggi nuovi, cancellati, flag e testo completo
/// dei messaggi recenti. `thorough` controlla i flag di tutta la cartella.
pub async fn sync_folder(s: &mut Session, db: &Db, acct: &AccountConfig, folder: &FolderRow, thorough: bool) -> Result<FolderSync> {
    let host = acct.incoming.as_ref().map(|c| c.host.clone()).unwrap_or_default();
    let mut changed = false;

    // Controllo veloce: se nulla è cambiato dall'ultima volta si passa oltre.
    if !thorough && folder.uidvalidity.is_some() && folder.role.as_deref() != Some("inbox") {
        let st = with_timeout(&host, COMMAND_TIMEOUT, async { Ok(s.status(&folder.path, "(MESSAGES UIDNEXT UIDVALIDITY UNSEEN)").await?) }).await?;
        let (total, unread) = server_counts(db, folder.id)?;
        if st.uid_validity == folder.uidvalidity && st.uid_next == folder.uidnext && st.exists as i64 == total && st.unseen.map(|u| u as i64) == Some(unread) {
            return Ok(FolderSync { changed: false });
        }
    }

    let mb = with_timeout(&host, COMMAND_TIMEOUT, async { Ok(s.select(&folder.path).await?) }).await?;

    if folder.uidvalidity.is_some() && mb.uid_validity.is_some() && folder.uidvalidity != mb.uid_validity {
        let ids: Vec<i64> = db.folder_uids(folder.id)?.into_iter().map(|(id, _)| id).collect();
        db.delete_messages(&ids)?;
        changed = true;
    }

    let server_uids: HashSet<u32> = if mb.exists == 0 {
        HashSet::new()
    } else {
        with_timeout(&host, COMMAND_TIMEOUT, async { Ok(s.uid_search("ALL").await?) }).await?
    };
    let local = db.folder_uids(folder.id)?;
    let local_uids: HashSet<u32> = local.iter().map(|(_, u)| *u).collect();

    // Cancellati sul server (da un altro programma o dispositivo).
    let gone: Vec<i64> = local.iter().filter(|(_, u)| !server_uids.contains(u)).map(|(id, _)| *id).collect();
    if !gone.is_empty() {
        db.delete_messages(&gone)?;
        changed = true;
    }

    // Nuovi: intestazioni, dal più recente.
    let mut new: Vec<u32> = server_uids.iter().filter(|u| !local_uids.contains(u)).copied().collect();
    new.sort_unstable_by(|a, b| b.cmp(a));
    let is_inbox = folder.role.as_deref() == Some("inbox");
    let is_drafts = folder.role.as_deref() == Some("drafts");
    // Appena arrivati: nella posta in arrivo, oltre l'ultimo UID già visto.
    // Alla prima sincronizzazione nessun messaggio conta come arrivato.
    let arrivals_from = if is_inbox && folder.uidvalidity.is_some() && folder.uidvalidity == mb.uid_validity { folder.uidnext } else { None };
    for chunk in new.chunks(HEADER_BATCH) {
        let set = uid_set(chunk);
        let mut batch: Vec<NewMessage> = Vec::new();
        with_timeout(&host, FETCH_TIMEOUT, async {
            let mut stream = s.uid_fetch(&set, "(UID FLAGS INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER])").await?;
            while let Some(f) = stream.next().await {
                let f = f?;
                let Some(uid) = f.uid else { continue };
                let (seen, flagged, answered, draft) = flags_of(&f);
                let internal = f.internal_date().map(|d| d.timestamp()).unwrap_or_else(now_secs);
                let size = f.size.unwrap_or(0) as i64;
                let header = f.header().unwrap_or(b"");
                let mut nm = match mime::parse(header) {
                    Some(p) => p.to_new_message(internal, false, size),
                    None => NewMessage { date: internal, size, ..Default::default() },
                };
                nm.uid = Some(uid);
                nm.arrival = arrivals_from.is_some_and(|n| uid >= n);
                nm.unread = !seen;
                nm.flagged = flagged;
                nm.answered = answered;
                nm.draft = draft || is_drafts;
                if !is_inbox {
                    nm.focused = true;
                }
                batch.push(nm);
            }
            Ok(())
        })
        .await?;
        for nm in &batch {
            db.insert_message(&acct.id, folder.id, nm)?;
        }
        if !batch.is_empty() {
            changed = true;
        }
    }

    // Flag dei messaggi già presenti.
    let mut known: Vec<u32> = local_uids.intersection(&server_uids).copied().collect();
    known.sort_unstable();
    if !known.is_empty() {
        let range = if thorough || known.len() <= 2000 { "1:*".to_string() } else { format!("{}:*", known[known.len() - 2000]) };
        let mut flags = Vec::new();
        with_timeout(&host, FETCH_TIMEOUT, async {
            let mut stream = s.uid_fetch(&range, "(UID FLAGS)").await?;
            while let Some(f) = stream.next().await {
                let f = f?;
                if let Some(uid) = f.uid {
                    let (seen, flagged, answered, _) = flags_of(&f);
                    flags.push((uid, seen, flagged, answered));
                }
            }
            Ok(())
        })
        .await?;
        if db.apply_server_flags(folder.id, &flags)? > 0 {
            changed = true;
        }
    }

    // Testo completo dei messaggi da tenere sul computer.
    let since = acct.sync.keep.days().map(|d| now_secs() - d * 86_400);
    let max_size = match acct.sync.attachments {
        AttachPolicy::Now => None,
        AttachPolicy::OnOpen => Some(3 * 1024 * 1024),
    };
    loop {
        let missing = db.missing_bodies(folder.id, since, max_size, 400)?;
        if missing.is_empty() {
            break;
        }
        let mut progressed = false;
        let mut batch: Vec<u32> = Vec::new();
        let mut bytes = 0;
        for (_, uid, size) in &missing {
            batch.push(*uid);
            bytes += size;
            if bytes >= BODY_BATCH_BYTES || batch.len() >= 50 {
                progressed |= fetch_bodies(s, db, folder.id, &host, &batch).await? > 0;
                batch.clear();
                bytes = 0;
            }
        }
        if !batch.is_empty() {
            progressed |= fetch_bodies(s, db, folder.id, &host, &batch).await? > 0;
        }
        changed |= progressed;
        if !progressed || missing.len() < 400 {
            break;
        }
    }

    let highest = server_uids.iter().max().copied().unwrap_or(0);
    db.set_folder_state(folder.id, mb.uid_validity, highest, mb.uid_next)?;
    if db.drop_done_placeholders(folder.id)? > 0 {
        changed = true;
    }
    Ok(FolderSync { changed })
}

fn server_counts(db: &Db, folder_id: i64) -> Result<(i64, i64)> {
    let c = db.conn();
    Ok(c.query_row(
        "SELECT COUNT(*), COALESCE(SUM(unread), 0) FROM messages WHERE folder_id = ?1 AND uid IS NOT NULL",
        [folder_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?)
}

async fn fetch_bodies(s: &mut Session, db: &Db, folder_id: i64, host: &str, uids: &[u32]) -> Result<usize> {
    let set = uid_set(uids);
    let mut got: Vec<(u32, Vec<u8>)> = Vec::new();
    with_timeout(host, FETCH_TIMEOUT, async {
        let mut stream = s.uid_fetch(&set, "(UID BODY.PEEK[])").await?;
        while let Some(f) = stream.next().await {
            let f = f?;
            if let (Some(uid), Some(body)) = (f.uid, f.body()) {
                got.push((uid, body.to_vec()));
            }
        }
        Ok(())
    })
    .await?;
    let local = db.folder_uids(folder_id)?;
    let mut n = 0;
    for (uid, raw) in got {
        let Some((id, _)) = local.iter().find(|(_, u)| *u == uid) else { continue };
        store_body(db, *id, &raw)?;
        n += 1;
    }
    Ok(n)
}

pub fn store_body(db: &Db, id: i64, raw: &[u8]) -> Result<()> {
    match mime::parse(raw) {
        Some(p) => {
            let text: String = p.text().chars().take(100_000).collect();
            db.set_body(id, raw, &p.preview(), p.has_real_attachments(), &text)
        }
        None => db.set_body(id, raw, "", false, ""),
    }
}

/// Scarica subito un messaggio non ancora sul computer.
pub async fn fetch_one(s: &mut Session, host: &str, path: &str, uid: u32) -> Result<Option<Vec<u8>>> {
    with_timeout(host, COMMAND_TIMEOUT, async { Ok(s.select(path).await?) }).await?;
    let mut out = None;
    with_timeout(host, FETCH_TIMEOUT, async {
        let mut stream = s.uid_fetch(uid.to_string(), "(UID BODY.PEEK[])").await?;
        while let Some(f) = stream.next().await {
            let f = f?;
            if f.uid == Some(uid) {
                out = f.body().map(|b| b.to_vec());
            }
        }
        Ok(())
    })
    .await?;
    Ok(out)
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

async fn resolve(s: &mut Session, targets: &[Target]) -> Result<Vec<u32>> {
    let mut uids: Vec<u32> = targets.iter().filter_map(|t| t.uid).collect();
    for t in targets.iter().filter(|t| t.uid.is_none()) {
        if let Some(mid) = &t.message_id {
            let found = s.uid_search(format!("HEADER Message-ID {}", quote(mid))).await?;
            uids.extend(found);
        }
    }
    uids.sort_unstable();
    uids.dedup();
    Ok(uids)
}

async fn store_flag(s: &mut Session, set: &str, add: bool, flag: &str) -> Result<()> {
    let q = format!("{}FLAGS.SILENT ({flag})", if add { "+" } else { "-" });
    let mut stream = s.uid_store(set, q).await?;
    while let Some(r) = stream.next().await {
        r?;
    }
    Ok(())
}

async fn expunge(s: &mut Session, set: &str, caps: &Caps) -> Result<()> {
    if caps.uidplus {
        let mut st = std::pin::pin!(s.uid_expunge(set).await?);
        while let Some(r) = st.next().await {
            r?;
        }
    } else {
        let mut st = std::pin::pin!(s.expunge().await?);
        while let Some(r) = st.next().await {
            r?;
        }
    }
    Ok(())
}

/// Esegue sul server un'operazione fatta sul computer.
pub async fn exec_op(s: &mut Session, db: &Db, caps: &Caps, op: &Op) -> Result<()> {
    match op {
        Op::Flags { folder, targets, seen, flagged, answered, local_ids } => {
            s.select(folder).await?;
            let uids = resolve(s, targets).await?;
            if !uids.is_empty() {
                let set = uid_set(&uids);
                if let Some(v) = seen {
                    store_flag(s, &set, *v, "\\Seen").await?;
                }
                if let Some(v) = flagged {
                    store_flag(s, &set, *v, "\\Flagged").await?;
                }
                if let Some(v) = answered {
                    store_flag(s, &set, *v, "\\Answered").await?;
                }
            }
            db.clear_dirty(local_ids)?;
        }
        Op::Move { folder, dest, targets, local_ids } => {
            s.select(folder).await?;
            let uids = resolve(s, targets).await?;
            if !uids.is_empty() {
                let set = uid_set(&uids);
                if caps.mv {
                    s.uid_mv(&set, dest).await?;
                } else {
                    s.uid_copy(&set, dest).await?;
                    store_flag(s, &set, true, "\\Deleted").await?;
                    expunge(s, &set, caps).await?;
                }
            }
            // Le copie locali aspettano di essere riconosciute nella cartella di destinazione.
            let c = db.conn();
            for id in local_ids {
                c.execute("UPDATE messages SET pending = 'done' WHERE id = ?1 AND uid IS NULL AND pending = 'move'", [id])?;
            }
        }
        Op::Expunge { folder, targets } => {
            s.select(folder).await?;
            let uids = resolve(s, targets).await?;
            if !uids.is_empty() {
                let set = uid_set(&uids);
                store_flag(s, &set, true, "\\Deleted").await?;
                expunge(s, &set, caps).await?;
            }
        }
        Op::Append { folder, local_id, flags } => {
            let Some(raw) = db.body(*local_id)? else { return Ok(()) };
            s.append(folder, Some(flags), None, &raw).await?;
            let c = db.conn();
            c.execute("UPDATE messages SET pending = 'done' WHERE id = ?1 AND uid IS NULL", [local_id])?;
        }
        Op::CreateFolder { path } => {
            if let Err(e) = s.create(path).await {
                let e: Error = e.into();
                if !e.to_string().to_ascii_lowercase().contains("exist") {
                    return Err(e);
                }
            }
            let _ = s.subscribe(path).await;
        }
        Op::RenameFolder { from, to } => {
            if let Err(e) = s.rename(from, to).await {
                let e: Error = e.into();
                let low = e.to_string().to_ascii_lowercase();
                // Già rinominata (per esempio da un tentativo precedente).
                if !(low.contains("exist") || low.contains("nonexistent")) {
                    return Err(e);
                }
            }
            let _ = s.unsubscribe(from).await;
            let _ = s.subscribe(to).await;
        }
        Op::DeleteFolder { path } => {
            let _ = s.unsubscribe(path).await;
            if let Err(e) = s.delete(path).await {
                let e: Error = e.into();
                if !e.to_string().to_ascii_lowercase().contains("exist") {
                    return Err(e);
                }
            }
        }
        Op::MarkAllRead { folder, local_ids } => {
            let mb = s.select(folder).await?;
            if mb.exists > 0 {
                store_flag(s, "1:*", true, "\\Seen").await?;
            }
            db.clear_dirty(local_ids)?;
        }
        Op::DeleteDraftVersions { folder, draft_key, keep_message_id } => {
            s.select(folder).await?;
            let mut q = format!("HEADER {} {}", mime::DRAFT_HEADER, quote(draft_key));
            if let Some(mid) = keep_message_id {
                q.push_str(&format!(" NOT HEADER Message-ID {}", quote(mid)));
            }
            let uids: Vec<u32> = s.uid_search(q).await?.into_iter().collect();
            if !uids.is_empty() {
                let set = uid_set(&uids);
                store_flag(s, &set, true, "\\Deleted").await?;
                expunge(s, &set, caps).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_from_names() {
        assert_eq!(role_from_name("INBOX", Some("/")), Some("inbox"));
        assert_eq!(role_from_name("INBOX.Sent", Some(".")), Some("sent"));
        assert_eq!(role_from_name("Posta inviata", Some("/")), Some("sent"));
        assert_eq!(role_from_name("Bozze", Some("/")), Some("drafts"));
        assert_eq!(role_from_name("Cestino", Some("/")), Some("deleted"));
        assert_eq!(role_from_name("Posta indesiderata", Some("/")), Some("junk"));
        assert_eq!(role_from_name("Clienti/Sent", Some("/")), None);
        assert_eq!(role_from_name("Clienti", Some("/")), None);
    }

    #[test]
    fn compact_uid_sets() {
        assert_eq!(uid_set(&[5, 1, 2, 3, 9, 10, 2]), "1:3,5,9:10");
        assert_eq!(uid_set(&[7]), "7");
    }
}
