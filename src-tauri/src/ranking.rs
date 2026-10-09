use parking_lot::Mutex;
use std::{collections::HashMap, fs, path::PathBuf};

/// How often each result was launched, so frequent picks float to the top.
pub struct Usage {
    counts: Mutex<HashMap<String, u32>>,
    file: PathBuf,
}

impl Usage {
    pub fn load(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let file = dir.join("usage.json");
        let counts = fs::read_to_string(&file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { counts: Mutex::new(counts), file }
    }

    pub fn boost(&self, id: &str) -> i64 {
        let n = *self.counts.lock().get(id).unwrap_or(&0) as f64;
        // Diminishing returns: the 1st launch matters more than the 50th.
        ((n + 1.0).ln() * 25.0) as i64
    }

    pub fn bump(&self, id: &str) {
        let mut c = self.counts.lock();
        *c.entry(id.to_string()).or_insert(0) += 1;
        if let Ok(json) = serde_json::to_string(&*c) {
            let _ = fs::write(&self.file, json);
        }
    }
}
