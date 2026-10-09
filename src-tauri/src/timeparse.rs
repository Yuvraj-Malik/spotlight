//! Turns time words ("6 pm", "18:00", "in 10 minutes", "tomorrow morning") into
//! real times. Done in Rust, not by the AI model, so times are always exact.
use chrono::{DateTime, Duration, Local, TimeZone};
use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).unwrap())
}

/// "10 minutes", "1h 30m", "90 sec", "half an hour" -> seconds.
pub fn duration_secs(s: &str) -> Option<i64> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = re(&R, r"(?i)(\d+(?:\.\d+)?)\s*(hours?|hrs?|h|minutes?|mins?|m|seconds?|secs?|s)\b");
    let t = s.to_lowercase();
    let mut total = 0.0;
    let mut found = false;
    for c in r.captures_iter(&t) {
        let Ok(n) = c[1].parse::<f64>() else { continue };
        let unit = &c[2];
        let mult = if unit.starts_with('h') { 3600.0 } else if unit.starts_with('m') { 60.0 } else { 1.0 };
        total += n * mult;
        found = true;
    }
    if found {
        return Some(total.round() as i64).filter(|&x| x > 0);
    }
    if t.contains("half an hour") {
        Some(1800)
    } else if t.contains("an hour") || t.contains("one hour") {
        Some(3600)
    } else if t.contains("a minute") || t.contains("one minute") {
        Some(60)
    } else {
        None
    }
}

/// Absolute or relative time -> local datetime in the future.
pub fn parse(s: &str) -> Option<DateTime<Local>> {
    let t = s.to_lowercase();
    if t.trim().is_empty() {
        return None;
    }
    let now = Local::now();

    // Relative: "in 10 minutes", "after an hour"
    static REL: OnceLock<Regex> = OnceLock::new();
    if re(&REL, r"\b(in|after)\s+(\d|an?\s|half|one\s)").is_match(&t) {
        if let Some(secs) = duration_secs(&t) {
            return Some(now + Duration::seconds(secs));
        }
    }

    let tomorrow = t.contains("tomorrow");

    // Clock time: "6 pm", "6:30pm", "18:00", "at 6"
    static CLOCK: OnceLock<Regex> = OnceLock::new();
    let clock = re(&CLOCK, r"\b(\d{1,2})(?::(\d{2}))?\s*(am|pm|a\.m\.|p\.m\.)?");
    let mut time: Option<(u32, u32, bool)> = None;
    for c in clock.captures_iter(&t) {
        let whole = c.get(0).unwrap();
        let ampm = c.get(3).map(|x| x.as_str().starts_with('p'));
        let has_minutes = c.get(2).is_some();
        let after_at = t[..whole.start()].trim_end().ends_with("at") || t[..whole.start()].trim_end().ends_with("by");
        // Bare numbers ("call 3 people") are not times unless they look like one.
        if ampm.is_none() && !has_minutes && !after_at && t.trim() != whole.as_str().trim() {
            continue;
        }
        let Ok(mut h) = c[1].parse::<u32>() else { continue };
        let m = c.get(2).and_then(|x| x.as_str().parse::<u32>().ok()).unwrap_or(0);
        match ampm {
            Some(true) if h < 12 => h += 12,
            Some(false) if h == 12 => h = 0,
            _ => {}
        }
        if h < 24 && m < 60 {
            time = Some((h, m, ampm.is_some()));
            break;
        }
    }
    if time.is_none() {
        let named = if t.contains("noon") {
            Some((12, 0))
        } else if t.contains("morning") {
            Some((9, 0))
        } else if t.contains("afternoon") {
            Some((15, 0))
        } else if t.contains("evening") {
            Some((18, 0))
        } else if t.contains("tonight") || t.contains("night") {
            Some((21, 0))
        } else if tomorrow {
            Some((9, 0))
        } else {
            None
        };
        time = named.map(|(h, m)| (h, m, true));
    }
    let (h, m, explicit) = time?;

    let mut date = now.date_naive();
    if tomorrow {
        date = date.succ_opt()?;
    }
    let at = |d: chrono::NaiveDate, h: u32| Local.from_local_datetime(&d.and_hms_opt(h, m, 0)?).single();
    let mut dt = at(date, h)?;
    if dt <= now && !tomorrow {
        // "at 6" at 3 pm almost certainly means 6 pm today.
        let pm = if !explicit && h < 12 { at(date, h + 12).filter(|x| *x > now) } else { None };
        dt = pm.unwrap_or(dt + Duration::days(1));
    }
    Some(dt)
}

pub fn human_duration(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 60 {
        format!("{secs} sec")
    } else if secs < 3600 {
        format!("{} min", (secs + 30) / 60)
    } else {
        let h = secs / 3600;
        let m = (secs % 3600 + 30) / 60;
        if m == 0 { format!("{h} h") } else { format!("{h} h {m} min") }
    }
}

/// "6:00 PM today · in 2 h 5 min"
pub fn describe(t: DateTime<Local>) -> String {
    let now = Local::now();
    let day = if t.date_naive() == now.date_naive() {
        "today".to_string()
    } else if Some(t.date_naive()) == now.date_naive().succ_opt() {
        "tomorrow".to_string()
    } else {
        t.format("%a %-d %b").to_string()
    };
    format!("{} {} · in {}", t.format("%-I:%M %p"), day, human_duration((t - now).num_seconds()))
}
