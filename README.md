# Smartpowers

Smartpowers is a lightweight development-workflow plugin and marketplace for Codex and Claude Code. It manages multiple independent initiatives inside an existing repository and takes each one through four explicit stages:

1. requirements in the selected project's `PRD.md`;
2. first-build design in its `DESIGN_DOC.md`;
3. reviewed implementation tasks in its `tasks/` directory and optional task-manager synchronization;
4. implementation in dependency order, with safe parallel work where supported.

Each initiative gets an immutable, sortable ID such as `20260908-183045-oauth-refresh` and lives under `docs/smartpowers/projects/<project-id>/`. PRD, design, and task-plan transitions require explicit user approval. Stable `REQ-*` and `NFR-*` IDs keep requirements traceable within that project through design and delivery.

```text
docs/smartpowers/projects/20260908-183045-oauth-refresh/
|-- PROJECT.md
|-- PRD.md
|-- DESIGN_DOC.md
`-- tasks/
    |-- PLAN.md
    `-- 01_infra_foundations.md
```

Starting another initiative creates another timestamped directory, even when it uses the same slug. Existing project artifacts are selected explicitly and never merged implicitly.

## Repository layout

```text
.agents/plugins/marketplace.json                  Codex marketplace
.claude-plugin/marketplace.json                   Claude Code marketplace
plugins/smartpowers/.codex-plugin/plugin.json     Codex plugin manifest
plugins/smartpowers/.claude-plugin/plugin.json    Claude Code plugin manifest
plugins/smartpowers/skills/smartpowers/SKILL.md   shared workflow
plugins/smartpowers/skills/smartpowers/assets/board/  progress board web UI
plugins/smartpowers/skills/smartpowers/board/         progress board server (Rust source)
CHANGELOG.md                                      release notes, one section per version
scripts/release-notes.sh                          release tag check and notes extraction
```

Both marketplaces expose the plugin as `smartpowers@smartpowers`.

## Progress board

