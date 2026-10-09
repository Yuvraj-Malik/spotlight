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

fn shortcut_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(p) = env::var("ProgramData") {
        dirs.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    if let Ok(p) = env::var("APPDATA") {
        dirs.push(PathBuf::from(p).join(r"Microsoft\Windows\Start Menu\Programs"));
    }
    // Desktop shortcuts (yours and the shared Public desktop).
    if let Ok(p) = env::var("USERPROFILE") {
        dirs.push(PathBuf::from(p).join("Desktop"));
    }
    if let Ok(p) = env::var("PUBLIC") {
        dirs.push(PathBuf::from(p).join("Desktop"));
    }
    dirs
}

fn is_junk(name: &str) -> bool {
    let l = name.to_lowercase();
    l.contains("uninstall") || l.contains("readme") || l.contains("release notes") || l == "desktop"
}

/// Shortcut files in the Start Menu and on the Desktop.
fn scan_shortcuts() -> Vec<AppEntry> {
    shortcut_dirs()
        .into_iter()
        .flat_map(|d| WalkDir::new(d).max_depth(6).into_iter().filter_map(Result::ok))
        .filter(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            matches!(ext.to_ascii_lowercase().as_str(), "lnk" | "url" | "exe" | "appref-ms")
        })
        .filter_map(|e| {
            let name = e.path().file_stem()?.to_string_lossy().to_string();
            if is_junk(&name) {
                return None;
            }
            Some(AppEntry { name, path: e.path().to_string_lossy().to_string() })
        })
        .collect()
}

/// Everything in the Windows "All apps" list, including Store / MSIX apps
/// (Claude, WhatsApp, Calculator, Settings...) that have no shortcut file.
/// Uses PowerShell's Get-StartApps, which reads shell:AppsFolder.
#[cfg(windows)]
fn scan_start_apps() -> Vec<AppEntry> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-StartApps | Select-Object Name, AppID | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(output) = output else { return vec![] };
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else { return vec![] };

    // A single result comes back as an object, several as an array.
    let items = match json {
        serde_json::Value::Array(a) => a,
        other => vec![other],
    };
    items
        .into_iter()
        .filter_map(|v| {
            let name = v.get("Name")?.as_str()?.trim().to_string();
            let id = v.get("AppID")?.as_str()?.to_string();
            if name.is_empty() || is_junk(&name) || id.starts_with("http") {
                return None;
            }
            // ShellExecute understands shell:AppsFolder\<AppID> for every kind of app.
            Some(AppEntry { name, path: format!(r"shell:AppsFolder\{id}") })
        })
        .collect()
}

#[cfg(not(windows))]
fn scan_start_apps() -> Vec<AppEntry> {
    vec![]
}

pub fn scan() -> Vec<AppEntry> {
    // Shortcut files first: when an app appears in both lists, keep the shortcut.
    let mut apps = scan_shortcuts();
    apps.extend(scan_start_apps());

    let mut seen = std::collections::HashSet::new();
    apps.retain(|a| seen.insert(a.name.to_lowercase()));
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
