use crate::commands::{ResultKind, SearchResult};

pub fn query(q: &str) -> Option<SearchResult> {
    // Only try maths when it looks like maths, so "chrome" never hits the parser.
    if !q.chars().any(|c| c.is_ascii_digit()) || !q.chars().any(|c| "+-*/^%()".contains(c) || c.is_alphabetic()) {
        return None;
    }
    let v = meval::eval_str(q).ok()?;
    if !v.is_finite() {
        return None;
    }
    let text = if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v:.6}").trim_end_matches('0').to_string() };
    Some(SearchResult {
        id: "calc".into(),
        title: format!("= {text}"),
        subtitle: q.into(),
        kind: ResultKind::Calc,
        target: text,
        score: 1000, // an exact maths answer always wins
    })
}
