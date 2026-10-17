# First Build Design

Status: Draft
Last updated: 2026-10-18
Project: 20261018-010816-execution-flowchart
PRD: ../PRD.md

## Design summary

タスクファイルがすでに `Depends on:` で書いている依存関係のグラフを、そのまま実行計画とする。ボードに新しいモジュール `project/graph.rs` を置き、`project/schedule.rs` を置き換える。このモジュールは、プロジェクトを読み込むたびに 1 度だけ、各タスクの深さ、着手できるかどうか、グラフの問題(存在しない依存先、循環)を計算する。ボードの API はこれを UI に渡す。新しい Flow 表示は、深さごとの列にタスクを並べ、依存の矢印を SVG で重ねて描く。`board next` と並列グループは取り除く。`validate_workflow.sh` はグラフを検査する。`SKILL.md` は、リードが着手できるタスクに、着手できるようになった時点ですぐ取りかかるよう指示する。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| グラフの計算 | ボードの std のみの Rust(`project/graph.rs`) | 着手できるか、深さ、問題の解釈を 1 か所にまとめる(NFR-002)。既存のタスクの読み込みをそのまま使える。 | JS で計算する: 規則が二重になり、検証スクリプトとずれていく。 |
| Flow の描画 | スクロールする領域に、HTML のノードのカードを絶対位置で置き、矢印だけを 1 つのインライン `<svg>` で重ねる | ノードは既存のカードのスタイル、フォーカス、クリックの動作を使える。SVG は矢印を描くだけで済む。新しい依存はない(NFR-001)。 | ノードもすべて SVG にする: 文字の折り返しとフォーカスの扱いが難しい。Mermaid: PRD で除外済み。 |
| 配置 | 決定的な層状の配置。列は、そのタスクまでの依存の最長の深さとする。列の中は `PLAN.md` の順に並べ、依存先の行の平均(重心)で 1 回だけ並べ替える。 | 単純で、リロードしても配置が変わらない。30 タスク程度なら十分である(NFR-003)。 | 交差を最小化する本格的な Sugiyama 法: 最初のビルドには過剰である。 |
| グラフの検査 | `validate_workflow.sh` の、bash 3.2 で動く関数 | リポジトリの規則は Python ではなく bash である。macOS の bash は 3.2 なので、連想配列は使わない。 | Rust で検査する: 検査がボードのビルドに依存してしまう。 |

## System context

変更するのは、リポジトリの次の部分である。

- **ボードのサーバー** `plugins/smartpowers/skills/smartpowers/board/`: `project/`(モデル)、`server/api.rs`(JSON)、`cli.rs`、`agent.rs`、`main.rs`、テスト
- **ボードの UI** `plugins/smartpowers/skills/smartpowers/assets/board/`: `index.html`、`app.css`、`js/main.js`、`js/router.js`、`js/state.js`、`js/views/content.js`、`js/views/task-parts.js`、`js/views/panel.js`、新しい `js/views/flow.js`
- **検証スクリプト** `scripts/validate_workflow.sh` と、リポジトリ直下の `tests/test_validate_workflow.sh`
- **ドキュメント** `SKILL.md`、`references/planning.md`、`references/task-manager.md`、`README.md`、`CHANGELOG.md`

意図して変えないもの:

- コメント、ログ、`inbox`、`reply` の流れ
- エージェントのトークンと SSE
- ファイルの監視
- ボード表示とリスト表示のステータスの列

`SKILL.md` の Parallel execution のうち、統合と `Done` にする手順は、あとで parallel-review-pr プロジェクトが書き換える。このプロジェクトで書き換えるのは、タスクを選んでサブエージェントに割り当てる手順だけである。

## Architecture and component boundaries

- **`project/graph.rs`(新規。`schedule.rs` を置き換える)**
  - `plan_order(plan: &str)` は `schedule.rs` から移す。
  - `analyze(tasks: &[Task], plan_order: &[String]) -> Graph` は副作用のない関数として、単体テストを持つ。
  - 着手できるか、深さ、問題についての規則は、すべてここに置く。
