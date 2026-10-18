# 06: 統合確認とリリース準備

Status: In Progress
Project: 20261018-010816-execution-flowchart
Requirements: REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-012, REQ-013, NFR-003, NFR-004
Depends on: 02, 03, 04, 05

## Objective

すべての変更がそろった状態で、Flow 表示を大きな計画と壊れた計画で確かめる。配色のコントラスト比を確かめ、0.9.0 のリリースを準備する。

## Design references

DESIGN_DOC.md の Testing strategy(UI の確認)、Deployment and rollout

## Scope

### Included

- 一時ディレクトリに、次の 3 つのフィクスチャを作り、Chrome で確かめる。
  - 30 タスク・深さ 8 の計画
  - Draft の計画
  - 循環のある計画
- ライトとダークのスクリーンショット
- 新しい色のコントラスト比の計算(node の使い捨てのスクリプト。リポジトリには置かない)
- `CHANGELOG.md` の 0.9.0
  - Removed: `board next`、並列グループ
  - Added: Flow 表示、グラフの検査
- プラグインのマニフェスト(`.claude-plugin`、`.codex-plugin`)のバージョンを 0.9.0 に上げる
- リポジトリ全体の確認
- 04 のレビューで見つかった問題の修正: 2 列以上をまたぐ矢印が途中の列のノードの下に潜り、別のノードにつながっているように見える。長い矢印を、ノードのない行の間を通る経路に変える(`js/views/flow.js`)。

### Excluded

- push、タグ、GitHub のリリース。push の承認のあと、いつものリリースの手順で行う。

## Expected code areas

- `CHANGELOG.md`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/flow.js`(矢印の経路の修正)
- `plugins/smartpowers/.claude-plugin/plugin.json`
- `plugins/smartpowers/.codex-plugin/plugin.json`
- 必要なら、マーケットプレイスのマニフェストのバージョン

## Implementation steps

1. フィクスチャを作ってボードを起動し、Chrome で確かめる。重なり、スクロール、ライブ更新、Ready の変化、警告の帯、URL の復元を見る。
2. コントラスト比を計算し、4.5:1 を下回る色があれば直す。
3. CHANGELOG とバージョンを更新する。
4. `cargo test`、`cargo clippy`、`bash tests/run.sh`、このプロジェクトの検証を通す。

## Verification

- 30 タスク・深さ 8 で、ノードが重ならず、領域の中でスクロールできる。
- すべての新しい文字色が、ライトとダークの両方で 4.5:1 以上である。
- `bash plugins/smartpowers/skills/smartpowers/scripts/validate_workflow.sh --root . --project 20261018-010816-execution-flowchart --phase implementation` が通る。

## Acceptance criteria

- CHANGELOG に、0.9.0 の互換性のない変更と追加が書かれ、マニフェストのバージョンが 0.9.0 である。
- リポジトリ全体の確認がすべて通る。

## Handoff notes

完了したら、`PROJECT.md` を `Completed` にし、push の承認を Crit で求める。
