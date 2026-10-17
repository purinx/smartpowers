//! Polls the projects tree and lets live-update clients wait for changes.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

use crate::files::files_recursive;
use crate::project::PROJECTS_PATH;

/// A version counter that increases whenever any project Markdown file
/// is added, removed, or modified.
pub struct Watcher {
    version: Mutex<u64>,
    changed: Condvar,
}

impl Watcher {
    pub fn new() -> Arc<Self> {
        Arc::new(Watcher {
            version: Mutex::new(0),
            changed: Condvar::new(),
        })
    }

    /// Start polling `repo_root` every `interval` on a background thread.
    pub fn spawn(self: &Arc<Self>, repo_root: PathBuf, interval: Duration) {
        let watcher = Arc::clone(self);
        thread::spawn(move || {
            let mut current = fingerprint(&repo_root);
            loop {
                thread::sleep(interval);
                let next = fingerprint(&repo_root);
                if next != current {
                    current = next;
                    *watcher.version.lock().unwrap() += 1;
                    watcher.changed.notify_all();
                }
            }
        });
    }

    pub fn current(&self) -> u64 {
        *self.version.lock().unwrap()
    }

    /// Block until the version differs from `seen` or `timeout` elapses,
    /// then return the current version.
    pub fn wait(&self, seen: u64, timeout: Duration) -> u64 {
        let guard = self.version.lock().unwrap();
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, timeout, |version| *version == seen)
            .unwrap();
        *guard
    }
}

/// Hash of every project Markdown file's path, mtime, and size.
fn fingerprint(repo_root: &Path) -> u64 {
    let mut entries: Vec<String> = files_recursive(&repo_root.join(PROJECTS_PATH))
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter_map(|path| {
            let meta = fs::metadata(path).ok()?;
            let nanos = meta
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(format!("{}:{nanos}:{}", path.display(), meta.len()))
        })
        .collect();
    entries.sort();
    let mut hasher = DefaultHasher::new();
    entries.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn detects_task_changes() {
        let repo = TempRepo::new();
        let watcher = Watcher::new();
        watcher.spawn(repo.root.clone(), Duration::from_millis(50));
        thread::sleep(Duration::from_millis(100));
        let task = repo.project().join("tasks/02_token_store.md");
        let text = fs::read_to_string(&task).unwrap();
        // Change the size too, in case mtime resolution is coarse.
        fs::write(&task, text.replace("In Progress", "Done!")).unwrap();
        assert_eq!(watcher.wait(0, Duration::from_secs(3)), 1);
    }
}
