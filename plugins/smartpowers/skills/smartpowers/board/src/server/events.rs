//! Server-Sent Events.
//!
//! - `stream`: for the web UI. Sends `ready`, then `change` whenever the
//!   workspace changes, plus a keepalive comment every 15 seconds.
//! - `agent_stream`: for agents. Sends every pending comment, then `ready`,
//!   then each new comment the moment it is posted. A comment is marked
//!   delivered once it has been written to an agent.

use std::collections::HashSet;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use super::hub::Hub;
use crate::agent::comment_block;
use crate::project::comments::{self, Comment};
use crate::project::Project;
use crate::watcher::Watcher;

const KEEPALIVE: Duration = Duration::from_secs(15);
/// Shorter for agents, so a closed connection stops counting as listening soon.
const AGENT_KEEPALIVE: Duration = Duration::from_secs(5);
const HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                    Cache-Control: no-store\r\nConnection: close\r\n\r\nretry: 2000\n";

/// Hold the connection open until the client disconnects.
pub fn stream(stream: &mut TcpStream, watcher: &Watcher) {
    if send(stream, &format!("{HEAD}event: ready\ndata: {{}}\n\n")).is_err() {
        return;
    }
    let mut seen = watcher.current();
    loop {
        let version = watcher.wait(seen, KEEPALIVE);
        let message = if version == seen {
            ": keepalive\n\n".to_string()
        } else {
            seen = version;
            format!("event: change\ndata: {version}\n\n")
        };
        if send(stream, &message).is_err() {
            return;
        }
    }
}

pub fn agent_stream(
    stream: &mut TcpStream,
    hub: &Hub,
    project_state: &Path,
    project: &Project,
    task: Option<&str>,
) {
    // Subscribe before reading pending comments so none slip through the gap.
    // A comment can then arrive both ways; `sent` drops the duplicate.
    let subscription = hub.subscribe(&project.summary.id, task);
    if send(stream, HEAD).is_err() {
        return;
    }
    let mut sent = HashSet::new();
    for comment in comments::pending(project_state, task) {
        if !send_comment(stream, project_state, project, &comment) {
            return;
        }
        sent.insert(comment.id);
    }
    if send(stream, "event: ready\ndata: {}\n\n").is_err() {
        return;
    }
    loop {
        match subscription.receiver.recv_timeout(AGENT_KEEPALIVE) {
            Ok(comment) => {
                if sent.insert(comment.id.clone())
                    && !send_comment(stream, project_state, project, &comment)
                {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if send(stream, ": keepalive\n\n").is_err() {
                    return;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Write one comment as plain text for the agent, then mark it delivered.
fn send_comment(
    stream: &mut TcpStream,
    project_state: &Path,
    project: &Project,
    comment: &Comment,
) -> bool {
    let data: String = comment_block(project, comment)
        .lines()
        .map(|line| format!("data: {line}\n"))
        .collect();
    let written = send(
        stream,
        &format!("event: comment\nid: {}\n{data}\n", comment.id),
    )
    .is_ok();
    if written {
        let _ = comments::mark_delivered(project_state, &comment.id);
    }
    written
}

fn send(stream: &mut TcpStream, text: &str) -> std::io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}
