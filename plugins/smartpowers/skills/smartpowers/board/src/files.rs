//! Filesystem helpers that tolerate files disappearing mid-scan.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Modification time, or the Unix epoch when it can't be read.
pub fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or(UNIX_EPOCH)
}

/// Every regular file under `dir`, recursively. Unreadable entries are skipped.
pub fn files_recursive(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(dir, &mut out);
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect(&entry.path(), out),
            Ok(kind) if kind.is_file() => out.push(entry.path()),
            _ => {}
        }
    }
}
