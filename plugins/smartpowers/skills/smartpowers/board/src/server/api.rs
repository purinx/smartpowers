//! JSON shapes for the web UI. Field names here are the contract with
//! `assets/board/js`; change both sides together.

use crate::json::Json;
use crate::project::comments::Comment;
use crate::project::graph::{Graph, GraphIssue};
use crate::project::log::LogEntry;
use crate::project::{Project, ProjectSummary, Remote, Task};
use crate::time::iso_utc;

use super::hub::Listeners;

/// `GET /api/projects`
pub fn projects(repo_name: &str, projects: &[ProjectSummary]) -> Json {
    Json::Obj(vec![
        ("repo", Json::str(repo_name)),
        (
            "projects",
            Json::Arr(
                projects
                    .iter()
                    .map(|p| Json::Obj(summary_fields(p)))
                    .collect(),
            ),
        ),
    ])
}

/// `GET /api/projects/<id>`: the summary fields plus phases, path, tasks
/// with their graph depth and readiness, graph issues, and which tasks have
/// an agent listening for comments.
pub fn project(project: &Project, listeners: &Listeners) -> Json {
    let mut fields = summary_fields(&project.summary);
    fields.push(("agentListening", Json::Bool(listeners.all_tasks)));
    fields.push((
        "phases",
        Json::Arr(
            project
                .phases
                .iter()
                .map(|phase| {
                    Json::Obj(vec![
                        ("name", Json::str(phase.name)),
                        ("file", Json::str(phase.file)),
                        ("status", Json::opt_str(phase.status.as_deref())),
                    ])
                })
                .collect(),
        ),
    ));
    fields.push(("path", Json::str(&project.path)));
    fields.push((
        "tasks",
        Json::Arr(
            project
                .tasks
                .iter()
                .map(|t| task(t, &project.graph, listeners.tasks.contains(&t.number)))
                .collect(),
        ),
    ));
    fields.push((
        "graphIssues",
        Json::Arr(project.graph.issues.iter().map(graph_issue).collect()),
    ));
    Json::Obj(fields)
}

/// One comment, as listed under a task and returned by
/// `POST /api/projects/<id>/tasks/<n>/comments`.
pub fn comment(comment: &Comment) -> Json {
    Json::Obj(vec![
        ("id", Json::str(&comment.id)),
        ("author", Json::str(comment.author.as_str())),
        ("created", Json::str(&comment.created)),
        ("status", Json::str(comment.status.as_str())),
        ("delivered", Json::Bool(comment.delivered)),
        ("replyTo", Json::opt_str(comment.reply_to.as_deref())),
        ("body", Json::str(&comment.body)),
    ])
}

pub fn error(message: &str) -> Json {
    Json::Obj(vec![("error", Json::str(message))])
}

fn summary_fields(summary: &ProjectSummary) -> Vec<(&'static str, Json)> {
    let counts = summary
        .counts
        .iter()
        .map(|(key, count)| (key.as_str(), Json::Num(*count)))
        .collect();
    vec![
        ("id", Json::str(&summary.id)),
        ("title", Json::str(&summary.title)),
        ("status", Json::str(&summary.status)),
        ("updated", Json::Str(iso_utc(summary.updated))),
        ("taskCount", Json::Num(summary.task_count as u64)),
        ("counts", Json::Obj(counts)),
    ]
}

fn task(task: &Task, graph: &Graph, listening: bool) -> Json {
    let depth = graph.depth.get(&task.number).copied().unwrap_or(0);
    Json::Obj(vec![
        ("number", Json::str(&task.number)),
        ("file", Json::str(&task.file)),
        ("title", Json::str(&task.title)),
        ("status", Json::str(&task.status)),
        ("statusKey", Json::str(task.status_key.as_str())),
        ("requirements", Json::strings(&task.requirements)),
        ("dependsOn", Json::strings(&task.depends_on)),
        ("depth", Json::Num(u64::from(depth))),
        ("ready", Json::Bool(graph.ready.contains(&task.number))),
        ("remote", task.remote.as_ref().map_or(Json::Null, remote)),
        ("updated", Json::Str(iso_utc(task.updated))),
        ("body", Json::str(&task.body)),
        (
            "comments",
            Json::Arr(task.comments.iter().map(comment).collect()),
        ),
        ("log", Json::Arr(task.log.iter().map(log_entry).collect())),
        ("listening", Json::Bool(listening)),
    ])
}

