# PRD guide

Use this guide only during requirements work. Create the document at `<project-root>/PRD.md` and keep it concise enough for a user to review in one sitting.

## Required structure

```markdown
# Product Requirements Document

Status: Draft
Last updated: YYYY-MM-DD
Project: YYYYMMDD-HHMMSS-slug

## Summary

## Problem and outcome

## Target users

## Scope

### In scope

### Out of scope

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria |
| --- | --- | --- | --- |
| REQ-001 | Must | ... | Given/when/then or another observable result |

## Non-functional requirements

| ID | Area | Requirement | Measure |
| --- | --- | --- | --- |
| NFR-001 | Performance | ... | ... |

## User journeys

## Assumptions and constraints

## Success measures

## Risks

## Open questions
```

## Quality rules

- Describe user or business outcomes, not implementation choices.
- Give every requirement exactly one stable ID. Never renumber an existing ID after review; retire it explicitly if needed.
- Make acceptance criteria observable. Replace adjectives such as "fast" or "easy" with a measure or concrete behavior.
- Separate requirements from assumptions and unresolved questions.
- Include priority so the first build can be reduced without ambiguity.
- Keep future ideas out of scope unless the first release depends on them.

Before review, confirm that the document captures the user's stated objective, each requirement is testable, and no open question prevents meaningful approval.

## Skipping the design

For small, self-contained work that adds no product feature (see [skipping the design](../SKILL.md#skipping-the-design)), add this field below `Status:`:

```markdown
Design: Skipped - helper script with no user-facing behavior
```

Mention the skip in the review summary; approving the PRD approves it.
