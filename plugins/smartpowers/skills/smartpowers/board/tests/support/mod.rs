//! Fixtures for black-box tests of the built `smartpowers-board` binary: a
//! throwaway repository, a running server, a tiny HTTP client, and a tiny
//! JSON parser. The crate has no dependencies, so neither do its tests.

#![allow(dead_code)] // Each test binary uses a different subset.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const PROJECT_ID: &str = "20260908-183045-oauth-refresh";
pub const BOARD: &str = env!("CARGO_BIN_EXE_smartpowers-board");

/// A repository with one project whose task 01 is In Progress, plus a state
/// directory beside it. Deleted on drop.
pub struct TempRepo {
    dir: PathBuf,
    pub root: PathBuf,
    pub state: PathBuf,
}

impl TempRepo {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "smartpowers-e2e-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let repo = TempRepo {
            root: dir.join("repo"),
            state: dir.join("state"),
            dir,
        };
        repo.write(
            "PROJECT.md",
            &format!("# OAuth refresh\n\nStatus: Active\nProject: {PROJECT_ID}\n"),
        );
        repo.write(
            "tasks/01_infra_foundations.md",
            &format!(
                "# 01: Infrastructure foundations\n\nStatus: In Progress\n\
                 Project: {PROJECT_ID}\nRequirements: REQ-001\nDepends on: None\n"
            ),
        );
        repo
    }

    pub fn project(&self) -> PathBuf {
        self.root.join("docs/smartpowers/projects").join(PROJECT_ID)
    }

    /// Writes a file relative to the project directory.
    pub fn write(&self, relative: &str, text: &str) {
        let path = self.project().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// `smartpowers-board <args> --root <repo> --project <id>`, unstarted.
    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(BOARD);
        command
            .args(args)
            .arg("--root")
            .arg(&self.root)
            .arg("--project")
            .arg(PROJECT_ID)
            .env("SMARTPOWERS_STATE_HOME", &self.state);
        command
    }

    /// Runs an agent subcommand to completion.
    pub fn board(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    /// Starts the server on a free port.
    pub fn serve(&self) -> Server {
        let mut child = self
            .command(&["--no-open"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        let line = line.trim();
        let rest = line
            .strip_prefix("Smartpowers board: http://localhost:")
            .unwrap_or_else(|| panic!("unexpected first line: {line:?}"));
        assert!(line.ends_with(&format!("#/{PROJECT_ID}")), "{line}");
        let port = rest.split('/').next().unwrap().parse().unwrap();
        Server {
            child,
            _stdout: stdout,
            port,
        }
    }

    /// Every path under the repository, for checking nothing was written there.
    pub fn repo_files(&self) -> Vec<PathBuf> {
        fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                }
                out.push(path);
            }
        }
        let mut files = Vec::new();
        walk(&self.root, &mut files);
        files
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// A running board server, killed on drop.
pub struct Server {
    child: Child,
    // Held open so the server never writes to a closed pipe.
    _stdout: BufReader<ChildStdout>,
    pub port: u16,
}

impl Server {
    pub fn origin(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn get(&self, path: &str) -> Response {
        self.request("GET", path, &[], "")
    }

    /// The project's API JSON.
    pub fn project(&self) -> Json {
        let response = self.get(&format!("/api/projects/{PROJECT_ID}"));
        assert_eq!(response.status, 200, "{}", response.body);
        response.json()
    }

    /// Sends one request with `Connection: close`. A `Host` in `headers`
    /// replaces the default loopback one.
    pub fn request(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Response {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut head = format!("{method} {path} HTTP/1.1\r\n");
        if !headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("host"))
        {
            head += &format!("Host: localhost:{}\r\n", self.port);
        }
        for (name, value) in headers {
            head += &format!("{name}: {value}\r\n");
        }
        head += &format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let raw = String::from_utf8(raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
        let status = head
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("bad response: {raw:?}"));
        Response {
            status,
            body: body.to_string(),
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Response {
    pub status: u16,
    pub body: String,
}

impl Response {
    pub fn json(&self) -> Json {
        Json::parse(&self.body).unwrap_or_else(|error| panic!("{error}: {}", self.body))
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A parsed JSON value. Indexing a missing key or position yields `Null`.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

static NULL: Json = Json::Null;

impl std::ops::Index<&str> for Json {
    type Output = Json;
    fn index(&self, key: &str) -> &Json {
        match self {
            Json::Obj(map) => map.get(key).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl std::ops::Index<usize> for Json {
    type Output = Json;
    fn index(&self, index: usize) -> &Json {
        match self {
            Json::Arr(items) => items.get(index).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl Json {
    pub fn str(&self) -> &str {
        match self {
            Json::Str(value) => value,
            other => panic!("not a string: {other:?}"),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Json::Arr(items) => items.len(),
            Json::Obj(map) => map.len(),
            other => panic!("no length: {other:?}"),
        }
    }

    /// Parses a complete JSON document.
    pub fn parse(text: &str) -> Result<Json, String> {
        let mut parser = Parser {
            chars: text.chars().collect(),
            at: 0,
        };
        let value = parser.value()?;
        parser.skip_space();
        if parser.at != parser.chars.len() {
            return Err(format!("trailing characters at {}", parser.at));
        }
        Ok(value)
    }
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Result<char, String> {
        let c = self.peek().ok_or("unexpected end of input")?;
        self.at += 1;
        Ok(c)
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        match self.next()? {
            c if c == expected => Ok(()),
            c => Err(format!(
                "expected {expected:?}, found {c:?} at {}",
                self.at - 1
            )),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for expected in word.chars() {
            self.expect(expected)?;
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_space();
        match self.peek().ok_or("unexpected end of input")? {
            '{' => self.object(),
            '[' => self.array(),
            '"' => self.string().map(Json::Str),
            't' => self.literal("true", Json::Bool(true)),
            'f' => self.literal("false", Json::Bool(false)),
            'n' => self.literal("null", Json::Null),
            '-' | '0'..='9' => self.number(),
            c => Err(format!("unexpected {c:?} at {}", self.at)),
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut map = BTreeMap::new();
        self.skip_space();
        if self.peek() == Some('}') {
            self.at += 1;
            return Ok(Json::Obj(map));
        }
        loop {
            self.skip_space();
            let key = self.string()?;
            self.skip_space();
            self.expect(':')?;
            map.insert(key, self.value()?);
            self.skip_space();
            match self.next()? {
                ',' => continue,
                '}' => return Ok(Json::Obj(map)),
                c => return Err(format!("expected ',' or '}}', found {c:?}")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(']') {
            self.at += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_space();
            match self.next()? {
                ',' => continue,
                ']' => return Ok(Json::Arr(items)),
                c => return Err(format!("expected ',' or ']', found {c:?}")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.next()? {
                '"' => return Ok(out),
                '\\' => match self.next()? {
                    '"' => out.push('"'),
                    '\\' => out.push('\\'),
                    '/' => out.push('/'),
                    'b' => out.push('\u{8}'),
                    'f' => out.push('\u{c}'),
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    'u' => {
                        let high = self.hex4()?;
                        let code = if (0xD800..0xDC00).contains(&high) {
                            self.expect('\\')?;
                            self.expect('u')?;
                            let low = self.hex4()?;
                            0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
                        } else {
                            high
                        };
                        out.push(char::from_u32(code).ok_or("invalid \\u escape")?);
                    }
                    c => return Err(format!("invalid escape \\{c}")),
                },
                c if (c as u32) < 0x20 => return Err("control character in string".into()),
                c => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut code = 0;
        for _ in 0..4 {
            let digit = self.next()?.to_digit(16).ok_or("invalid \\u escape")?;
            code = code * 16 + digit;
        }
        Ok(code)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.at;
        while matches!(self.peek(), Some('-' | '+' | '.' | 'e' | 'E' | '0'..='9')) {
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        text.parse()
            .map(Json::Num)
            .map_err(|_| format!("invalid number {text:?}"))
    }
}
