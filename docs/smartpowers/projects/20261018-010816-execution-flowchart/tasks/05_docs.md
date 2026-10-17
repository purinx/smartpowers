# 05: ドキュメントの更新

Status: Planned
Project: 20261018-010816-execution-flowchart
Requirements: REQ-001, REQ-002, REQ-004, REQ-005, REQ-006
Depends on: None

## Objective

`SKILL.md`、計画ガイド、タスクマネージャーのガイド、README を、依存関係のグラフを実行計画とする進め方に書き換える。`board next` と並列グループの記述を取り除く。

## Design references

DESIGN_DOC.md の Interfaces and integrations(計画の書式)、Critical flows(逐次の着手)

## Scope

### Included

- `references/planning.md`
  - 表から `Parallel group` の列を、タスクファイルのテンプレートから `Parallel group:` を取り除く。
  - 衝突するタスクの間に依存を張る規則と、Execution flow の段落を加える。
- `SKILL.md`
  - Phase 3 の「safe parallel groups」を、同時に進められるタスクの説明に変える。
  - Parallel execution の手順のうち、タスクを選んで割り当てる部分を、逐次の着手に書き換える。担当範囲が重なりそうなタスクの保留、ログ、報告も書く。
  - サブエージェントへのブリーフから、並列グループを取り除く。
  - ボードの説明に Flow 表示を加える。
- `references/task-manager.md` から、並列グループを取り除く。
- `README.md` から `board next` の行を取り除き、並列タスクの段落と、ボードの説明に Flow 表示を加える。

### Excluded

- Parallel execution の、統合と `Done` にする手順の変更(parallel-review-pr)
- `CHANGELOG.md` とバージョン(06)

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/SKILL.md`
- `plugins/smartpowers/skills/smartpowers/references/planning.md`
- `plugins/smartpowers/skills/smartpowers/references/task-manager.md`
- `README.md`

## Implementation steps

1. `planning.md` の書式と規則を書き換える。
2. `SKILL.md` の Phase 3、Parallel execution、ブリーフ、ボードの説明を書き換える。
3. `task-manager.md` と `README.md` を更新する。
4. `board next`、`parallel group`、`Parallel group` を grep し、残りがないことを確かめる。

## Verification

- `grep -rni 'board next\|parallel group' plugins/smartpowers/skills/smartpowers/SKILL.md plugins/smartpowers/skills/smartpowers/references README.md` の結果が空である。
- `bash tests/run.sh` が通る(マーケットプレイスのテストなど)。

## Acceptance criteria

- 計画ガイドに、依存の張り方の規則と、依存のないタスクは同時に走りうることが書かれている。
- `SKILL.md` に次のことが書かれている。
  - 着手できるタスクの条件
  - 完了のたびに見直す手順
  - 一斉に待たないこと
  - 担当範囲が重なりそうなときの保留、ログ、報告

## Handoff notes

parallel-review-pr は、この Parallel execution の新しい本文の上に、レビューと PR の手順を書き足す。
