//! Comandi chiamati dall'interfaccia (invoke). I nomi dei parametri arrivano
//! da JavaScript in camelCase.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager, State, Webview, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::engine::{Engine, InviteAction, NewAccount, SavedDraft};
use crate::error::{Error, Result};
use crate::model::*;

type E<'a> = State<'a, Arc<Engine>>;

/// Esce dal programma (Cmd+Q sul Mac, anche quando il tasto arriva alla pagina).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn accounts(e: E<'_>) -> Result<Vec<AccountInfo>> {
    e.accounts()
}

#[tauri::command]
pub async fn add_account(e: E<'_>, input: NewAccount) -> Result<AccountInfo> {
    e.inner().clone().add_account(input).await
}

#[tauri::command]
pub async fn update_account(e: E<'_>, config: AccountConfig) -> Result<()> {
    e.update_account(config)
}

#[tauri::command]
pub async fn set_password(e: E<'_>, id: String, password: String) -> Result<()> {
    e.set_password(&id, &password)
}

#[tauri::command]
pub async fn remove_account(e: E<'_>, id: String) -> Result<()> {
    e.remove_account(&id)
}

#[tauri::command]
pub async fn folders(e: E<'_>) -> Result<Vec<FolderInfo>> {
    e.folders()
}

#[tauri::command]
pub async fn messages(e: E<'_>, folder_id: i64, tab: Option<String>, offset: Option<i64>, limit: Option<i64>) -> Result<Vec<MessageSummary>> {
    e.messages(folder_id, tab.as_deref(), offset.unwrap_or(0), limit.unwrap_or(500))
}

#[tauri::command]
pub async fn search(e: E<'_>, query: String, account_id: Option<String>) -> Result<Vec<MessageSummary>> {
    e.search(&query, account_id.as_deref())
}

#[tauri::command]
pub async fn message(e: E<'_>, id: i64, allow_remote: bool) -> Result<MessageDetail> {
    e.detail(id, allow_remote).await
}

#[tauri::command]
pub async fn quote_html(e: E<'_>, id: i64) -> Result<String> {
    e.quote_html(id).await
}

#[tauri::command]
pub async fn signatures(e: E<'_>) -> Result<Vec<Signature>> {
    e.signatures()
}

#[tauri::command]
pub async fn save_signature(e: E<'_>, signature: Signature) -> Result<Signature> {
    e.save_signature(signature)
}

#[tauri::command]
pub async fn delete_signature(e: E<'_>, id: String) -> Result<()> {
    e.delete_signature(&id)
}

#[tauri::command]
pub async fn fetch_image(e: E<'_>, url: String) -> Result<Option<String>> {
    e.fetch_image(&url).await
}

#[tauri::command]
pub async fn set_flags(e: E<'_>, ids: Vec<i64>, seen: Option<bool>, flagged: Option<bool>) -> Result<()> {
    e.set_flags(&ids, seen, flagged, None)
}

#[tauri::command]
pub async fn move_messages(e: E<'_>, ids: Vec<i64>, folder_id: i64) -> Result<()> {
    e.move_messages(&ids, folder_id).await
}

#[tauri::command]
pub async fn move_to_role(e: E<'_>, ids: Vec<i64>, role: String) -> Result<()> {
    e.move_to_role(&ids, &role).await
}

#[tauri::command]
pub async fn delete_messages(e: E<'_>, ids: Vec<i64>) -> Result<()> {
    e.delete_messages(&ids).await
}

#[tauri::command]
pub async fn categories(e: E<'_>) -> Result<Vec<Category>> {
    e.categories()
}

#[tauri::command]
pub async fn save_categories(e: E<'_>, list: Vec<Category>) -> Result<Vec<Category>> {
    e.save_categories(list)
}

#[tauri::command]
pub async fn set_category(e: E<'_>, ids: Vec<i64>, category: String, on: bool) -> Result<()> {
    e.set_category(&ids, &category, on)
}

