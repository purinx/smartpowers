//! Command-line options and browser launching.

use std::path::PathBuf;
use std::process::Command;

pub const USAGE: &str = "\
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

#[derive(Debug)]
pub struct Options {
    pub root: PathBuf,
    pub project: Option<String>,
    pub host: String,
    pub port: u16,
    pub interval: f64,
    pub open: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            root: PathBuf::from("."),
            project: None,
            host: "127.0.0.1".into(),
            port: 0,
            interval: 1.0,
            open: true,
        }
    }
}

#[derive(Debug)]
pub enum ParseOutcome {
    Run(Options),
    Help,
    Error(String),
}

/// Parse `--flag value` and `--flag=value` style arguments.
pub fn parse_args(args: impl Iterator<Item = String>) -> ParseOutcome {
    match try_parse(args) {
        Ok(Some(options)) => ParseOutcome::Run(options),
        Ok(None) => ParseOutcome::Help,
        Err(message) => ParseOutcome::Error(message),
    }
}

fn try_parse(args: impl Iterator<Item = String>) -> Result<Option<Options>, String> {
    let mut options = Options::default();
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
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
        match flag.as_str() {
            "-h" | "--help" => return Ok(None),
            "--root" => options.root = PathBuf::from(value()?),
            "--project" => options.project = Some(value()?),
            "--host" => options.host = value()?,
            "--port" => {
                options.port = value()?
                    .parse()
                    .map_err(|_| "--port must be 0-65535".to_string())?;
            }
            "--interval" => {
                options.interval = value()?
                    .parse()
                    .ok()
                    .filter(|seconds: &f64| *seconds > 0.0)
                    .ok_or("--interval must be a positive number")?;
            }
            "--no-open" => options.open = false,
            other => return Err(format!("Unknown argument: {other}")),
        }
    }
    Ok(Some(options))
}

pub fn open_browser(url: &str) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> ParseOutcome {
        parse_args(args.iter().map(|arg| arg.to_string()))
    }

    #[test]
    fn parses_separate_and_inline_values() {
        let ParseOutcome::Run(options) = parse(&["--root", "/repo", "--port=8080", "--no-open"])
        else {
            panic!("expected options");
        };
        assert_eq!(options.root, PathBuf::from("/repo"));
        assert_eq!(options.port, 8080);
        assert!(!options.open);
        assert_eq!(options.host, "127.0.0.1");
    }

    #[test]
    fn reports_help_and_errors() {
        assert!(matches!(parse(&["--help"]), ParseOutcome::Help));
        assert!(
            matches!(parse(&["--port"]), ParseOutcome::Error(m) if m == "--port requires a value")
        );
        assert!(matches!(
            parse(&["--interval", "0"]),
            ParseOutcome::Error(_)
        ));
        assert!(matches!(parse(&["--bogus"]), ParseOutcome::Error(m) if m.contains("--bogus")));
    }
}
