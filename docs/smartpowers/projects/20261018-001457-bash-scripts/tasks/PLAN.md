# Implementation Plan

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-18
Project: 20261018-001457-bash-scripts
Design: ../DESIGN_DOC.md

## Delivery strategy

テストの土台とボードの E2E テストを先に用意し、その上でスクリプトを 1 本ずつ移す。Python 版との比較が済んでから、切り替えと削除を行い、最後に 0.8.0 として出す。

## Execution order

| Order | Task | Depends on | Parallel group | Requirements |
| --- | --- | --- | --- | --- |
| 01 | [bash のテストの土台](01_bash_test_runner.md) | None | A | REQ-006, NFR-001, NFR-002 |
| 02 | [ボードの E2E テストの Rust 化](02_board_e2e_in_rust.md) | None | A | REQ-007 |
| 03 | [init_project.sh](03_init_project_sh.md) | 01 | B | REQ-001, REQ-002 |
| 04 | [validate_workflow.sh](04_validate_workflow_sh.md) | 01 | B | REQ-003, REQ-004, NFR-003 |
| 05 | [切り替えと Python の削除](05_switch_over.md) | 02, 03, 04 | None | REQ-005, REQ-008, REQ-009 |
| 06 | [0.8.0 のリリース](06_release.md) | 05 | None | REQ-010 |

## Integration checkpoints

- 04 の完了時: このリポジトリの全プロジェクトで、Python 版と bash 版の出力が一致する。
- 05 の完了時: CI が 3 つの OS で緑になる。

## Plan risks

- 04 の比較が終わる前に Python 版を消さない(05 は 04 に依存する)。
