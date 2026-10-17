//! Serve a read-only, Linear-style progress board for Smartpowers projects.
//!
//! Std-only on purpose: the plugin ships a prebuilt binary and must build
//! offline with nothing but a Rust toolchain.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const PROJECTS_PATH: &str = "docs/smartpowers/projects";
const LOOPBACK_HOSTS: [&str; 4] = ["127.0.0.1", "localhost", "::1", "[::1]"];
const INDEX_HTML: &str = include_str!("../../assets/board/index.html");
const APP_CSS: &str = include_str!("../../assets/board/app.css");
const APP_JS: &str = include_str!("../../assets/board/app.js");

const USAGE: &str = "\
Usage: smartpowers-board [options]

Serve a local, read-only Smartpowers progress board.

Options:
  --root <dir>        Repository root (default: current directory)
  --project <id>      Project ID to open first
  --host <host>       Loopback host to bind (default: 127.0.0.1)
  --port <n>          Port; 0 picks a free port (default: 0)
  --interval <secs>   Seconds between file scans (default: 1.0)
  --no-open           Do not open a browser
  -h, --help          Show this help";

// ---- JSON ------------------------------------------------------------------

enum Json {
    Null,
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(&'static str, Json)>),
}

impl Json {
    fn str(value: impl Into<String>) -> Json {
        Json::Str(value.into())
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => write_json_str(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_str(key, out);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }
        }
    }

    fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }
}

fn write_json_str(value: &str, out: &mut String) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // Keep "</script>"-style sequences inert if the JSON is ever inlined.
            '<' => out.push_str("\\u003c"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---- Parsing ---------------------------------------------------------------

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// `YYYYMMDD-HHMMSS-lowercase-kebab-slug`
fn is_project_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() > 16
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'-'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'-'
        && is_slug(&value[16..])
}

/// Two-digit prefix of `NN_lowercase_snake_case.md`.
fn task_number(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".md")?;
    let bytes = stem.as_bytes();
    let valid = bytes.len() >= 4
        && bytes[..2].iter().all(u8::is_ascii_digit)
        && bytes[2] == b'_'
        && bytes[3..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_');
    valid.then(|| &stem[..2])
}

fn parse_field(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim_end();
    let mut chars = key.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || !chars.all(|c| c.is_ascii_alphabetic() || c == ' ') {
        return None;
    }
    Some((key.to_lowercase(), line[colon + 1..].trim().to_string()))
}

/// Return the H1 title and the `Key: value` fields before the first H2.
fn parse_header(text: &str) -> (String, BTreeMap<String, String>) {
    let mut title = String::new();
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with("## ") {
            break;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            if title.is_empty() {
                title = rest.trim().to_string();
                continue;
            }
        }
        if let Some((key, value)) = parse_field(line) {
            fields.insert(key, value);
        }
    }
    (title, fields)
}

fn status_key(status: &str) -> &'static str {
    match status.trim().to_lowercase().as_str() {
        "planned" | "todo" => "planned",
        "in progress" | "in-progress" => "in_progress",
        "blocked" => "blocked",
        "done" | "completed" => "done",
        _ => "other",
    }
}

/// `REQ-###` and `NFR-###` IDs, sorted and deduplicated.
fn requirement_ids(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = BTreeSet::new();
    for i in 0..bytes.len() {
        let rest = &bytes[i..];
        if !(rest.starts_with(b"REQ-") || rest.starts_with(b"NFR-")) {
            continue;
        }
        if i > 0 && is_word(bytes[i - 1]) {
            continue;
        }
        let end = i + 7;
        if end <= bytes.len()
            && bytes[i + 4..end].iter().all(u8::is_ascii_digit)
            && (end == bytes.len() || !is_word(bytes[end]))
        {
            found.insert(text[i..end].to_string());
        }
    }
    found.into_iter().collect()
}

/// Two-digit task references such as `01, 02`.
fn task_refs(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| token.len() == 2 && token.bytes().all(|b| b.is_ascii_digit()))
        .map(str::to_string)
        .collect()
}