- **`project/mod.rs`**: `Project` に `graph: Graph` を加え、`load_project` でタスクを読み込んだあとに計算する。`plan_order` は `analyze` への入力として `Project` に残す。
- **`project/task.rs`**: `parallel_group` を取り除く。古いファイルの `Parallel group:` の行は、もう読まれないだけである(REQ-002)。
- **`server/api.rs`**: プロジェクトの JSON にグラフの項目を加え、`parallelGroup` を取り除く。
- **`cli.rs`、`agent.rs`、`main.rs`**: `Next`、`NextOptions`、`--limit`、`agent::next`、使い方の説明を取り除く。
- **`js/views/flow.js`(新規)**: Flow 表示の配置と描画だけを行う。`depth`、`ready`、`graphIssues` を状態から読み、着手できるかを自分で判定しない。
- **`js/router.js` と `js/state.js`**: 表示の種類をハッシュのルートに加える。

## Data model and state

`load_project` のたびに 1 度計算する `Graph`:

```text
Graph {
  depth:  タスク番号 -> u32          // 依存のないタスクは 0
  ready:  タスク番号の集合            // Planned で、依存先がすべて存在して Done、循環に含まれない
  issues: Vec<GraphIssue>           // MissingDependency{task, dep} | Cycle{tasks}
}
```

**深さ**は、根からの最長の経路の長さとする。既知の辺について Kahn のアルゴリズムで求める。

- 存在しないタスクへの辺は除き、問題として報告する。
- Kahn のアルゴリズムで残ったタスクは、循環に含まれるか、循環に依存している。これらは `depth = 1 + 解決済みの依存先の深さの最大値`(依存先がなければ 0)とし、表示上の置き場所を確保する。まとめて 1 つの `Cycle` の問題として、該当するタスクを列挙する。

**着手できる**条件は、`status_key == Planned` であり、依存先がすべて存在して `status_key == Done` であり、循環の集合に含まれないことである。依存のないタスクも、この条件を満たす。

`GET /api/projects/<id>` に加えるもの:

- タスクごとに `"depth": n` と `"ready": bool`
- プロジェクトに `"graphIssues": [{"kind": "missingDependency", "task": "03", "dependency": "09"} | {"kind": "cycle", "tasks": ["04","05"]}]`

`GET /api/projects/<id>` から取り除くもの:

- `"parallelGroup"`

新たに保存する状態はない。表示の種類の設定は今までどおり `localStorage`(`smartpowers.view`)に置き、`flow` も受け付ける。

## Interfaces and integrations

### 計画の書式(`references/planning.md`)

- 実行順の表を `| Order | Task | Depends on | Requirements |` にする。
- タスクファイルのテンプレートから `Parallel group:` を除く。
- サイズの規則のうち、並列グループの規則を次の規則に置き換える。同時に進めると衝突するタスク(ファイルの共有、移行の共有、順序の制約があるもの)の間には、必ず依存を張る。依存の経路がないタスク同士は、同時に進めてよい。
- 「Execution flow」の短い段落を加える。依存関係のグラフが計画のレビューの対象であることと、ボードの Flow 表示で確認できることを書く。

### 検証スクリプト

対象は `validate_workflow.sh` の `plan` と `implementation` のフェーズで、`validate_plan` の中で行う。

