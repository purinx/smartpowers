# 07: Flow 表示の配置と見た目の改善

Status: In Progress
Project: 20261018-010816-execution-flowchart
Requirements: REQ-014, REQ-015, REQ-016, NFR-003, NFR-004
Depends on: 06

## Objective

Flow 表示を、表示エリアの中央にいっぱいに、縦の位置を格子に揃えない自由な配置で描き、見た目を整える。

## Design references

DESIGN_DOC.md の Technology choices(配置、表示エリアへの収め方)、Critical flows(Flow の描画、手順 1〜7)

## Scope

### Included

- 配置
  - 列の中の順番を、重心で 2 往復並べ替える。
  - 縦の位置を、緩和で決める。
- 列をまたぐ矢印
  - 間の列の空いた高さを通す。
  - 同じ隙間を通る矢印は、6 px ずつずらす。
- 全体表示
  - 中央に置き、エリアに収まる最大の拡大率で描く(0.4〜1.5)。
  - ホイールで拡大縮小、背景のドラッグで移動、「全体表示」のボタン。
  - ライブ更新のあいだは、拡大率と位置を保つ。
- 見た目
  - ドットの格子
  - 状態の色の帯
  - Ready の枠
  - マウスを乗せたときやフォーカスしたときの、矢印の強調
- CHANGELOG の 0.9.0 の Flow 表示の説明に、全体表示、拡大縮小、移動を加える。

### Excluded

- ノードのドラッグでの移動
- サーバーと API の変更

## Expected code areas

- `plugins/smartpowers/skills/smartpowers/assets/board/js/views/flow.js`
- `plugins/smartpowers/skills/smartpowers/assets/board/app.css`
- `plugins/smartpowers/skills/smartpowers/assets/board/js/state.js`(プロジェクトごとの表示の状態を保持する場合)
- `CHANGELOG.md`

## Implementation steps

1. 配置を書き換える。順番は重心で 2 往復、縦の位置は緩和で決め、重なりは押し広げる。
2. 列をまたぐ矢印の経路と、ずらしを実装する。
3. 拡大率と位置、全体表示、ホイール、ドラッグ、「全体表示」のボタンを実装し、プロジェクトごとにメモリに保持する。
4. 見た目のスタイルを加える。
5. モック、30 タスク、Draft、循環の各フィクスチャと、このリポジトリのプロジェクトで、Chrome で確かめる。

## Verification

- `cargo test` と `cargo clippy --all-targets -- -D warnings` が通る。
- Chrome で次のことを確かめる。
  - 5 タスクと 30 タスクの計画で、開いた直後に図全体が中央に収まる。
  - ホイール、ドラッグ、「全体表示」が動く。
  - 拡大と移動のあとにタスクファイルを編集しても、位置が変わらない。
  - 依存先が 1 つのタスクが、依存先とほぼ同じ高さにある。
  - 長い矢印が見分けられる。
  - マウスを乗せると、つながる矢印が強調される。
- 新しい色のコントラスト比が、ライトとダークの両方で 4.5:1 以上である。

## Acceptance criteria

- REQ-014、REQ-015、REQ-016 の受け入れ条件を満たす。
- 着手できるかどうかの判定を UI に持たない。新しい外部の依存がない。

## Handoff notes

完了したら、`PROJECT.md` を `Completed` に戻し、PR を出す前の push の承認を Crit で求める。
