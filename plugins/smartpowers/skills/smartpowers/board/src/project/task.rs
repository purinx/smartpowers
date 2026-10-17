//! Numbered task files: `tasks/NN_lowercase_snake_case.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::markdown::{find_issue_key, find_url, requirement_ids, task_refs, Header};
use crate::files::mtime;

/// Normalized task status. Declaration order is board column order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusKey {
    Planned,
    InProgress,
    Blocked,
    Done,
    Other,
}

impl StatusKey {
    pub fn from_status(status: &str) -> Self {
        match status.trim().to_lowercase().as_str() {
            "planned" | "todo" => StatusKey::Planned,
            "in progress" | "in-progress" => StatusKey::InProgress,
            "blocked" => StatusKey::Blocked,
            "done" | "completed" => StatusKey::Done,
            _ => StatusKey::Other,
        }
    }

    /// The identifier the web UI uses.
    pub fn as_str(self) -> &'static str {
        match self {
            StatusKey::Planned => "planned",
            StatusKey::InProgress => "in_progress",
            StatusKey::Blocked => "blocked",
            StatusKey::Done => "done",
            StatusKey::Other => "other",
        }
    }
}

/// A `Remote task:` line, such as `ENG-42 https://linear.app/...`.
pub struct Remote {
    pub raw: String,
    pub url: Option<String>,
    pub key: Option<String>,
}

pub struct Task {
    pub number: String,
    pub file: String,
    pub title: String,
    pub status: String,
    pub status_key: StatusKey,
    pub requirements: Vec<String>,
    pub depends_on: Vec<String>,
    pub parallel_group: String,
    pub remote: Option<Remote>,
    pub updated: SystemTime,
    pub body: String,
}

impl Task {
    fn parse(path: &Path, number: &str) -> Option<Self> {
        let body = fs::read_to_string(path).ok()?;
        let header = Header::parse(&body);
        let title = match strip_number(&header.title, number) {
            title if title.is_empty() => path.file_stem()?.to_string_lossy().into_owned(),
            title => title,
        };
        let status = header.status();
        let depends = header.field("depends on").unwrap_or("");
        let depends_on = if matches!(depends.to_lowercase().as_str(), "" | "none" | "-") {
            Vec::new()
        } else {
            task_refs(depends)
        };
        Some(Task {
            number: number.to_string(),
            file: path.file_name()?.to_string_lossy().into_owned(),
            title,
            status_key: StatusKey::from_status(&status),
            status,
            requirements: requirement_ids(header.field("requirements").unwrap_or("")),
            depends_on,
            parallel_group: header.field("parallel group").unwrap_or("").to_string(),
            remote: parse_remote(&body),
            updated: mtime(path),
            body,
        })
    }
}

/// Tasks in `<project>/tasks`, ordered by file name. PLAN.md and
/// misnamed files are ignored.
pub fn load_tasks(project_root: &Path) -> Vec<Task> {
    let Ok(entries) = fs::read_dir(project_root.join("tasks")) else {
        return Vec::new();
    };
    let mut files: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = task_number(&name)?.to_string();
            let path = entry.path();
            path.is_file().then_some((number, path))
        })
        .collect();
    files.sort_by(|a, b| a.1.cmp(&b.1));
    files
        .iter()
        .filter_map(|(number, path)| Task::parse(path, number))
        .collect()
}

/// Two-digit prefix of `NN_lowercase_snake_case.md`.
fn task_number(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".md")?;
    let bytes = stem.as_bytes();
    let valid = bytes.len() >= 4
        && bytes[..2].iter().all(u8::is_ascii_digit)
        && bytes[2] == b'_'
        && bytes[3..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_');
    valid.then(|| &stem[..2])
}

/// Drop a leading `01:` / `01.` / `01 -` from a task heading.
fn strip_number(title: &str, number: &str) -> String {
    if let Some(rest) = title.strip_prefix(number) {
        if let Some(rest) = rest.trim_start().strip_prefix([':', '.', '-']) {
            return rest.trim_start().to_string();
        }
    }
    title.to_string()
}

/// The first non-empty `Remote task:` line anywhere in the file
/// (it may live under Handoff notes rather than in the header).
fn parse_remote(text: &str) -> Option<Remote> {
    const PREFIX: &str = "remote task:";
    text.lines().find_map(|line| {
        let head = line.get(..PREFIX.len())?;
        if !head.eq_ignore_ascii_case(PREFIX) {
            return None;
        }
        let value = line[PREFIX.len()..].trim();
        (!value.is_empty()).then(|| Remote {
            raw: value.to_string(),
            url: find_url(value).map(str::to_string),
            key: find_issue_key(value).map(str::to_string),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn recognizes_task_file_names() {
        assert_eq!(task_number("01_infra_foundations.md"), Some("01"));
        assert_eq!(task_number("PLAN.md"), None);
        assert_eq!(task_number("1_bad.md"), None);
        assert_eq!(task_number("01_Bad.md"), None);
    }

    #[test]
    fn strips_number_prefixes_from_titles() {
        assert_eq!(strip_number("01: Title", "01"), "Title");
        assert_eq!(strip_number("01 - Title", "01"), "Title");
        assert_eq!(strip_number("Title", "01"), "Title");
    }

    #[test]
    fn maps_statuses() {
        assert_eq!(
            StatusKey::from_status(" In Progress "),
            StatusKey::InProgress
        );
        assert_eq!(StatusKey::from_status("Completed"), StatusKey::Done);
        assert_eq!(StatusKey::from_status("Review"), StatusKey::Other);
    }

    #[test]
    fn loads_tasks_from_a_project() {
        let repo = TempRepo::new();
        let tasks = load_tasks(&repo.project());
        assert_eq!(tasks.len(), 2);

        let first = &tasks[0];
        assert_eq!(first.title, "Infrastructure foundations");
        assert_eq!(first.status_key, StatusKey::Done);
        assert_eq!(first.requirements, ["NFR-001", "REQ-002"]);
        assert!(first.depends_on.is_empty());
        assert_eq!(first.parallel_group, "A");
        let remote = first.remote.as_ref().unwrap();
        assert_eq!(remote.key.as_deref(), Some("ENG-42"));
        assert_eq!(
            remote.url.as_deref(),
            Some("https://linear.app/acme/issue/ENG-42/infra")
        );

        let second = &tasks[1];
        assert_eq!(second.status, "In Progress");
        assert_eq!(second.depends_on, ["01"]);
        assert!(second.remote.is_none());
    }
}