1. 各タスクの `Depends on:` を読み、タスク番号に分ける。`None`、`-`、空は、依存なしとして扱う。`01`、`Task 01`、リンクを受け付け、ボードの `task_refs` と動作をそろえる。
2. 参照された番号は、すべて存在するタスクファイルでなければならない。違反したときのメッセージは `Task 03 depends on missing task 09 in <path>` とする。
3. 循環があってはならない。依存先がすべて取り除かれたタスクを繰り返し取り除いて確かめる。連想配列ではなく、空白区切りの文字列を使う。違反したときのメッセージは `Dependency cycle among tasks: 04, 05` とする。
4. `PLAN.md` から、見出し行に `Depends on` の列がある表を探す。
   - 表の行のうち、`NN_*.md` のファイルへのリンクがある行について、その行の依存の集合がタスクファイルの依存の集合と等しくなければならない。
   - タスクファイルは、すべて表のいずれかの行に現れなければならない。
   - 違反したときのメッセージは、該当するタスクを示す。
   - 表や列がなければ、`No execution order table with a Depends on column in <path>` で失敗する。
5. `Parallel group` の列や欄は無視する。

### CLI

`smartpowers-board next` を取り除く。`next` はもうサブコマンドではないので、パーサーは `serve` の位置引数として扱う。その結果、`Unexpected argument: next` で失敗し、終了コードは 2 になる。これで REQ-004 を満たす。

### ルート

形式は `#/<project>[/<task>][?view=board|list|flow]` とする。

- `applyHash` は、`view` があれば読み取り、設定を上書きして保存する。
- `switchView` は、新しい `?view=` でハッシュを書き換える。そのため、コピーした URL で表示の種類とパネルの両方が復元される(REQ-010)。
- `view` のないハッシュでは、保存済みの設定を使う。古いリンクもそのまま動く。

## Critical flows

### 計画のレビュー(REQ-011)

1. 計画を作るエージェントが、`PLAN.md`(Draft)とタスクファイル(Planned)を書く。
2. 監視がそれを検知し、SSE がリロードを起こす。
3. ユーザーが `?view=flow` を開くと、深さごとの列に、すべてのタスクが着手できる状態か待ちの状態で並ぶ。
4. 「03 は 04 より前に」といったフィードバックは、`Depends on:` の編集になる。図は約 1 秒で描き直される。

### 逐次の着手(REQ-005、REQ-006。`SKILL.md`)

1. リードは、`Status: Planned` で、`Depends on:` のタスクがすべて `Done` のタスクを、タスクファイルを直接読んで列挙する。ボードの Flow 表示は、同じ集合を Ready として示す。
2. 着手できるタスクごとに、次のように判断する。
   - 予定している担当範囲が、`In Progress` のタスクと重なる場合: 保留にし、理由を `board log` に残し、計画に依存が足りないことをユーザーに伝える。
   - 重ならない場合: `In Progress` にし、専用の worktree でサブエージェントを起動する。
   - 1 回の確認で見つけたタスクは、すべて同じターンで起動する。
3. どれかのサブエージェントが終わり、そのタスクが `Done` になったら(統合の手順はここでは変えない)、ただちに手順 1 に戻る。ほかに走っているサブエージェントは待たない。
4. すべてのタスクが `Done` になったら止める。着手できるタスクも走っているタスクもないときは、`Blocked` のタスクを報告する。

### Flow の描画

1. タスクを `depth` で列に分ける。
2. 各列を計画の順に並べ、左から右へ 1 回だけ重心で並べ替える。キーは、前の列にある依存先の行番号の平均とし、同じ値なら計画の順を保つ。
3. ノードは 232×76 px の固定サイズのカードとし、列の間隔を 72 px、行の間隔を 16 px にする。絶対位置は JS で計算する。
4. 矢印は、依存先の右端の中央から、後続の左端の中央へ引く 3 次ベジェ曲線とし、矢じりのマーカーを付ける。依存先が `Done` の矢印は、目立たない色の実線で描く。それ以外は、アクセント色の破線で描く。
5. ノードには次のものを表示する。
   - 番号
   - タイトル(2 行まで)
   - 状態のアイコンとラベル
   - 接続中なら Agent のチップ
   - `ready` なら ▶ のアイコン付きの「Ready」のチップ

   クラスは `taskClasses` に従うので、`selected` と `flash` はカードと同じように動く。
