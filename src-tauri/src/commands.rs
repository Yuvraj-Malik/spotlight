use crate::{ai, aliases::Alias, icons, providers, recent, semantic, window, AppState};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ResultKind {
    App,
    File,
    Calc,
    Web,
    Command,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub kind: ResultKind,
    pub target: String,
    pub score: i64,
}

const MAX_RESULTS: usize = 8;
pub const SETTINGS_TARGET: &str = "spotlight:settings";

#[tauri::command]
pub fn search(query: String, state: State<'_, Arc<AppState>>) -> Vec<SearchResult> {
    let q = query.trim();
    if q.is_empty() {
        return vec![];
    }

    let mut out = Vec::new();
    out.extend(state.aliases.query(q));
    out.extend(providers::calculator::query(q));
    out.extend(providers::apps::query(q, &state.apps.read(), &state.usage));
    out.extend(providers::system::query(q));

    out.sort_by(|a, b| b.score.cmp(&a.score));
    // The same app can come from a nickname and from app search; keep the best-scored one.
    let mut seen = HashSet::new();
    out.retain(|r| seen.insert(r.id.clone()));
    out.truncate(MAX_RESULTS - 1);

    out.push(providers::web::fallback(q));
    out
}

#[tauri::command]
pub fn execute(
    result: SearchResult,
    query: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.usage.bump(&result.id);
    if let Some(q) = query {
        if let Some(learned) = state.aliases.learn(&q, &result) {
            let _ = app.emit("spotlight://alias-learned", (learned, result.title.clone()));
        }
    }
    match result.kind {
        ResultKind::App | ResultKind::File | ResultKind::Web => {
            open::that_detached(&result.target).map_err(|e| e.to_string())
        }
        ResultKind::Command if result.target == SETTINGS_TARGET => {
            open_settings(app);
            Ok(())
        }
        ResultKind::Command if result.target.starts_with("remind:") => {
            // remind:<unix time>:<message>
            let mut parts = result.target.splitn(3, ':').skip(1);
            let at = parts.next().and_then(|t| t.parse::<i64>().ok()).ok_or("bad reminder")?;
            let msg = parts.next().unwrap_or("Reminder").to_string();
            state.reminders.add(at, msg);
            Ok(())
        }
        ResultKind::Command => providers::system::run(&result.target),
        ResultKind::Calc => Ok(()), // TODO: copy result to clipboard
    }
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn open_settings(app: AppHandle) {
    // Creating a window from a sync command can deadlock on Windows, so do it off-thread.
    std::thread::spawn(move || window::open_settings(&app));
}

#[tauri::command]
pub fn list_aliases(state: State<'_, Arc<AppState>>) -> Vec<Alias> {
    state.aliases.list()
}

#[tauri::command]
pub fn set_alias(alias: String, result: SearchResult, state: State<'_, Arc<AppState>>) {
    state.aliases.set(&alias, result, false);
}

#[tauri::command]
pub fn remove_alias(alias: String, state: State<'_, Arc<AppState>>) {
    state.aliases.remove(&alias);
}

/// Icon for an app or file result as a PNG data URL (None if Windows has none).
#[tauri::command]
pub async fn get_icon(target: String, state: State<'_, Arc<AppState>>) -> Result<Option<String>, ()> {
    let dir = state.icon_dir.clone();
    Ok(tauri::async_runtime::spawn_blocking(move || icons::get(&target, &dir))
        .await
        .unwrap_or(None))
}

/// Ask the local model what a sentence means. Runs off the UI thread.
#[tauri::command]
pub async fn ai_interpret(query: String, state: State<'_, Arc<AppState>>) -> Result<Vec<SearchResult>, String> {
    let cfg = state.ai.get();
    if !cfg.enabled || query.trim().is_empty() {
        return Ok(vec![]);
    }
    let apps = state.apps.read().clone();
    tauri::async_runtime::spawn_blocking(move || ai::interpret(query.trim(), &cfg, &apps))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ai_models() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(ai::list_models)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_ai_settings(state: State<'_, Arc<AppState>>) -> ai::AiSettings {
    state.ai.get()
}

#[tauri::command]
pub fn set_ai_settings(settings: ai::AiSettings, state: State<'_, Arc<AppState>>) {
    let warm = settings.enabled.then(|| settings.model.clone());
    state.ai.set(settings);
    if let Some(model) = warm {
        std::thread::spawn(move || ai::warmup(&model));
    }
}

/// "Snooze" on the full-screen reminder: schedule it again in `minutes`.
#[tauri::command]
pub fn snooze_reminder(message: String, minutes: i64, state: State<'_, Arc<AppState>>) {
    let at = chrono::Local::now().timestamp() + minutes.max(1) * 60;
    state.reminders.add(at, message);
}

/// Answer a question inline. Text streams to the UI through `on_chunk`.
#[tauri::command]
pub async fn ai_answer(
    query: String,
    on_chunk: tauri::ipc::Channel<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<Option<ai::Source>, String> {
    let cfg = state.ai.get();
    if !cfg.enabled {
        return Err("AI is turned off in Settings".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        ai::answer(&cfg.model, query.trim(), |c| {
            let _ = on_chunk.send(c.to_string());
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Files whose content matches the query's meaning (local embeddings).
#[tauri::command]
pub async fn semantic_search(query: String, state: State<'_, Arc<AppState>>) -> Result<Vec<SearchResult>, String> {
    let sem = state.semantic.clone();
    let hits = tauri::async_runtime::spawn_blocking(move || sem.search(query.trim(), 5))
        .await
        .map_err(|e| e.to_string())?;
    Ok(hits
        .into_iter()
        .map(|(path, score)| {
            let p = std::path::Path::new(&path);
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.clone());
            let folder = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            SearchResult {
                id: format!("file:{path}"),
                title: name,
                subtitle: format!("✨ {}% match by meaning · {folder}", (score * 100.0).round() as i64),
                kind: ResultKind::File,
                target: path,
                score: 1000 + (score * 100.0) as i64,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_semantic_settings(state: State<'_, Arc<AppState>>) -> semantic::SemanticSettings {
    state.semantic.settings()
}

#[tauri::command]
pub fn set_semantic_settings(settings: semantic::SemanticSettings, state: State<'_, Arc<AppState>>) {
    state.semantic.set_settings(settings);
}

#[tauri::command]
pub fn semantic_status(state: State<'_, Arc<AppState>>) -> semantic::Status {
    state.semantic.status()
}

#[tauri::command]
pub fn semantic_reindex(state: State<'_, Arc<AppState>>) {
    state.semantic.request_reindex();
}

/// Native "choose folder" dialog.
#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|f| f.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .ok()
    .flatten()
}

/// "files i changed today", "pdfs from yesterday"... Empty if the query isn't a time-based file search.
#[tauri::command]
pub async fn recent_files(query: String, state: State<'_, Arc<AppState>>) -> Result<Vec<SearchResult>, String> {
    let Some(q) = recent::parse(&query) else { return Ok(vec![]) };
    let folders = state.semantic.settings().folders;
    let hits = tauri::async_runtime::spawn_blocking(move || {
        let hits = recent::find(&q, &folders, 8);
        (hits, q.label)
    })
    .await
    .map_err(|e| e.to_string())?;
    let (hits, label) = hits;
    let total = hits.len();
    Ok(hits
        .into_iter()
        .enumerate()
        .map(|(i, (path, modified))| {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let folder = path.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
            let path = path.to_string_lossy().to_string();
            SearchResult {
                id: format!("file:{path}"),
                title: name,
                subtitle: format!("Modified {} · {folder}", recent::when(modified)),
                kind: ResultKind::File,
                target: path,
                // Keep newest first, above everything else.
                score: 4000 - i as i64,
            }
        })
        .chain((total == 0).then(|| SearchResult {
            id: "recent:none".into(),
            title: format!("No matching files changed {label}"),
            subtitle: "Searched the folders set in Settings → Search files by meaning".into(),
            kind: ResultKind::Command,
            target: "spotlight:settings".into(),
            score: 4000,
        }))
        .collect())
}
