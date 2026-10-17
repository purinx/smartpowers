//! Agent-facing commands: read new task comments, reply to them, and log
//! progress. Output is plain text written for a coding agent to read.

use std::thread;
use std::time::{Duration, Instant};

use crate::cli::{InboxOptions, LogOptions, ReplyOptions};
use crate::project::comments::{self, Author, Comment, CommentStatus};
use crate::project::{load_project, log, Project};
use crate::workspace::Workspace;

const POLL: Duration = Duration::from_millis(500);

/// Print comments no agent has seen yet and mark them delivered. With
/// `wait`, block until there is at least one (or the timeout passes).
pub fn inbox(workspace: &Workspace, options: &InboxOptions) -> Result<(), String> {
    let project = require_project(workspace, &options.project)?;
    let state = workspace.project_state(&options.project);
    let deadline = options
        .timeout
        .map(|seconds| Instant::now() + Duration::from_secs_f64(seconds));
    loop {
        let pending: Vec<Comment> = comments::load(&state)
            .into_iter()
            .filter(Comment::is_pending)
            .collect();
        if !pending.is_empty() {
            print!("{}", describe(&project, &pending));
            for mut comment in pending {
                comment.delivered = true;
                comments::save(&state, &comment)?;
            }
            return Ok(());
        }
        let timed_out = deadline.is_some_and(|deadline| Instant::now() >= deadline);
        if !options.wait || timed_out {
            println!("No new comments on {}.", options.project);
            return Ok(());
        }
        thread::sleep(POLL);
    }
}

fn describe(project: &Project, pending: &[Comment]) -> String {
    let id = &project.summary.id;
    let noun = if pending.len() == 1 {
        "comment"
    } else {
        "comments"
    };
    let mut out = format!(
        "{} new {noun} on project {id} from the Smartpowers board.\n\
         The user wrote these while watching your work; treat them as their feedback.\n",
        pending.len()
    );
    for comment in pending {
        let title = project
            .tasks
            .iter()
            .find(|task| task.number == comment.task)
            .map_or("", |task| task.title.as_str());
        out.push_str(&format!(
            "\n--- Comment {} on task {} \"{title}\" ({}) ---\n{}\n",
            comment.id, comment.task, comment.created, comment.body
        ));
    }
    out.push_str(&format!(
        "\nRespond to each with:\n  board reply --project {id} --comment <comment id> [--resolve] \"<what you changed or a question>\"\n\
         Then run `board inbox --project {id} --wait` in the background again to keep listening.\n"
    ));
    out
}

/// Reply as the agent. Replies always attach to the thread's first comment;
/// `--resolve` marks that thread addressed.
pub fn reply(workspace: &Workspace, options: &ReplyOptions) -> Result<(), String> {
    require_project(workspace, &options.project)?;
    let state = workspace.project_state(&options.project);
    let target = comments::find(&state, &options.comment)
        .ok_or_else(|| format!("Comment not found: {}", options.comment))?;
    let mut root = match &target.reply_to {
        Some(root_id) => comments::find(&state, root_id)
            .ok_or_else(|| format!("Comment not found: {root_id}"))?,
        None => target,
    };
    comments::create(
        &state,
        &root.task,
        Author::Agent,
        &options.message,
        Some(&root.id),
    )?;
    root.delivered = true;
    if options.resolve {
        root.status = CommentStatus::Addressed;
    }
    comments::save(&state, &root)?;
    let resolved = if options.resolve {
        " and marked it addressed"
    } else {
        ""
    };
    println!(
        "Replied to comment {} on task {}{resolved}.",
        root.id, root.task
    );
    Ok(())
}

/// Append to a task's progress log, shown in the board's Activity section.
pub fn log(workspace: &Workspace, options: &LogOptions) -> Result<(), String> {
    let project = require_project(workspace, &options.project)?;
    let task = normalize_task(&options.task);
    if !project.tasks.iter().any(|t| t.number == task) {
        return Err(format!("Task {task} not found in {}", options.project));
    }
    log::append(
        &workspace.project_state(&options.project),
        &task,
        &options.message,
    )?;
    println!("Logged progress on task {task}.");
    Ok(())
}

fn require_project(workspace: &Workspace, id: &str) -> Result<Project, String> {
    load_project(workspace, id).ok_or_else(|| format!("Project not found: {id}"))
}

/// Accept `5` as well as `05`.
fn normalize_task(task: &str) -> String {
    match task.len() {
        1 => format!("0{task}"),
        _ => task.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempRepo, PROJECT_ID};

    fn inbox_options(wait: bool) -> InboxOptions {
        InboxOptions {
            project: PROJECT_ID.into(),
            wait,
            timeout: Some(0.2),
        }
    }

    #[test]
    fn delivers_each_comment_once_and_records_replies() {
        let repo = TempRepo::new();
        let state = repo.project_state();
        let first = comments::create(&state, "02", Author::User, "Add retries", None).unwrap();
        let project = load_project(&repo.workspace, PROJECT_ID).unwrap();
        let text = describe(&project, std::slice::from_ref(&first));
        assert!(text.contains("task 02 \"Token store\""));
        assert!(text.contains("Add retries"));

        inbox(&repo.workspace, &inbox_options(false)).unwrap();
        assert!(!comments::find(&state, &first.id).unwrap().is_pending());
        // A waiting inbox with nothing new times out instead of hanging.
        inbox(&repo.workspace, &inbox_options(true)).unwrap();

        let reply_options = ReplyOptions {
            project: PROJECT_ID.into(),
            comment: first.id.clone(),
            resolve: true,
            message: "Added exponential backoff".into(),
        };
        reply(&repo.workspace, &reply_options).unwrap();
        let all = comments::load(&state);
        assert_eq!(all.len(), 2);
        // Oldest first, even when the reply lands within the same second.
        assert_eq!(all[0].id, first.id);
        assert_eq!(all[0].status, CommentStatus::Addressed);
        assert_eq!(all[1].author, Author::Agent);
        assert_eq!(all[1].reply_to.as_deref(), Some(first.id.as_str()));
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