6. ノードをクリックするか、Enter を押すと、`openTask` を呼ぶ。ノードは列の順にフォーカスできる。
7. `graphIssues` が空でなければ、図の上の警告の帯に問題を 1 つずつ並べる(REQ-012)。存在しないタスクへの矢印は描かない。

## Security and privacy

エンドポイントも書き込みも増えない。プロジェクトの JSON に、計算で得た項目が加わるだけである。ループバックでの待ち受けと、Host と Origin の検査は変わらない(NFR-005)。ノードのタイトルは `el()` でテキストノードとして入れ、`innerHTML` は使わない。

## Reliability and failure handling

- 壊れたグラフでもボードは止まらない。循環や存在しない依存先は問題として報告し、すべてのタスクに深さを与える。
- 検証スクリプトは、該当するタスクを示すメッセージですぐに失敗する。
- 古いプロジェクトも読み込める。`Parallel group:` の行は無視する。`Parallel group` の列がある古い `PLAN.md` の表も、列を見出しの名前で探すので、表の検査を通る。

## Observability and operations

Flow 表示そのものが、進み具合を見るための面になる。ログやメトリクスは増やさない。リードがタスクを保留にした記録は、今までどおり `board log` に残す(REQ-006)。

## Testing strategy

- **Rust の単体テスト(`graph.rs`)**
  - 直列の依存の深さ
  - ひし形の依存で、深さが最長の経路になること
  - 依存先がすべて `Done` のときだけ着手できること
  - `Blocked` や `In Progress` のタスクは着手できないこと
  - 存在しない依存先で問題が出て、そのタスクは着手できないこと
  - 2 タスクの循環で問題が 1 つ出て、両方に深さが付き、どちらも着手できないこと
  - `plan_order` のテストは `schedule.rs` から移す
- **Rust の API のテスト**
  - プロジェクトの JSON に `depth`、`ready`、`graphIssues` があり、`parallelGroup` がないこと
  - `test_support.rs` のフィクスチャには `Parallel group: A` の行を残し、無視されることを確かめる
- **CLI のテスト**
  - `next` のテストを、エラーになることを確かめる 1 件に置き換える
  - `board_e2e.rs` から `next_lists_parallel_tasks_in_plan_order` を取り除く
  - `board next` が 0 以外で終了することを e2e で確かめる
- **検証スクリプトのテスト(`tests/test_validate_workflow.sh`)**
  - 正しいひし形の計画が通ること
  - 存在しない依存先、循環、表とファイルの食い違い、表にないタスクで失敗すること
  - `Parallel group` の列がある表が通ること
  - `valid_project` に最小限の表を加える
- **UI の確認**
  - 30 タスク・深さ 8 のフィクスチャでボードを起動する。
  - Chrome で次のことを確かめる: ノードが重ならないこと、スクロールできること、`f` キーと `?view=flow` の URL、ノードからパネルが開くこと、`Status:` の編集でライブに更新されること。
  - ライトとダークでスクリーンショットを撮る。
  - 新しいノード、チップ、矢印の色のコントラスト比を、node の使い捨ての計算で確かめる(NFR-004)。
- **リポジトリ全体の確認**: `cargo test`、`cargo clippy`、`tests/run.sh`、このプロジェクトの検証

## Deployment and rollout

リリースは 0.9.0 とする。

- `CHANGELOG.md` に記載する。
  - **Removed**: `board next` と並列グループ(互換性のない変更)
  - **Added**: Flow 表示とグラフの検査
