# 02: 依存グラフのモデルと API

Status: Planned
Project: 20261018-010816-execution-flowchart
Requirements: REQ-002, REQ-007, REQ-009, REQ-012, NFR-002, NFR-005
Depends on: 01

## Objective

依存関係のグラフから、各タスクの深さ、着手できるかどうか、グラフの問題を計算する `project/graph.rs` を作り、ボードの API で UI に渡す。並列グループをモデルと API から取り除く。

## Design references

DESIGN_DOC.md の Architecture and component boundaries、Data model and state、Reliability and failure handling

## Scope

### Included

- `project/graph.rs` の新規作成(`plan_order` の移動、`analyze`、`Graph`、`GraphIssue`)と、`schedule.rs` の削除
- `Project` に `graph` を加え、`load_project` で計算する
- `Task` から `parallel_group` を取り除く
- `api.rs`: タスクに `depth` と `ready`、プロジェクトに `graphIssues` を加え、`parallelGroup` を取り除く
- 単体テストと API のテスト

### Excluded

- UI の変更(04)。`parallelGroup` を読んでいる UI のコードは 04 で取り除く。それまでは、空の値として扱われるだけで壊れない。

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/board/src/project/graph.rs`(新規)
- `plugins/smartpowers/skills/smartpowers/board/src/project/schedule.rs`(削除)
- `plugins/smartpowers/skills/smartpowers/board/src/project/mod.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/project/task.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/server/api.rs`
- `plugins/smartpowers/skills/smartpowers/board/src/test_support.rs`

## Implementation steps

1. `graph.rs` を作り、`plan_order` とそのテストを移して、`schedule.rs` を消す。
2. `analyze` を実装する。
   - 存在しない依存先への辺は除き、問題として記録する。
   - Kahn のアルゴリズムで最長の深さを求める。
   - 残ったタスクには、解決済みの依存先から深さを与え、`Cycle` として記録する。
   - 着手できる集合を求める。
3. `Project` と `load_project` に組み込む。
4. `Task` から `parallel_group` を取り除く。`test_support.rs` の `Parallel group: A` の行は残し、無視されることを確かめる。
5. `api.rs` に項目を加え、API のテストを更新する。

## Verification

- `cargo test` と `cargo clippy --all-targets -- -D warnings` が通る。
- 単体テストで次のことを確かめる。
  - 直列の依存とひし形の依存で、深さが正しいこと
  - 依存先がすべて `Done` のときだけ着手できること
  - `Blocked` と `In Progress` のタスクは着手できないこと
  - 存在しない依存先と 2 タスクの循環で、問題が出ること

## Acceptance criteria

- `GET /api/projects/<id>` の JSON に、タスクごとの `depth` と `ready`、プロジェクトの `graphIssues` があり、`parallelGroup` がない。
- 循環のあるプロジェクトでも、`load_project` が成功し、すべてのタスクに深さが付く。
- サーバーに、リポジトリへの新しい書き込みの経路がない。

## Handoff notes

04 は `depth`、`ready`、`graphIssues` だけを使い、着手できるかを UI で判定しない。