#[tauri::command]
pub async fn clear_categories(e: E<'_>, ids: Vec<i64>) -> Result<()> {
    e.clear_categories(&ids)
}

#[tauri::command]
pub async fn thread(e: E<'_>, account_id: String, thread: String) -> Result<Vec<MessageSummary>> {
    e.thread(&account_id, &thread)
}

#[tauri::command]
pub async fn create_folder(e: E<'_>, account_id: String, parent_id: Option<i64>, name: String) -> Result<i64> {
    e.create_folder(&account_id, parent_id, &name)
}

#[tauri::command]
pub async fn rename_folder(e: E<'_>, id: i64, name: String) -> Result<()> {
    e.rename_folder(id, &name)
}

#[tauri::command]
pub async fn delete_folder(e: E<'_>, id: i64) -> Result<()> {
    e.delete_folder(id).await
}

#[tauri::command]
pub async fn mark_folder_read(e: E<'_>, id: i64) -> Result<()> {
    e.mark_folder_read(id)
}

#[tauri::command]
pub async fn empty_folder(e: E<'_>, id: i64) -> Result<usize> {
    e.empty_folder(id)
}

#[tauri::command]
pub async fn save_draft(e: E<'_>, draft: Draft) -> Result<SavedDraft> {
    e.save_draft(&draft).await
}

#[tauri::command]
pub async fn close_draft(e: E<'_>, draft: Draft) -> Result<Option<SavedDraft>> {
    e.close_draft(&draft).await
}

#[tauri::command]
pub async fn discard_draft(e: E<'_>, account_id: String, key: String) -> Result<()> {
    e.discard_draft(&account_id, &key)
}

#[tauri::command]
pub async fn open_draft(e: E<'_>, id: i64) -> Result<Draft> {
    e.open_draft(id).await
}

#[tauri::command]
pub async fn send(e: E<'_>, draft: Draft) -> Result<i64> {
    e.send(&draft).await
}

#[tauri::command]
pub async fn send_later(e: E<'_>, draft: Draft, at: i64) -> Result<i64> {
    e.send_later(&draft, at).await
}

#[tauri::command]
pub async fn mailbox_usage(e: E<'_>, account_id: String) -> Result<Vec<FolderUsage>> {
    e.mailbox_usage(&account_id)
}

#[tauri::command]
pub async fn large_messages(e: E<'_>, account_id: String, min_size: i64) -> Result<Vec<MessageSummary>> {
    e.large_messages(&account_id, min_size)
}

#[tauri::command]
pub async fn older_messages(e: E<'_>, folder_id: i64, before: i64, archive: bool) -> Result<usize> {
    e.older_messages(folder_id, before, archive).await
}

#[tauri::command]
pub async fn address_book(e: E<'_>) -> Result<Vec<Contact>> {
    e.address_book()
}

#[tauri::command]
pub async fn save_contact(e: E<'_>, contact: Contact) -> Result<Contact> {
    e.save_contact(contact)
}

#[tauri::command]
pub async fn delete_contacts(e: E<'_>, ids: Vec<String>) -> Result<()> {
    e.delete_contacts(&ids)
}

#[tauri::command]
pub async fn import_contacts(e: E<'_>, path: PathBuf) -> Result<ContactImport> {
    let eng = e.inner().clone();
    tauri::async_runtime::spawn_blocking(move || eng.import_contacts(&path)).await.map_err(|err| Error::user(err.to_string()))?
}

#[tauri::command]
pub async fn export_contacts(e: E<'_>, path: PathBuf, ids: Option<Vec<String>>) -> Result<usize> {
    e.export_contacts(&path, ids.as_deref())
}

#[tauri::command]
pub async fn ignore_thread(e: E<'_>, id: i64, on: bool) -> Result<usize> {
    e.ignore_thread(id, on).await
}

#[tauri::command]
pub async fn thread_ignored(e: E<'_>, id: i64) -> Result<bool> {
    e.thread_ignored(id)
}