fn find_url(text: &str) -> Option<&str> {
    let start = [text.find("https://"), text.find("http://")]
        .into_iter()
        .flatten()
        .min()?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | ']'))
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// First issue key such as `ENG-42`.
fn find_issue_key(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_uppercase() || (start > 0 && is_word(bytes[start - 1])) {
            continue;
        }
        let mut j = start + 1;
        while j < bytes.len() && (bytes[j].is_ascii_uppercase() || bytes[j].is_ascii_digit()) {
            j += 1;
        }
        if !(2..=10).contains(&(j - start)) || bytes.get(j) != Some(&b'-') {
            continue;
        }
        let mut k = j + 1;
        while k < bytes.len() && bytes[k].is_ascii_digit() {
            k += 1;
        }
        if k > j + 1 && (k == bytes.len() || !is_word(bytes[k])) {
            return Some(&text[start..k]);
        }
    }
    None
}

fn parse_remote(text: &str) -> Json {
    for line in text.lines() {
        let prefix = "remote task:";
        if line.len() < prefix.len() || !line.is_char_boundary(prefix.len()) {
            continue;
        }
        if !line[..prefix.len()].eq_ignore_ascii_case(prefix) {
            continue;
        }
        let value = line[prefix.len()..].trim();
        if value.is_empty() {
            continue;
        }
        let opt = |v: Option<&str>| v.map_or(Json::Null, Json::str);
        return Json::Obj(vec![
            ("raw", Json::str(value)),
            ("url", opt(find_url(value))),
            ("key", opt(find_issue_key(value))),
        ]);
    }
    Json::Null
}

// ---- Time ------------------------------------------------------------------

/// RFC 3339 UTC timestamp; std has no local-timezone support.
fn iso_utc(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    )
}

fn mtime(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(UNIX_EPOCH)
}

fn files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => files_recursive(&path, out),
            Ok(kind) if kind.is_file() => out.push(path),
            _ => {}
        }
    }
}

// ---- Model -----------------------------------------------------------------

struct Task {
    status_key: &'static str,
    json: Json,
}

fn parse_task(path: &Path, number: &str) -> Option<Task> {
    let text = fs::read_to_string(path).ok()?;
    let (title, fields) = parse_header(&text);
    let title = strip_number(&title, number);
    let title = if title.is_empty() {
        path.file_stem()?.to_string_lossy().into_owned()
    } else {
        title
    };
    let status = fields
        .get("status")
        .cloned()
        .unwrap_or_else(|| "Unknown".into());
    let key = status_key(&status);
    let depends = fields.get("depends on").map(String::as_str).unwrap_or("");
    let depends_on = if matches!(depends.to_lowercase().as_str(), "" | "none" | "-") {
        Vec::new()
    } else {
        task_refs(depends)
    };
    let strings = |items: Vec<String>| Json::Arr(items.into_iter().map(Json::Str).collect());
    let json = Json::Obj(vec![
        ("number", Json::str(number)),
        ("file", Json::str(path.file_name()?.to_string_lossy())),
        ("title", Json::Str(title)),
        ("status", Json::Str(status)),
        ("statusKey", Json::str(key)),
        (
            "requirements",
            strings(requirement_ids(
                fields.get("requirements").map_or("", String::as_str),
            )),
        ),
        ("dependsOn", strings(depends_on)),
        (
            "parallelGroup",
            Json::str(fields.get("parallel group").cloned().unwrap_or_default()),
        ),
        ("remote", parse_remote(&text)),
        ("updated", Json::Str(iso_utc(mtime(path)))),
        ("body", Json::Str(text)),
    ]);
    Some(Task {
        status_key: key,
        json,
    })
}

/// Drop a leading `01:` / `01.` / `01 -` from a task heading.
fn strip_number(title: &str, number: &str) -> String {
    if let Some(rest) = title.strip_prefix(number) {
        let rest = rest.trim_start();
        if let Some(rest) = rest.strip_prefix([':', '.', '-']) {
            return rest.trim_start().to_string();
        }
    }
    title.to_string()
}

fn load_tasks(project_root: &Path) -> Vec<Task> {
    let Ok(entries) = fs::read_dir(project_root.join("tasks")) else {
        return Vec::new();
    };
    let mut paths: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let number = task_number(&name)?.to_string();
            let path = entry.path();
            path.is_file().then_some((number, path))
        })
        .collect();
    paths.sort_by(|a, b| a.1.cmp(&b.1));
    paths
        .iter()
        .filter_map(|(number, path)| parse_task(path, number))
        .collect()
}

fn artifact_status(path: &Path) -> Json {
    match fs::read_to_string(path) {
        Ok(text) => Json::Str(
            parse_header(&text)
                .1
                .remove("status")
                .unwrap_or_else(|| "Unknown".into()),
        ),
        Err(_) => Json::Null,
    }
}

