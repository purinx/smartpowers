# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210651-progress-board
PRD: ../PRD.md

## Design summary

ローカル専用の HTTP サーバーで、プロジェクトとタスクを JSON として返し、埋め込んだ Web UI がそれを描画する。ファイルの変化はサーバーがポーリングで検出し、Server-Sent Events(SSE)でブラウザに知らせる。ブラウザは知らせを受けたらデータを取り直して描き直す。サーバーは std のみの Rust、UI はビルド不要のバニラ JS(ES モジュール)と CSS で書く。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| サーバー | std のみの Rust(接続ごとに 1 スレッド) | 依存がなく、オフラインでビルドでき、単一のバイナリになる。 | Python の標準ライブラリによるサーバー(最初の版。その後 Rust に置き換えた) |
| UI | バニラ JS の ES モジュールと CSS | ビルド工程がなく、配信するコードと読むコードが同じになる。 | Rust から WASM にコンパイルする方式(中身の見えないバイナリが増え、DOM 操作もしにくいので不採用) |
| ライブ更新 | SSE と、サーバー側でのファイル指紋のポーリング | 標準の `EventSource` は自動で再接続し、サーバー側も std だけで実装できる。 | WebSocket(std だけでは実装が重い) |
| UI の配信 | `include_str!` で埋め込んだ静的ファイルの表 | 単一のバイナリで完結し、パス一致による検索でパストラバーサルが起きない。 | ディスクから配信する(配置場所に依存する) |

## System context

ワークフローの成果物(`docs/smartpowers/projects/<id>/` の `PROJECT.md`、`PRD.md`、`DESIGN_DOC.md`、`tasks/PLAN.md`、`tasks/NN_*.md`)を読むだけの追加コンポーネントである。既存の検証スクリプトとファイルの書式は変えない。起動は `scripts/board`(または `board.cmd`)から行う(board-build-from-source を参照)。

## Architecture and component boundaries

サーバー(`board/src`):

- `project/`: プロジェクトとタスクを読み込む。
  - `markdown.rs`: H1 のタイトルと `Key: value` フィールドを読み、要件 ID、2 桁のタスク参照、URL、課題キーを抽出する。
  - `task.rs`: `NN_name.md` の判定、`StatusKey` の列挙型、`Remote task:` を扱う。
  - `mod.rs`: 一覧、詳細、フェーズを扱う。
- `server/`:
  - `http.rs`: リクエストの解析と、Host/Origin の検査を行う。
  - `api.rs`: UI との JSON の約束を 1 か所に集める。
  - `assets.rs`: 埋め込んだファイルの表を持つ。
  - `events.rs`: SSE を扱う。
  - `mod.rs`: ルーティングを行う。
- `watcher.rs`: ファイルの指紋をポーリングし、変化があればバージョン番号を進めて、待っている SSE 接続を起こす。
- `json.rs`、`time.rs`、`files.rs`: std だけで書いた小さな補助。

UI(`assets/board`):

- `js/state.js`: 状態と、閲覧者ごとの設定を持つ。
- `js/api.js`: データを取得する。
- `js/router.js`: URL の `#/<project>/<task>` と表示を対応させる。
- `js/live.js`: SSE を受け取る。
- `js/status.js`: 列とアイコンを定める。
- `js/markdown.js`: 入力をエスケープしてから描画する、最小限の Markdown レンダラー。
- `js/dom.js`: 補助関数。
- `js/views/*`: board、list、sidebar、header、content、panel、panel-resize、task-parts を、画面の領域ごとに 1 ファイルで持つ。
- `app.css`: デザイントークン(CSS 変数)とレイアウトを定める。

## Data model and state

- タスクの状態は `StatusKey`(Planned、InProgress、Blocked、Done、Other)に正規化する。宣言の順序が、そのまま列の順序になる。
- ヘッダのフィールドは、最初の H2 より前の `Key: value` から読む。
- サイドバーの並びは、プロジェクト ID(作成時刻で始まる)の降順にする。
- 進捗は、タスクの `statusKey` を数えた `counts` から計算する。
- 閲覧者ごとの設定(ボードとリストのどちらで表示するか、パネルの幅)は `localStorage` に保存する。保存できない環境では無視する。
- 直前に描画したときの各タスクの `statusKey|updated` を覚えておき、変わったタスクを強調する。

## Interfaces and integrations

- `GET /api/projects`: リポジトリ名と、プロジェクトの要約の一覧(`id`、`title`、`status`、`updated`、`taskCount`、`counts`)を返す。
- `GET /api/projects/<id>`: 要約に、`phases`、`path`、`tasks`(`number`、`title`、`status`、`statusKey`、`requirements`、`dependsOn`、`parallelGroup`、`remote`、`updated`、`body`)を加えて返す。
- `GET /api/events`: 接続時に `ready` を、変化のたびに `change` を送り、15 秒ごとに keepalive を送る SSE。
- それ以外の `GET`: 埋め込んだ UI のファイルを返す(`/` は `index.html`)。

## Critical flows

1. **最初の表示**: `index.html` → `main.js` が URL のハッシュを解釈 → `/api/projects` と `/api/projects/<id>` を取得 → サイドバー、ヘッダ、本文、パネルを描画する。
2. **ライブ更新**: タスクファイルを保存 → 監視スレッドが指紋の違いを検出(1 秒間隔)→ バージョン番号を進める → SSE で `change` を送る → UI がデータを取り直して描き直し、変わったタスクを強調する。
3. **詳細を見る**: カードをクリック → URL のハッシュを `#/<project>/<task>` に変える → パネルを描画する(本文は最初の H2 から先を Markdown として描画)。
4. **パネルの幅を変える**: 左端のつまみを pointer capture でドラッグ → `幅 = ウィンドウ幅 − ポインタの X 座標` を範囲内に収める → 終わったら幅を保存する。

