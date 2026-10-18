# Changelog

All notable changes to Smartpowers are recorded here. Versions follow [Semantic Versioning](https://semver.org/). Each `## [x.y.z]` section becomes the GitHub release notes for tag `vx.y.z`; see [Releasing](README.md#releasing).

## [0.9.0] - 2026-10-18

### Added

- Flow view on the board. Press `f` or choose **Flow** to see the plan's dependency graph: tasks flow left to right by dependency depth, placed near the tasks they connect to, with arrows for `Depends on`, a status-colored edge on each task, live status, the Agent badge, and a **Ready** chip on tasks whose dependencies are all Done. The graph opens fitted to the view and centered; scroll or drag the background to pan, zoom with Ctrl/Cmd + scroll or a pinch, and press **Fit** to see it all again. Hovering a task highlights its arrows. Click a node to open its details. `#/<project>/<task>?view=flow` links restore the view and the panel. The view works on draft plans, so the execution order can be reviewed before approval, and it shows a warning when a dependency is missing or tasks form a cycle.
- The plan and implementation phases of `validate_workflow.sh` check the dependency graph: every dependency must exist, there must be no cycle, and the `Depends on` column of the execution order table in `tasks/PLAN.md` must match each task file.

### Changed

- The lead agent starts tasks as soon as they become ready. It lists `Planned` tasks whose dependencies are all `Done`, dispatches each to a subagent unless its files overlap a running task, and checks again every time a task finishes, without waiting for the other running tasks.
- Each task finished by a subagent gets its own pull request. When its verification passes, the subagent pushes its task branch and opens the pull request with `gh pr create`; the user reviews and merges it there, without a separate Crit review. The task stays In Progress until the merge; the lead then updates the working branch, checks it, and marks the task Done. Task pull requests are stacked on the working branch, which the lead pushes first; pushing to the head branch of an open pull request needs no separate approval. Without `gh` or a remote, the lead verifies the branch and merges it locally. Subagents stop their comment listener when they finish.
- Plans express what may run concurrently only through dependencies: tasks that would conflict when run together must depend on each other, and tasks with no dependency path may run at the same time.

### Removed

- `board next`. The lead agent picks ready tasks itself; the board's Flow view shows the same set.
- Parallel groups. The planning guide no longer has a `Parallel group` column or field. Existing task files that still have one load and validate as before; the line is ignored.

## [0.8.0] - 2026-10-18

### Added

- Small, self-contained work that adds no product feature (a helper script, CI or release tooling, a test-only change) can skip the design doc. Record `Design: Skipped - <reason>` in the PRD; approving the PRD approves the skip, and planning starts from the PRD. The validator accepts it and the board shows the design phase as **Skipped**.

### Changed

- The workflow scripts are bash: `init_project.sh` and `validate_workflow.sh` replace the Python scripts with the same arguments, output, and exit codes. Smartpowers no longer needs `python3`; on Windows the scripts run in Git Bash.
- A requirement ID written directly next to Japanese text, such as `REQ-001の`, now counts toward traceability. The Python validator missed it.
- The repository's tests no longer use Python: shell tests run with `bash tests/run.sh`, and the board's end-to-end tests run under `cargo test`.

## [0.7.0] - 2026-10-17

### Added

- Parallel task execution with subagents. The lead agent runs `board next`, which reads `tasks/PLAN.md` in order and lists the ready tasks that may run together: their dependencies are Done and they share a parallel group with each other and with any running task. The lead hands each listed task to its own subagent in its own git worktree. `SKILL.md` describes the lead and subagent protocol, including merging, status ownership, and the subagent brief.
- Subagents connect to the running board. `board inbox --task <nn> --wait` holds a live connection, so a comment reaches the subagent working on that task as soon as it is posted. Cards of tasks with a connected agent show a live **Agent** badge, and the header shows when the lead agent is listening.
- Token-protected agent API. The board writes its address and a per-run token to the state directory (owner-only on Unix). While a board is running, `inbox`, `reply`, and `log` go through the API; without one, they read and write files directly.

## [0.6.0] - 2026-10-17

### Added

- Task comments. Write feedback in a task's detail panel and press ⌘/Ctrl+Enter. An agent keeping `board inbox --wait` running in the background receives it during its current work, answers with `board reply`, and the comment moves from **Waiting for agent** to **Sent to agent** to **Addressed**.
- Agent progress logs. `board log --task <nn> <message>` adds an entry to the task's new **Activity** timeline, and in-progress cards show the latest entry.
- The board feedback loop in `SKILL.md`: log progress, keep a listener armed, act on comments immediately within approved scope, answer every comment, and re-arm.

### Security

- Comments and logs live in a per-machine state directory outside the repository (`~/.local/state/smartpowers`, or `SMARTPOWERS_STATE_HOME`), so a committed file can never pose as user feedback. Posting a comment requires a loopback `Origin`.

## [0.5.0] - 2026-10-17

### Changed

- Spotify-style palette with a green accent, pill-shaped controls, and every text color at WCAG AA contrast in both light and dark themes.
- Scrollbars are a thin accent line that thickens on hover.

### Fixed

- A long task detail panel can now be scrolled. Before, it was clipped at the bottom of the window.

## [0.4.0] - 2026-10-17

### Added

- The task detail panel can be resized by dragging its left edge. The width is remembered, and a double-click resets it.
- When `cargo` is missing, the board launcher installs a private Rust toolchain with rustup into `~/.cache/smartpowers/rust` (Windows: `%LOCALAPPDATA%\smartpowers\rust`). It leaves your PATH, shell profile, and `~/.cargo` untouched.

### Fixed

- Installing the private toolchain on Windows. rustup only acts as an installer when the file is named `rustup-init.exe`.

## [0.3.0] - 2026-10-17

### Added

- A Linear-style progress board: board and list views grouped by task status, phase approvals, overall progress, dependency waits, and remote issue links. It updates live as task files change. The server is a std-only Rust program built from source on first launch and serves only loopback addresses.
- CI for the Rust and Python test suites on macOS, Linux, and Windows.
- Push approval: before pushing, Smartpowers shows the unpushed commit log and diff in Crit and pushes only after an approved review.
- The workflow initializes a git repository first when the directory isn't one yet.
- Linear progress sync: when tasks are linked, their status and progress update on Linear as work happens.

## [0.2.0] - 2026-09-09

### Added

- Multiple timestamped project workspaces in one repository under `docs/smartpowers/projects/<id>/`.

## [0.1.0] - 2026-09-08

### Added

- First release: a review-gated workflow from requirements through design, planning, and implementation, with marketplaces for Codex and Claude Code.
