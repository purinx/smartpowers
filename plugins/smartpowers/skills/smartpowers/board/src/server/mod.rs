//! HTTP server: one thread per connection, loopback only.
//!
//! Web UI routes (writes require a loopback `Origin`):
//! - `GET  /api/projects`                          project list (sidebar)
//! - `GET  /api/projects/<id>`                     one project with phases and tasks
//! - `POST /api/projects/<id>/tasks/<n>/comments`  add a comment (text/plain body)
//! - `GET  /api/events`                            Server-Sent Events on changes
//! - `GET  anything else`                          embedded web UI files
//!
//! Agent routes (require `Authorization: Bearer <token>` from
//! `<state dir>/board-server`):
//! - `GET  /api/agent/projects/<id>/events[?task=<n>]`        comments as SSE
//! - `POST /api/agent/projects/<id>/tasks/<n>/log`            progress entry
//! - `POST /api/agent/projects/<id>/comments/<c>/reply[?resolve=1]`
//!
//! Every write goes to the state directory, never to the repository.

mod api;
mod assets;
mod events;
mod http;
mod hub;

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::project::comments::{self, Author};
use crate::project::{self, load_project, log, Project};
use crate::watcher::Watcher;
use crate::workspace::Workspace;
pub use http::is_loopback;
use http::{Request, Response};
use hub::Hub;

/// Everything request handlers share.
pub struct Context {
    pub workspace: Arc<Workspace>,
    pub watcher: Arc<Watcher>,
    hub: Hub,
    token: String,
}

impl Context {
    pub fn new(workspace: Arc<Workspace>, watcher: Arc<Watcher>, token: String) -> Self {
        Context {
            hub: Hub::new(Arc::clone(&watcher)),
            workspace,
            watcher,
            token,
        }
    }
}

/// Accept connections forever.
pub fn serve(listener: TcpListener, context: Arc<Context>) {
    for stream in listener.incoming().flatten() {
        let context = Arc::clone(&context);
        thread::spawn(move || handle(stream, &context));
    }
}

fn handle(mut stream: TcpStream, context: &Context) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = http::read_request(&stream) else {
        return;
    };
    if !request.is_trusted() {
        return Response::text("403 Forbidden", "Forbidden").send(&mut stream);
    }
    let path = request.path.clone();
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if let ["api", "agent", rest @ ..] = parts.as_slice() {
        return agent(&request, rest, context, &mut stream);
    }
    match (request.method.as_str(), parts.as_slice()) {
        ("GET", ["api", "events"]) => events::stream(&mut stream, &context.watcher),
        ("GET", _) => get(&request.path, context).send(&mut stream),
        ("POST", ["api", "projects", id, "tasks", task, "comments"]) => {
            post_comment(&request, id, task, context).send(&mut stream)
        }
        ("POST", _) => Response::text("404 Not Found", "Not found").send(&mut stream),
        _ => Response::text("405 Method Not Allowed", "Method not allowed").send(&mut stream),
    }
}

fn get(path: &str, context: &Context) -> Response {
    let workspace = &context.workspace;
    if path == "/api/projects" {
        let projects = project::list_projects(workspace);
        return Response::json("200 OK", &api::projects(&workspace.repo_name(), &projects));
    }
    if let Some(id) = path.strip_prefix("/api/projects/") {
        return match load_project(workspace, id) {
            Some(project) => {
                let listeners = context.hub.listeners(id);
                Response::json("200 OK", &api::project(&project, &listeners))
            }
            None => not_found("Project not found"),
        };
    }
    match assets::find(path) {
        Some(asset) => Response::new("200 OK", asset.content_type, asset.body.as_bytes()),
        None => Response::text("404 Not Found", "Not found"),
    }
}

fn post_comment(request: &Request, id: &str, task: &str, context: &Context) -> Response {
    // Browsers send Origin on every POST; its absence means a non-browser
    // client, which must use the token-protected agent API instead.
    if !request.has_origin() {
        return Response::text("403 Forbidden", "Forbidden");
    }
    let (_, body) = match target(request, id, Some(task), context) {
        Ok(found) => found,
        Err(response) => return response,
    };
    match comments::create(
        &context.workspace.project_state(id),
        task,
        Author::User,
        body,
        None,
    ) {
        Ok(comment) => {
            context.hub.publish(id, &comment);
            Response::json("201 Created", &api::comment(&comment))
        }
        Err(message) => bad_request(&message),
    }
}

