# 03: init_project.sh

Status: Done
Project: 20261018-001457-bash-scripts
Requirements: REQ-001, REQ-002
Depends on: 01
Parallel group: B

## Objective

init_project.py を bash に移し、テストを tests/test_init_project.sh に移す。

## Design references

DESIGN_DOC.md の Interfaces and integrations、Critical flows(プロジェクトの作成)

## Verification

bash tests/run.sh が通る。同じ引数で Python 版と同じ結果になる。
