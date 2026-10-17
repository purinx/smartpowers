//! HTTP server: one thread per connection, GET only, loopback only.
//!
//! Routes:
//! - `GET /api/projects`       project list (sidebar)
//! - `GET /api/projects/<id>`  one project with phases and tasks
//! - `GET /api/events`         Server-Sent Events on file changes
//! - anything else             embedded web UI files

mod api;
mod assets;
mod events;
mod http;

use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::project;
use crate::watcher::Watcher;
pub use http::is_loopback;
use http::Response;

/// Accept connections forever.
pub fn serve(listener: TcpListener, repo_root: PathBuf, watcher: Arc<Watcher>) {
    let repo_root = Arc::new(repo_root);
    for stream in listener.incoming().flatten() {
        let repo_root = Arc::clone(&repo_root);
        let watcher = Arc::clone(&watcher);
        thread::spawn(move || handle(stream, &repo_root, &watcher));
    }
}

fn handle(mut stream: TcpStream, repo_root: &Path, watcher: &Watcher) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = http::read_request(&stream) else {
        return;
    };
    if !request.is_trusted() {
        return Response::text("403 Forbidden", "Forbidden").send(&mut stream);
    }
    if request.method != "GET" {
        return Response::text("405 Method Not Allowed", "Method not allowed").send(&mut stream);
    }
    if request.path == "/api/events" {
        return events::stream(&mut stream, watcher);
    }
    route(&request.path, repo_root).send(&mut stream);
}

fn route(path: &str, repo_root: &Path) -> Response {
    if path == "/api/projects" {
        let repo_name = repo_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let projects = project::list_projects(repo_root);
        return Response::json("200 OK", &api::projects(&repo_name, &projects));
    }
    if let Some(id) = path.strip_prefix("/api/projects/") {
        return match project::load_project(repo_root, id) {
            Some(project) => Response::json("200 OK", &api::project(&project)),
            None => Response::json("404 Not Found", &api::error("Project not found")),
        };
    }
    match assets::find(path) {
        Some(asset) => Response::new("200 OK", asset.content_type, asset.body.as_bytes()),
        None => Response::text("404 Not Found", "Not found"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{TempRepo, PROJECT_ID};
    use std::io::{Read, Write};

    fn get(port: u16, path: &str, extra_headers: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let host = if extra_headers.contains("Host:") {
            String::new()
        } else {
            format!("Host: 127.0.0.1:{port}\r\n")
        };
        write!(stream, "GET {path} HTTP/1.1\r\n{host}{extra_headers}\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn serves_api_and_ui_and_rejects_foreign_origins() {
        let repo = TempRepo::new();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let root = repo.root.clone();
        thread::spawn(move || serve(listener, root, Watcher::new()));

        let projects = get(port, "/api/projects", "");
        assert!(projects.starts_with("HTTP/1.1 200"));
        assert!(projects.contains(PROJECT_ID));
        assert!(get(port, &format!("/api/projects/{PROJECT_ID}"), "").contains("Token store"));
        assert!(get(port, "/", "").contains("js/main.js"));
        assert!(get(port, "/js/main.js", "").contains("text/javascript"));
        assert!(get(port, "/api/projects/nope", "").starts_with("HTTP/1.1 404"));
        assert!(get(port, "/missing", "").starts_with("HTTP/1.1 404"));

        let forbidden =
            |headers: &str| get(port, "/api/projects", headers).starts_with("HTTP/1.1 403");
        assert!(forbidden("Host: evil.example\r\n"));
        assert!(forbidden("Origin: https://evil.example\r\n"));
        assert!(!forbidden("Origin: http://localhost:1234\r\n"));
    }
}