#[tauri::command]
pub async fn clean_up(e: E<'_>, id: Option<i64>, folder_id: Option<i64>, subfolders: bool) -> Result<usize> {
    e.clean_up(id, folder_id, subfolders).await
}

#[tauri::command]
pub async fn quick_steps(e: E<'_>) -> Result<Vec<QuickStep>> {
    e.quick_steps()
}

#[tauri::command]
pub async fn save_quick_steps(e: E<'_>, list: Vec<QuickStep>) -> Result<Vec<QuickStep>> {
    e.save_quick_steps(list)
}

#[tauri::command]
pub async fn templates(e: E<'_>) -> Result<Vec<MessageTemplate>> {
    e.templates()
}

#[tauri::command]
pub async fn save_template(e: E<'_>, template: MessageTemplate) -> Result<MessageTemplate> {
    e.save_template(template)
}

#[tauri::command]
pub async fn delete_template(e: E<'_>, id: String) -> Result<()> {
    e.delete_template(&id)
}

#[tauri::command]
pub async fn task_lists(e: E<'_>) -> Result<Vec<TaskList>> {
    e.task_lists()
}

#[tauri::command]
pub async fn save_task_list(e: E<'_>, list: TaskList) -> Result<TaskList> {
    e.save_task_list(list)
}

#[tauri::command]
pub async fn delete_task_list(e: E<'_>, id: String) -> Result<usize> {
    e.delete_task_list(&id)
}

#[tauri::command]
pub async fn tasks(e: E<'_>) -> Result<Vec<Task>> {
    e.tasks()
}

/// Restituisce l'attività salvata e quella nuova di una ripetizione.
#[tauri::command]
pub async fn save_task(e: E<'_>, task: Task) -> Result<(Task, Option<Task>)> {
    e.save_task(task)
}

#[tauri::command]
pub async fn delete_task(e: E<'_>, id: String) -> Result<()> {
    e.delete_task(&id)
}

#[tauri::command]
pub async fn flagged_messages(e: E<'_>) -> Result<Vec<MessageSummary>> {
    e.flagged_messages()
}

#[tauri::command]
pub async fn events(e: E<'_>, from: i64, to: i64) -> Result<Vec<CalEvent>> {
    e.events(from, to)
}

#[tauri::command]
pub async fn save_event(e: E<'_>, event: CalEvent, send: bool) -> Result<CalEvent> {
    e.save_event(event, send)
}

#[tauri::command]
pub async fn delete_event(e: E<'_>, id: String, occurrence: Option<i64>, send: bool) -> Result<()> {
    e.delete_event(&id, occurrence, send)
}

#[tauri::command]
pub async fn respond_invite(e: E<'_>, id: i64, action: InviteAction, send: bool) -> Result<Option<CalEvent>> {
    e.respond_invite(id, action, send).await
}

#[tauri::command]
pub async fn respond_event(e: E<'_>, id: String, action: InviteAction, send: bool) -> Result<Option<CalEvent>> {
    e.respond_event(&id, action, send)
}

#[tauri::command]
pub async fn import_calendar(e: E<'_>, path: PathBuf) -> Result<CalImport> {
    let eng = e.inner().clone();
    tauri::async_runtime::spawn_blocking(move || eng.import_calendar(&path)).await.map_err(|err| Error::user(err.to_string()))?
}

#[tauri::command]
pub async fn export_calendar(e: E<'_>, path: PathBuf) -> Result<usize> {
    e.export_calendar(&path)
}

#[tauri::command]
pub async fn rules(e: E<'_>) -> Result<Vec<Rule>> {
    e.rules()
}

#[tauri::command]
pub async fn save_rules(e: E<'_>, list: Vec<Rule>) -> Result<Vec<Rule>> {
    e.save_rules(list)
}

#[tauri::command]
pub async fn run_rules(e: E<'_>, ids: Vec<String>, folder_id: i64) -> Result<usize> {
    e.run_rules(&ids, folder_id).await
}

#[tauri::command]
pub async fn set_auto_reply(e: E<'_>, account_id: String, reply: AutoReply) -> Result<()> {
    e.set_auto_reply(&account_id, reply)
}

