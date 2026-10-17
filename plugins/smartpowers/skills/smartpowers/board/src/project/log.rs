//! Agent progress logs, one Markdown file per task under
//! `<project state>/logs/<task>.md`, appended by `smartpowers-board log`:
//!
//! ```text
//! # Task 05 log
//!
//! - 2026-10-17T10:15:30.123Z Started the token store migration
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::time::iso_utc_millis;

pub const MAX_MESSAGE_CHARS: usize = 1_000;

pub struct LogEntry {
    /// RFC 3339 UTC with milliseconds.
    pub time: String,
    pub message: String,
}

fn log_path(project_state: &Path, task: &str) -> PathBuf {
    project_state.join("logs").join(format!("{task}.md"))
}

/// Entries for one task, oldest first.
pub fn load(project_state: &Path, task: &str) -> Vec<LogEntry> {
    let Ok(text) = fs::read_to_string(log_path(project_state, task)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (time, message) = line.strip_prefix("- ")?.split_once(' ')?;
            Some(LogEntry {
                time: time.to_string(),
                message: message.trim().to_string(),
            })
        })
        .collect()
}

/// Append one entry. Line breaks in `message` become spaces so every entry
/// stays on one line.
pub fn append(project_state: &Path, task: &str, message: &str) -> Result<LogEntry, String> {
    let message = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if message.is_empty() {
        return Err("Log message is empty".into());
    }
    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(format!(
            "Log message is longer than {MAX_MESSAGE_CHARS} characters"
        ));
    }
    let path = log_path(project_state, task);
    let dir = path.parent().ok_or("invalid log path")?;
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;

    let entry = LogEntry {
        time: iso_utc_millis(SystemTime::now()),
        message,
    };
    let is_new = !path.exists();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let header = if is_new {
        format!("# Task {task} log\n\n")
    } else {
        String::new()
    };
    writeln!(file, "{header}- {} {}", entry.time, entry.message)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn appends_and_loads_entries() {
        let repo = TempRepo::new();
        let state = repo.project_state();
        assert!(load(&state, "02").is_empty());
        append(&state, "02", "Started\nthe migration").unwrap();
        append(&state, "02", "Tests pass").unwrap();
        let entries = load(&state, "02");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "Started the migration");
        assert_eq!(entries[1].message, "Tests pass");
        assert!(entries[0].time.ends_with('Z'));
        assert!(append(&state, "02", "  ").is_err());
    }
}
