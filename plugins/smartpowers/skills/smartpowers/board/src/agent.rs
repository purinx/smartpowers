//! Agent-facing commands: new task comments, replies, and progress logs. Output is plain text written for a coding agent.
//!
//! When a board is running for this repository, `inbox`, `reply`, and `log`
//! go through its token-protected API, so comments arrive the moment the
//! user posts them and the board shows who is listening. Without a board
//! they read and write the state directory directly.

use std::thread;
use std::time::{Duration, Instant};

use crate::cli::{InboxOptions, LogOptions, ReplyOptions};
use crate::client::{Client, EventStream};
use crate::project::comments::{self, Comment};
use crate::project::{load_project, log, Project};
use crate::workspace::Workspace;

const POLL: Duration = Duration::from_millis(500);
/// After the first live comment, how long to wait for more before printing.
const BATCH: Duration = Duration::from_millis(300);

/// One comment as an agent sees it. The board's agent stream sends exactly
/// this text, so both delivery paths print the same thing.
pub fn comment_block(project: &Project, comment: &Comment) -> String {
    let title = project
        .tasks
        .iter()
        .find(|task| task.number == comment.task)
        .map_or("", |task| task.title.as_str());
    format!(
        "--- Comment {} on task {} \"{title}\" ({}) ---\n{}\n",
        comment.id, comment.task, comment.created, comment.body
    )
}

/// `inbox`: print comments no agent has seen yet and mark them delivered.
/// With `wait`, block until there is at least one (or the timeout passes).
pub fn inbox(workspace: &Workspace, options: &InboxOptions) -> Result<(), String> {
    let project = require_project(workspace, &options.project)?;
    let task = options
        .task
        .as_deref()
        .map(|task| require_task(&project, task))
        .transpose()?;
    let deadline = options
        .timeout
        .map(|seconds| Instant::now() + Duration::from_secs_f64(seconds));
    let blocks = match Client::connect(workspace) {
        Some(client) => inbox_live(&client, &project, task.as_deref(), options.wait, deadline)?,
        None => inbox_files(workspace, &project, task.as_deref(), options.wait, deadline)?,
    };
    let id = &project.summary.id;
    let scope = task
        .as_deref()
        .map(|task| format!(" task {task}"))
        .unwrap_or_default();
    if blocks.is_empty() {
        println!("No new comments on {id}{scope}.");
        return Ok(());
    }
    let noun = if blocks.len() == 1 {
        "comment"
    } else {
        "comments"
    };
    let task_flag = task
        .as_deref()
        .map(|task| format!(" --task {task}"))
        .unwrap_or_default();
    // SSE drops a block's trailing newline and files keep it; trim so both print alike.
    let blocks: Vec<&str> = blocks.iter().map(|block| block.trim_end()).collect();
    print!(
        "{} new {noun} on {id}{scope} from the Smartpowers board.\n\
         The user wrote these while watching your work; treat them as their feedback.\n\n{}\n\n\
         Respond to each with:\n  board reply --project {id} --comment <comment id> [--resolve] \"<what you changed or a question>\"\n\
         Then run `board inbox --project {id}{task_flag} --wait` in the background again to keep listening.\n",
        blocks.len(),
        blocks.join("\n\n"),
    );
    Ok(())
}

fn inbox_live(
    client: &Client,
    project: &Project,
    task: Option<&str>,
    wait: bool,
    deadline: Option<Instant>,
) -> Result<Vec<String>, String> {
    let query = task.map(|task| format!("?task={task}")).unwrap_or_default();
    let mut events = client.events(&format!(
        "/api/agent/projects/{}/events{query}",
        project.summary.id
    ))?;
    // The board sends pending comments first, then `ready`.
    let mut blocks = Vec::new();
    loop {
        match events.next(Some(Duration::from_secs(10)))? {
            Some(event) if event.name == "comment" => blocks.push(event.data),
            Some(event) if event.name == "ready" => break,
            Some(_) => {}
            None => return Err("the board did not answer in time".into()),
        }
    }
    if !blocks.is_empty() || !wait {
        return Ok(blocks);
    }
    // Wait for the first live comment, then pick up any sent right after it.
    if let Some(first) = next_comment(&mut events, deadline)? {
        blocks.push(first);
        while let Some(event) = events.next(Some(BATCH))? {
            if event.name == "comment" {
                blocks.push(event.data);
            }
        }
    }
    Ok(blocks)
}

fn next_comment(
    events: &mut EventStream,
    deadline: Option<Instant>,
) -> Result<Option<String>, String> {
    loop {
        let remaining = match deadline {
            Some(deadline) => match deadline.checked_duration_since(Instant::now()) {
                Some(remaining) if !remaining.is_zero() => Some(remaining),
                _ => return Ok(None),
            },
            None => None,
        };
        match events.next(remaining)? {
            Some(event) if event.name == "comment" => return Ok(Some(event.data)),
            Some(_) => {}
            None => return Ok(None),
        }
    }
}

