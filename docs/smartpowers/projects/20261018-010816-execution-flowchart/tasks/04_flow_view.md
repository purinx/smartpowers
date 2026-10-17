# 04: ボードの Flow 表示

Status: In Progress
Project: 20261018-010816-execution-flowchart
Requirements: REQ-002, REQ-007, REQ-008, REQ-009, REQ-010, REQ-011, REQ-012, REQ-013, NFR-001, NFR-003, NFR-004, NFR-005
Depends on: 02

## Objective

ボード表示とリスト表示に並ぶ 3 つ目の表示として、依存関係のグラフを描く Flow 表示を加える。UI から並列グループを取り除く。

## Design references

DESIGN_DOC.md の Critical flows(Flow の描画)、Interfaces and integrations(ルート)、Technology choices(Flow の描画、配置)

## Scope

### Included

- `js/views/flow.js` の新規作成
  - 深さごとの列
  - 重心での並べ替え
  - 固定サイズのノード
  - SVG の矢印
  - Ready のチップ
  - 警告の帯
- `index.html` に Flow のボタンを加え、`app.css` にスタイルを加える(ライトとダーク)
- `router.js` と `state.js`: `?view=` のルートと `flow` の設定
- `main.js`: `f` キー、表示の切り替えでのハッシュの書き換え
- `content.js`: Flow 表示への振り分けと、フィルタから `parallelGroup` を外すこと
- `task-parts.js` と `panel.js`: 並列グループのチップと属性を取り除く

### Excluded

- 30 タスクのフィクスチャでの最終的な確認とコントラスト比の計算(06)

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/flow.js`(新規)
- `plugins/smartpowers/skills/smartpowers/assets/board/index.html`
- `plugins/smartpowers/skills/smartpowers/assets/board/app.css`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/main.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/router.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/state.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/content.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/task-parts.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/panel.js`

## Implementation steps

1. ルートと設定に `view` を加え、`f` キーと Flow のボタンをつなぐ。
2. `flow.js` で、タスクを列と行に配置する。
3. ノードを HTML で、矢印を SVG で描く。ノードのクリックと Enter で `openTask` を呼ぶ。
4. `graphIssues` があれば、警告の帯を出す。
5. 並列グループの表示とフィルタの項目を取り除く。
6. このリポジトリのプロジェクトでボードを起動し、Chrome で確かめる。

## Verification

- `cargo test` が通る。UI はサーバーに埋め込まれるので、アセットのテストも含まれる。
- Chrome で次のことを確かめる。
  - このリポジトリのプロジェクトで、Flow 表示が描かれること
  - ノードのクリックでパネルが開くこと
  - `#/<id>/<nn>?view=flow` を開き直すと、同じ表示とパネルが復元されること
  - `f`、`b`、`l` で表示が切り替わること
  - タスクファイルの `Status:` の編集が約 1 秒で反映されること

## Acceptance criteria

- 各タスクが依存先より右の列に描かれ、`Depends on` の数だけ矢印がある。
- 着手できるタスクに、▶ のアイコン付きの Ready のチップがある。
- 循環のあるプロジェクトで、警告の帯が出て、ボード表示とリスト表示は動く。
- UI に並列グループの表示がない。新しい外部の依存がない。

## Handoff notes

06 で、30 タスク・深さ 8 のフィクスチャでの表示と、コントラスト比を確かめる。
