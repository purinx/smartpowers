# 05: 切り替えと Python の削除

Status: Planned
Project: 20261018-001457-bash-scripts
Requirements: REQ-005, REQ-008, REQ-009
Depends on: 02, 03, 04
Parallel group: None

## Objective

SKILL.md、README、CI、.gitignore を bash 版に切り替え、Python のファイルを消す。

## Design references

DESIGN_DOC.md の Interfaces and integrations(CI)

## Verification

git ls-files '*.py' が空。CI が 3 つの OS で緑。