fn agent(request: &Request, parts: &[&str], context: &Context, stream: &mut TcpStream) {
    let expected = format!("Bearer {}", context.token);
    if request.header("authorization") != Some(expected.as_str()) {
        return Response::json(
            "401 Unauthorized",
            &api::error("Missing or wrong board token"),
        )
        .send(stream);
    }
    let state = |id: &str| context.workspace.project_state(id);
    let response = match (request.method.as_str(), parts) {
        ("GET", ["projects", id, "events"]) => {
            let task = request.query_param("task");
            return match target(request, id, task, context) {
                Ok((project, _)) => {
                    events::agent_stream(stream, &context.hub, &state(id), &project, task)
                }
                Err(response) => response.send(stream),
            };
        }
        ("POST", ["projects", id, "tasks", task, "log"]) => {
            match target(request, id, Some(task), context) {
                Ok((_, body)) => match log::append(&state(id), task, body) {
                    Ok(entry) => Response::json("201 Created", &api::log_entry(&entry)),
                    Err(message) => bad_request(&message),
                },
                Err(response) => response,
            }
        }
        ("POST", ["projects", id, "comments", comment, "reply"]) => {
            match target(request, id, None, context) {
                Ok((_, body)) => {
                    let resolve = request.query_param("resolve") == Some("1");
                    match comments::reply(&state(id), comment, body, resolve) {
                        Ok((_, reply)) => Response::json("201 Created", &api::comment(&reply)),
                        Err(message) if message.starts_with("Comment not found") => {
                            not_found(&message)
                        }
                        Err(message) => bad_request(&message),
                    }
                }
                Err(response) => response,
            }
        }
        _ => Response::text("404 Not Found", "Not found"),
    };
    response.send(stream);
}

/// Resolve the project (and task, when given) a request targets, and its
/// UTF-8 body. Errors come back as the response to send.
fn target<'r>(
    request: &'r Request,
    id: &str,
    task: Option<&str>,
    context: &Context,
) -> Result<(Project, &'r str), Response> {
    let project =
        load_project(&context.workspace, id).ok_or_else(|| not_found("Project not found"))?;
    if let Some(task) = task {
        if !project.tasks.iter().any(|t| t.number == task) {
            return Err(not_found("Task not found"));
        }
    }
    if request.body_too_large {
        return Err(Response::json(
            "413 Payload Too Large",
            &api::error("Request body is too large"),
        ));
    }
    let body =
        std::str::from_utf8(&request.body).map_err(|_| bad_request("Body must be UTF-8 text"))?;
    Ok((project, body))
}

fn not_found(message: &str) -> Response {
    Response::json("404 Not Found", &api::error(message))
}

