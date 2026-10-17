//! Serve a Linear-style progress board for Smartpowers projects, and give
//! agents commands to read task comments, reply, and log progress.
//!
//! Std-only on purpose: every user builds the board from source on first
//! launch, so it must compile offline with nothing but a Rust toolchain.
//!
//! Modules:
//! - `cli`        command-line parsing and browser launching
//! - `agent`      `next`, `inbox`, `reply`, and `log` commands for coding agents
//! - `client`     HTTP client those commands use to reach a running board
//! - `runtime`    the running board's address and agent token
//! - `project`    projects and tasks, plus their comments and logs
//! - `server`     HTTP routing, JSON API, embedded web UI, and live events
//! - `watcher`    polling the workspace for changes
//! - `workspace`  the repository and its per-machine state directory
//! - `json`, `time`, `files`  small std-only helpers

mod agent;
mod cli;
mod client;
mod files;
mod json;
mod project;
mod runtime;
mod server;
mod time;
mod watcher;
mod workspace;

#[cfg(test)]
mod test_support;

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use cli::{Command, ParseOutcome, ServeOptions};
use watcher::Watcher;
use workspace::Workspace;

fn main() -> ExitCode {
    let invocation = match cli::parse_args(std::env::args().skip(1)) {
        ParseOutcome::Run(invocation) => invocation,
        ParseOutcome::Help => {
            println!("{}", cli::USAGE);
            return ExitCode::SUCCESS;
        }
        ParseOutcome::Error(message) => {
            eprintln!("error: {message}\n\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    let result = open_workspace(&invocation.root).and_then(|workspace| match &invocation.command {
        Command::Serve(options) => serve(workspace, options),
        Command::Next(options) => agent::next(&workspace, options),
        Command::Inbox(options) => agent::inbox(&workspace, options),
        Command::Reply(options) => agent::reply(&workspace, options),
        Command::Log(options) => agent::log(&workspace, options),
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn open_workspace(root: &Path) -> Result<Workspace, String> {
    let repo_root = root
        .canonicalize()
        .map_err(|error| format!("cannot open --root {}: {error}", root.display()))?;
    Ok(Workspace::new(repo_root))
}

fn serve(workspace: Workspace, options: &ServeOptions) -> Result<(), String> {
    if !server::is_loopback(&options.host) {
        return Err("the board serves repository files; bind it to a loopback host only".into());
    }
    if let Some(id) = &options.project {
        if project::load_project(&workspace, id).is_none() {
            return Err(format!("Project not found: {id}"));
        }
    }

    let host = options.host.trim_matches(['[', ']']);
    let listener = TcpListener::bind((host, options.port))
        .map_err(|error| format!("cannot listen on {host}:{}: {error}", options.port))?;
    let addr = listener
        .local_addr()
        .map_err(|error| format!("cannot read the listening address: {error}"))?;
    let port = addr.port();
    // Agent commands find this board, and prove they may use it, through this file.
    let info = runtime::ServerInfo::new(addr);
    info.save(&workspace.state_dir)?;

    let mut url = format!("http://localhost:{port}/");
    if let Some(id) = &options.project {
        url.push_str(&format!("#/{id}"));
    }
    // Agents read this line to find the URL; keep its format stable.
    println!("Smartpowers board: {url}");
    let _ = std::io::stdout().flush();
    eprintln!(
        "Comments and agent logs are stored in {}",
        workspace.state_dir.display()
    );
    if options.open {
        cli::open_browser(&url);
    }

    let workspace = Arc::new(workspace);
    let watcher = Watcher::new();
    watcher.spawn(
        Arc::clone(&workspace),
        Duration::from_secs_f64(options.interval),
    );
    let context = server::Context::new(workspace, watcher, info.token);
    server::serve(listener, Arc::new(context));
    Ok(())
}