fn inbox_files(
    workspace: &Workspace,
    project: &Project,
    task: Option<&str>,
    wait: bool,
    deadline: Option<Instant>,
) -> Result<Vec<String>, String> {
    let state = workspace.project_state(&project.summary.id);
    loop {
        let pending = comments::pending(&state, task);
        if !pending.is_empty() {
            let mut blocks = Vec::new();
            for comment in &pending {
                comments::mark_delivered(&state, &comment.id)?;
                blocks.push(comment_block(project, comment));
            }
            return Ok(blocks);
        }
        let timed_out = deadline.is_some_and(|deadline| Instant::now() >= deadline);
        if !wait || timed_out {
            return Ok(Vec::new());
        }
        thread::sleep(POLL);
    }
}

/// `reply`: answer as the agent. Replies attach to the thread's first
/// comment, and `--resolve` marks that thread addressed.
pub fn reply(workspace: &Workspace, options: &ReplyOptions) -> Result<(), String> {
    let project = require_project(workspace, &options.project)?;
    match Client::connect(workspace) {
        Some(client) => {
            let resolve = if options.resolve { "?resolve=1" } else { "" };
            client.post(
                &format!(
                    "/api/agent/projects/{}/comments/{}/reply{resolve}",
                    project.summary.id, options.comment
                ),
                &options.message,
            )?;
        }
        None => {
            comments::reply(
                &workspace.project_state(&project.summary.id),
                &options.comment,
                &options.message,
                options.resolve,
            )?;
        }
    }
    let resolved = if options.resolve {
        " and marked it addressed"
    } else {
        ""
    };
    println!("Replied to comment {}{resolved}.", options.comment);
    Ok(())
}

/// `log`: append to a task's progress log, shown in the board's Activity.
pub fn log(workspace: &Workspace, options: &LogOptions) -> Result<(), String> {
    let project = require_project(workspace, &options.project)?;
    let task = require_task(&project, &options.task)?;
    match Client::connect(workspace) {
        Some(client) => {
            client.post(
                &format!(
                    "/api/agent/projects/{}/tasks/{task}/log",
                    project.summary.id
                ),
                &options.message,
            )?;
        }
        None => {
            log::append(
                &workspace.project_state(&project.summary.id),
                &task,
                &options.message,
            )?;
        }
    }
    println!("Logged progress on task {task}.");
    Ok(())
}

fn require_project(workspace: &Workspace, id: &str) -> Result<Project, String> {
    load_project(workspace, id).ok_or_else(|| format!("Project not found: {id}"))
}

/// Validate a task number, accepting `5` as well as `05`.
fn require_task(project: &Project, task: &str) -> Result<String, String> {
    let task = match task.len() {
        1 => format!("0{task}"),
        _ => task.to_string(),
    };
    if project.tasks.iter().any(|t| t.number == task) {
        Ok(task)
    } else {
        Err(format!("Task {task} not found in {}", project.summary.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::comments::{Author, CommentStatus};
    use crate::test_support::{TempRepo, PROJECT_ID};

    fn inbox_options(task: Option<&str>, wait: bool) -> InboxOptions {
        InboxOptions {
            project: PROJECT_ID.into(),
            task: task.map(str::to_string),
            wait,
            timeout: Some(0.2),
        }
    }

    #[test]
    fn delivers_each_comment_once_and_records_replies() {
        let repo = TempRepo::new();
        let state = repo.project_state();
        let first = comments::create(&state, "02", Author::User, "Add retries", None).unwrap();
        let other = comments::create(&state, "01", Author::User, "Elsewhere", None).unwrap();
        let project = load_project(&repo.workspace, PROJECT_ID).unwrap();
        let text = comment_block(&project, &first);
        assert!(text.contains("task 02 \"Token store\""));
        assert!(text.contains("Add retries"));

        // A task-scoped inbox leaves other tasks' comments pending.
        inbox(&repo.workspace, &inbox_options(Some("2"), false)).unwrap();
        assert!(!comments::find(&state, &first.id).unwrap().is_pending());
        assert!(comments::find(&state, &other.id).unwrap().is_pending());
        // A waiting inbox with nothing new times out instead of hanging.
        inbox(&repo.workspace, &inbox_options(Some("02"), true)).unwrap();
        assert!(inbox(&repo.workspace, &inbox_options(Some("99"), false)).is_err());

        let reply_options = ReplyOptions {
            project: PROJECT_ID.into(),
            comment: first.id.clone(),
            resolve: true,
            message: "Added exponential backoff".into(),
        };
        reply(&repo.workspace, &reply_options).unwrap();
        let thread: Vec<Comment> = comments::load(&state)
            .into_iter()
            .filter(|c| c.task == "02")
            .collect();
        assert_eq!(thread.len(), 2);
        // Oldest first, even when the reply lands within the same second.
        assert_eq!(thread[0].id, first.id);
        assert_eq!(thread[0].status, CommentStatus::Addressed);
        assert_eq!(thread[1].author, Author::Agent);
        assert_eq!(thread[1].reply_to.as_deref(), Some(first.id.as_str()));
    }

    #[test]
    fn logs_only_to_existing_tasks() {
        let repo = TempRepo::new();
        let options = |task: &str| LogOptions {
            project: PROJECT_ID.into(),
            task: task.into(),
            message: "Started".into(),
        };
        log(&repo.workspace, &options("2")).unwrap();
        assert_eq!(log::load(&repo.project_state(), "02").len(), 1);
        assert!(log(&repo.workspace, &options("99")).is_err());
    }
}