fn project_fields(project_root: &Path, tasks: &[Task]) -> Vec<(&'static str, Json)> {
    let (title, fields) = fs::read_to_string(project_root.join("PROJECT.md"))
        .map(|text| parse_header(&text))
        .unwrap_or_default();
    let id = project_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut counts: BTreeMap<&str, u64> = BTreeMap::new();
    for task in tasks {
        *counts.entry(task.status_key).or_default() += 1;
    }
    let mut files = Vec::new();
    files_recursive(project_root, &mut files);
    let updated = files
        .iter()
        .map(|p| mtime(p))
        .max()
        .unwrap_or_else(|| mtime(project_root));

    vec![
        ("id", Json::str(&id)),
        (
            "title",
            Json::Str(if title.is_empty() { id } else { title }),
        ),
        (
            "status",
            Json::str(
                fields
                    .get("status")
                    .cloned()
                    .unwrap_or_else(|| "Unknown".into()),
            ),
        ),
        ("updated", Json::Str(iso_utc(updated))),
        ("taskCount", Json::Num(tasks.len() as u64)),
        (
            "counts",
            Json::Obj(
                counts
                    .into_iter()
                    .map(|(k, v)| (status_static(k), Json::Num(v)))
                    .collect(),
            ),
        ),
    ]
}

fn status_static(key: &str) -> &'static str {
    match key {
        "planned" => "planned",
        "in_progress" => "in_progress",
        "blocked" => "blocked",
        "done" => "done",
        _ => "other",
    }
}

fn list_projects(repo_root: &Path) -> Json {
    let Ok(entries) = fs::read_dir(repo_root.join(PROJECTS_PATH)) else {
        return Json::Arr(Vec::new());
    };
    let mut roots: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.path().is_dir() && is_project_id(&e.file_name().to_string_lossy()))
        .map(|e| e.path())
        .collect();
    roots.sort_by(|a, b| b.cmp(a));
    Json::Arr(
        roots
            .iter()
            .map(|root| Json::Obj(project_fields(root, &load_tasks(root))))
            .collect(),
    )
}

fn project_detail(repo_root: &Path, project_id: &str) -> Option<Json> {
    if !is_project_id(project_id) {
        return None;
    }
    let project_root = repo_root.join(PROJECTS_PATH).join(project_id);
    if !project_root.is_dir() {
        return None;
    }
    let tasks = load_tasks(&project_root);
    let mut fields = project_fields(&project_root, &tasks);
    let phase = |name: &str, file: &str| {
        Json::Obj(vec![
            ("name", Json::str(name)),
            ("file", Json::str(file)),
            ("status", artifact_status(&project_root.join(file))),
        ])
    };
    fields.push((
        "phases",
        Json::Arr(vec![
            phase("Requirements", "PRD.md"),
            phase("Design", "DESIGN_DOC.md"),
            phase("Plan", "tasks/PLAN.md"),
        ]),
    ));
    fields.push(("path", Json::Str(format!("{PROJECTS_PATH}/{project_id}"))));
    fields.push((
        "tasks",
        Json::Arr(tasks.into_iter().map(|t| t.json).collect()),
    ));
    Some(Json::Obj(fields))
}

// ---- Watcher ---------------------------------------------------------------

fn fingerprint(repo_root: &Path) -> u64 {
    let projects_root = repo_root.join(PROJECTS_PATH);
    let mut files = Vec::new();
    files_recursive(&projects_root, &mut files);
    let mut entries: Vec<String> = files
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .filter_map(|p| {
            let meta = fs::metadata(p).ok()?;
            let nanos = meta
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(format!("{}:{nanos}:{}", p.display(), meta.len()))
        })
        .collect();
    entries.sort();
    let mut hasher = DefaultHasher::new();
    entries.hash(&mut hasher);
    hasher.finish()
}

/// Poll the projects tree and let SSE clients wait for changes.
struct Watcher {
    version: Mutex<u64>,
    changed: Condvar,
}

impl Watcher {
    fn new() -> Arc<Self> {
        Arc::new(Watcher {
            version: Mutex::new(0),
            changed: Condvar::new(),
        })
    }

    fn spawn(self: &Arc<Self>, repo_root: PathBuf, interval: Duration) {
        let watcher = Arc::clone(self);
        thread::spawn(move || {
            let mut current = fingerprint(&repo_root);
            loop {
                thread::sleep(interval);
                let next = fingerprint(&repo_root);
                if next != current {
                    current = next;
                    *watcher.version.lock().unwrap() += 1;
                    watcher.changed.notify_all();
                }
            }
        });
    }

