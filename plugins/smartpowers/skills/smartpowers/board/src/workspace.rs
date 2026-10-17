//! Where the board reads and writes: the repository (read-only) and a
//! per-machine state directory for task comments and agent progress logs.
//!
//! State lives outside the repository on purpose. Comments are delivered to
//! agents as user feedback, so they must only come from this machine's
//! board, never from files someone committed to the repository.

use std::env;
use std::path::{Path, PathBuf};

use crate::project::PROJECTS_PATH;

pub struct Workspace {
    pub repo_root: PathBuf,
    /// State for this repository: `<state home>/<repo name>-<path hash>`.
    pub state_dir: PathBuf,
}

impl Workspace {
    /// `repo_root` must be canonical so the board and agent commands map it
    /// to the same state directory.
    pub fn new(repo_root: PathBuf) -> Self {
        let state_dir = repo_state_dir(&state_home(), &repo_root);
        Workspace {
            repo_root,
            state_dir,
        }
    }

    #[cfg(test)]
    pub fn with_state_dir(repo_root: PathBuf, state_dir: PathBuf) -> Self {
        Workspace {
            repo_root,
            state_dir,
        }
    }

    pub fn repo_name(&self) -> String {
        self.repo_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.repo_root.join(PROJECTS_PATH)
    }

    /// Comments and logs for one project. Callers validate `id` first.
    pub fn project_state(&self, id: &str) -> PathBuf {
        self.state_dir.join(id)
    }
}

/// `$SMARTPOWERS_STATE_HOME`, else `%LOCALAPPDATA%\smartpowers\state` on
/// Windows, else `$XDG_STATE_HOME/smartpowers`, else `~/.local/state/smartpowers`.
fn state_home() -> PathBuf {
    if let Some(dir) = env::var_os("SMARTPOWERS_STATE_HOME") {
        return PathBuf::from(dir);
    }
    if cfg!(windows) {
        if let Some(dir) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(dir).join("smartpowers").join("state");
        }
    }
    if let Some(dir) = env::var_os("XDG_STATE_HOME") {
        return PathBuf::from(dir).join("smartpowers");
    }
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(env::temp_dir, PathBuf::from)
        .join(".local")
        .join("state")
        .join("smartpowers")
}

/// Readable and unique per checkout: the directory name plus a hash of the
/// full path, so two clones of the same repository don't share comments.
fn repo_state_dir(home: &Path, repo_root: &Path) -> PathBuf {
    let name: String = repo_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let hash = fnv1a(repo_root.to_string_lossy().as_bytes());
    home.join(format!("{name}-{hash:016x}"))
}

/// FNV-1a. Unlike `DefaultHasher`, its output is stable across Rust
/// releases, so rebuilding the board never orphans existing state.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_reference_values() {
        assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn state_dir_is_named_after_the_repo_and_its_path() {
        let home = Path::new("/state");
        let a = repo_state_dir(home, Path::new("/src/my repo"));
        let b = repo_state_dir(home, Path::new("/other/my repo"));
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("my_repo-"), "{name}");
        assert_eq!(name.len(), "my_repo-".len() + 16);
        assert_ne!(a, b);
    }
}
