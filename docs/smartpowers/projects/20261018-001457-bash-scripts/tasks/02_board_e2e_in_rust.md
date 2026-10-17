# 02: ボードの E2E テストの Rust 化

Status: In Progress
Project: 20261018-001457-bash-scripts
Requirements: REQ-007
Depends on: None
Parallel group: A

## Objective

test_board.py の 5 ケースを board/tests/board_e2e.rs に移し、manifests.rs でマニフェストの JSON の構文を確かめる。

## Design references

DESIGN_DOC.md の Critical flows(ボードの E2E テスト)

## Verification

cargo test と cargo clippy --all-targets -D warnings が通る。
