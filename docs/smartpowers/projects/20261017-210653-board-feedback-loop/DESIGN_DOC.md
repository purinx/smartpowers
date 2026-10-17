# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210653-board-feedback-loop
PRD: ../PRD.md

## Design summary

既存のボード(Rust の std だけで書いた HTTP サーバーと、ES モジュールの UI)に、次の 4 つを加える。

1. コメント投稿用の POST エンドポイント
2. エージェント用のサブコマンド `inbox` / `reply` / `log`
3. コメントとログを 1 件ずつ Markdown ファイルとして保存する状態ディレクトリ
4. Activity 欄とコメント欄を持つ UI

エージェントへの配信は、`inbox --wait` が状態ディレクトリを 500ms ごとに見る方式で行う。バックグラウンドのコマンドが終了したことを、ホストが作業中のエージェントに通知する仕組みを利用する。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| 保存先 | リポジトリの外の状態ディレクトリ(`~/.local/state/smartpowers/<repo>-<FNV-1a hash>/<project>/`) | commit されたファイルがユーザーのコメントになりすますことを防ぐ(NFR-001)。リポジトリの名前とパスのハッシュで、クローンごとに分ける。 | リポジトリ内の `docs/` に置く(履歴は残るが、なりすましの入口になるので不採用) |
| 保存形式 | 1 件ずつの Markdown ファイル(タイトル行、`Key: value` のフィールド、空行、本文) | 人とエージェントが読め、JSON パーサーを持たない std だけの実装でも安全に読み書きできる。 | JSONL(パーサーが必要)、ログを 1 本のファイルにまとめる(同時に書くと壊れやすい) |
| 配信 | `board inbox --wait` が 500ms ごとに状態ディレクトリを見て、届いたら終了する | ホストのバックグラウンド完了通知だけで動き、hook が要らない(NFR-004)。ボードが起動していなくても動く。 | Claude Code の PostToolUse hook(Codex で使えない)、`claude -p --input-format stream-json`(Claude 専用) |
| 識別子 | `YYYYMMDD-HHMMSS-xxxx`(UTC と 16 bit の値) | 読みやすく、ファイル名として安全で、検証もしやすい。 | UUID(乱数のための依存が要る) |
| ハッシュ | FNV-1a | Rust のバージョンが変わっても結果が変わらない。`DefaultHasher` だと再ビルドで状態ディレクトリの場所が変わってしまう。 | `DefaultHasher` |

## System context

`20261017-210651-progress-board` のボードを拡張する。サーバーの構成(`server/`、`project/`、`watcher`)と、タスクファイルを読み取り専用で扱う方針は変えない。新しく書き込むのは状態ディレクトリだけである。CLI は、サブコマンドがなければ従来どおり `serve` として動く。

## Architecture and component boundaries

- `workspace.rs`: リポジトリのルートと状態ディレクトリを決める。`SMARTPOWERS_STATE_HOME` があればそれを、Windows では `%LOCALAPPDATA%\smartpowers\state` を、ほかでは `XDG_STATE_HOME` か `~/.local/state/smartpowers` を使う。
- `project/comments.rs`: コメントの読み書き、検証(空でないこと、8,000 文字以内)、ID の生成、アトミックな保存。
- `project/log.rs`: タスクごとの 1 行ログを追記する。改行は空白にし、1,000 文字以内とする。
- `project/mod.rs`: `load_project` が、コメントとログをタスクに結び付ける。
- `agent.rs`: `inbox`(保留中のコメントを表示して配信済みにする。`--wait` と `--timeout` を指定できる)、`reply`(スレッドの最初のコメントに返信し、`--resolve` で Addressed にする)、`log`。
- `server/mod.rs`: `POST /api/projects/<id>/tasks/<n>/comments`。
- `server/api.rs`: タスクの JSON に `comments` と `log` を加える。
- `watcher.rs`: 監視対象に状態ディレクトリを加える。
- UI: `views/activity.js`(時系列とスレッド)、`views/composer.js`(静的なフォームで、下書きを保持する)、`task-parts.js`(コメント数のチップと最新ログ)。

