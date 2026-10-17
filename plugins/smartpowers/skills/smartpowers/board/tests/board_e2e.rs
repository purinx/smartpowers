//! Black-box checks of the built board: the HTTP API, the agent subcommands,
//! and live delivery between them. Parsing and server internals are covered
//! by the unit tests in `src/`. The launcher scripts are exercised by CI's
//! "Launcher without Rust" job.

mod support;

use std::process::Stdio;
use std::thread::sleep;
use std::time::{Duration, Instant};

use support::{stderr, stdout, Json, TempRepo, BOARD, PROJECT_ID};

#[test]
fn serves_project_and_rejects_foreign_host() {
    let repo = TempRepo::new();
    let server = repo.serve();
    let task = &server.project()["tasks"][0];
    assert_eq!(task["title"].str(), "Infrastructure foundations");
    assert_eq!(task["statusKey"].str(), "in_progress");

    let response = server.request("GET", "/api/projects", &[("Host", "evil.example")], "");
    assert_eq!(response.status, 403);
}

#[test]
fn comment_feedback_loop_with_the_agent() {
    let repo = TempRepo::new();
    let server = repo.serve();
    let origin = server.origin();
    let response = server.request(
        "POST",
        &format!("/api/projects/{PROJECT_ID}/tasks/01/comments"),
        &[
            ("Origin", &origin),
            ("Content-Type", "text/plain; charset=utf-8"),
        ],
        "Use the existing HTTP client",
    );
    assert_eq!(response.status, 201, "{}", response.body);
    let comment_id = response.json()["id"].str().to_string();

    let inbox = repo.board(&["inbox"]);
    assert!(inbox.status.success(), "{}", stderr(&inbox));
    let text = stdout(&inbox);
    assert!(
        text.contains(&format!("Comment {comment_id} on task 01")),
        "{text}"
    );
    assert!(text.contains("Use the existing HTTP client"), "{text}");
    // Each comment is delivered once.
    assert!(stdout(&repo.board(&["inbox"])).contains("No new comments"));

    let reply = repo.board(&[
        "reply",
        "--comment",
        &comment_id,
        "--resolve",
        "Switched to it",
    ]);
    assert!(reply.status.success(), "{}", stderr(&reply));
    let logged = repo.board(&["log", "--task", "01", "Swapped the client; tests pass"]);
    assert!(logged.status.success(), "{}", stderr(&logged));

    let project = server.project();
    let task = &project["tasks"][0];
    assert_eq!(task["comments"].len(), 2);
    let (root, answer) = (&task["comments"][0], &task["comments"][1]);
    assert_eq!(root["status"].str(), "addressed");
    assert_eq!(answer["author"].str(), "agent");
    assert_eq!(answer["replyTo"].str(), comment_id);
    assert_eq!(
        task["log"][0]["message"].str(),
        "Swapped the client; tests pass"
    );
    // Nothing was written inside the repository.
    for path in repo.repo_files() {
        let name = path.to_string_lossy();
        assert!(
            !name.ends_with(".tmp") && !path.components().any(|c| c.as_os_str() == "comments"),
            "unexpected file in the repository: {name}"
        );
    }
}

#[test]
fn subagent_receives_comments_live() {
    let repo = TempRepo::new();
    let server = repo.serve();
    let mut listener = repo
        .command(&["inbox", "--task", "01", "--wait", "--timeout", "60"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // The board shows the task as having a listening agent once it connects.
    let deadline = Instant::now() + Duration::from_secs(30);
    while server.project()["tasks"][0]["listening"] != Json::Bool(true) {
        if Instant::now() > deadline {
            let _ = listener.kill();
            panic!("the inbox never connected to the board");
        }
        sleep(Duration::from_millis(200));
    }

    let sent = Instant::now();
    let origin = server.origin();
    let response = server.request(
        "POST",
        &format!("/api/projects/{PROJECT_ID}/tasks/01/comments"),
        &[("Origin", &origin)],
        "Rename the module",
    );
    assert_eq!(response.status, 201, "{}", response.body);

    let deadline = Instant::now() + Duration::from_secs(30);
    while listener.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = listener.kill();
            panic!("the inbox never received the comment");
        }
        sleep(Duration::from_millis(20));
    }
    let elapsed = sent.elapsed();
    let output = listener.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Rename the module"), "{text}");
    assert!(text.contains("--task 01 --wait"), "{text}");
    // Pushed over SSE, not found by polling.
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

#[test]
fn next_lists_parallel_tasks_in_plan_order() {
    let repo = TempRepo::new();
    repo.write(
        "tasks/01_infra_foundations.md",
        &format!(
            "# 01: Infrastructure foundations\n\nStatus: Done\nProject: {PROJECT_ID}\n\
             Depends on: None\n"
        ),
    );
    for (number, name) in [("02", "domain"), ("03", "api")] {
        repo.write(
            &format!("tasks/{number}_{name}.md"),
            &format!(
                "# {number}: {name}\n\nStatus: Planned\nProject: {PROJECT_ID}\n\
                 Depends on: 01\nParallel group: B\n"
            ),
        );
    }
    repo.write(
        "tasks/PLAN.md",
        "| 01 | [a](01_infra_foundations.md) |\n| 03 | [c](03_api.md) |\n| 02 | [b](02_domain.md) |\n",
    );

    let result = repo.board(&["next"]);
    assert!(result.status.success(), "{}", stderr(&result));
    let text = stdout(&result);
    assert!(text.contains("Start now"), "{text}");
    let api = text.find("03  api").expect(&text);
    let domain = text.find("02  domain").expect(&text);
    assert!(api < domain, "{text}");
}

#[test]
fn rejects_non_loopback_host() {
    let repo = TempRepo::new();
    let output = std::process::Command::new(BOARD)
        .args(["--root"])
        .arg(&repo.root)
        .args(["--host", "0.0.0.0", "--no-open"])
        .env("SMARTPOWERS_STATE_HOME", &repo.state)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("loopback"), "{}", stderr(&output));
}
