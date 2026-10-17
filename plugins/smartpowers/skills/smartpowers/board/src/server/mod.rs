//! HTTP server: one thread per connection, loopback only.
//!
//! Routes:
//! - `GET  /api/projects`                          project list (sidebar)
//! - `GET  /api/projects/<id>`                     one project with phases and tasks
//! - `POST /api/projects/<id>/tasks/<n>/comments`  add a comment (text/plain body)
//! - `GET  /api/events`                            Server-Sent Events on changes
//! - `GET  anything else`                          embedded web UI files
//!
//! Adding comments is the only write. It goes to the state directory, never
//! to the repository.

mod api;
mod assets;
mod events;
mod http;

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::project::comments::{self, Author};
use crate::project::{self, load_project};
use crate::watcher::Watcher;
use crate::workspace::Workspace;
pub use http::is_loopback;
use http::{Request, Response};

/// Accept connections forever.
pub fn serve(listener: TcpListener, workspace: Arc<Workspace>, watcher: Arc<Watcher>) {
    for stream in listener.incoming().flatten() {
        let workspace = Arc::clone(&workspace);
        let watcher = Arc::clone(&watcher);
        thread::spawn(move || handle(stream, &workspace, &watcher));
    }
}

fn handle(mut stream: TcpStream, workspace: &Workspace, watcher: &Watcher) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = http::read_request(&stream) else {
        return;
    };
    if !request.is_trusted() {
        return Response::text("403 Forbidden", "Forbidden").send(&mut stream);
    }
    match request.method.as_str() {
        "GET" if request.path == "/api/events" => events::stream(&mut stream, watcher),
        "GET" => get(&request.path, workspace).send(&mut stream),
        "POST" => post(&request, workspace).send(&mut stream),
        _ => Response::text("405 Method Not Allowed", "Method not allowed").send(&mut stream),
    }
}

fn get(path: &str, workspace: &Workspace) -> Response {
    if path == "/api/projects" {
        let projects = project::list_projects(workspace);
        return Response::json("200 OK", &api::projects(&workspace.repo_name(), &projects));
    }
    if let Some(id) = path.strip_prefix("/api/projects/") {
        return match load_project(workspace, id) {
            Some(project) => Response::json("200 OK", &api::project(&project)),
            None => not_found("Project not found"),
        };
    }
    match assets::find(path) {
        Some(asset) => Response::new("200 OK", asset.content_type, asset.body.as_bytes()),
        None => Response::text("404 Not Found", "Not found"),
    }
}

fn post(request: &Request, workspace: &Workspace) -> Response {
    if !request.has_origin() {
        return Response::text("403 Forbidden", "Forbidden");
    }
    let Some((id, task)) = comment_target(&request.path) else {
        return Response::text("404 Not Found", "Not found");
    };
    let Some(project) = load_project(workspace, id) else {
        return not_found("Project not found");
    };
    if !project.tasks.iter().any(|t| t.number == task) {
        return not_found("Task not found");
    }
    if request.body_too_large {
        return Response::json("413 Payload Too Large", &api::error("Comment is too long"));
    }
    let Ok(body) = std::str::from_utf8(&request.body) else {
        return bad_request("Comment must be UTF-8 text");
    };
    match comments::create(&workspace.project_state(id), task, Author::User, body, None) {
        Ok(comment) => Response::json("201 Created", &api::comment(&comment)),
        Err(message) => bad_request(&message),
    }
}

/// `/api/projects/<id>/tasks/<n>/comments` -> `(id, n)`.
fn comment_target(path: &str) -> Option<(&str, &str)> {
    let (id, rest) = path.strip_prefix("/api/projects/")?.split_once('/')?;
    let task = rest.strip_prefix("tasks/")?.strip_suffix("/comments")?;
    Some((id, task))
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
    use std::io::{Read, Write};

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
        thread::spawn(move || serve(listener, workspace, Watcher::new()));
        port
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
    fn parses_comment_targets() {
        assert_eq!(
            comment_target("/api/projects/p/tasks/05/comments"),
            Some(("p", "05"))
        );
        assert_eq!(comment_target("/api/projects/p"), None);
        assert_eq!(comment_target("/api/projects/p/tasks/05"), None);
    }
}
