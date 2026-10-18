# Implementation Plan

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-18
Project: 20261018-010816-execution-flowchart
Design: ../DESIGN_DOC.md

## Delivery strategy

このプロジェクト自体を、新しい進め方で実装する。依存関係だけで順番を決め、依存先がすべて `Done` になったタスクには、ほかのタスクを待たずに着手する。

最初に同時に着手できるのは、次の 3 つである。担当するファイルは重ならない。

- 01: `board next` の削除
- 03: 検証スクリプト
- 05: ドキュメント

01 の次に 02(グラフのモデルと API)、その次に 04(Flow 表示)と進める。最後に 06 で、全体を動かして確かめ、0.9.0 のリリースを準備する。

## Execution order

| Order | Task | Depends on | Requirements |
| --- | --- | --- | --- |
| 01 | [board next の削除](01_remove_board_next.md) | None | REQ-004 |
| 02 | [依存グラフのモデルと API](02_graph_model_api.md) | 01 | REQ-002, REQ-007, REQ-009, REQ-012, NFR-002, NFR-005 |
| 03 | [検証スクリプトのグラフ検査](03_validate_graph.md) | None | REQ-002, REQ-003 |
| 04 | [ボードの Flow 表示](04_flow_view.md) | 02 | REQ-002, REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-012, REQ-013, NFR-001, NFR-003, NFR-004, NFR-005 |
| 05 | [ドキュメントの更新](05_docs.md) | None | REQ-001, REQ-002, REQ-004, REQ-005, REQ-006 |
| 06 | [統合確認とリリース準備](06_integration_release.md) | 02, 03, 04, 05 | REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-012, REQ-013, NFR-003, NFR-004 |
| 07 | [Flow 表示の配置と見た目の改善](07_flow_layout_look.md) | 06 | REQ-014, REQ-015, REQ-016, NFR-003, NFR-004 |

## Execution flow

```text
01 ──▶ 02 ──▶ 04 ──┐
03 ────────────────┼──▶ 06 ──▶ 07
05 ────────────────┘
```

01 と 02 は、どちらも `schedule.rs` と `agent.rs` に触れるので、依存を張って順に進める。03 と 05 は、ほかのどのタスクともファイルが重ならない。

## Integration checkpoints

- 02 の完了時: `cargo test` が通り、プロジェクトの JSON に `depth`、`ready`、`graphIssues` がある。
- 04 の完了時: このリポジトリのプロジェクトをボードで開くと、Flow 表示が描かれる。
- 06 の完了時: `cargo test`、`cargo clippy`、`bash tests/run.sh` が通り、このプロジェクトの `--phase implementation` の検証が通る。
- 07 の完了時: 開いた直後に図全体が中央に収まり、モックのプロジェクトで状態と矢印が見分けられる。06 と同じ確認がすべて通る。

## Plan risks

- 03 で表の検査を入れると、この `PLAN.md` も検査の対象になる。この計画の表は、新しい書式(`Depends on` の列あり、並列グループなし)で書いている。
- 05 で書き換える `SKILL.md` の Parallel execution は、parallel-review-pr でも編集する。このプロジェクトを先に終える。
- 07 は、0.9.0 の `Release 0.9.0` のコミット(未 push)のあとに入る。`v0.9.0` のタグは、PR のマージ後に、07 を含む main の先頭に付ける。
