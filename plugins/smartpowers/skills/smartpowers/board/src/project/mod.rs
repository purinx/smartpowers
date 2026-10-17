//! Smartpowers project workspaces under `docs/smartpowers/projects/<id>/`.

mod markdown;
mod task;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::files::{files_recursive, mtime};
use markdown::Header;
pub use task::{Remote, StatusKey, Task};

pub const PROJECTS_PATH: &str = "docs/smartpowers/projects";

/// Approval-gated artifacts shown in the project header, as (name, file).
const PHASES: [(&str, &str); 3] = [
    ("Requirements", "PRD.md"),
    ("Design", "DESIGN_DOC.md"),
    ("Plan", "tasks/PLAN.md"),
];

/// What the sidebar needs for every project.
pub struct ProjectSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    /// Latest modification time of any file in the project.
    pub updated: SystemTime,
    pub task_count: usize,
    pub counts: BTreeMap<StatusKey, u64>,
}

pub struct Phase {
    pub name: &'static str,
    pub file: &'static str,
    /// `None` when the artifact doesn't exist yet.
    pub status: Option<String>,
}

/// Everything the board shows for the selected project.
pub struct Project {
    pub summary: ProjectSummary,
    pub phases: Vec<Phase>,
    /// Repository-relative project directory.
    pub path: String,
    pub tasks: Vec<Task>,
}

/// `YYYYMMDD-HHMMSS-lowercase-kebab-slug`. Also guards against path
/// traversal, since IDs come from request URLs.
pub fn is_project_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 16
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'-'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'-'
        && is_slug(&value[16..])
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// All projects, newest first (IDs start with their creation timestamp).
pub fn list_projects(repo_root: &Path) -> Vec<ProjectSummary> {
    let Ok(entries) = fs::read_dir(repo_root.join(PROJECTS_PATH)) else {
        return Vec::new();
    };
    let mut roots: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            entry.path().is_dir() && is_project_id(&entry.file_name().to_string_lossy())
        })
        .map(|entry| entry.path())
        .collect();
    roots.sort_by(|a, b| b.cmp(a));
    roots
        .iter()
        .map(|root| summarize(root, &task::load_tasks(root)))
        .collect()
}

pub fn load_project(repo_root: &Path, id: &str) -> Option<Project> {
    if !is_project_id(id) {
        return None;
    }
    let root = repo_root.join(PROJECTS_PATH).join(id);
    if !root.is_dir() {
        return None;
    }
    let tasks = task::load_tasks(&root);
    let phases = PHASES
        .iter()
        .map(|&(name, file)| Phase {
            name,
            file,
            status: fs::read_to_string(root.join(file))
                .ok()
                .map(|text| Header::parse(&text).status()),
        })
        .collect();
    Some(Project {
        summary: summarize(&root, &tasks),
        phases,
        path: format!("{PROJECTS_PATH}/{id}"),
        tasks,
    })
}

fn summarize(project_root: &Path, tasks: &[Task]) -> ProjectSummary {
    let id = project_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let header = fs::read_to_string(project_root.join("PROJECT.md"))
        .map(|text| Header::parse(&text))
        .ok();
    let title = header
        .as_ref()
        .map(|h| h.title.clone())
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| id.clone());
    let status = header.map_or_else(|| "Unknown".into(), |h| h.status());

    let mut counts = BTreeMap::new();
    for task in tasks {
        *counts.entry(task.status_key).or_default() += 1;
    }
    let updated = files_recursive(project_root)
        .iter()
        .map(|path| mtime(path))
        .max()
        .unwrap_or_else(|| mtime(project_root));

    ProjectSummary {
        id,
        title,
        status,
        updated,
        task_count: tasks.len(),
        counts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempRepo, PROJECT_ID};

    #[test]
    fn validates_project_ids() {
        assert!(is_project_id(PROJECT_ID));
        assert!(!is_project_id("../etc"));
        assert!(!is_project_id("20260908-183045-Bad"));
        assert!(!is_project_id("20260908-183045-trailing-"));
    }

    #[test]
    fn loads_project_summary_and_phases() {
        let repo = TempRepo::new();
        let project = load_project(&repo.root, PROJECT_ID).unwrap();
        assert_eq!(project.summary.title, "OAuth refresh");
        assert_eq!(project.summary.status, "Active");
        assert_eq!(project.summary.task_count, 2);
        assert_eq!(project.summary.counts[&StatusKey::Done], 1);
        assert_eq!(project.summary.counts[&StatusKey::InProgress], 1);
        let phases: Vec<_> = project.phases.iter().map(|p| p.status.as_deref()).collect();
        assert_eq!(phases, [Some("Approved"), Some("Draft"), Some("Approved")]);

        assert!(load_project(&repo.root, "../etc").is_none());
        assert_eq!(list_projects(&repo.root).len(), 1);
    }
}