fn bad_request(message: &str) -> Response {
    Response::json("400 Bad Request", &api::error(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempRepo, PROJECT_ID};
    use std::io::{BufRead, BufReader, Read, Write};

    const TOKEN: &str = "test-token";

    fn send(port: u16, method: &str, path: &str, headers: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let host = if headers.contains("Host:") {
            String::new()
        } else {
            format!("Host: 127.0.0.1:{port}\r\n")
        };
        let length = if body.is_empty() {
            String::new()
        } else {
            format!("Content-Length: {}\r\n", body.len())
        };
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\n{host}{length}{headers}\r\n{body}"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    fn start(repo: &TempRepo) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let workspace = Arc::new(Workspace::with_state_dir(
            repo.root.clone(),
            repo.workspace.state_dir.clone(),
        ));
        let context = Arc::new(Context::new(workspace, Watcher::new(), TOKEN.into()));
        thread::spawn(move || serve(listener, context));
        port
    }

    fn auth() -> String {
        format!("Authorization: Bearer {TOKEN}\r\n")
    }

    #[test]
    fn serves_api_and_ui_and_rejects_foreign_origins() {
        let repo = TempRepo::new();
        let port = start(&repo);
        let get = |path: &str, headers: &str| send(port, "GET", path, headers, "");

        let projects = get("/api/projects", "");
        assert!(projects.starts_with("HTTP/1.1 200"));
        assert!(projects.contains(PROJECT_ID));
        assert!(get(&format!("/api/projects/{PROJECT_ID}"), "").contains("Token store"));
        assert!(get("/", "").contains("js/main.js"));
        assert!(get("/js/main.js", "").contains("text/javascript"));
        assert!(get("/api/projects/nope", "").starts_with("HTTP/1.1 404"));
        assert!(get("/missing", "").starts_with("HTTP/1.1 404"));

        let forbidden = |headers: &str| get("/api/projects", headers).starts_with("HTTP/1.1 403");
        assert!(forbidden("Host: evil.example\r\n"));
        assert!(forbidden("Origin: https://evil.example\r\n"));
        assert!(!forbidden("Origin: http://localhost:1234\r\n"));
    }

    #[test]
    fn posts_comments_only_from_the_board_origin() {
        let repo = TempRepo::new();
        let port = start(&repo);
        let path = format!("/api/projects/{PROJECT_ID}/tasks/02/comments");
        let local = "Origin: http://localhost:1\r\n";
        let post = |path: &str, headers: &str, body: &str| send(port, "POST", path, headers, body);

        let created = post(&path, local, "Please add retries");
        assert!(created.starts_with("HTTP/1.1 201"), "{created}");
        assert!(created.contains(r#""body":"Please add retries""#));
        let detail = send(port, "GET", &format!("/api/projects/{PROJECT_ID}"), "", "");
        assert!(detail.contains("Please add retries"));

        // No Origin (e.g. curl from another tool) or a foreign one: rejected.
        assert!(post(&path, "", "x").starts_with("HTTP/1.1 403"));
        assert!(post(&path, "Origin: https://evil.example\r\n", "x").starts_with("HTTP/1.1 403"));
        assert!(post(&path, local, "  ").starts_with("HTTP/1.1 400"));
        let missing_task = format!("/api/projects/{PROJECT_ID}/tasks/99/comments");
        assert!(post(&missing_task, local, "x").starts_with("HTTP/1.1 404"));
        assert_eq!(comments::load(&repo.project_state()).len(), 1);
    }

    #[test]
    fn agent_api_requires_the_token() {
        let repo = TempRepo::new();
        let port = start(&repo);
        let log_path = format!("/api/agent/projects/{PROJECT_ID}/tasks/02/log");
        let rejected = send(
            port,
            "POST",
            &log_path,
            "Authorization: Bearer wrong\r\n",
            "x",
        );
        assert!(rejected.starts_with("HTTP/1.1 401"), "{rejected}");
        assert!(send(port, "POST", &log_path, "", "x").starts_with("HTTP/1.1 401"));

        let logged = send(port, "POST", &log_path, &auth(), "Started");
        assert!(logged.starts_with("HTTP/1.1 201"), "{logged}");
        assert_eq!(log::load(&repo.project_state(), "02")[0].message, "Started");

        let comment =
            comments::create(&repo.project_state(), "02", Author::User, "Hi", None).unwrap();
        let reply_path = format!(
            "/api/agent/projects/{PROJECT_ID}/comments/{}/reply?resolve=1",
            comment.id
        );
        assert!(send(port, "POST", &reply_path, &auth(), "Done").starts_with("HTTP/1.1 201"));
        let root = comments::find(&repo.project_state(), &comment.id).unwrap();
        assert_eq!(root.status, comments::CommentStatus::Addressed);
    }

    #[test]
    fn streams_comments_to_a_listening_agent() {
        let repo = TempRepo::new();
        let port = start(&repo);
        let pending =
            comments::create(&repo.project_state(), "02", Author::User, "Earlier", None).unwrap();

        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(
            stream,
            "GET /api/agent/projects/{PROJECT_ID}/events?task=02 HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{}\r\n",
            auth()
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut read_until = |marker: &str| {
            let mut seen = String::new();
            loop {
                let mut line = String::new();
                assert!(
                    reader.read_line(&mut line).unwrap() > 0,
                    "stream closed: {seen}"
                );
                seen.push_str(&line);
                if line.contains(marker) {
                    return seen;
                }
            }
        };
        // Pending comments first, then `ready`.
        let initial = read_until("event: ready");
        assert!(initial.contains("data: Earlier"), "{initial}");
        assert!(
            comments::find(&repo.project_state(), &pending.id)
                .unwrap()
                .delivered
        );

        // The board shows task 02 as having a listener.
        let detail = send(port, "GET", &format!("/api/projects/{PROJECT_ID}"), "", "");
        assert!(detail.contains(r#""listening":true"#), "{detail}");

        // A new comment from the board arrives immediately; one on another task does not.
        let local = "Origin: http://localhost:1\r\n";
        send(
            port,
            "POST",
            &format!("/api/projects/{PROJECT_ID}/tasks/01/comments"),
            local,
            "Other task",
        );
        send(
            port,
            "POST",
            &format!("/api/projects/{PROJECT_ID}/tasks/02/comments"),
            local,
            "Use retries",
        );
        let live = read_until("data: Use retries");
        assert!(!live.contains("Other task"), "{live}");
    }
}
