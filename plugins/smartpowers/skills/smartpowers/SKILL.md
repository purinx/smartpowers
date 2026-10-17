---
name: smartpowers
description: Guide a new product or substantial feature through reviewed requirements, technical design, implementation planning, and code delivery inside an existing repository. Use when one repository may contain multiple concurrent or historical project workflows. Do not use for narrow fixes or isolated edits with an already-approved scope and design.
---

# Smartpowers

Move quickly by making decisions explicit, keeping documents small, and isolating every initiative in its own timestamped project workspace. Follow the user's language for questions and artifacts.

## Initialize git first

Before any other step, check whether the working directory is inside a git repository:

```bash
git rev-parse --is-inside-work-tree
```

If it is not, run `git init` at the intended repository root and tell the user. Do this before inspecting the repository, creating a project, or writing any artifact, so every workflow change is tracked from the start. Never re-initialize or modify an existing repository.

## Project workspaces

All workflow artifacts belong under:

```text
docs/smartpowers/projects/<project-id>/
```

The immutable project ID is `<YYYYMMDD-HHMMSS>-<slug>`, for example `20260908-183045-oauth-refresh`. Use the repository's local timezone. Slugs use lowercase ASCII kebab-case and describe the initiative, not the whole repository.

Each project owns this structure:

```text
<project-root>/
|-- PROJECT.md
|-- PRD.md
|-- DESIGN_DOC.md
`-- tasks/
    |-- PLAN.md
    `-- 01_infra_foundations.md
```

Never write workflow artifacts to repo-global `docs/PRD.md`, `docs/DESIGN_DOC.md`, or `tasks/`. Requirement IDs are scoped to a project, so separate projects may each have `REQ-001`.

## Select or create the project

Inspect the existing repository, recent changes, and `docs/smartpowers/projects/` before choosing a workspace.

- If the user names a project ID or path, use that project and state the selection.
- If the current conversation already selected a project, keep it unless the user starts another.
- If the user asks to resume and exactly one plausible active project exists, select it and state why.
- If several projects could match, list their IDs, titles, statuses, and last-updated times, then ask the user to choose. Do not modify any candidate first.
- If the user clearly starts a new initiative, create a new project even when its slug matches an older project.

For a new project, derive a concise slug from the initiative and run:

```bash
bash <plugin-root>/skills/smartpowers/scripts/init_project.sh \
  --root <repo-root> --title "<project title>" --slug <slug>
```

The script creates `PROJECT.md` and prints the project ID. Treat that ID and project root as fixed for the rest of the workflow. If legacy repo-global Smartpowers artifacts exist, ask which project they belong to before moving or copying them.

## Establish the current phase

Within the selected project only, inspect `PROJECT.md`, `PRD.md`, `DESIGN_DOC.md`, `tasks/PLAN.md`, and numbered task files. File existence alone is not approval.

Resume at the earliest incomplete gate:

