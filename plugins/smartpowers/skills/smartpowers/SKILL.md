---
name: smartpowers
description: Guide a new product or substantial feature from an initial idea through reviewed requirements, technical design, implementation planning, and code delivery. Use when the user wants a structured end-to-end development workflow. Do not use for narrow fixes, isolated code edits, or requests that cover only one already-defined phase.
---

# Smartpowers

Move quickly by making decisions explicit, keeping documents small, and stopping only at decisions that require the user. Follow the user's language for questions and artifacts.

## Establish the current phase

Inspect the repository and these artifacts before acting:

- `docs/PRD.md`
- `docs/design_docs/FIRST_BUILD.md`
- `tasks/PLAN.md` and numbered `tasks/*.md` files
- relevant source, project instructions, and connected task-management tools

Use the latest explicit approval recorded in the documents and conversation. File existence alone is not approval. Never infer user approval from silence, previous edits, or your own assessment.

Resume at the earliest incomplete gate:

1. Requirements: PRD missing or `Status: Draft`.
2. Design: PRD approved, design missing or `Status: Draft`.
3. Planning: design approved, task structure not agreed, or `tasks/PLAN.md` is missing/draft.
4. Implementation: PRD, design, and task plan approved.

If the artifacts conflict, explain the conflict and repair it in the earliest affected phase. Do not silently rewrite an approved upstream artifact.

## Approval protocol

Use the same protocol for PRD, design, and detailed implementation plan reviews:

1. Write or revise the artifact with `Status: Draft`.
2. Summarize material decisions, assumptions, and unresolved items.
3. Ask the user to review the artifact and explicitly approve it or give feedback.
4. If feedback arrives, update the artifact and repeat the review.
5. Only after explicit approval of the latest revision, change it to `Status: Approved` and add `Approved: YYYY-MM-DD by user`.

Stop at each approval request. Do not start work from the next phase in the same turn that produces a draft for review. If scope changes after approval, mark the affected artifact and all downstream artifacts `Status: Draft`, revise them in order, and regain approval.

## Phase 1: requirements

Before drafting, inspect existing product context. Ask a small batch of only the questions whose answers could materially change scope, users, constraints, or success criteria. When the user does not know, offer a recommended assumption and record it instead of conducting an exhaustive interview.

Create `docs/PRD.md` from [the PRD guide](references/prd.md). Requirements must be testable and use stable IDs: `REQ-001`, `REQ-002`, and `NFR-001`. Include explicit out-of-scope decisions so later phases do not grow the product accidentally.

Do not select a technology stack or write implementation design during this phase.

## Phase 2: design

Start only from an approved PRD. Before drafting, ask the user for their preferred technology stack and material constraints such as deployment target, existing infrastructure, required integrations, budget, and operational ownership. If they have no preference, recommend a compact default with tradeoffs and obtain their choice before writing the design.

Create `docs/design_docs/FIRST_BUILD.md` from [the design guide](references/design.md). Keep it at first-build depth: enough to implement without rediscovering architecture, but no speculative future platform work.

Every PRD requirement and non-functional requirement must appear in the traceability matrix and map to one or more concrete design sections. Resolve every uncovered requirement before asking for design approval.

## Phase 3: implementation planning

Start only from an approved design. Planning has two gates:

1. Discuss a proposed task outline with the user before creating files. Show task titles, boundaries, dependency order, and safe parallel groups. Ask whether the granularity and order are right.
2. After the outline is agreed, create `tasks/PLAN.md` and numbered task files such as `tasks/01_infra_foundations.md` using [the planning guide](references/planning.md). Set `tasks/PLAN.md` to `Status: Draft`, then run the detailed-plan review protocol.

Each task must be independently understandable, cite its covered `REQ-*`/`NFR-*` IDs, state dependencies, name expected code areas, describe verification, and define completion criteria. Avoid tasks that merely say "implement the design."

After the detailed local plan is approved, use [the task-manager integration guide](references/task-manager.md) when a Linear or equivalent task-management tool is available. Never create remote issues until the user identifies the target project and confirms the final task mapping.

## Phase 4: implementation

Implement approved tasks in dependency order. Before each task, read its task file and relevant design sections. Keep the task status current: `Planned`, `In Progress`, `Blocked`, or `Done`. Mark it done only after its stated checks and acceptance criteria pass.

Use the host's native subagent mechanism for tasks in the same declared parallel group when it is available and the user permits parallel agent work. Give each subagent a bounded task, explicit file ownership, dependencies, and verification commands. Do not parallelize tasks that edit the same files, depend on uncommitted outputs, share a migration boundary, or require the same external resource. Integrate and verify all parallel results in the main agent.

Stay within the approved plan. If implementation reveals a material requirement or architecture change, stop the affected work and return to the earliest impacted approval gate. Small implementation details consistent with the design do not require a new gate.

After all tasks are done, run the repository's relevant test, lint, typecheck, and build commands. Report completed requirements, verification evidence, residual risks, and any deferred work.

## Mechanical validation

Run the bundled validator before requesting approval for a transition:

```bash
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py --root <project-root> --phase requirements
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py --root <project-root> --phase design
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py --root <project-root> --phase plan
python3 <plugin-root>/skills/smartpowers/scripts/validate_workflow.py --root <project-root> --phase implementation
```

Use the phase matching the artifact being reviewed; use `implementation` immediately before coding. Use `${CLAUDE_PLUGIN_ROOT}` as `<plugin-root>` in Claude Code when available. In other hosts, resolve the installed skill directory. The validator checks structure, approval gates, filenames, and requirement-ID coverage; it does not replace user review or engineering judgment.