## Data model and state

コメント(`comments/<task>/<id>.md`):

```text
# Comment 20261017-101530-3f2a

Task: 05
Author: user | agent
Created: 2026-10-17T10:15:30.123Z
Status: Open | Addressed
Delivered: yes | no
Reply to: <thread root id>      (返信のときだけ)

本文
```

- フィールドは最初の空行までとし、本文の中の `Note: ...` のような行をフィールドとして読まない。
- 保留中とは、`Author: user`、`Status: Open`、`Delivered: no` がそろった状態である。エージェントのコメントは、作成時から配信済みとする。
- ログ(`logs/<task>.md`)は、`- <RFC 3339 のミリ秒付き時刻> <message>` の行を追記していく。
- 時刻はミリ秒まで記録した UTC の文字列で、文字列として並べると時系列順になる。

## Interfaces and integrations

- `POST /api/projects/<id>/tasks/<n>/comments`: 本文は `text/plain`。成功すると 201 でコメントの JSON を返す。失敗時は 400(空、長すぎる、UTF-8 でない)、403(Origin がない、またはループバックでない)、404(プロジェクトかタスクがない)、413(本文が 64KiB を超える)を返す。
- `GET /api/projects/<id>`: 各タスクに `comments`(id、author、created、status、delivered、replyTo、body)と `log`(time、message)を含める。
- CLI: `board inbox --project <id> [--wait] [--timeout <secs>]`、`board reply --project <id> --comment <id> [--resolve] <message>`、`board log --project <id> --task <nn> <message>`。`--` のあとの引数は、ダッシュで始まっていてもメッセージとして扱う。

## Critical flows

1. **コメント**: UI が POST する → サーバーがファイルを作る → watcher が変化を検知して SSE の `change` を送る → UI が Waiting for agent と表示する。
2. **配信**: エージェントの `inbox --wait` が保留中のコメントを見つける → タスク名と ID を付けて表示し、`Delivered: yes` に更新して終了する → ホストがエージェントに通知する → UI は Sent to agent と表示する。
3. **対応**: エージェントが `reply --resolve` を実行する → スレッドの最初のコメントに返信ファイルを作り、元のコメントを Addressed に更新する → UI が更新される。エージェントは `inbox --wait` を再開する。
4. **ログ**: `board log` が追記する → watcher → UI の Activity と、カードの最新ログが更新される。

## Security and privacy

- 保存先をリポジトリの外に置き、なりすましを防ぐ(NFR-001)。
- POST では Host と Origin の両方がループバックであることを求める(NFR-002)。ブラウザは同一オリジンへの POST にも Origin を付けるので、ボードの画面からの送信は通る。
- UI はコメントの本文をテキストノードとして描画し、HTML として解釈しない。
- ボードがリポジトリのファイルに書き込むことはない。

## Reliability and failure handling

- 一時ファイルに書いてから rename で置き換えるので、読み込み中のファイルが半端な状態になることはない(NFR-003)。
- ID が衝突したときは作り直す。ファイルの読み込みに失敗したものは飛ばす。
- `inbox --wait --timeout` は、期限が来たら「No new comments」と表示して正常終了する。

## Observability and operations

- 起動時に、状態ディレクトリの場所を標準エラー出力に表示する。
- コメントとログは Markdown なので、そのまま読める。容量を空けたいときは状態ディレクトリを削除すればよい。

## Testing strategy

- Rust の単体テスト:
  - コメントの往復変換と、本文中のフィールド風の行の扱い
  - 作成、読み込み、更新と、検証のエラー
  - ログの追記と読み込み
  - `inbox` が 1 回だけ配信すること、タイムアウト、スレッドの最初への返信
  - POST の Origin と、各エラー状態
  - API の JSON の形
  - watcher が状態ディレクトリの変化を検知すること
  - FNV-1a の参照値