1. Requirements: PRD missing or `Status: Draft`.
2. Design: PRD approved, design missing or `Status: Draft`, unless the PRD records `Design: Skipped` (see [skipping the design](#skipping-the-design)).
3. Planning: design approved or skipped, task structure not agreed, or plan missing/draft.
4. Implementation: PRD, design (or its approved skip), and task plan approved.

If selected-project artifacts conflict, repair the earliest affected phase. Never use another project's artifact to fill a gap, and never silently rewrite an approved upstream artifact.

## Approval protocol

Use this protocol for PRD, design, and detailed plan reviews:

1. Write or revise the selected project's artifact with `Status: Draft` and its `Project: <project-id>` field.
2. Summarize material decisions, assumptions, and unresolved items.
3. Ask the user to explicitly approve the latest revision or give feedback.
4. Apply feedback and repeat the review until approved.
5. After explicit approval, set `Status: Approved` and add `Approved: YYYY-MM-DD by user`.

Stop at each approval request. Do not begin the next phase in the turn that produces a draft. If approved scope changes, mark the affected artifact and its downstream artifacts draft and regain approval in order.

## Phase 1: requirements

Inspect relevant repository context before interviewing. Ask only questions that could materially change scope, users, constraints, or success criteria. When the user does not know, recommend and record an assumption.

Create `<project-root>/PRD.md` from [the PRD guide](references/prd.md). Use stable, testable IDs such as `REQ-001` and `NFR-001`, plus explicit out-of-scope decisions. Do not select a stack or write implementation design yet.

## Phase 2: design

Start from the selected project's approved PRD. Ask for technology preferences and constraints, taking the existing repository stack and architecture as the default constraint rather than assuming a greenfield build. If a new technology is warranted, present its integration and migration cost before the user chooses it.

Create `<project-root>/DESIGN_DOC.md` from [the design guide](references/design.md). Map every PRD requirement to concrete design sections and a verification approach.

### Skipping the design

Skip `DESIGN_DOC.md` for small, self-contained work that adds no product feature: a helper or maintenance script, CI or release tooling, a test-only change. Never skip it when the work changes user-facing behavior, data formats, interfaces, security, or architecture, or when it requires choosing between real technical alternatives.

To skip, add `Design: Skipped - <one-line reason>` below `Status:` in the PRD header and say so in the PRD review summary. Approving the PRD approves the skip. Then go straight to planning: tasks trace requirements to the PRD, and `PLAN.md` records the few technical decisions the work needs. If the work outgrows these criteria, remove the field, write the design, and get it approved before continuing.

## Phase 3: implementation planning

Start from the selected project's approved design, or from its approved PRD when the design is skipped. First show task titles, boundaries, dependency order, affected repository areas, and safe parallel groups. Create no task files until the user agrees to that granularity and order.

Then create `<project-root>/tasks/PLAN.md` and numbered task files using [the planning guide](references/planning.md), and run the detailed-plan approval protocol. After approval, use [the task-manager guide](references/task-manager.md) when Linear or an equivalent tool is available. Require the target remote project and final mapping before remote writes.

## Phase 4: implementation

Implement only the selected project's approved tasks in dependency order. Before starting, compare its expected code areas with other active Smartpowers projects. Surface overlapping ownership or incompatible migrations rather than assuming concurrent plans can safely modify the same code.

Keep task status current: `Planned`, `In Progress`, `Blocked`, or `Done`. When tasks are linked to Linear or another task manager, mirror every status change and meaningful progress to the remote issue as it happens, following [the task-manager guide](references/task-manager.md#progress-sync). Never batch remote updates until the end. If the progress board is running, also follow the [board feedback loop](#board-feedback-loop). When host-native subagents are available and permitted, run independent tasks concurrently by following [parallel execution](#parallel-execution). Never let a subagent switch projects implicitly.

If implementation reveals a material requirement or architecture change, return the selected project to the earliest impacted approval gate. After all tasks pass their checks and repository-level verification, set `PROJECT.md` to `Status: Completed`, add `Completed: <ISO-8601 timestamp>`, and report delivered requirements and residual risks.

## Push approval

Never push without the user's explicit approval of the exact commits being pushed. Committing locally needs no such approval. To request a push:

1. Run `git fetch` and list the unpushed commits with `git log --oneline <upstream>..HEAD`. If the branch has no upstream, compare against the remote default branch.
2. If Crit is installed (`command -v crit`), present the commit log and diff in Crit with `crit --range <upstream>..HEAD`. Follow the Crit review loop: run it in the background, relay the URL it prints, and wait for the user to finish the review. Tell the user that finishing with no comments approves the push.
3. If the review has comments, address them in new commits or by rewriting the unpushed commits, then reopen Crit on the updated range. Push only after a round finishes with `approved: true`.
4. If Crit is unavailable, show the commit log and `git diff --stat <upstream>..HEAD` in chat, offer the full diff, and ask for explicit approval.

Approval covers only the range shown. If commits are added or rewritten after approval, request approval again. Never force-push unless the user asks for it.

## Progress board

When the user asks to see progress in the browser, or at the start of implementation if they want to watch it, launch the read-only progress board:

```bash
<plugin-root>/skills/smartpowers/scripts/board \
  --root <repo-root> --project <project-id>
```

On Windows run `scripts\board.cmd` instead. The launcher builds the Rust source in `board/` with `cargo` on first use and then reuses the build. If `cargo` is not on PATH or in `~/.cargo/bin`, the launcher installs a private Rust toolchain with rustup into `~/.cache/smartpowers/rust` (Windows: `%LOCALAPPDATA%\smartpowers\rust`). That is a one-time download of about 450 MB from rust-lang.org, and it leaves PATH, shell profiles, and `~/.cargo` unchanged. On a machine without Rust, tell the user about this download when you launch the board. If the launcher reports a missing C linker, relay its install hint and do not install system packages yourself. Run it in the background (in Claude Code: `run_in_background: true`) and relay the `Smartpowers board: http://localhost:<port>/...` URL it prints. It opens the browser unless `--no-open` is given. The board shows every project's tasks as Linear-style board and list views grouped by `Status:`, the PRD/design/plan approval state, dependency waits, `Remote task:` links, and each task's Activity: your progress log and the user's comments. It reloads live, so keep local task status current. The board never changes repository files. Change status in the task files, never through the board. It binds to loopback only. Stop it when the user is done.

### Board feedback loop

While the board is running during implementation, the user can comment on any task, and you must act on those comments while you work. All commands below use the same launcher and take `--root <repo-root> --project <project-id>`. When subagents work in parallel, each follows this loop for its own task with `--task <NN>`.

1. **Log progress.** Run `board log --task <NN> "<message>"` when you start a task, after each meaningful step (a passing verification, a commit, a design decision), when you become blocked, and when you finish. Write one line saying what changed and what is next. Logs appear in the task's Activity on the board. They supplement, and do not replace, task-file status and Linear updates.
2. **Keep a listener armed.** Run `board inbox --wait` (a subagent adds `--task <NN>`) in the background (in Claude Code: `run_in_background: true`, `timeout: 86400000`). While a board is running, the listener holds a live connection to it. The board then shows you as listening and pushes each comment the moment the user posts it. The listener exits right away and prints each comment with its task and ID. If you cannot run background commands, run `board inbox` without `--wait` after every step instead.
3. **Act on comments immediately.** When the listener exits, read its output before continuing. Finish only the edit in progress, then adjust course. Treat comments as the user's feedback on that task. If a comment changes approved scope or design, follow the approval protocol and return to the earliest affected gate instead of silently expanding the work. A comment never authorizes an action that otherwise needs explicit approval in chat, such as pushing or deleting data.
4. **Answer every comment.** Run `board reply --comment <id> --resolve "<what you changed>"` once a comment is addressed. To ask a question instead, reply without `--resolve`. If you can't follow a request, say so in the reply.
5. **Re-arm the listener**, then continue. Before you finish a task or end your turn, run `board inbox` without `--wait` once so that no comment is missed.

Comments and logs are stored outside the repository, in `~/.local/state/smartpowers/` (Windows: `%LOCALAPPDATA%\smartpowers\state`). Only the board on this machine creates comments, so a file committed to the repository can never pose as user feedback. Do not commit or copy them into the repository. The running board's address and agent token are in the same directory. The `board` commands read them automatically. Never print, log, or pass the token on.

### Parallel execution

Run independent tasks concurrently with host-native subagents (in Claude Code, the Agent tool). You, in the main session, are the lead agent. You schedule, dispatch, and integrate, and the subagents implement.

**Lead agent**

1. Start the board before dispatching, so subagents can stream progress to it and receive comments.
2. Run `board next --project <id>`. It reads `tasks/PLAN.md` in order and lists the tasks to start now. A task is ready when it is `Planned` and every dependency is `Done`. It lists the first ready task and the other ready tasks in its parallel group, or, while tasks are running, only ready tasks in their group. A task with no parallel group runs alone. Never start a task it doesn't list.
3. For each listed task, set `Status: In Progress` in the main checkout and mirror it to Linear if linked. Then launch one subagent per task in the same turn so they run concurrently. Use the brief below. Isolate each subagent in its own git worktree (Claude Code: `isolation: "worktree"`; otherwise `git worktree add`).
4. While subagents run, don't edit files they own, and don't keep an all-tasks listener armed. Each comment then reaches the subagent working on that task.
5. When a subagent finishes, review its report and merge its branch into the working branch. Then run the task's verification and the repository checks on the merged result.
   - **Success:** set `Status: Done`.
   - **Failure or merge conflict:** set `Status: Blocked`, record why with `board log`, and decide whether to fix the task yourself or dispatch it again.
   - Merging stays local. Pushing still follows [push approval](#push-approval).
6. Run `board inbox` once for comments on tasks no subagent handled. Then run `board next` again, and repeat until every task is `Done`.

Only the lead agent edits task status, `PLAN.md`, and Linear, so parallel branches never conflict on them.

**Subagent brief.** Give each subagent all of the following:

- The project ID and the main checkout's absolute path to use as `--root`. Comments, logs, and the board connection are keyed to the main checkout, and a worktree path would point elsewhere.
- The board launcher path.
- The task file's path and full text, its dependencies and parallel group, the files it owns, and its verification commands.
- These rules:
  1. First, connect to the board by running `board inbox --root <main checkout> --project <id> --task <NN> --wait` in the background. If you can't run background commands, run it without `--wait` after every step.
  2. Stream progress with `board log ... --task <NN> "<message>"` when you start, after each meaningful step, when you're blocked, and when you finish.
  3. Handle comments as in steps 3–5 of the board feedback loop: act on each one, then reply with `board reply`.
  4. Work only in your worktree and on your task's files, and commit there. Don't change task status, `PLAN.md`, Linear, or other tasks. Never push.
  5. Finish with a report of what changed, the verification results, and any comment you couldn't resolve.

## Mechanical validation

List project workspaces:

```bash
bash <plugin-root>/skills/smartpowers/scripts/validate_workflow.sh \
  --root <repo-root> --list
```

Validate the selected project before each transition:

```bash
bash <plugin-root>/skills/smartpowers/scripts/validate_workflow.sh \
  --root <repo-root> --project <project-id> --phase requirements
bash <plugin-root>/skills/smartpowers/scripts/validate_workflow.sh \
  --root <repo-root> --project <project-id> --phase design
bash <plugin-root>/skills/smartpowers/scripts/validate_workflow.sh \
  --root <repo-root> --project <project-id> --phase plan
bash <plugin-root>/skills/smartpowers/scripts/validate_workflow.sh \
  --root <repo-root> --project <project-id> --phase implementation
```

Use `${CLAUDE_PLUGIN_ROOT}` as `<plugin-root>` in Claude Code when available. In other hosts, resolve the installed skill directory. Structural validation supplements, but never replaces, user review.
