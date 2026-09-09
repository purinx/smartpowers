# First-build design guide

Use this guide only after the selected project's `PRD.md` is approved and the user has chosen or accepted a technology stack. Create the design at `<project-root>/DESIGN_DOC.md`.

## Required structure

```markdown
# First Build Design

Status: Draft
Last updated: YYYY-MM-DD
Project: YYYYMMDD-HHMMSS-slug
PRD: ../PRD.md

## Design summary

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |

## System context

## Architecture and component boundaries

## Data model and state

## Interfaces and integrations

## Critical flows

## Security and privacy

## Reliability and failure handling

## Observability and operations

## Testing strategy

## Deployment and rollout

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture; Critical flows | Integration test ... |

## Risks and tradeoffs

## Open questions
```

Omit an irrelevant operational section only with a one-line explanation. Add domain-specific sections when the implementation needs them.

## Traceability rules

- Include every `REQ-*` and `NFR-*` ID from the approved PRD exactly as written.
- Map each ID to concrete section names and a verification approach.
- Do not use "covered globally" or another non-specific mapping.
- When one design decision covers several requirements, list each requirement separately.
- If a requirement cannot be designed, return to the PRD rather than hiding the gap.

Record decisions that constrain implementation. Avoid class-by-class pseudodesign, speculative scaling systems, and abstractions without a first-build use case.

Treat the existing repository as part of the system context. Identify reused components, compatibility constraints, migrations, and intentionally unchanged areas. Do not describe the project as a greenfield application unless it actually is one.
