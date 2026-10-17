//! `GET /api/events`: Server-Sent Events. Sends `ready` on connect,
//! `change` whenever the watcher's version moves, and a keepalive comment
//! every 15 seconds so idle connections aren't dropped.

use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use crate::watcher::Watcher;

const KEEPALIVE: Duration = Duration::from_secs(15);

/// Hold the connection open until the client disconnects.
pub fn stream(stream: &mut TcpStream, watcher: &Watcher) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                Cache-Control: no-store\r\nConnection: close\r\n\r\n\
                retry: 2000\nevent: ready\ndata: {}\n\n";
    if stream.write_all(head.as_bytes()).is_err() {
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
        if stream.write_all(message.as_bytes()).is_err() || stream.flush().is_err() {
            return;
        }
    }
}