#[tauri::command]
pub async fn cancel_send(e: E<'_>, id: i64) -> Result<Option<Draft>> {
    e.cancel_send(id)
}

#[tauri::command]
pub async fn outbox(e: E<'_>) -> Result<Vec<OutboxItem>> {
    e.outbox()
}

#[tauri::command]
pub async fn outbox_take(e: E<'_>, id: i64) -> Result<Option<Draft>> {
    e.outbox_take(id)
}

#[tauri::command]
pub async fn sync_now(e: E<'_>, account_id: Option<String>) -> Result<()> {
    e.sync_now(account_id.as_deref());
    Ok(())
}

#[tauri::command]
pub async fn contacts(e: E<'_>, prefix: String) -> Result<Vec<Person>> {
    e.contacts(&prefix)
}

/// Estensioni che possono eseguire codice: Better-Look non le apre, come Outlook.
const BLOCKED: [&str; 28] = [
    "exe", "bat", "cmd", "com", "scr", "pif", "vbs", "vbe", "js", "jse", "wsf", "wsh", "msi", "msp", "ps1", "psm1", "lnk", "reg", "hta",
    "cpl", "jar", "app", "command", "sh", "dll", "inf", "url", "iso",
];

fn safe_name(name: &str) -> String {
    let n: String = name.chars().map(|c| if "/\\:*?\"<>|".contains(c) || c.is_control() { '_' } else { c }).collect();
    let n = n.trim().trim_matches('.').to_string();
    if n.is_empty() {
        "allegato".into()
    } else {
        n
    }
}

#[tauri::command]
pub async fn open_attachment(app: AppHandle, e: E<'_>, id: i64, index: u32) -> Result<()> {
    let (name, _, bytes) = e.attachment(id, index).await?;
    let name = safe_name(&name);
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if BLOCKED.contains(&ext.as_str()) {
        return Err(Error::user(format!(
            "Per sicurezza Better-Look non apre i file .{ext}: potrebbero contenere programmi dannosi. Se sei sicuro della provenienza, salvalo e controllalo con l'antivirus."
        )));
    }
    let dir = app.path().app_cache_dir().map_err(|e| Error::user(e.to_string()))?.join("allegati").join(id.to_string());
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(&name);
    tokio::fs::write(&path, bytes).await?;
    app.opener().open_path(path.to_string_lossy().to_string(), None::<&str>).map_err(|e| Error::user(format!("Impossibile aprire il file: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn save_attachment(e: E<'_>, id: i64, index: u32, path: PathBuf) -> Result<()> {
    let (_, _, bytes) = e.attachment(id, index).await?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

#[tauri::command]
pub async fn export_eml(e: E<'_>, id: i64, path: PathBuf) -> Result<()> {
    e.export_eml(id, &path).await
}

#[tauri::command]
pub async fn export_mbox(e: E<'_>, folder_id: i64, path: PathBuf) -> Result<usize> {
    e.export_mbox(folder_id, &path).await
}

#[tauri::command]
pub async fn open_link(app: AppHandle, url: String) -> Result<()> {
    let low = url.trim().to_ascii_lowercase();
    if !(low.starts_with("https://") || low.starts_with("http://")) {
        return Err(Error::user("Questo collegamento non si può aprire."));
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| Error::user(format!("Impossibile aprire il collegamento: {e}")))
}

// ----------------------------------------------------------------------
// Finestre separate per scrivere i messaggi
// ----------------------------------------------------------------------

#[derive(Default)]
pub struct ComposeWindows {
    drafts: Mutex<HashMap<String, Draft>>,
    next: AtomicU32,
}

#[tauri::command]
pub async fn open_compose_window(app: AppHandle, w: State<'_, ComposeWindows>, draft: Draft) -> Result<()> {
    let n = w.next.fetch_add(1, Ordering::Relaxed) + 1;
    let label = format!("messaggio-{n}");
    let title = if draft.subject.trim().is_empty() { "Senza titolo - Messaggio".to_string() } else { format!("{} - Messaggio", draft.subject.trim()) };
    w.drafts.lock().unwrap_or_else(|e| e.into_inner()).insert(label.clone(), draft);
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("index.html?finestra=messaggio".into()))
        .title(title)
        .inner_size(980.0, 760.0)
        .min_inner_size(640.0, 480.0)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| Error::user(format!("Impossibile aprire la finestra: {e}")))?;
    Ok(())
}

// ----------------------------------------------------------------------
// Stampa
// ----------------------------------------------------------------------

/// Apre la finestra di stampa di un messaggio.
#[tauri::command]
pub async fn open_print_window(app: AppHandle, w: State<'_, ComposeWindows>, id: i64, remote: bool) -> Result<()> {
    let n = w.next.fetch_add(1, Ordering::Relaxed) + 1;
    let label = format!("stampa-{n}");
    let url = format!("index.html?finestra=stampa&id={id}&remoto={}", if remote { 1 } else { 0 });
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(url.into()))
        .title("Stampa")
        .inner_size(860.0, 900.0)
        .min_inner_size(560.0, 480.0)
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| Error::user(format!("Impossibile aprire la finestra di stampa: {e}")))?;
    Ok(())
}

