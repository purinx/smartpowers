//! Task comments, one Markdown file each under
//! `<project state>/comments/<task>/<id>.md`:
//!
//! ```text
//! # Comment 20261017-101530-3f2a
//!
//! Task: 05
//! Author: user
//! Created: 2026-10-17T10:15:30.123Z
//! Status: Open
//! Delivered: no
//! Reply to: 20261017-101000-aa01
//!
//! Body text, any number of lines.
//! ```
//!
//! Users write comments from the board. Agents pick them up with
//! `smartpowers-board inbox`, which marks them delivered, and answer with
//! `smartpowers-board reply`, which can mark the thread addressed.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::files::files_recursive;
use crate::time::{compact_utc, iso_utc_millis};

pub const MAX_BODY_CHARS: usize = 8_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    User,
    Agent,
}

impl Author {
    pub fn as_str(self) -> &'static str {
        match self {
            Author::User => "user",
            Author::Agent => "agent",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "user" => Some(Author::User),
            "agent" => Some(Author::Agent),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentStatus {
    Open,
    Addressed,
}

impl CommentStatus {
    /// The identifier the web UI uses.
    pub fn as_str(self) -> &'static str {
        match self {
            CommentStatus::Open => "open",
            CommentStatus::Addressed => "addressed",
        }
    }

    fn label(self) -> &'static str {
        match self {
            CommentStatus::Open => "Open",
            CommentStatus::Addressed => "Addressed",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "open" => Some(CommentStatus::Open),
            "addressed" => Some(CommentStatus::Addressed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub id: String,
    pub task: String,
    pub author: Author,
    /// RFC 3339 UTC with milliseconds; sorts chronologically as text.
    pub created: String,
    pub status: CommentStatus,
    /// Whether `inbox` has shown this comment to an agent.
    pub delivered: bool,
    /// The thread's first comment, for replies.
    pub reply_to: Option<String>,
    pub body: String,
}

impl Comment {
    /// A user comment no agent has seen yet.
    pub fn is_pending(&self) -> bool {
        self.author == Author::User && self.status == CommentStatus::Open && !self.delivered
    }

    fn render(&self) -> String {
        let reply_to = self
            .reply_to
            .as_ref()
            .map(|id| format!("Reply to: {id}\n"))
            .unwrap_or_default();
        format!(
            "# Comment {}\n\nTask: {}\nAuthor: {}\nCreated: {}\nStatus: {}\nDelivered: {}\n{reply_to}\n{}\n",
            self.id,
            self.task,
            self.author.as_str(),
            self.created,
            self.status.label(),
            if self.delivered { "yes" } else { "no" },
            self.body
        )
    }

    /// The title line, then `Key: value` fields up to the first blank line,
    /// then the body. Unlike task headers, fields stop at that blank line so
    /// a body line such as `Note: ...` is never read as a field.
    fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let id = lines.next()?.strip_prefix("# Comment ")?.trim().to_string();
        if !is_comment_id(&id) {
            return None;
        }
        let mut fields = BTreeMap::new();
        let mut body = Vec::new();
        let mut in_body = false;
        for line in lines {
            if in_body {
                body.push(line);
            } else if line.trim().is_empty() {
                in_body = !fields.is_empty();
            } else {
                let (key, value) = line.split_once(':')?;
                fields.insert(key.trim().to_lowercase(), value.trim().to_string());
            }
        }
        let field = |name: &str| fields.get(name).map(String::as_str);
        Some(Comment {
            id,
            task: field("task")?.to_string(),
            author: Author::parse(field("author")?)?,
            created: field("created")?.to_string(),
            status: CommentStatus::parse(field("status")?)?,
            delivered: field("delivered") == Some("yes"),
            reply_to: field("reply to").map(str::to_string),
            body: body.join("\n").trim().to_string(),
        })
    }
}

/// `YYYYMMDD-HHMMSS-xxxx` with four lowercase hex digits.
pub fn is_comment_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[8] == b'-'
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'-'
        && bytes[16..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}

fn comments_dir(project_state: &Path) -> PathBuf {
    project_state.join("comments")
}

fn path_for(project_state: &Path, task: &str, id: &str) -> PathBuf {
    comments_dir(project_state)
        .join(task)
        .join(format!("{id}.md"))
}

/// Every comment in the project, oldest first.
pub fn load(project_state: &Path) -> Vec<Comment> {
    let mut comments: Vec<Comment> = files_recursive(&comments_dir(project_state))
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter_map(|path| Comment::parse(&fs::read_to_string(path).ok()?))
        .collect();
    comments.sort_by(|a, b| (&a.created, &a.id).cmp(&(&b.created, &b.id)));
    comments
}

pub fn find(project_state: &Path, id: &str) -> Option<Comment> {
    if !is_comment_id(id) {
        return None;
    }
    load(project_state)
        .into_iter()
        .find(|comment| comment.id == id)
}

/// Validate and store a new comment. Agent comments are born delivered.
pub fn create(
    project_state: &Path,
    task: &str,
    author: Author,
    body: &str,
    reply_to: Option<&str>,
) -> Result<Comment, String> {
    let body = body.replace("\r\n", "\n").trim().to_string();
    if body.is_empty() {
        return Err("Comment is empty".into());
    }
    if body.chars().count() > MAX_BODY_CHARS {
        return Err(format!(
            "Comment is longer than {MAX_BODY_CHARS} characters"
        ));
    }
    let now = SystemTime::now();
    let id = loop {
        let id = new_id(now);
        if !path_for(project_state, task, &id).exists() {
            break id;
        }
    };
    let comment = Comment {
        id,
        task: task.to_string(),
        author,
        created: iso_utc_millis(now),
        status: CommentStatus::Open,
        delivered: author == Author::Agent,
        reply_to: reply_to.map(str::to_string),
        body,
    };
    save(project_state, &comment)?;
    Ok(comment)
}

/// Write a comment atomically (temp file, then rename) so the board and
/// agents never read a half-written file.
pub fn save(project_state: &Path, comment: &Comment) -> Result<(), String> {
    let path = path_for(project_state, &comment.task, &comment.id);
    let dir = path.parent().ok_or("invalid comment path")?;
    fs::create_dir_all(dir).map_err(|error| format!("cannot create {}: {error}", dir.display()))?;
    let temp = path.with_extension("md.tmp");
    fs::write(&temp, comment.render())
        .and_then(|()| fs::rename(&temp, &path))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn new_id(now: SystemTime) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos()) as u64;
    let mixed = (nanos ^ COUNTER.fetch_add(1, Ordering::Relaxed).rotate_left(32))
        .wrapping_add(u64::from(std::process::id()))
        .wrapping_mul(0x9e37_79b9_7f4a_7c15);
    format!("{}-{:04x}", compact_utc(now), mixed >> 48)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempRepo;

    #[test]
    fn round_trips_through_markdown() {
        let comment = Comment {
            id: "20261017-101530-3f2a".into(),
            task: "05".into(),
            author: Author::User,
            created: "2026-10-17T10:15:30Z".into(),
            status: CommentStatus::Open,
            delivered: false,
            reply_to: None,
            body: "Use the existing client.\n\nNote: this is not a field".into(),
        };
        let parsed = Comment::parse(&comment.render()).unwrap();
        assert_eq!(parsed.id, comment.id);
        assert_eq!(parsed.body, comment.body);
        assert_eq!(parsed.author, Author::User);
        assert!(parsed.is_pending());
        assert!(Comment::parse("# Comment ../x\n\nTask: 05\n").is_none());
    }

    #[test]
    fn creates_loads_and_updates_comments() {
        let repo = TempRepo::new();
        let state = repo.project_state();
        let first = create(
            &state,
            "02",
            Author::User,
            "  Please add retries \r\n",
            None,
        )
        .unwrap();
        assert_eq!(first.body, "Please add retries");
        assert!(is_comment_id(&first.id));
        let reply = create(&state, "02", Author::Agent, "Done", Some(&first.id)).unwrap();
        assert!(reply.delivered && !reply.is_pending());

        let mut loaded = find(&state, &first.id).unwrap();
        assert!(loaded.is_pending());
        loaded.status = CommentStatus::Addressed;
        save(&state, &loaded).unwrap();
        assert_eq!(
            find(&state, &first.id).unwrap().status,
            CommentStatus::Addressed
        );
        assert_eq!(load(&state).len(), 2);

        assert!(create(&state, "02", Author::User, " \n ", None).is_err());
        let long = "x".repeat(MAX_BODY_CHARS + 1);
        assert!(create(&state, "02", Author::User, &long, None).is_err());
    }
}
