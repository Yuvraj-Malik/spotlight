use crate::commands::{ResultKind, SearchResult};
use crate::ranking::Usage;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::{env, path::PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
}

fn start_menu_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(p) = env::var("ProgramData") {
        dirs.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(p) = env::var("APPDATA") {
        dirs.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    dirs
}

/// Index Start Menu shortcuts. TODO: add UWP/Store apps (shell:AppsFolder).
pub fn scan() -> Vec<AppEntry> {
    let mut apps: Vec<AppEntry> = start_menu_dirs()
        .into_iter()
        .flat_map(|d| WalkDir::new(d).into_iter().filter_map(Result::ok))
        .filter(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            matches!(ext.to_ascii_lowercase().as_str(), "lnk" | "url" | "exe")
        })
        .filter_map(|e| {
            let name = e.path().file_stem()?.to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if lower.contains("uninstall") || lower.contains("readme") {
                return None;
            }
            Some(AppEntry { name, path: e.path().to_string_lossy().to_string() })
        })
        .collect();
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps.dedup_by(|a, b| a.name == b.name);
    apps
}

pub fn query(q: &str, apps: &[AppEntry], usage: &Usage) -> Vec<SearchResult> {
    let matcher = SkimMatcherV2::default();
    apps.iter()
        .filter_map(|a| {
            let base = matcher.fuzzy_match(&a.name, q)?;
            let id = format!("app:{}", a.path);
            let prefix_bonus = if a.name.to_lowercase().starts_with(&q.to_lowercase()) { 40 } else { 0 };
            Some(SearchResult {
                score: base + prefix_bonus + usage.boost(&id),
                id,
                title: a.name.clone(),
                subtitle: "Application".into(),
                kind: ResultKind::App,
                target: a.path.clone(),
            })
        })
        .collect()
}
