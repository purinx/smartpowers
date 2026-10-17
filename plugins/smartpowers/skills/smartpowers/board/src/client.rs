//! Minimal HTTP client that agent commands use to talk to a running board:
//! plain requests for logs and replies, and a Server-Sent Events reader for
//! comments.

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use crate::runtime::ServerInfo;
use crate::workspace::Workspace;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);

pub struct Client {
    info: ServerInfo,
}

impl Client {
    /// The board serving this workspace, if one is running and answering.
    pub fn connect(workspace: &Workspace) -> Option<Self> {
        let info = ServerInfo::load(&workspace.state_dir)?;
        TcpStream::connect_timeout(&info.addr, CONNECT_TIMEOUT).ok()?;
        Some(Client { info })
    }

    fn open(&self, method: &str, path: &str, body: &str) -> Result<TcpStream, String> {
        let mut stream = TcpStream::connect_timeout(&self.info.addr, CONNECT_TIMEOUT)
            .map_err(|error| format!("cannot reach the board at {}: {error}", self.info.addr))?;
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\n\
             Content-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.info.addr,
            self.info.token,
            body.len()
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|error| format!("cannot send to the board: {error}"))?;
        Ok(stream)
    }

    /// Send a request and return the response body, or an error for any
    /// non-2xx status.
    pub fn post(&self, path: &str, body: &str) -> Result<String, String> {
        let mut stream = self.open("POST", path, body)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|error| error.to_string())?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|error| format!("cannot read the board's response: {error}"))?;
        let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
        let status = head.split_whitespace().nth(1).unwrap_or("");
        if status.starts_with('2') {
            Ok(body.to_string())
        } else {
            Err(format!("board answered {status}: {}", body.trim()))
        }
    }

    pub fn events(&self, path: &str) -> Result<EventStream, String> {
        let stream = self.open("GET", path, "")?;
        let mut reader = BufReader::new(stream);
        reader
            .get_ref()
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|error| error.to_string())?;
        let mut status = String::new();
        reader
            .read_line(&mut status)
            .map_err(|error| format!("cannot read the board's response: {error}"))?;
        let mut header = String::new();
        loop {
            header.clear();
            match reader.read_line(&mut header) {
                Ok(0) => break,
                Ok(_) if header.trim().is_empty() => break,
                Ok(_) => {}
                Err(error) => return Err(format!("cannot read the board's response: {error}")),
            }
        }
        if !status
            .split_whitespace()
            .nth(1)
            .is_some_and(|code| code == "200")
        {
            let mut body = String::new();
            let _ = reader.read_to_string(&mut body);
            return Err(format!("board answered {}: {}", status.trim(), body.trim()));
        }
        Ok(EventStream {
            reader,
            line: String::new(),
            event: Event::default(),
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Event {
    pub name: String,
    /// `data:` lines joined with newlines.
    pub data: String,
}

pub struct EventStream {
    reader: BufReader<TcpStream>,
    /// Partial line kept across timeouts.
    line: String,
    event: Event,
}

impl EventStream {
    /// The next event, or `None` if `timeout` passes first.
    pub fn next(&mut self, timeout: Option<Duration>) -> Result<Option<Event>, String> {
        let deadline = timeout.map(|timeout| Instant::now() + timeout);
        loop {
            let wait = match deadline {
                Some(deadline) => match deadline.checked_duration_since(Instant::now()) {
                    Some(remaining) if !remaining.is_zero() => Some(remaining),
                    _ => return Ok(None),
                },
                None => None,
            };
            self.reader
                .get_ref()
                .set_read_timeout(wait)
                .map_err(|error| error.to_string())?;
            match self.reader.read_line(&mut self.line) {
                Ok(0) => return Err("the board closed the connection".into()),
                Ok(_) if !self.line.ends_with('\n') => continue,
                Ok(_) => {
                    let line = std::mem::take(&mut self.line);
                    if let Some(event) = self.feed(line.trim_end_matches(['\r', '\n'])) {
                        return Ok(Some(event));
                    }
                }
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
                {
                    return Ok(None)
                }
                Err(error) => return Err(format!("lost the board connection: {error}")),
            }
        }
    }

    /// Apply one SSE line; a blank line completes the current event.
    fn feed(&mut self, line: &str) -> Option<Event> {
        if line.is_empty() {
            let event = std::mem::take(&mut self.event);
            return (!event.name.is_empty() || !event.data.is_empty()).then_some(event);
        }
        if let Some(name) = line.strip_prefix("event:") {
            self.event.name = name.trim().to_string();
        } else if let Some(data) = line.strip_prefix("data:") {
            if !self.event.data.is_empty() {
                self.event.data.push('\n');
            }
            self.event
                .data
                .push_str(data.strip_prefix(' ').unwrap_or(data));
        }
        None
    }
}