- Python の E2E テスト(起動スクリプト経由): POST → `inbox` → 2 回目は空 → `reply --resolve` → `log` → API で状態を確認し、リポジトリ内にファイルができていないことを確かめる。
- ブラウザでの手動確認: 送信、配信状態の切り替わり、返信の表示、💬 チップが消えること、下書きの保持。

## Deployment and rollout

プラグイン 0.6.0 で配布した。既存のボードとの互換性は保ち、CLI のサブコマンドがない場合の動作は変わらない。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Interfaces and integrations(POST); Architecture and component boundaries(composer.js) | ブラウザでの手動確認。Rust のテストで空や長すぎる本文が 400 になること |
| REQ-002 | Technology choices(配信); Critical flows(配信) | バックグラウンドの `inbox --wait` に、送信から数秒以内に届くことを手動で確認。Python の E2E テスト |
| REQ-003 | Data model and state(保留中の定義); Critical flows(配信) | Rust のテスト `delivers_each_comment_once_and_records_replies`。E2E テストで 2 回目の inbox が空になること |
| REQ-004 | Architecture and component boundaries(agent.rs reply); Data model and state(Reply to) | Rust のテスト(返信がスレッドの最初に付くこと、Addressed になること)。E2E テスト |
| REQ-005 | Critical flows; Architecture and component boundaries(activity.js) | ブラウザで 3 つの状態の切り替わりを確認 |
| REQ-006 | Architecture and component boundaries(log.rs, activity.js); Interfaces and integrations(CLI) | Rust のテスト `appends_and_loads_entries` と、存在しないタスクを指定したときのエラー。E2E テスト |
| REQ-007 | Architecture and component boundaries(task-parts.js) | ブラウザで進行中のカードの表示を確認 |
| REQ-008 | Architecture and component boundaries(composer.js) | ブラウザで、自動更新とタスクの切り替えを挟んでも下書きが残ることを確認 |
| REQ-009 | Architecture and component boundaries(task-parts.js) | ブラウザで、チップの表示と対応済みにしたあと消えることを確認 |
| REQ-010 | Critical flows(対応); System context | SKILL.md の「Board feedback loop」節の内容をレビュー |
| NFR-001 | Technology choices(保存先); Security and privacy | E2E テストで、リポジトリ内にコメントのファイルがないことを確認。Rust のテストで状態ディレクトリの名前を確認 |
| NFR-002 | Security and privacy; Interfaces and integrations | Rust のテスト `posts_comments_only_from_the_board_origin` |
| NFR-003 | Reliability and failure handling; Data model and state(時刻) | Rust のテストを 15 回続けて実行し、順序が安定することを確認。`iso_utc_millis` のテスト |
| NFR-004 | Technology choices(配信); System context | SKILL.md の手順が CLI とバックグラウンド実行だけでできていることをレビュー |
| NFR-005 | Technology choices | `Cargo.toml` に依存 crate がないことを確認 |

## Risks and tradeoffs

- ファイルをポーリングする方式なので、届くまで最大で約 500ms(と、ホストの通知の遅れ)がかかり、ボードが起動していなくても動く代わりに即時ではない。0.7.0(`20261017-210654-parallel-subagents`)では、ボードが起動していれば、トークン付きの API と SSE でその場で届けるように拡張した。ボードがない場合は、今もこの設計の方式で動く。
- 0.6.0 では、保存用の一時ファイルの名前が固定(`<id>.md.tmp`)だった。同じコメントを同時に保存すると衝突しうるため、0.7.0 で書き込みごとに別の名前を付けるように直した。
- `inbox` だけが配信済みに更新するので、エージェントが表示内容を読み落とすと、そのコメントは失われる。SKILL.md で、届いたらすぐ読むことを義務づけている。
- コメントやログが git に残らないため、チームで共有したり、あとから見返したりはできない。なりすましを防ぐことを優先した。

## Open questions

なし。
