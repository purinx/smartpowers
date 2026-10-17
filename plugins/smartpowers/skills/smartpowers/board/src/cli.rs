//! Command-line parsing and browser launching.

use std::path::PathBuf;
use std::process::Command as Process;

pub const USAGE: &str = "\
Usage:
  smartpowers-board [serve] [options]   Serve the progress board
  smartpowers-board inbox --project <id> [--wait] [--timeout <secs>]
                                        Print new task comments for the agent
  smartpowers-board reply --project <id> --comment <id> [--resolve] <message>
                                        Reply to a comment as the agent
  smartpowers-board log --project <id> --task <nn> <message>
                                        Add a progress entry to a task's log

Serve options:
  --project <id>      Project ID to open first
  --host <host>       Loopback host to bind (default: 127.0.0.1)
  --port <n>          Port; 0 picks a free port (default: 0)
  --interval <secs>   Seconds between file scans (default: 1.0)
  --no-open           Do not open a browser

Inbox options:
  --wait              Block until at least one new comment arrives
  --timeout <secs>    With --wait, give up after this many seconds

Common options:
  --root <dir>        Repository root (default: current directory)
  -h, --help          Show this help

Use -- before a message that starts with a dash.";

#[derive(Debug)]
pub struct ServeOptions {
    pub project: Option<String>,
    pub host: String,
    pub port: u16,
    pub interval: f64,
    pub open: bool,
}

#[derive(Debug)]
pub struct InboxOptions {
    pub project: String,
    pub wait: bool,
    pub timeout: Option<f64>,
}

#[derive(Debug)]
pub struct ReplyOptions {
    pub project: String,
    pub comment: String,
    pub resolve: bool,
    pub message: String,
}

#[derive(Debug)]
pub struct LogOptions {
    pub project: String,
    pub task: String,
    pub message: String,
}

#[derive(Debug)]
pub enum Command {
    Serve(ServeOptions),
    Inbox(InboxOptions),
    Reply(ReplyOptions),
    Log(LogOptions),
}

#[derive(Debug)]
pub struct Invocation {
    pub root: PathBuf,
    pub command: Command,
}

#[derive(Debug)]
pub enum ParseOutcome {
    Run(Invocation),
    Help,
    Error(String),
}

const COMMANDS: [&str; 4] = ["serve", "inbox", "reply", "log"];

/// Parse a subcommand (default `serve`) and its `--flag value`,
/// `--flag=value`, and positional arguments.
pub fn parse_args(args: impl Iterator<Item = String>) -> ParseOutcome {
    let mut args: Vec<String> = args.collect();
    let name = if args.first().is_some_and(|a| COMMANDS.contains(&a.as_str())) {
        args.remove(0)
    } else {
        "serve".to_string()
    };
    match parse_command(&name, args) {
        Ok(Some(invocation)) => ParseOutcome::Run(invocation),
        Ok(None) => ParseOutcome::Help,
        Err(message) => ParseOutcome::Error(message),
    }
}

fn parse_command(name: &str, args: Vec<String>) -> Result<Option<Invocation>, String> {
    let mut root = PathBuf::from(".");
    let mut project = None;
    let mut serve = ServeOptions {
        project: None,
        host: "127.0.0.1".into(),
        port: 0,
        interval: 1.0,
        open: true,
    };
    let (mut wait, mut timeout) = (false, None);
    let (mut comment, mut resolve, mut task) = (None, false, None);
    let mut positionals = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            positionals.extend(args.by_ref());
            break;
        }
        if !arg.starts_with('-') || arg == "-" {
            positionals.push(arg);
            continue;
        }
        let (flag, inline) = match arg.split_once('=') {
            Some((flag, value)) if flag.starts_with("--") => {
                (flag.to_string(), Some(value.to_string()))
            }
            _ => (arg, None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| args.next())
                .ok_or_else(|| format!("{flag} requires a value"))
        };
        match (name, flag.as_str()) {
            (_, "-h" | "--help") => return Ok(None),
            (_, "--root") => root = PathBuf::from(value()?),
            (_, "--project") => project = Some(value()?),
            ("serve", "--host") => serve.host = value()?,
            ("serve", "--port") => {
                serve.port = value()?
                    .parse()
                    .map_err(|_| "--port must be 0-65535".to_string())?;
            }
            ("serve", "--interval") => serve.interval = positive(&value()?, "--interval")?,
            ("serve", "--no-open") => serve.open = false,
            ("inbox", "--wait") => wait = true,
            ("inbox", "--timeout") => timeout = Some(positive(&value()?, "--timeout")?),
            ("reply", "--comment") => comment = Some(value()?),
            ("reply", "--resolve") => resolve = true,
            ("log", "--task") => task = Some(value()?),
            _ => return Err(format!("Unknown argument for {name}: {flag}")),
        }
    }

    let message = positionals.join(" ");
    let require =
        |value: Option<String>, flag: &str| value.ok_or_else(|| format!("{name} requires {flag}"));
    let require_message = || {
        if message.trim().is_empty() {
            Err(format!("{name} requires a message"))
        } else {
            Ok(message.clone())
        }
    };
    if name != "reply" && name != "log" && !positionals.is_empty() {
        return Err(format!("Unexpected argument: {}", positionals[0]));
    }

    let command = match name {
        "inbox" => Command::Inbox(InboxOptions {
            project: require(project, "--project")?,
            wait,
            timeout,
        }),
        "reply" => Command::Reply(ReplyOptions {
            project: require(project, "--project")?,
            comment: require(comment, "--comment")?,
            resolve,
            message: require_message()?,
        }),
        "log" => Command::Log(LogOptions {
            project: require(project, "--project")?,
            task: require(task, "--task")?,
            message: require_message()?,
        }),
        _ => {
            serve.project = project;
            Command::Serve(serve)
        }
    };
    Ok(Some(Invocation { root, command }))
}

