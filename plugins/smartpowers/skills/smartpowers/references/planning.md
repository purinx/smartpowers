# Implementation planning guide

Use this guide only after the design is approved (or skipped in the approved PRD) and the user agrees to the proposed granularity and execution order.

When the PRD records `Design: Skipped`, write `Design: Skipped (see ../PRD.md)` in the plan header instead of the design link, and add a short `## Technical decisions` section after `## Delivery strategy`.

## Plan index

Create `<project-root>/tasks/PLAN.md`:

```markdown
# Implementation Plan

Status: Draft
Last updated: YYYY-MM-DD
Project: YYYYMMDD-HHMMSS-slug
Design: ../DESIGN_DOC.md

## Delivery strategy

## Execution order

| Order | Task | Depends on | Parallel group | Requirements |
| --- | --- | --- | --- | --- |
| 01 | [Infrastructure foundations](01_infra_foundations.md) | None | A | NFR-001 |

## Integration checkpoints

## Plan risks
```

Use two-digit numeric prefixes and lowercase snake_case filenames: `01_infra_foundations.md`, `02_domain_model.md`. Order numbers represent integration order, not priority.

## Task file

```markdown
# 01: Infrastructure foundations

Status: Planned
Project: YYYYMMDD-HHMMSS-slug
Requirements: NFR-001
Depends on: None
Parallel group: A

## Objective

## Design references

## Scope

### Included

### Excluded

## Expected code areas

## Implementation steps

## Verification

## Acceptance criteria

## Handoff notes
```

## Sizing rules

- A task should produce one coherent, verifiable increment and normally fit one focused agent assignment.
- Split tasks when they span unrelated ownership boundaries or have independently useful verification.
- Merge tasks when separation would create placeholder code or unavoidable back-and-forth edits.
- Put shared foundations before consumers.
- Assign a parallel group only when tasks have no dependency edge and no overlapping file or migration ownership.
- Compare expected code areas with other active projects before declaring tasks safe to run concurrently.
- Include integration tasks when independently built pieces need explicit assembly or end-to-end verification.
- Cover every PRD requirement across the numbered task files. A task may cover multiple related requirements.
