---
name: smartpowers
description: Guide a new product or substantial feature through reviewed requirements, technical design, implementation planning, and code delivery inside an existing repository. Use when one repository may contain multiple concurrent or historical project workflows. Do not use for narrow fixes or isolated edits with an already-approved scope and design.
---

# Smartpowers

Move quickly by making decisions explicit, keeping documents small, and isolating every initiative in its own timestamped project workspace. Follow the user's language for questions and artifacts.

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
|-- design_docs/
|   `-- FIRST_BUILD.md
`-- tasks/
    |-- PLAN.md
    `-- 01_infra_foundations.md
```

Never write workflow artifacts to repo-global `docs/PRD.md`, `docs/design_docs/FIRST_BUILD.md`, or `tasks/`. Requirement IDs are scoped to a project, so separate projects may each have `REQ-001`.

## Select or create the project

Inspect the existing repository, recent changes, and `docs/smartpowers/projects/` before choosing a workspace.

- If the user names a project ID or path, use that project and state the selection.
- If the current conversation already selected a project, keep it unless the user starts another.
- If the user asks to resume and exactly one plausible active project exists, select it and state why.
- If several projects could match, list their IDs, titles, statuses, and last-updated times, then ask the user to choose. Do not modify any candidate first.
- If the user clearly starts a new initiative, create a new project even when its slug matches an older project.

For a new project, derive a concise slug from the initiative and run:

```bash
python3 <plugin-root>/skills/smartpowers/scripts/init_project.py \
  --root <repo-root> --title "<project title>" --slug <slug>
```

The script creates `PROJECT.md` and prints the project ID. Treat that ID and project root as fixed for the rest of the workflow. If legacy repo-global Smartpowers artifacts exist, ask which project they belong to before moving or copying them.

## Establish the current phase

Within the selected project only, inspect `PROJECT.md`, `PRD.md`, `design_docs/FIRST_BUILD.md`, `tasks/PLAN.md`, and numbered task files. File existence alone is not approval.

Resume at the earliest incomplete gate:

1. Requirements: PRD missing or `Status: Draft`.
2. Design: PRD approved, design missing or `Status: Draft`.
3. Planning: design approved, task structure not agreed, or plan missing/draft.
4. Implementation: PRD, design, and task plan approved.

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

Create `<project-root>/design_docs/FIRST_BUILD.md` from [the design guide](references/design.md). Map every PRD requirement to concrete design sections and a verification approach.

## Phase 3: implementation planning

Start from the selected project's approved design. First show task titles, boundaries, dependency order, affected repository areas, and safe parallel groups. Create no task files until the user agrees to that granularity and order.

Then create `<project-root>/tasks/PLAN.md` and numbered task files using [the planning guide](references/planning.md), and run the detailed-plan approval protocol. After approval, use [the task-manager guide](references/task-manager.md) when Linear or an equivalent tool is available. Require the target remote project and final mapping before remote writes.

## Phase 4: implementation

Implement only the selected project's approved tasks in dependency order. Before starting, compare its expected code areas with other active Smartpowers projects. Surface overlapping ownership or incompatible migrations rather than assuming concurrent plans can safely modify the same code.

Keep task status current: `Planned`, `In Progress`, `Blocked`, or `Done`. Use host-native subagents for tasks in the same parallel group when available and permitted. Give each one the selected project ID, bounded task, explicit file ownership, dependencies, and verification commands. Never let a subagent switch projects implicitly.

If implementation reveals a material requirement or architecture change, return the selected project to the earliest impacted approval gate. After all tasks pass their checks and repository-level verification, set `PROJECT.md` to `Status: Completed`, add `Completed: <ISO-8601 timestamp>`, and report delivered requirements and residual risks.

## Mechanical validation

List project workspaces:

```bash
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py \
  --root <repo-root> --list
```

Validate the selected project before each transition:

```bash
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py \
  --root <repo-root> --project <project-id> --phase requirements
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py \
  --root <repo-root> --project <project-id> --phase design
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py \
  --root <repo-root> --project <project-id> --phase plan
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py \
  --root <repo-root> --project <project-id> --phase implementation
```

Use `${CLAUDE_PLUGIN_ROOT}` as `<plugin-root>` in Claude Code when available. In other hosts, resolve the installed skill directory. Structural validation supplements, but never replaces, user review.
