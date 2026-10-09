//! Reminders and timers. Saved to disk so they survive restarts; one background
//! thread checks every second and shows a Windows notification when one is due.
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Arc, time::Duration};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

#[derive(Serialize, Deserialize, Clone)]
pub struct Reminder {
    pub at: i64,
    pub message: String,
}

pub struct Reminders {
    list: Mutex<Vec<Reminder>>,
    file: PathBuf,
}

impl Reminders {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("reminders.json");
        let list = fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { list: Mutex::new(list), file }
    }

    fn save(&self, list: &[Reminder]) {
        if let Ok(json) = serde_json::to_string_pretty(list) {
            let _ = fs::write(&self.file, json);
        }
    }

    pub fn add(&self, at: i64, message: String) {
        let mut l = self.list.lock();
        l.push(Reminder { at, message });
        self.save(&l);
    }
}

pub fn start(app: AppHandle, reminders: Arc<crate::AppState>) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let now = chrono::Local::now().timestamp();
        let due: Vec<Reminder> = {
            let mut l = reminders.reminders.list.lock();
            let (due, keep): (Vec<_>, Vec<_>) = l.drain(..).partition(|r| r.at <= now);
            *l = keep;
            if !due.is_empty() {
                reminders.reminders.save(&l);
            }
            due
        };
        for r in due {
            let missed = now - r.at > 120;
            show_overlay(&app, &r, missed);
        }
    });
}

/// Full-screen, always-on-top reminder card. Each reminder gets its own window,
/// so two reminders due at once both show.
fn show_overlay(app: &AppHandle, r: &Reminder, missed: bool) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let label = format!("reminder-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
    let url = format!(
        "index.html?view=reminder&msg={}&at={}&missed={}",
        urlencoding::encode(&r.message),
        r.at,
        missed as u8
    );
    let built = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title("Reminder")
        .fullscreen(true)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(false)
        .focused(true)
        .build();
    if let Ok(w) = built {
        let _ = w.set_focus();
    }
    play_chime();
}

/// Windows' own "Reminder" sound (the one Calendar uses).
#[cfg(windows)]
fn play_chime() {
    use windows::core::HSTRING;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC};
    unsafe {
        let _ = PlaySoundW(&HSTRING::from("Notification.Reminder"), None, SND_ALIAS | SND_ASYNC);
    }
}

#[cfg(not(windows))]
fn play_chime() {}