    fn current(&self) -> u64 {
        *self.version.lock().unwrap()
    }

    fn wait(&self, seen: u64, timeout: Duration) -> u64 {
        let guard = self.version.lock().unwrap();
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, timeout, |version| *version == seen)
            .unwrap();
        *guard
    }
}

// ---- HTTP ------------------------------------------------------------------

struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
}

fn read_request(stream: &TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.take(64 * 1024));
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

fn host_name(value: &str) -> &str {
    // "[::1]:8080" -> "[::1]", "localhost:8080" -> "localhost", "::1" stays.
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

/// Reject DNS-rebinding and cross-origin reads; the board exposes repo files.
fn trusted(request: &Request) -> bool {
    let host = request.headers.get("host").map_or("", |h| host_name(h));
    if !LOOPBACK_HOSTS.contains(&host) {
        return false;
    }
    match request.headers.get("origin") {
        None => true,
        Some(origin) => {
            let authority = origin.split_once("://").map_or("", |(_, rest)| rest);
            let authority = authority.split('/').next().unwrap_or("");
            LOOPBACK_HOSTS.contains(&host_name(authority))
        }
    }
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn respond_json(stream: &mut TcpStream, status: &str, payload: &Json) {
    respond(
        stream,
        status,
        "application/json; charset=utf-8",
        payload.render().as_bytes(),
    );
}

fn stream_events(stream: &mut TcpStream, watcher: &Watcher) {
    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                Cache-Control: no-store\r\nConnection: close\r\n\r\n\
                retry: 2000\nevent: ready\ndata: {}\n\n";
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let mut seen = watcher.current();
    loop {
        let version = watcher.wait(seen, Duration::from_secs(15));
        let message = if version != seen {
            seen = version;
            format!("event: change\ndata: {version}\n\n")
        } else {
            ": keepalive\n\n".to_string()
        };
        if stream.write_all(message.as_bytes()).is_err() || stream.flush().is_err() {
            return;
        }
    }
}

fn handle(mut stream: TcpStream, repo_root: &Path, watcher: &Watcher) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let Some(request) = read_request(&stream) else {
        return;
    };
    if !trusted(&request) {
        respond(&mut stream, "403 Forbidden", "text/plain", b"Forbidden");
        return;
    }
    if request.method != "GET" {
        respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed",
        );
        return;
    }
    let path = request.path.as_str();
    match path {
        "/" | "/index.html" => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML.as_bytes(),
        ),
        "/app.css" => respond(
            &mut stream,
            "200 OK",
            "text/css; charset=utf-8",
            APP_CSS.as_bytes(),
        ),
        "/app.js" => respond(
            &mut stream,
            "200 OK",
            "text/javascript; charset=utf-8",
            APP_JS.as_bytes(),
        ),
        "/api/projects" => {
            let repo = repo_root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let payload = Json::Obj(vec![
                ("repo", Json::Str(repo)),
                ("projects", list_projects(repo_root)),
            ]);
            respond_json(&mut stream, "200 OK", &payload);
        }
        "/api/events" => stream_events(&mut stream, watcher),
        _ => match path.strip_prefix("/api/projects/") {
            Some(id) => match project_detail(repo_root, id) {
                Some(detail) => respond_json(&mut stream, "200 OK", &detail),
                None => respond_json(
                    &mut stream,
                    "404 Not Found",
                    &Json::Obj(vec![("error", Json::str("Project not found"))]),
                ),
            },
            None => respond(&mut stream, "404 Not Found", "text/plain", b"Not found"),
        },
    }
}

fn serve(listener: TcpListener, repo_root: PathBuf, watcher: Arc<Watcher>) {
    let repo_root = Arc::new(repo_root);
    for stream in listener.incoming().flatten() {
        let repo_root = Arc::clone(&repo_root);
        let watcher = Arc::clone(&watcher);
        thread::spawn(move || handle(stream, &repo_root, &watcher));
    }
}

// ---- CLI -------------------------------------------------------------------