fn positive(value: &str, flag: &str) -> Result<f64, String> {
    value
        .parse()
        .ok()
        .filter(|seconds: &f64| *seconds > 0.0)
        .ok_or_else(|| format!("{flag} must be a positive number"))
}

pub fn open_browser(url: &str) {
    let result = if cfg!(target_os = "macos") {
        Process::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        Process::new("cmd").args(["/C", "start", "", url]).spawn()
    } else {
        Process::new("xdg-open").arg(url).spawn()
    };
    if result.is_err() {
        eprintln!("Could not open a browser; visit {url}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> ParseOutcome {
        parse_args(args.iter().map(|arg| arg.to_string()))
    }

    fn run(args: &[&str]) -> Invocation {
        match parse(args) {
            ParseOutcome::Run(invocation) => invocation,
            other => panic!("expected a command, got {other:?}"),
        }
    }

    #[test]
    fn serve_is_the_default_command() {
        let invocation = run(&["--root", "/repo", "--port=8080", "--no-open"]);
        assert_eq!(invocation.root, PathBuf::from("/repo"));
        let Command::Serve(options) = invocation.command else {
            panic!("expected serve");
        };
        assert_eq!(options.port, 8080);
        assert!(!options.open);
        assert_eq!(options.host, "127.0.0.1");
        assert!(matches!(run(&["serve"]).command, Command::Serve(_)));
    }

    #[test]
    fn parses_agent_commands() {
        let Command::Inbox(inbox) = run(&["inbox", "--project", "p", "--wait"]).command else {
            panic!("expected inbox");
        };
        assert!(inbox.wait && inbox.timeout.is_none());

        let Command::Reply(reply) = run(&[
            "reply",
            "--project=p",
            "--comment",
            "c",
            "--resolve",
            "Fixed",
            "it",
        ])
        .command
        else {
            panic!("expected reply");
        };
        assert_eq!((reply.comment.as_str(), reply.resolve), ("c", true));
        assert_eq!(reply.message, "Fixed it");

        let Command::Log(log) = run(&[
            "log",
            "--project",
            "p",
            "--task",
            "05",
            "--",
            "-v flag works",
        ])
        .command
        else {
            panic!("expected log");
        };
        assert_eq!(
            (log.task.as_str(), log.message.as_str()),
            ("05", "-v flag works")
        );
    }

    #[test]
    fn reports_help_and_errors() {
        let error = |args: &[&str]| match parse(args) {
            ParseOutcome::Error(message) => message,
            other => panic!("expected an error, got {other:?}"),
        };
        assert!(matches!(parse(&["--help"]), ParseOutcome::Help));
        assert!(matches!(parse(&["log", "-h"]), ParseOutcome::Help));
        assert_eq!(error(&["--port"]), "--port requires a value");
        assert!(error(&["--interval", "0"]).contains("positive"));
        assert!(error(&["--bogus"]).contains("--bogus"));
        assert!(error(&["inbox"]).contains("--project"));
        assert!(error(&["inbox", "--project", "p", "--port", "1"]).contains("--port"));
        assert!(error(&["log", "--project", "p", "--task", "01"]).contains("message"));
        assert!(error(&["serve", "stray"]).contains("stray"));
    }
}
