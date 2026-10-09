use crate::{aliases::Alias, providers, window, AppState};
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
