//! Shared fixtures for unit tests.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::project::PROJECTS_PATH;
use crate::workspace::Workspace;

pub const PROJECT_ID: &str = "20260908-183045-oauth-refresh";

/// A throwaway repository with one project: task 01 is Done and linked to
/// ENG-42, task 02 is In Progress and depends on 01. Task 01 keeps a legacy
/// `Parallel group:` line, which the board ignores. Its state directory
/// lives inside the temp root. Everything is deleted on drop.
pub struct TempRepo {
    pub root: PathBuf,
    pub workspace: Workspace,
}

impl TempRepo {
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "smartpowers-board-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let workspace = Workspace::with_state_dir(root.clone(), root.join("state"));
        let repo = TempRepo { root, workspace };
        let project = repo.project();
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
        repo
    }

    pub fn project(&self) -> PathBuf {
        self.root.join(PROJECTS_PATH).join(PROJECT_ID)
    }

    pub fn project_state(&self) -> PathBuf {
        self.workspace.project_state(PROJECT_ID)
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
