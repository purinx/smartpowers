# 03: 検証スクリプトのグラフ検査

Status: Planned
Project: 20261018-010816-execution-flowchart
Requirements: REQ-002, REQ-003
Depends on: None

## Objective

`validate_workflow.sh` の `plan` と `implementation` のフェーズで、依存関係のグラフと、`PLAN.md` の表とタスクファイルの一致を検査する。

## Design references

DESIGN_DOC.md の Interfaces and integrations(検証スクリプト)、Testing strategy(検証スクリプトのテスト)

## Scope

### Included

- `validate_plan` での、`Depends on:` の解析、存在しない依存先の検査、循環の検査
- `PLAN.md` の `Depends on` の列がある表の検出と、タスクファイルとの一致の検査
- `Parallel group` の列と欄を無視すること
- `tests/test_validate_workflow.sh` へのテストの追加と、`valid_project` の更新

### Excluded

- ボード側のグラフの計算(02)

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/scripts/validate_workflow.sh`
- `tests/test_validate_workflow.sh`

## Implementation steps

1. タスクファイルの `Depends on:` からタスク番号を取り出す関数を書く。`None`、`-`、空は依存なしとし、`01`、`Task 01`、リンクを受け付ける。
2. 存在しない依存先を検査する。
3. 依存先がすべて取り除かれたタスクを繰り返し取り除き、残ったタスクを循環として報告する。
4. `PLAN.md` の表から、見出し行で `Depends on` の列の位置を探す。各行のタスクと依存の集合を、タスクファイルと比べる。
5. 正常な場合と、違反の種類ごとのテストを加える。

## Verification

- `bash tests/run.sh` が通る。`/bin/bash`(3.2)でも通る。
- 違反の種類ごとに、タスクを示すメッセージで失敗する。

## Acceptance criteria

- 存在しない依存先、循環、表とファイルの食い違い、表にないタスク、表や列がないことで、それぞれ失敗する。
- 正しいひし形の計画と、`Parallel group` の列がある表は通る。
- 連想配列など、bash 4 以降の機能を使っていない。

## Handoff notes

06 で、このプロジェクト自身の `--phase implementation` の検証に使う。
