//! Time-based file search: "files i changed today", "pdfs from yesterday",
//! "screenshots this week", "recent downloads", "code edited in the last 3 hours".
//! Pure rules, no AI: we spot a file word + a time phrase, then walk the folders
//! and return the most recently modified matches.
use chrono::{Datelike, Local, TimeZone};
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::UNIX_EPOCH,
};
use walkdir::WalkDir;

pub struct RecentQuery {
    pub since: i64,
    pub until: i64,
    pub exts: Option<&'static [&'static str]>,
    pub downloads_only: bool,
    /// e.g. "today", "in the last 3 hours"
    pub label: String,
}

const TYPES: &[(&str, &[&str])] = &[
    (r"\bpdfs?\b", &["pdf"]),
    (r"\b(word|docx?|documents?)\b", &["doc", "docx", "odt", "rtf", "txt", "md"]),
    (r"\b(ppts?|pptx|slides?|presentations?|decks?)\b", &["ppt", "pptx", "key", "odp"]),
    (r"\b(excel|sheets?|spreadsheets?|xlsx?|csv)\b", &["xls", "xlsx", "csv", "ods"]),
    (r"\b(images?|photos?|pictures?|pics?|screenshots?|pngs?|jpe?gs?)\b", &["png", "jpg", "jpeg", "gif", "webp", "heic", "bmp"]),
    (r"\b(videos?|recordings?|clips?|mp4)\b", &["mp4", "mkv", "mov", "avi", "webm"]),
    (r"\b(code|scripts?|source)\b", &["rs", "py", "js", "ts", "tsx", "jsx", "java", "c", "cpp", "h", "cs", "go", "kt", "html", "css", "sql", "ipynb"]),
    (r"\b(zips?|archives?)\b", &["zip", "rar", "7z", "tar", "gz"]),
];

fn rx(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).unwrap())
}

pub fn parse(q: &str) -> Option<RecentQuery> {
    let t = q.to_lowercase();

    static FILEWORD: OnceLock<Regex> = OnceLock::new();
    let mut exts = None;
    for (pat, e) in TYPES {
        if Regex::new(pat).ok()?.is_match(&t) {
            exts = Some(*e);
            break;
        }
    }
    let downloads_only = t.contains("download");
    let has_file_word = exts.is_some()
        || downloads_only
        || rx(&FILEWORD, r"\b(files?|stuff|things|work)\b").is_match(&t);
    if !has_file_word {
        return None;
    }

    let now = Local::now();
    let midnight = |d: chrono::NaiveDate| Local.from_local_datetime(&d.and_hms_opt(0, 0, 0)?).single().map(|x| x.timestamp());
    let today = midnight(now.date_naive())?;
    let end = now.timestamp() + 60;

    static LAST_N: OnceLock<Regex> = OnceLock::new();
    let last_n = rx(&LAST_N, r"\b(?:last|past)\s+(\d+)\s*(minutes?|mins?|hours?|hrs?|days?|weeks?)\b");

    let (since, until, label) = if let Some(c) = last_n.captures(&t) {
        let n: i64 = c[1].parse().ok()?;
        let unit = &c[2];
        let secs = if unit.starts_with('m') { 60 } else if unit.starts_with('h') { 3600 } else if unit.starts_with('d') { 86400 } else { 7 * 86400 };
        (end - n * secs, end, format!("in the last {n} {unit}"))
    } else if t.contains("yesterday") {
        (today - 86400, today, "yesterday".into())
    } else if t.contains("today") || t.contains("this morning") {
        (today, end, "today".into())
    } else if t.contains("this week") {
        let back = now.weekday().num_days_from_monday() as i64;
        (today - back * 86400, end, "this week".into())
    } else if t.contains("last week") {
        let back = now.weekday().num_days_from_monday() as i64;
        let this_monday = today - back * 86400;
        (this_monday - 7 * 86400, this_monday, "last week".into())
    } else if t.contains("this month") {
        let first = midnight(now.date_naive().with_day(1)?)?;
        (first, end, "this month".into())
    } else if t.contains("last hour") || t.contains("past hour") {
        (end - 3600, end, "in the last hour".into())
    } else if t.contains("recent") || t.contains("latest") || t.contains("new ") || t.ends_with(" new") {
        (end - 3 * 86400, end, "recently".into())
    } else {
        return None;
    };
    Some(RecentQuery { since, until, exts, downloads_only, label })
}

const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", "venv", "env", "__pycache__",
    "AppData", "$RECYCLE.BIN", "site-packages", "bin", "obj",
];

fn skip_file(name: &str) -> bool {
    let n = name.to_lowercase();
    n.starts_with("~$") || n.starts_with('.') || n == "desktop.ini" || n == "thumbs.db"
        || n.ends_with(".tmp") || n.ends_with(".crdownload") || n.ends_with(".part") || n.ends_with(".lnk")
}

/// Most recently modified files matching the query: (path, modified unix time).
pub fn find(q: &RecentQuery, folders: &[String], limit: usize) -> Vec<(PathBuf, i64)> {
    let mut roots: Vec<PathBuf> = folders.iter().map(PathBuf::from).collect();
    if q.downloads_only {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        roots = vec![Path::new(&home).join("Downloads")];
    }
    let mut hits = Vec::new();
    for root in roots {
        let walker = WalkDir::new(&root).max_depth(8).into_iter().filter_entry(|e| {
            if !e.file_type().is_dir() || e.depth() == 0 {
                return true;
            }
            let n = e.file_name().to_string_lossy();
            !n.starts_with('.') && !SKIP_DIRS.iter().any(|d| d.eq_ignore_ascii_case(&n))
        });
        for e in walker.filter_map(Result::ok) {
            if !e.file_type().is_file() || skip_file(&e.file_name().to_string_lossy()) {
                continue;
            }
            if let Some(exts) = q.exts {
                let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("").to_lowercase();
                if !exts.contains(&ext.as_str()) {
                    continue;
                }
            }
            let Ok(meta) = e.metadata() else { continue };
            let Some(m) = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()) else { continue };
            let m = m.as_secs() as i64;
            if m >= q.since && m < q.until {
                hits.push((e.into_path(), m));
            }
        }
    }
    hits.sort_by(|a, b| b.1.cmp(&a.1));
    hits.dedup_by(|a, b| a.0 == b.0);
    hits.truncate(limit);
    hits
}

/// "3:42 PM", "Yesterday 6:10 PM", "Mon 9:05 AM", "2 Oct"
pub fn when(ts: i64) -> String {
    let Some(t) = Local.timestamp_opt(ts, 0).single() else { return String::new() };
    let now = Local::now();
    let days = (now.date_naive() - t.date_naive()).num_days();
    match days {
        0 => t.format("%-I:%M %p").to_string(),
        1 => t.format("Yesterday %-I:%M %p").to_string(),
        2..=6 => t.format("%a %-I:%M %p").to_string(),
        _ => t.format("%-d %b").to_string(),
    }
}

