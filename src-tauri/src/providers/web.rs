use crate::commands::{ResultKind, SearchResult};

pub fn fallback(q: &str) -> SearchResult {
    SearchResult {
        id: "web".into(),
        title: format!("Search the web for \"{q}\""),
        subtitle: "Google".into(),
        kind: ResultKind::Web,
        target: format!("https://www.google.com/search?q={}", urlencoding::encode(q)),
        score: -1_000_000, // lowest, but safe to round-trip through JavaScript
    }
}
