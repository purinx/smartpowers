# 01: bash のテストの土台

Status: Done
Project: 20261018-001457-bash-scripts
Requirements: REQ-006, NFR-001, NFR-002
Depends on: None
Parallel group: A

## Objective

tests/run.sh と tests/lib.sh を作り、マーケットプレイスのテストを bash に移す。

## Design references

DESIGN_DOC.md の Architecture and component boundaries(tests/lib.sh、tests/run.sh)

## Verification

bash tests/run.sh が通り、わざと失敗させると名前と理由が出て 0 以外で終わる。/bin/bash(3.2)でも通る。