/// Apre la finestra di dialogo di stampa del sistema per la pagina.
#[tauri::command]
pub async fn print_page(webview: Webview) -> Result<()> {
    webview.print().map_err(|e| Error::user(format!("Impossibile stampare: {e}")))
}

// ----------------------------------------------------------------------
// Lettura ad alta voce con la voce del sistema (quando la pagina non ne ha)
// ----------------------------------------------------------------------

#[derive(Default)]
pub struct Speech(Mutex<Option<std::process::Child>>);

fn speech_command(text: &str) -> Option<std::process::Command> {
    use std::process::Command;
    #[cfg(target_os = "macos")]
    {
        let mut c = Command::new("say");
        c.arg(text);
        return Some(c);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Speak([Console]::In.ReadToEnd())",
        ]);
        c.creation_flags(0x0800_0000);
        return Some(c);
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for (prog, args) in [("spd-say", vec!["-w", "-l", "it"]), ("espeak-ng", vec!["-v", "it"]), ("espeak", vec!["-v", "it"])] {
            let found = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file())).unwrap_or(false);
            if found {
                let mut c = Command::new(prog);
                c.args(args).arg(text);
                return Some(c);
            }
        }
        None
    }
}

#[tauri::command]
pub async fn speak(s: State<'_, Speech>, text: String) -> Result<()> {
    use std::io::Write;
    let mut guard = s.0.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(mut c) = guard.take() {
        let _ = c.kill();
    }
    let text: String = text.chars().take(20_000).collect();
    let mut cmd = speech_command(&text).ok_or_else(|| Error::user("Su questo computer non c'è una voce per leggere il testo. Su Linux si installa con il pacchetto speech-dispatcher o espeak-ng."))?;
    cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    let mut child = cmd.spawn().map_err(|e| Error::user(format!("Impossibile avviare la lettura: {e}")))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    *guard = Some(child);
    Ok(())
}

/// Vero mentre la voce del sistema sta ancora leggendo.
#[tauri::command]
pub async fn speaking(s: State<'_, Speech>) -> Result<bool> {
    let mut guard = s.0.lock().unwrap_or_else(|e| e.into_inner());
    Ok(match guard.as_mut() {
        Some(c) => matches!(c.try_wait(), Ok(None)),
        None => false,
    })
}

#[tauri::command]
pub async fn stop_speaking(s: State<'_, Speech>) -> Result<()> {
    if let Some(mut c) = s.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = c.kill();
    }
    Ok(())
}

#[tauri::command]
pub async fn take_compose_draft(window: WebviewWindow, w: State<'_, ComposeWindows>) -> Result<Option<Draft>> {
    Ok(w.drafts.lock().unwrap_or_else(|e| e.into_inner()).remove(window.label()))
}
