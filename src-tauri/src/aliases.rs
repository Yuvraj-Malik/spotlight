//! Nicknames for any result. Set manually in Settings, or learned automatically:
//! if you type the same short query and pick the same result LEARN_AFTER times,
//! that query becomes a nickname for it.
use crate::commands::{ResultKind, SearchResult};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

const LEARN_AFTER: u32 = 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct Alias {
    pub alias: String,
    pub auto: bool,
    pub result: SearchResult,
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    aliases: Vec<Alias>,
    /// "query\u{1f}result_id" -> times picked
    picks: HashMap<String, u32>,
}

pub struct Aliases {
    store: Mutex<Store>,
    file: PathBuf,
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase()
}

impl Aliases {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("aliases.json");
        let store = fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { store: Mutex::new(store), file }
    }

    fn save(&self, s: &Store) {
        if let Ok(json) = serde_json::to_string_pretty(s) {
            let _ = fs::write(&self.file, json);
        }
    }

    pub fn list(&self) -> Vec<Alias> {
        self.store.lock().aliases.clone()
    }

    pub fn set(&self, alias: &str, result: SearchResult, auto: bool) {
        let key = normalize(alias);
        if key.is_empty() {
            return;
        }
        let mut s = self.store.lock();
        s.aliases.retain(|a| a.alias != key);
        s.aliases.push(Alias { alias: key, auto, result });
        s.aliases.sort_by(|a, b| a.alias.cmp(&b.alias));
        self.save(&s);
    }

    pub fn remove(&self, alias: &str) {
        let key = normalize(alias);
        let prefix = format!("{key}\u{1f}");
        let mut s = self.store.lock();
        s.aliases.retain(|a| a.alias != key);
        // Forget the learning history too, so a deleted nickname isn't re-learned straight away.
        s.picks.retain(|k, _| !k.starts_with(&prefix));
        self.save(&s);
    }

    /// Exact nickname match beats everything; a nickname prefix comes next.
    pub fn query(&self, q: &str) -> Vec<SearchResult> {
        let q = normalize(q);
        let s = self.store.lock();
        s.aliases
            .iter()
            .filter_map(|a| {
                let score = if a.alias == q {
                    2000
                } else if a.alias.starts_with(&q) {
                    1500
                } else {
                    return None;
                };
                let mut r = a.result.clone();
                r.subtitle = format!("Nickname “{}” · {}", a.alias, r.subtitle);
                r.score = score;
                Some(r)
            })
            .collect()
    }

    /// Record that `q` led to `result`. Returns the nickname if one was just learned.
    pub fn learn(&self, q: &str, result: &SearchResult) -> Option<String> {
        let key = normalize(q);
        if key.len() < 2 || key.len() > 8 || key.contains(' ') {
            return None;
        }
        if matches!(result.kind, ResultKind::Calc | ResultKind::Web) {
            return None;
        }
        // Plain prefixes ("chr" -> Chrome) already work; don't clutter the list with them.
        if result.title.to_lowercase().starts_with(&key) {
            return None;
        }
        let mut s = self.store.lock();
        if s.aliases.iter().any(|a| a.alias == key) {
            return None;
        }
        let n = s.picks.entry(format!("{key}\u{1f}{}", result.id)).or_insert(0);
        *n += 1;
        if *n < LEARN_AFTER {
            self.save(&s);
            return None;
        }
        let mut stored = result.clone();
        stored.score = 0;
        s.aliases.push(Alias { alias: key.clone(), auto: true, result: stored });
        s.aliases.sort_by(|a, b| a.alias.cmp(&b.alias));
        self.save(&s);
        Some(key)
    }
}
