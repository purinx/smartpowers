# Smartpowers

Smartpowers is a lightweight development-workflow plugin for Codex and Claude Code. It takes a product idea through four explicit stages:

1. requirements in `docs/PRD.md`;
2. first-build design in `docs/design_docs/FIRST_BUILD.md`;
3. reviewed implementation tasks in `tasks/` and optional task-manager synchronization;
4. implementation in dependency order, with safe parallel work where supported.

PRD, design, and task-plan transitions require explicit user approval. Stable `REQ-*` and `NFR-*` IDs keep requirements traceable through design and delivery.

## Repository layout

```text
.codex-plugin/plugin.json       Codex manifest
.claude-plugin/plugin.json      Claude Code manifest
skills/smartpowers/SKILL.md     shared workflow
skills/smartpowers/references/  phase-specific guides and templates
skills/smartpowers/scripts/     artifact validator
```

## Try with Claude Code

From the parent directory:

```bash
claude --plugin-dir ./smartpowers
```

Then run `/smartpowers:smartpowers` with a product idea. During development, `/reload-plugins` reloads changes.

## Package for Codex

The repository root is a valid Codex plugin directory and can be added to a Codex plugin marketplace as `smartpowers`. The shared skill is discovered through the `skills` path declared in `.codex-plugin/plugin.json`.

## Validate

```bash
python3 -m json.tool .codex-plugin/plugin.json >/dev/null
claude plugin validate . --strict
python3 -m unittest discover -s tests -v
```

To validate workflow artifacts in a target project:

```bash
python3 skills/smartpowers/scripts/validate_workflow.py --root /path/to/project --phase implementation
```
