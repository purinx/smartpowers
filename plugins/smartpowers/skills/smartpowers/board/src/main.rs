//! Serve a read-only, Linear-style progress board for Smartpowers projects.
//!
//! Std-only on purpose: every user builds the board from source on first
//! launch, so it must compile offline with nothing but a Rust toolchain.
//!
//! Modules:
//! - `cli`      command-line options and browser launching
//! - `project`  reading projects and tasks from `docs/smartpowers/projects`
//! - `server`   HTTP routing, JSON API, embedded web UI, and live events
//! - `watcher`  polling the projects tree for changes
//! - `json`, `time`, `files`  small std-only helpers

mod cli;
mod files;
mod json;
mod project;
mod server;
mod time;
mod watcher;

#[cfg(test)]
mod test_support;

use std::io::Write;
use std::net::TcpListener;
use std::process::ExitCode;
use std::time::Duration;

use cli::{Options, ParseOutcome};
use watcher::Watcher;

fn main() -> ExitCode {
    let options = match cli::parse_args(std::env::args().skip(1)) {
        ParseOutcome::Run(options) => options,
        ParseOutcome::Help => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        ParseOutcome::Error(message) => {
            eprintln!("error: {message}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    match run(options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(options: Options) -> Result<(), String> {
    if !server::is_loopback(&options.host) {
        return Err("the board serves repository files; bind it to a loopback host only".into());
    }
    let repo_root = options
        .root
        .canonicalize()
        .map_err(|error| format!("cannot open --root {}: {error}", options.root.display()))?;
    if let Some(id) = &options.project {
        if project::load_project(&repo_root, id).is_none() {
            return Err(format!("Project not found: {id}"));
        }
    }

    let host = options.host.trim_matches(['[', ']']);
    let listener = TcpListener::bind((host, options.port))
        .map_err(|error| format!("cannot listen on {host}:{}: {error}", options.port))?;
    let port = listener
        .local_addr()
        .map_or(options.port, |address| address.port());

    let mut url = format!("http://localhost:{port}/");
    if let Some(id) = &options.project {
        url.push_str(&format!("#/{id}"));
    }
    // Agents read this line to find the URL; keep its format stable.
    println!("Smartpowers board: {url}");
    let _ = std::io::stdout().flush();
    if options.open {
        cli::open_browser(&url);
    }

    let watcher = Watcher::new();
    watcher.spawn(repo_root.clone(), Duration::from_secs_f64(options.interval));
    server::serve(listener, repo_root, watcher);
    Ok(())
}
