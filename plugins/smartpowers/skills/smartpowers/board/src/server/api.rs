//! JSON shapes for the web UI. Field names here are the contract with
//! `assets/board/js`; change both sides together.

use crate::json::Json;
use crate::project::{Project, ProjectSummary, Remote, Task};
use crate::time::iso_utc;

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

/// `GET /api/projects/<id>`: the summary fields plus phases, path, and tasks.
pub fn project(project: &Project) -> Json {
    let mut fields = summary_fields(&project.summary);
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
    fields.push(("tasks", Json::Arr(project.tasks.iter().map(task).collect())));
    Json::Obj(fields)
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

fn task(task: &Task) -> Json {
    Json::Obj(vec![
        ("number", Json::str(&task.number)),
        ("file", Json::str(&task.file)),
        ("title", Json::str(&task.title)),
        ("status", Json::str(&task.status)),
        ("statusKey", Json::str(task.status_key.as_str())),
        ("requirements", Json::strings(&task.requirements)),
        ("dependsOn", Json::strings(&task.depends_on)),
        ("parallelGroup", Json::str(&task.parallel_group)),
        ("remote", task.remote.as_ref().map_or(Json::Null, remote)),
        ("updated", Json::Str(iso_utc(task.updated))),
        ("body", Json::str(&task.body)),
    ])
}

fn remote(remote: &Remote) -> Json {
    Json::Obj(vec![
        ("raw", Json::str(&remote.raw)),
        ("url", Json::opt_str(remote.url.as_deref())),
        ("key", Json::opt_str(remote.key.as_deref())),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::load_project;
    use crate::test_support::{TempRepo, PROJECT_ID};

    #[test]
    fn renders_the_project_contract() {
        let repo = TempRepo::new();
        let json = project(&load_project(&repo.root, PROJECT_ID).unwrap()).render();
        for expected in [
            r#""title":"OAuth refresh""#,
            r#""counts":{"in_progress":1,"done":1}"#,
            r#"{"name":"Design","file":"DESIGN_DOC.md","status":"Draft"}"#,
            r#""statusKey":"done""#,
            r#""requirements":["NFR-001","REQ-002"]"#,
            r#""dependsOn":["01"]"#,
            r#""key":"ENG-42""#,
            r#""remote":null"#,
        ] {
            assert!(json.contains(expected), "missing {expected} in {json}");
        }
    }
}
