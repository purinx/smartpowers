//! Just enough HTTP/1.1: parse a request head, check its origin, and send
//! a complete response with `Connection: close`.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use crate::json::Json;

const LOOPBACK_HOSTS: [&str; 4] = ["127.0.0.1", "localhost", "::1", "[::1]"];
const MAX_HEAD_BYTES: u64 = 64 * 1024;

pub fn is_loopback(host: &str) -> bool {
    LOOPBACK_HOSTS.contains(&host)
}

pub struct Request {
    pub method: String,
    /// Path without query string or fragment.
    pub path: String,
    /// Header names are lowercased.
    headers: BTreeMap<String, String>,
}

impl Request {
    /// The board exposes repository files, so reject requests whose Host or
    /// Origin is not loopback. This blocks DNS rebinding and cross-site reads.
    pub fn is_trusted(&self) -> bool {
        let host = self.headers.get("host").map_or("", |h| host_name(h));
        if !is_loopback(host) {
            return false;
        }
        match self.headers.get("origin") {
            None => true,
            Some(origin) => {
                let authority = origin.split_once("://").map_or("", |(_, rest)| rest);
                is_loopback(host_name(authority.split('/').next().unwrap_or("")))
            }
        }
    }
}

pub fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.take(MAX_HEAD_BYTES));
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let path = target.split(['?', '#']).next().unwrap_or("/").to_string();

    let mut headers = BTreeMap::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 {
            break;
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
        }
    }
    Some(Request {
        method,
        path,
        headers,
    })
}

/// The host part of a Host header or URL authority:
/// `[::1]:8080` -> `[::1]`, `localhost:8080` -> `localhost`, `::1` stays.
fn host_name(value: &str) -> &str {
    if value.starts_with('[') {
        return value.find(']').map_or(value, |end| &value[..=end]);
    }
    match value.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && port.bytes().all(|b| b.is_ascii_digit()) => {
            host
        }
        _ => value,
    }
}

pub struct Response {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    pub fn new(status: &'static str, content_type: &'static str, body: &[u8]) -> Self {
        Response {
            status,
            content_type,
            body: body.to_vec(),
        }
    }

    pub fn text(status: &'static str, body: &str) -> Self {
        Self::new(status, "text/plain; charset=utf-8", body.as_bytes())
    }

    pub fn json(status: &'static str, payload: &Json) -> Self {
        Self::new(
            status,
            "application/json; charset=utf-8",
            payload.render().as_bytes(),
        )
    }

    /// Write the response. Errors mean the client went away; nothing to do.
    pub fn send(self, stream: &mut TcpStream) {
        let head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
             Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
            self.status,
            self.content_type,
            self.body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&self.body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(headers: &[(&str, &str)]) -> Request {
        Request {
            method: "GET".into(),
            path: "/".into(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn extracts_host_names() {
        assert_eq!(host_name("localhost:8080"), "localhost");
        assert_eq!(host_name("[::1]:8080"), "[::1]");
        assert_eq!(host_name("::1"), "::1");
        assert_eq!(host_name("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn trusts_only_loopback_host_and_origin() {
        assert!(request(&[("host", "localhost:1")]).is_trusted());
        assert!(request(&[("host", "[::1]:1"), ("origin", "http://127.0.0.1:1")]).is_trusted());
        assert!(!request(&[]).is_trusted());
        assert!(!request(&[("host", "evil.example")]).is_trusted());
        assert!(
            !request(&[("host", "localhost:1"), ("origin", "https://evil.example")]).is_trusted()
        );
    }
}