fn graph_issue(issue: &GraphIssue) -> Json {
    match issue {
        GraphIssue::MissingDependency { task, dependency } => Json::Obj(vec![
            ("kind", Json::str("missingDependency")),
            ("task", Json::str(task)),
            ("dependency", Json::str(dependency)),
        ]),
        GraphIssue::Cycle { tasks } => Json::Obj(vec![
            ("kind", Json::str("cycle")),
            ("tasks", Json::strings(tasks)),
        ]),
    }
}

fn remote(remote: &Remote) -> Json {
    Json::Obj(vec![
        ("raw", Json::str(&remote.raw)),
        ("url", Json::opt_str(remote.url.as_deref())),
        ("key", Json::opt_str(remote.key.as_deref())),
    ])
}

pub fn log_entry(entry: &LogEntry) -> Json {
    Json::Obj(vec![
        ("time", Json::str(&entry.time)),
        ("message", Json::str(&entry.message)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::project::comments::{self, Author};
    use crate::project::{load_project, log};
    use crate::test_support::{TempRepo, PROJECT_ID};

    #[test]
    fn renders_the_project_contract() {
        let repo = TempRepo::new();
        comments::create(&repo.project_state(), "02", Author::User, "Hi", None).unwrap();
        log::append(&repo.project_state(), "02", "Started").unwrap();
        let mut listeners = Listeners::default();
        listeners.tasks.insert("02".into());
        let project_data = load_project(&repo.workspace, PROJECT_ID).unwrap();
        let json = project(&project_data, &listeners).render();
        for expected in [
            r#""title":"OAuth refresh""#,
            r#""counts":{"in_progress":1,"done":1}"#,
            r#"{"name":"Design","file":"DESIGN_DOC.md","status":"Draft"}"#,
            r#""statusKey":"done""#,
            r#""requirements":["NFR-001","REQ-002"]"#,
            r#""dependsOn":["01"]"#,
            r#""key":"ENG-42""#,
            r#""remote":null"#,
            r#""author":"user","#,
            r#""status":"open","delivered":false,"replyTo":null,"body":"Hi""#,
            r#""message":"Started""#,
            r#""agentListening":false"#,
            r#""listening":true}"#,
            r#""dependsOn":["01"],"depth":1,"ready":false,"#,
            r#""graphIssues":[]"#,
        ] {
            assert!(json.contains(expected), "missing {expected} in {json}");
        }
        assert!(!json.contains("parallelGroup"), "{json}");
    }

    #[test]
    fn renders_graph_readiness_and_issues() {
        let repo = TempRepo::new();
        let write = |file: &str, text: &str| fs::write(repo.project().join(file), text).unwrap();
        write(
            "tasks/02_token_store.md",
            "# 02: Token store\n\nStatus: Planned\nDepends on: 01\n",
        );
        write(
            "tasks/03_cache.md",
            "# 03: Cache\n\nStatus: Planned\nDepends on: [left](02_token_store.md), 09\n",
        );
        write(
            "tasks/04_a.md",
            "# 04: A\n\nStatus: Planned\nDepends on: 05\n",
        );
        write(
            "tasks/05_b.md",
            "# 05: B\n\nStatus: Planned\nDepends on: 04\n",
        );
        let project_data = load_project(&repo.workspace, PROJECT_ID).unwrap();
        let json = project(&project_data, &Listeners::default()).render();
        for expected in [
            r#""dependsOn":["01"],"depth":1,"ready":true,"#,
            r#""dependsOn":["02","09"],"depth":2,"ready":false,"#,
            concat!(
                r#""graphIssues":[{"kind":"missingDependency","task":"03","dependency":"09"},"#,
                r#"{"kind":"cycle","tasks":["04","05"]}]"#
            ),
        ] {
            assert!(json.contains(expected), "missing {expected} in {json}");
        }
    }
}