struct Options {
    root: PathBuf,
    project: Option<String>,
    host: String,
    port: u16,
    interval: f64,
    open: bool,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        root: PathBuf::from("."),
        project: None,
        host: "127.0.0.1".into(),
        port: 0,
        interval: 1.0,
        open: true,
    };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_string(), Some(value.to_string()))
            }
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{name} requires a value"))
        };
        match flag.as_str() {
            "-h" | "--help" => return Err(String::new()),
            "--root" => options.root = PathBuf::from(value("--root")?),
            "--project" => options.project = Some(value("--project")?),
            "--host" => options.host = value("--host")?,
            "--port" => {
                options.port = value("--port")?
                    .parse()
                    .map_err(|_| "--port must be 0-65535".to_string())?
            }
            "--interval" => {
                options.interval = value("--interval")?
                    .parse()
                    .ok()
                    .filter(|v: &f64| *v > 0.0)
                    .ok_or("--interval must be a positive number")?
            }
            "--no-open" => options.open = false,
            other => return Err(format!("Unknown argument: {other}")),
        }
    }
    Ok(options)
}

fn open_browser(url: &str) {
    let result = if cfg!(target_os = "macos") {
        Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", "", url]).spawn()
    } else {
        Command::new("xdg-open").arg(url).spawn()
    };
    if result.is_err() {
        eprintln!("Could not open a browser; visit {url}");
    }
}

