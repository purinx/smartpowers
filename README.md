# Smartpowers

Smartpowers is a lightweight development-workflow plugin and marketplace for Codex and Claude Code. It takes a product idea through four explicit stages:

1. requirements in `docs/PRD.md`;
2. first-build design in `docs/design_docs/FIRST_BUILD.md`;
3. reviewed implementation tasks in `tasks/` and optional task-manager synchronization;
4. implementation in dependency order, with safe parallel work where supported.

PRD, design, and task-plan transitions require explicit user approval. Stable `REQ-*` and `NFR-*` IDs keep requirements traceable through design and delivery.

## Repository layout

```text
.agents/plugins/marketplace.json                  Codex marketplace
.claude-plugin/marketplace.json                   Claude Code marketplace
plugins/smartpowers/.codex-plugin/plugin.json     Codex plugin manifest
plugins/smartpowers/.claude-plugin/plugin.json    Claude Code plugin manifest
plugins/smartpowers/skills/smartpowers/SKILL.md   shared workflow
```

Both marketplaces expose the plugin as `smartpowers@smartpowers`.

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
$smartpowers Build ...

# Claude Code
/smartpowers:smartpowers Build ...
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
python3 -m json.tool .agents/plugins/marketplace.json >/dev/null
python3 -m json.tool plugins/smartpowers/.codex-plugin/plugin.json >/dev/null
claude plugin validate . --strict
python3 -m unittest discover -s tests -v
```

To validate workflow artifacts in a target project:

```bash
python3 plugins/smartpowers/skills/smartpowers/scripts/validate_workflow.py --root /path/to/project --phase implementation
```