## Security and privacy

- `127.0.0.1`、`localhost`、`::1` 以外のホストで起動しようとすると、起動を拒否する。
- すべてのリクエストについて Host を検査し、Origin があればそれも検査して、ループバック以外なら 403 を返す。これで DNS リバインディングと、クロスサイトからの読み取りを防ぐ。
- この範囲の API は読み取り専用で、リポジトリには書き込まない。
- Markdown は先にエスケープしてから、自前で生成したタグだけを加える。テキストは `textContent` で挿入する。JSON の文字列では `<` をエスケープする。
- レスポンスに `X-Content-Type-Options: nosniff` と `Cache-Control: no-store` を付ける。

## Reliability and failure handling

- 書式の崩れたファイルや、読み取れないファイルは飛ばす。欠けたフィールドは既定値(Unknown や空)として扱う。
- 走査の途中で消えたファイルは無視する。
- `EventSource` は自動で再接続し、接続し直したら全体を取り直す。

## Observability and operations

接続の状態は、サイドバー下部の Live 表示で示す(Live または Reconnecting)。サーバーは起動時に URL を標準出力へ 1 行出す。エージェントはこの行を読むので、書式は変えない。

## Testing strategy

- Rust の単体テストで、次の点を確かめる。
  - フィールドの解析、要件 ID・タスク参照・URL・課題キーの抽出
  - JSON の約束(`api.rs`)
  - Host/Origin の検査
  - 埋め込みの表がファイルと一致していること(ファイルを足して表に載せ忘れたら失敗する)
  - 監視スレッドが変化を検出すること
  - 時刻の書式
- Python の E2E テストで、起動スクリプトから起動したサーバーの API と、外部ホストの拒否を確かめる。
- ブラウザで手動確認する(表示の切り替え、パネル、幅の変更、ライブ更新、80 行あるタスクでのスクロール、コンソールにエラーがないこと)。
- 配色はコントラスト比を計算して確かめる。

## Deployment and rollout

プラグインの一部として配布した。版ごとの内容は次のとおり。

- 0.3.0: 表示、ライブ更新、詳細パネル、モジュール構成
- 0.4.0: パネルの幅の変更
- 0.5.0: スクロールの修正と配色

UI をモジュールに分割したことと、サーバーをモジュールに分割したことは、0.3.0 に含まれている。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture and component boundaries(views/board、views/list、status.js); Data model and state | Rust の単体テスト(StatusKey の対応)と、ブラウザでの表示の確認 |
| REQ-002 | Interfaces and integrations(`GET /api/projects`); Data model and state | Rust の単体テスト(プロジェクト一覧)と、ブラウザでの確認 |
| REQ-003 | Interfaces and integrations(`phases`、`counts`); Architecture and component boundaries(views/header) | Rust の単体テスト(フェーズと counts の JSON) |
| REQ-004 | Architecture and component boundaries(views/task-parts、markdown.rs); Interfaces and integrations | Rust の単体テスト(要件 ID、依存、リモートキーの抽出) |
| REQ-005 | Critical flows(詳細を見る); Architecture and component boundaries(router.js、views/panel、markdown.js) | URL を直接開いてパネルが開くことをブラウザで確認する |
| REQ-006 | Critical flows(ライブ更新); Architecture and component boundaries(watcher.rs、events.rs、live.js) | Rust の単体テスト(監視スレッドの変化検出)と、ブラウザでの反映の確認 |
| REQ-007 | Architecture and component boundaries(main.js、views/content) | ブラウザでキー操作とフィルタを確認する |
| REQ-008 | Critical flows(パネルの幅を変える); Data model and state | ブラウザで、ドラッグ、上限と下限、保存、ダブルクリックでのリセット、キー操作を測って確認する |
| REQ-009 | Architecture and component boundaries(app.css のグリッド行 `minmax(0, 1fr)` と `.panel-scroll`) | 80 行のタスクで、パネルの高さが画面の高さに等しく、中身がスクロールすることを確認する |
| REQ-010 | Architecture and component boundaries(app.css のトークンとスクロールバー) | ブラウザでの確認と、コントラスト比の計算 |
| NFR-001 | Security and privacy | Rust の単体テスト(403)と、Python の E2E テスト(外部ホストと、ループバック以外への bind の拒否) |
| NFR-002 | Security and privacy; Interfaces and integrations | コードレビュー(この範囲の API が GET だけであること) |
| NFR-003 | Architecture and component boundaries(app.css のトークン) | ライト/ダークの全組み合わせのコントラスト比を計算する |
| NFR-004 | Architecture and component boundaries; Technology choices | 埋め込みの表を検査する単体テストと、コードレビュー |
| NFR-005 | Architecture and component boundaries(app.css の 900px のメディアクエリ) | 狭い幅のブラウザでの確認 |

## Risks and tradeoffs

- ポーリングは実装が単純で、どの OS でも同じように動く。その代わり、変化の検出は最大で監視間隔ぶん遅れる。
- 小さな Markdown レンダラーを自作したので、表現できる書式は限られる。その代わり依存がなく、エスケープを確実に行える。
- 変化があるたびに全体を取り直すため、大きなプロジェクトでは無駄が出る。

## Open questions

なし。
