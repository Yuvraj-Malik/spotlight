use crate::commands::{ResultKind, SearchResult};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use std::process::Command;

/// (id, title, subtitle). Settings pages open via ms-settings: URIs.
const COMMANDS: &[(&str, &str, &str)] = &[
    ("spotlight:settings", "Spotlight Settings", "Nicknames and preferences"),
    ("lock", "Lock", "Lock this PC"),
    ("sleep", "Sleep", "Put the PC to sleep"),
    ("shutdown", "Shut Down", "Turn off the PC"),
    ("restart", "Restart", "Restart the PC"),
    ("ms-settings:", "Settings", "Open Windows Settings"),
    ("ms-settings:bluetooth", "Bluetooth", "Bluetooth settings"),
    ("ms-settings:network-wifi", "Wi-Fi", "Wi-Fi settings"),
    ("ms-settings:display", "Display", "Display settings"),
    ("ms-settings:sound", "Sound", "Sound settings"),
];

pub fn query(q: &str) -> Vec<SearchResult> {
    let m = SkimMatcherV2::default();
    COMMANDS
        .iter()
        .filter_map(|(id, title, sub)| {
            let s = m.fuzzy_match(title, q)?;
            Some(SearchResult {
                id: format!("cmd:{id}"),
                title: title.to_string(),
                subtitle: sub.to_string(),
                kind: ResultKind::Command,
                target: id.to_string(),
                score: s - 10, // slightly below apps with the same match
            })
        })
        .collect()
}

pub fn run(target: &str) -> Result<(), String> {
    let r = match target {
        "lock" => Command::new("rundll32.exe").args(["user32.dll,LockWorkStation"]).spawn(),
        "sleep" => Command::new("rundll32.exe").args(["powrprof.dll,SetSuspendState", "0,1,0"]).spawn(),
        "shutdown" => Command::new("shutdown").args(["/s", "/t", "0"]).spawn(),
        "restart" => Command::new("shutdown").args(["/r", "/t", "0"]).spawn(),
        uri => return open::that_detached(uri).map_err(|e| e.to_string()),
    };
    r.map(|_| ()).map_err(|e| e.to_string())
}
