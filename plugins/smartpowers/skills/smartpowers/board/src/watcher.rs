//! Polls the projects tree and lets live-update clients wait for changes.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, UNIX_EPOCH};

use crate::files::files_recursive;
use crate::workspace::Workspace;

/// A version counter that increases whenever any project Markdown file,
/// comment, or log is added, removed, or modified.
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

    /// Start polling the workspace every `interval` on a background thread.
    pub fn spawn(self: &Arc<Self>, workspace: Arc<Workspace>, interval: Duration) {
        let watcher = Arc::clone(self);
        thread::spawn(move || {
            let mut current = fingerprint(&workspace);
            loop {
                thread::sleep(interval);
                let next = fingerprint(&workspace);
                if next != current {
                    current = next;
                    *watcher.version.lock().unwrap() += 1;
                    watcher.changed.notify_all();
                }
            }
        });
    }

    /// Signal a change that isn't a file change, such as an agent connecting.
    pub fn bump(&self) {
        *self.version.lock().unwrap() += 1;
        self.changed.notify_all();
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

/// Hash of the path, mtime, and size of every Markdown file in the projects
/// tree and the state directory.
fn fingerprint(workspace: &Workspace) -> u64 {
    let mut files = files_recursive(&workspace.projects_dir());
    files.extend(files_recursive(&workspace.state_dir));
    let mut entries: Vec<String> = files
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
    use crate::project::log;
    use crate::test_support::TempRepo;

    fn watch(repo: &TempRepo) -> Arc<Watcher> {
        let watcher = Watcher::new();
        let workspace =
            Workspace::with_state_dir(repo.root.clone(), repo.workspace.state_dir.clone());
        watcher.spawn(Arc::new(workspace), Duration::from_millis(50));
        thread::sleep(Duration::from_millis(100));
        watcher
    }

    #[test]
    fn detects_task_changes() {
        let repo = TempRepo::new();
        let watcher = watch(&repo);
        let task = repo.project().join("tasks/02_token_store.md");
        let text = fs::read_to_string(&task).unwrap();
        // Change the size too, in case mtime resolution is coarse.
        fs::write(&task, text.replace("In Progress", "Done!")).unwrap();
        assert_eq!(watcher.wait(0, Duration::from_secs(3)), 1);
    }

    #[test]
    fn detects_new_logs_in_the_state_directory() {
        let repo = TempRepo::new();
        let watcher = watch(&repo);
        log::append(&repo.project_state(), "02", "Started").unwrap();
        assert_eq!(watcher.wait(0, Duration::from_secs(3)), 1);
    }
}