fn main() -> ExitCode {
    let options = match parse_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) if message.is_empty() => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("error: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    if !LOOPBACK_HOSTS.contains(&options.host.as_str()) {
        eprintln!("error: the board serves repository files; bind it to a loopback host only");
        return ExitCode::from(2);
    }
    let repo_root = match options.root.canonicalize() {
        Ok(root) => root,
        Err(error) => {
            eprintln!(
                "error: cannot open --root {}: {error}",
                options.root.display()
            );
            return ExitCode::from(2);
        }
    };
    if let Some(project) = &options.project {
        if project_detail(&repo_root, project).is_none() {
            eprintln!("error: Project not found: {project}");
            return ExitCode::from(2);
        }
    }

    let host = options.host.trim_matches(['[', ']']);
    let listener = match TcpListener::bind((host, options.port)) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("error: cannot listen on {host}:{}: {error}", options.port);
            return ExitCode::FAILURE;
        }
    };
    let port = listener
        .local_addr()
        .map(|a| a.port())
        .unwrap_or(options.port);
    let mut url = format!("http://localhost:{port}/");
    if let Some(project) = &options.project {
        url.push_str(&format!("#/{project}"));
    }
    println!("Smartpowers board: {url}");
    let _ = std::io::stdout().flush();
    if options.open {
        open_browser(&url);
    }

    let watcher = Watcher::new();
    watcher.spawn(repo_root.clone(), Duration::from_secs_f64(options.interval));
    serve(listener, repo_root, watcher);
    ExitCode::SUCCESS
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PROJECT_ID: &str = "20260908-183045-oauth-refresh";

    struct TempRepo(PathBuf);

    impl TempRepo {
        fn new() -> Self {
            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "smartpowers-board-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ));
            let project = root.join(PROJECTS_PATH).join(PROJECT_ID);
            fs::create_dir_all(project.join("tasks")).unwrap();
            let write = |path: &str, text: String| fs::write(project.join(path), text).unwrap();
            write(
                "PROJECT.md",
                format!("# OAuth refresh\n\nStatus: Active\nProject: {PROJECT_ID}\n"),
            );
            write("PRD.md", "# PRD\n\nStatus: Approved\n".into());
            write("DESIGN_DOC.md", "# Design\n\nStatus: Draft\n".into());
            write("tasks/PLAN.md", "# Plan\n\nStatus: Approved\n".into());
            write(
                "tasks/01_infra_foundations.md",
                format!(
                    "# 01: Infrastructure foundations\n\nStatus: Done\nProject: {PROJECT_ID}\n\
                     Requirements: NFR-001, REQ-002\nDepends on: None\nParallel group: A\n\
                     Remote task: ENG-42 https://linear.app/acme/issue/ENG-42/infra\n\n\
                     ## Objective\n\nBuild it.\n"
                ),
            );
            write(
                "tasks/02_token_store.md",
                format!(
                    "# 02: Token store\n\nStatus: In Progress\nProject: {PROJECT_ID}\n\
                     Requirements: REQ-001\nDepends on: 01\n\n\
                     ## Objective\n\nStatus: not a header field\n"
                ),
            );
            TempRepo(root)
        }

        fn project(&self) -> PathBuf {
            self.0.join(PROJECTS_PATH).join(PROJECT_ID)
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn get(port: u16, path: &str, extra: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let host = if extra.contains("Host:") {
            String::new()
        } else {
            format!("Host: 127.0.0.1:{port}\r\n")
        };
        write!(stream, "GET {path} HTTP/1.1\r\n{host}{extra}\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn validates_ids_and_task_names() {
        assert!(is_project_id(PROJECT_ID));
        assert!(!is_project_id("../etc"));
        assert!(!is_project_id("20260908-183045-Bad"));
        assert_eq!(task_number("01_infra_foundations.md"), Some("01"));
        assert_eq!(task_number("PLAN.md"), None);
        assert_eq!(task_number("1_bad.md"), None);
    }

    #[test]
    fn extracts_ids_refs_and_issue_keys() {
        assert_eq!(
            requirement_ids("NFR-001, REQ-002 REQ-002 XREQ-003 REQ-0044"),
            ["NFR-001", "REQ-002"]
        );
        assert_eq!(task_refs("01, 02 and 103"), ["01", "02"]);
        assert_eq!(
            find_issue_key("see ENG-42 https://x/ENG-42"),
            Some("ENG-42")
        );
        assert_eq!(find_issue_key("eng-42 E-1"), None);
        assert_eq!(find_url("ENG-1 (https://a.b/c) x"), Some("https://a.b/c"));
        assert_eq!(strip_number("01: Title", "01"), "Title");
        assert_eq!(
            iso_utc(UNIX_EPOCH + Duration::from_secs(1_788_000_000)),
            "2026-08-29T10:40:00Z"
        );
    }

    #[test]
    fn parses_tasks_and_project_detail() {
        let repo = TempRepo::new();
        let tasks = load_tasks(&repo.project());
        assert_eq!(tasks.len(), 2);
        let first = tasks[0].json.render();
        assert!(first.contains(r#""title":"Infrastructure foundations""#));
        assert!(first.contains(r#""statusKey":"done""#));
        assert!(first.contains(r#""requirements":["NFR-001","REQ-002"]"#));
        assert!(first.contains(r#""dependsOn":[]"#));
        assert!(first.contains(r#""key":"ENG-42""#));
        assert!(first.contains(r#""url":"https://linear.app/acme/issue/ENG-42/infra""#));
        let second = tasks[1].json.render();
        assert!(second.contains(r#""status":"In Progress""#));
        assert!(second.contains(r#""dependsOn":["01"]"#));
        assert!(second.contains(r#""remote":null"#));

        let detail = project_detail(&repo.0, PROJECT_ID).unwrap().render();
        assert!(detail.contains(r#""title":"OAuth refresh""#));
        assert!(detail.contains(r#""counts":{"done":1,"in_progress":1}"#));
        assert!(detail.contains(
            r#""status":"Approved"},{"name":"Design","file":"DESIGN_DOC.md","status":"Draft"}"#
        ));
        assert!(project_detail(&repo.0, "../etc").is_none());
    }

    #[test]
    fn serves_api_and_rejects_foreign_hosts() {
        let repo = TempRepo::new();
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let root = repo.0.clone();
        thread::spawn(move || serve(listener, root, Watcher::new()));

        let projects = get(port, "/api/projects", "");
        assert!(projects.starts_with("HTTP/1.1 200"));
        assert!(projects.contains(PROJECT_ID));
        assert!(get(port, "/", "").contains("app.js"));
        assert!(get(port, "/api/projects/nope", "").starts_with("HTTP/1.1 404"));
        assert!(get(port, "/api/projects", "Host: evil.example\r\n").starts_with("HTTP/1.1 403"));
        assert!(
            get(port, "/api/projects", "Origin: https://evil.example\r\n")
                .starts_with("HTTP/1.1 403")
        );
        assert!(
            get(port, "/api/projects", "Origin: http://localhost:1234\r\n")
                .starts_with("HTTP/1.1 200")
        );
    }

    #[test]
    fn watcher_detects_task_changes() {
        let repo = TempRepo::new();
        let watcher = Watcher::new();
        watcher.spawn(repo.0.clone(), Duration::from_millis(50));
        thread::sleep(Duration::from_millis(100));
        let task = repo.project().join("tasks/02_token_store.md");
        let text = fs::read_to_string(&task).unwrap();
        fs::write(&task, text.replace("In Progress", "Done!")).unwrap();
        assert_eq!(watcher.wait(0, Duration::from_secs(3)), 1);
    }
}
