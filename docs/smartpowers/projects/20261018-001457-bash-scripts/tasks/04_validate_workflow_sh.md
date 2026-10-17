# 04: validate_workflow.sh

Status: Done
Project: 20261018-001457-bash-scripts
Requirements: REQ-003, REQ-004, NFR-003
Depends on: 01
Parallel group: B

## Objective

validate_workflow.py を bash に移し、テストを tests/test_validate_workflow.sh に移す。PRD の Design: Skipped にも対応する(SKILL.md の追加ルール)。

## Design references

DESIGN_DOC.md の Data model and state、Critical flows(検証)

## Verification

全プロジェクトの全フェーズと --list で、Python 版と出力が一致する。
