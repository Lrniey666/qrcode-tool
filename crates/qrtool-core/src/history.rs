//! Optional local scan history stored as JSON Lines.
//!
//! Privacy rules (enforced here, not only in the UI):
//! * history is off unless the user enables it;
//! * sensitive payloads (2FA secrets, Wi-Fi passwords) are never written;
//! * only the most recent `max_entries` entries are kept.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::payload::{Payload, PayloadKind};

/// Where a code was scanned from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanSource {
    Screen,
    Camera,
    Image,
    Clipboard,
}

/// A single history entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unix timestamp (seconds, UTC).
    pub ts: i64,
    pub kind: PayloadKind,
    pub source: ScanSource,
    pub text: String,
}

/// JSON Lines history file.
#[derive(Debug, Clone)]
pub struct History {
    path: PathBuf,
    max_entries: usize,
}

impl History {
    pub fn new(path: impl Into<PathBuf>, max_entries: usize) -> Self {
        Self {
            path: path.into(),
            max_entries: max_entries.max(1),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads all valid entries, newest last. Invalid lines are skipped.
    pub fn load(&self) -> Vec<HistoryEntry> {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        text.lines()
            .filter_map(|l| serde_json::from_str::<HistoryEntry>(l).ok())
            .collect()
    }

    /// Records a scan unless the payload is sensitive. Returns `true` if it was written.
    pub fn record(
        &self,
        payload: &Payload,
        text: &str,
        source: ScanSource,
        ts: i64,
    ) -> std::io::Result<bool> {
        if payload.is_sensitive() {
            return Ok(false);
        }
        let mut entries = self.load();
        // Do not create consecutive duplicates.
        if entries.last().is_some_and(|e| e.text == text) {
            return Ok(false);
        }
        entries.push(HistoryEntry {
            ts,
            kind: payload.kind(),
            source,
            text: text.to_string(),
        });
        let start = entries.len().saturating_sub(self.max_entries);
        self.write_all(&entries[start..])?;
        Ok(true)
    }

    /// Deletes all entries.
    pub fn clear(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn write_all(&self, entries: &[HistoryEntry]) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            for e in entries {
                serde_json::to_writer(&mut f, e)?;
                f.write_all(b"\n")?;
            }
            f.sync_all()?;
        }
        std::fs::rename(tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::classify;

    fn temp_history(name: &str, max: usize) -> History {
        let dir = std::env::temp_dir().join(format!("qrtool-hist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        History::new(dir.join("history.jsonl"), max)
    }

    #[test]
    fn records_and_trims() {
        let h = temp_history("trim", 3);
        for i in 0..5 {
            let t = format!("https://example.com/{i}");
            assert!(h.record(&classify(&t, None), &t, ScanSource::Screen, i).unwrap());
        }
        let all = h.load();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].text, "https://example.com/2");
        assert_eq!(all[2].kind, PayloadKind::Url);
        h.clear().unwrap();
        assert!(h.load().is_empty());
        h.clear().unwrap(); // clearing twice is fine
    }

    #[test]
    fn never_stores_sensitive_payloads() {
        let h = temp_history("sensitive", 10);
        let otp = "otpauth://totp/x?secret=ABC";
        assert!(!h
            .record(&classify(otp, None), otp, ScanSource::Camera, 1)
            .unwrap());
        let wifi = "WIFI:T:WPA;S:Home;P:password;;";
        assert!(!h
            .record(&classify(wifi, None), wifi, ScanSource::Camera, 2)
            .unwrap());
        assert!(h.load().is_empty());
    }

    #[test]
    fn skips_consecutive_duplicates() {
        let h = temp_history("dup", 10);
        let t = "hello";
        assert!(h.record(&classify(t, None), t, ScanSource::Image, 1).unwrap());
        assert!(!h.record(&classify(t, None), t, ScanSource::Image, 2).unwrap());
        assert_eq!(h.load().len(), 1);
    }
}
