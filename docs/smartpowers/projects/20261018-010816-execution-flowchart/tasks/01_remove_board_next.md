# 01: board next の削除

Status: In Progress
Project: 20261018-010816-execution-flowchart
Requirements: REQ-004
Depends on: None

## Objective

`smartpowers-board next` のサブコマンドと、その実装、テストを取り除く。

## Design references

DESIGN_DOC.md の Interfaces and integrations(CLI)、Testing strategy(CLI のテスト)

## Scope

### Included

- `cli.rs` から次のものを取り除く: `Next`、`NextOptions`、`--limit`、`COMMANDS` の `next`、USAGE の `next` の行と Next options
- `agent.rs` から `next` 関数と、`schedule` の import を取り除く
- `main.rs` の `Command::Next` の分岐と、モジュールの説明の `next` を取り除く
- `schedule.rs` から `Schedule`、`next`、`idle_reason` と、そのテストを取り除く(`plan_order` とそのテストは残す)
- `board_e2e.rs` の `next_lists_parallel_tasks_in_plan_order` を取り除き、`board next` が失敗する e2e テストを加える

### Excluded

- `graph.rs` の作成と、`schedule.rs` の置き換え(02)
- ドキュメントの `board next` の記述(05)

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/board/src/cli.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/agent.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/main.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/project/schedule.rs`
- `plugins/smartpowers/skills/smartpowers/board/tests/board_e2e.rs`

## Implementation steps

1. `cli.rs` の `next` に関わる型、フラグ、使い方の説明を取り除く。CLI のテストを、`next` が `Unexpected argument: next` になることを確かめる 1 件に置き換える。
2. `agent.rs` と `main.rs` から `next` を取り除く。
3. `schedule.rs` を `plan_order` だけにする。
4. e2e のテストを差し替える。

## Verification

- `cargo test` と `cargo clippy --all-targets -- -D warnings` が通る(`board/` で実行)。
- `board next --project <id>` が終了コード 2 で、`Unexpected argument: next` を出す。

## Acceptance criteria

- `cli.rs`、`agent.rs`、`main.rs`、`board_e2e.rs` に `next` のサブコマンドへの参照がない。
- 残りの CLI(`serve`、`inbox`、`reply`、`log`)のテストは、そのまま通る。

## Handoff notes

02 は、残した `plan_order` を `graph.rs` に移す。
