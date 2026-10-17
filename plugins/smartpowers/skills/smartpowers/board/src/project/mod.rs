//! Smartpowers project workspaces under `docs/smartpowers/projects/<id>/`,
//! plus their comments and agent logs from the per-machine state directory.

pub mod comments;
pub mod log;
mod markdown;
pub mod schedule;
mod task;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::files::{files_recursive, mtime};
use crate::workspace::Workspace;
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
    /// Task numbers in `tasks/PLAN.md` execution order.
    pub plan_order: Vec<String>,
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
pub fn list_projects(workspace: &Workspace) -> Vec<ProjectSummary> {
    let Ok(entries) = fs::read_dir(workspace.projects_dir()) else {
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

pub fn load_project(workspace: &Workspace, id: &str) -> Option<Project> {
    if !is_project_id(id) {
        return None;
    }
    let root = workspace.projects_dir().join(id);
    if !root.is_dir() {
        return None;
    }
    let state = workspace.project_state(id);
    let mut comments = comments::load(&state);
    let mut tasks = task::load_tasks(&root);
    for task in &mut tasks {
        let (mine, rest) = comments.into_iter().partition(|c| c.task == task.number);
        task.comments = mine;
        comments = rest;
        task.log = log::load(&state, &task.number);
    }
    let read_status = |file: &str| {
        fs::read_to_string(root.join(file))
            .ok()
            .map(|text| Header::parse(&text).status())
    };
    let design_skipped = fs::read_to_string(root.join("PRD.md"))
        .ok()
        .and_then(|text| Header::parse(&text).field("design").map(str::to_lowercase))
        .is_some_and(|design| design.starts_with("skipped"));
    let phases = PHASES
        .iter()
        .map(|&(name, file)| Phase {
            name,
            file,
            status: read_status(file).or_else(|| {
                // Small non-feature work may skip the design in its approved PRD.
                (file == "DESIGN_DOC.md" && design_skipped).then(|| "Skipped".to_string())
            }),
        })
        .collect();
    Some(Project {
        summary: summarize(&root, &tasks),
        phases,
        path: format!("{PROJECTS_PATH}/{id}"),
        plan_order: fs::read_to_string(root.join("tasks").join("PLAN.md"))
            .map(|plan| schedule::plan_order(&plan))
            .unwrap_or_default(),
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
    use comments::Author;

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
        let project = load_project(&repo.workspace, PROJECT_ID).unwrap();
        assert_eq!(project.summary.title, "OAuth refresh");
        assert_eq!(project.summary.status, "Active");
        assert_eq!(project.summary.task_count, 2);
        assert_eq!(project.summary.counts[&StatusKey::Done], 1);
        assert_eq!(project.summary.counts[&StatusKey::InProgress], 1);
        let phases: Vec<_> = project.phases.iter().map(|p| p.status.as_deref()).collect();
        assert_eq!(phases, [Some("Approved"), Some("Draft"), Some("Approved")]);

        fs::remove_file(repo.project().join("DESIGN_DOC.md")).unwrap();
        let design = |repo: &TempRepo| {
            load_project(&repo.workspace, PROJECT_ID).unwrap().phases[1]
                .status
                .clone()
        };
        assert_eq!(design(&repo), None);
        fs::write(
            repo.project().join("PRD.md"),
            "# PRD\n\nStatus: Approved\nDesign: Skipped - helper script\n",
        )
        .unwrap();
        assert_eq!(design(&repo).as_deref(), Some("Skipped"));

        assert!(load_project(&repo.workspace, "../etc").is_none());
        assert_eq!(list_projects(&repo.workspace).len(), 1);
    }

    #[test]
    fn attaches_comments_and_logs_to_their_tasks() {
        let repo = TempRepo::new();
        let state = repo.project_state();
        comments::create(&state, "02", Author::User, "Add retries", None).unwrap();
        log::append(&state, "02", "Started").unwrap();
        let project = load_project(&repo.workspace, PROJECT_ID).unwrap();
        assert!(project.tasks[0].comments.is_empty() && project.tasks[0].log.is_empty());
        assert_eq!(project.tasks[1].comments[0].body, "Add retries");
        assert_eq!(project.tasks[1].log[0].message, "Started");
    }
}
