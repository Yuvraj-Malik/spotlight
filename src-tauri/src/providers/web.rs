use crate::commands::{ResultKind, SearchResult};

pub fn fallback(q: &str) -> SearchResult {
    SearchResult {
        id: "web".into(),
        title: format!("Search the web for \"{q}\""),
        subtitle: "Google".into(),
        kind: ResultKind::Web,
        target: format!("https://www.google.com/search?q={}", urlencoding::encode(q)),
        score: i64::MIN,
    }
}