> [!NOTE]
> The board is built from source on your machine. No prebuilt binaries are shipped. If `cargo` is not installed, the launcher uses [rustup](https://rustup.rs) to install a private Rust toolchain into `~/.cache/smartpowers/rust` (Windows: `%LOCALAPPDATA%\smartpowers\rust`). This is a one-time download of about 450 MB, and it does not change your PATH, shell profile, or `~/.cargo`. To reclaim the space, delete that directory. The rest of the Smartpowers workflow does not need Rust.
>
> Building needs a C linker: the Xcode Command Line Tools on macOS (`xcode-select --install`), or `build-essential`/`gcc` on Linux. Windows needs nothing extra.
>
> CI builds and tests the board on macOS, Linux, and Windows, including first launch without Rust. Day-to-day use has been verified on macOS. **Contributions welcome!** Reports or fixes from Linux and Windows users are especially appreciated. See [Develop with Claude Code](#develop-with-claude-code) for local setup.

Ask the agent to "show the Smartpowers board", or run it yourself:

```bash
# macOS / Linux
plugins/smartpowers/skills/smartpowers/scripts/board --root <repo-root>

# Windows
plugins\smartpowers\skills\smartpowers\scripts\board.cmd --root <repo-root>
```

The board server is a single Rust program with the web UI embedded, using only the standard library (no third-party crates). The first launch compiles it with `cargo build --release` into `board/target/`, which takes a few seconds. Later launches reuse that build and recompile only after a plugin update changes the source. Run the board's tests with `cargo test --manifest-path plugins/smartpowers/skills/smartpowers/board/Cargo.toml`.

It starts a local server on `127.0.0.1` and opens a Linear-style board in your browser. You get board and list views per project grouped by task status, the approval state of each phase, overall progress, dependency waits, and links to remote issues. The page updates live, so you can watch implementation progress as the agent works. Keyboard: `b` board, `l` list, `/` filter, `Esc` close. Options: `--project <id>`, `--port <n>`, `--no-open`.

### Steer the agent from the board

Each task's detail panel has an **Activity** section and a comment box:

- **Progress log:** the agent records what it did and what is next with `board log`. The latest entry also appears on in-progress cards.
- **Comments:** type feedback and press <kbd>⌘/Ctrl</kbd>+<kbd>Enter</kbd>. While the agent implements tasks, it keeps `board inbox --wait` running in the background. Your comment ends that wait, so the agent sees it during its current work rather than at the end. The agent then adjusts course and answers with `board reply`, and the comment shows **Waiting for agent → Sent to agent → Addressed**.

```bash
board next  --project <id> [--limit <n>]                          # tasks the lead agent can start now
board inbox --project <id> [--task <nn>] [--wait] [--timeout <secs>]  # new comments, each delivered once
board reply --project <id> --comment <id> [--resolve] <message>  # answer as the agent
board log   --project <id> --task <nn> <message>                 # add a progress entry
```

### Parallel tasks with subagents

The lead agent starts the board, runs `board next`, and hands each listed task to its own subagent in its own git worktree. `board next` returns ready tasks from `tasks/PLAN.md` that share a parallel group. Each subagent connects to the board with `board inbox --task <nn> --wait`, streams progress with `board log`, and receives comments on its task the moment you post them. Cards of tasks with a connected agent show a live **Agent** badge. As subagents finish, the lead agent merges their branches, marks tasks Done, and dispatches the next ready group. While a board is running, these commands use its API, authenticated with a per-run token stored next to the comments. Without a board, they fall back to reading and writing those files directly.

Comments and logs live outside the repository, in `~/.local/state/smartpowers/<repo>-<hash>/` (Windows: `%LOCALAPPDATA%\smartpowers\state`; override with `SMARTPOWERS_STATE_HOME`). Agents treat comments as your instructions, so keeping them out of version control means a file someone commits can never pose as your feedback. The board writes nothing to the repository, and it accepts new comments only from its own page.

## Install from GitHub

Codex:

```bash
codex plugin marketplace add purinx/smartpowers
codex plugin add smartpowers@smartpowers
```

Claude Code:

```bash
claude plugin marketplace add purinx/smartpowers
claude plugin install smartpowers@smartpowers
```

Start a new Codex or Claude Code session after installation. Invoke the workflow with:

```text
# Codex
$smartpowers Start a new project for ...

# Claude Code
/smartpowers:smartpowers Start a new project for ...
```

## Update

Codex:

```bash
codex plugin marketplace upgrade smartpowers
codex plugin add smartpowers@smartpowers
```

Claude Code:

```bash
claude plugin marketplace update smartpowers
claude plugin update smartpowers@smartpowers
```

Start a new session after updating. In an existing Claude Code session, `/reload-plugins` can activate the updated plugin.

## Uninstall

Codex:

```bash
codex plugin remove smartpowers@smartpowers
codex plugin marketplace remove smartpowers
```

Claude Code:

```bash
claude plugin uninstall smartpowers@smartpowers
claude plugin marketplace remove smartpowers
```

## Install from a local checkout

Clone the repository, then register its absolute path:

```bash
git clone https://github.com/purinx/smartpowers.git
cd smartpowers

codex plugin marketplace add "$PWD"
codex plugin add smartpowers@smartpowers

claude plugin marketplace add "$PWD"
claude plugin install smartpowers@smartpowers
```

## Develop with Claude Code

From the repository root:

```bash
claude --plugin-dir ./plugins/smartpowers
```

Then run `/smartpowers:smartpowers` with a product idea. During development, `/reload-plugins` reloads changes.

## Validate

```bash
claude plugin validate . --strict
bash tests/run.sh
cargo test --manifest-path plugins/smartpowers/skills/smartpowers/board/Cargo.toml
```

`tests/run.sh` runs the shell tests for the workflow scripts and plugin manifests; it needs only bash (3.2 or later) and standard Unix tools, and on Windows runs in Git Bash. `cargo test` covers the board, including end-to-end checks of the built server and a JSON syntax check of the manifests.

To validate workflow artifacts in a target project:

```bash
bash plugins/smartpowers/skills/smartpowers/scripts/validate_workflow.sh \
  --root /path/to/repository --list

bash plugins/smartpowers/skills/smartpowers/scripts/validate_workflow.sh \
  --root /path/to/repository \
  --project 20260908-183045-oauth-refresh \
  --phase implementation
```

## Releasing

Every release gets a `vX.Y.Z` tag and a GitHub release whose notes come from `CHANGELOG.md`.

1. In one commit named `Release X.Y.Z`, bump the version in both plugin manifests and `plugins/smartpowers/skills/smartpowers/board/Cargo.toml` (then run `cargo build` to update `Cargo.lock`), and add a `## [X.Y.Z] - YYYY-MM-DD` section to `CHANGELOG.md`. CI fails if the versions disagree or the current version has no changelog entry.
2. Push it to `main`.
3. Tag that commit and push the tag:

   ```bash
   git tag -a vX.Y.Z -m "Smartpowers X.Y.Z"
   git push origin vX.Y.Z
   ```

The [Release workflow](.github/workflows/release.yml) checks that the tag matches every declared version, then publishes the GitHub release with that changelog section as its notes. Preview the notes locally with `scripts/release-notes.sh --tag vX.Y.Z`.

Tags before 0.8.0 were added after the fact. `v0.1.0`–`v0.3.0` predate release commits and point at the last commit that declared that version. `v0.7.0` points at `d962bcb` (the commit that added the release workflow, right after `Release 0.7.0`) so the first automated release could run. Releases from 0.8.0 on tag the `Release X.Y.Z` commit itself.