- README のコマンドの表と、並列タスクの段落を更新する。
- プラグインのマニフェストのバージョンを上げる。
- タグと GitHub のリリースは、push の承認のあと、いつものリリースの手順で行う。
- 完了済みのプロジェクトは書き換えない。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Interfaces and integrations(計画の書式) | `references/planning.md` に、依存の規則と同時実行の意味が書かれていることをレビューで確かめる |
| REQ-002 | Architecture and component boundaries; Interfaces and integrations; Reliability and failure handling | ドキュメントと UI に並列グループがないことを grep で確かめる。`Parallel group: A` のフィクスチャが読み込める。`Parallel group` の列がある表で検証スクリプトのテストが通る |
| REQ-003 | Interfaces and integrations(検証スクリプト) | 存在しない依存先、循環、食い違い、表にないタスクの検証スクリプトのテスト |
| REQ-004 | Interfaces and integrations(CLI); Testing strategy | CLI と e2e のテストで `next` が失敗する。`SKILL.md` と README を grep で確かめる |
| REQ-005 | Critical flows(逐次の着手) | `SKILL.md` の Parallel execution の手順をレビューする |
| REQ-006 | Critical flows(逐次の着手、手順 2) | `SKILL.md` の、保留、ログ、報告の規則をレビューする |
| REQ-007 | Architecture and component boundaries; Data model and state; Critical flows(Flow の描画) | 深さのグラフの単体テスト。Chrome で列と矢印の数を確かめる |
| REQ-008 | Critical flows(Flow の描画、手順 5) | Chrome で、`Status:` を編集すると約 1 秒でノードが変わることと、`inbox --wait` の間 Agent のチップが出ることを確かめる |
| REQ-009 | Data model and state(着手できる条件); Critical flows(Flow の描画) | 着手できる条件のグラフの単体テスト。Chrome で、最後の依存先が `Done` になると後続が Ready に変わることを確かめる |
| REQ-010 | Interfaces and integrations(ルート); Critical flows(Flow の描画、手順 6) | Chrome で、ノードのクリックでパネルが開くことと、`#/<id>/<nn>?view=flow` のリロードで復元されることを確かめる |
| REQ-011 | Critical flows(計画のレビュー) | Draft の計画と Planned のタスクのフィクスチャで、Chrome で確かめる |
| REQ-012 | Data model and state(問題); Critical flows(Flow の描画、手順 7); Reliability and failure handling | 問題のグラフの単体テスト。循環のフィクスチャで警告の帯が出て、ボード表示とリスト表示が動くことを Chrome で確かめる |
| REQ-013 | Interfaces and integrations(ルート) | Chrome で `f`、`b`、`l` のキーを確かめる |
| NFR-001 | Technology choices | 差分に、新しい crate、npm、CDN への参照がないことを確かめる |
| NFR-002 | Technology choices; Architecture and component boundaries | `flow.js` が `ready` と `depth` を読み、着手できるかの判定を持たないことをレビューで確かめる |
| NFR-003 | Technology choices(配置); Critical flows(Flow の描画) | 30 タスク・深さ 8 のフィクスチャで、重なりがなく、領域の中でスクロールできることを Chrome で確かめる |
| NFR-004 | Critical flows(Flow の描画、手順 5) | 両方の配色で新しい色のコントラスト比を計算する。Ready と状態がアイコンとラベルを持つことを確かめる |
| NFR-005 | Security and privacy | サーバーの差分に新しい書き込みの経路がないことをレビューで確かめる |

## Risks and tradeoffs

- 重心での並べ替えが 1 回だけなので、密なグラフでは交差が残る。最初のビルドではこれを受け入れる。ノードのサイズを固定して、配置を予測しやすくする。
- 表の検査で `PLAN.md` が厳しくなる。`Depends on` の列がない計画は失敗するようになる。影響するのは新しい計画だけで、完了済みのプロジェクトは検査し直さない。
- 着手できるかどうかは、リードへの目安であり、起動の時点では強制しない。`board next` をなくしたので、これは意図どおりである。
- `SKILL.md` の Parallel execution は、parallel-review-pr でも編集する。このプロジェクトを先に入れ、あちらはこの新しい本文の上に手順を書き足す。

## Open questions

- なし
