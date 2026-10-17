# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210654-parallel-subagents
PRD: ../PRD.md

## Design summary

スケジューリング、エージェントの接続、コメントのライブ配信は、既存のボード(std のみで書いた Rust の実行ファイル)に追加する。サブエージェントの起動と統合は、`SKILL.md` に定めるリードの手順で行う。ボードは起動時に、自分のアドレスと起動ごとのトークンを状態ディレクトリに書き出す。エージェント用のコマンドはそれを読み、トークン付きの API と SSE でボードにつなぐ。ボードがなければ、同じコマンドがファイルを直接読み書きする。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| 並列実行の主体 | ホスト標準のサブエージェント(リードが起動する) | 権限の確認が普段どおり働き、リードがコメントや統合を仲介できる。追加の実行環境もいらない。 | `board run` で `claude -p` / `codex exec` を起動する独立したオーケストレーター(権限を事前に与える必要があり、費用も見えにくい) |
| スケジューラ | Rust の `board next`(`project/schedule.rs`) | タスクファイルの解析を再利用でき、判断が決定的でテストできる。 | エージェントがその場で判断する(見落としやすい) |
| コメントの配信 | SSE(エージェント用の `events` ストリーム) | 既存の UI 向け SSE と同じ仕組みで、std だけで実装できる。片方向で十分。 | WebSocket(std に実装がない)、ファイルのポーリング(遅く、接続状態が分からない) |
| 認証 | 起動ごとのランダムなトークン(Bearer) | ブラウザから読めないファイルにあるため、他サイト経由の悪用を防げる。 | Origin の確認だけ(CLI の要求には Origin が付かない) |
| 乱数 | std の `RandomState`(OS から初期化される SipHash の鍵) | 依存を増やさずに、予測できない 128 ビットのトークンを作れる。 | 外部 crate(std のみの方針に反する) |

## System context

既存のボード(進捗ボード、フィードバックループ)と状態ディレクトリを使う。追加するのは次のとおり。

- **CLI**: `next` サブコマンドと、`inbox` の `--task` オプション
- **サーバ**: `/api/agent/...` のルート、購読のハブ、エージェント用の SSE
- **API**: プロジェクトの JSON に `listening` と `agentListening` を加える
- **UI**: Agent バッジとリード接続の表示

ブラウザ向けの API とコメント投稿の口(Origin 必須)は変えない。

## Architecture and component boundaries

- **`project/schedule.rs`**: `plan_order` は PLAN.md に出てくる `NN_name.md` の順序を取り出す。`next` は PLAN の順に並べ、実行中のタスクと準備済みのタスクから、開始できるタスクとその理由を返す。
- **`runtime.rs`**: `ServerInfo` は `addr` と `token` を `<state dir>/board-server` に保存し、読み込む。Unix では 0600 で作成する。
- **`client.rs`**: `Client::connect` は状態ファイルを読み、接続できるか確かめる。リクエストを送る `post` と、SSE を読み取る `EventStream` を持ち、`EventStream` はタイムアウトしても途中まで読んだ行を保持する。
- **`server/hub.rs`**: `Hub` は購読者を管理する。購読者は「プロジェクトとタスク」または「全タスク」のどちらかを指定して登録する。`publish` で該当する購読者に配り、`listeners` で接続状況を返す。`Subscription` は破棄されるときに登録を外し、watcher の版を上げる。
- **`server/events.rs`**: `agent_stream` は、購読 → 保留中のコメントの送信 → `ready` → ライブ配信の順に進む。5 秒ごとに keepalive を送り、送信できたコメントを配信済みにする。
- **`server/mod.rs`**: `Context` が workspace、watcher、hub、token をまとめる。`/api/agent/...` ではトークンを検証してから振り分ける。ブラウザからコメントが投稿されたら `hub.publish` を呼ぶ。
- **`agent.rs`**: `next` を表示する。`inbox`、`reply`、`log` は、ボードがあれば API を、なければファイルを使う。`comment_block` は、どちらの経路でも同じ本文を作る。
- **`SKILL.md` › Parallel execution**: リードの手順(6 ステップ)とサブエージェントへのブリーフ(5 つのルール)を定める。
- **UI**: `task-parts.js` は `listening` のタスクに Agent チップを、`header.js` は `agentListening` のときにリード接続を表示する。

## Data model and state

- `<state dir>/board-server`: `addr=<SocketAddr>` と `token=<32 桁の 16 進数>`。ボードが起動するたびに上書きする。
- 購読者の情報はメモリ上だけに持ち、永続化しない。
- タスクの `Status`、`PLAN.md`、Linear は、リードだけが変更する。
- `Project.plan_order`: PLAN.md から取り出したタスク番号の順序

## Interfaces and integrations

| Method | Path | Auth | 内容 |
| --- | --- | --- | --- |
| GET | `/api/agent/projects/<id>/events[?task=<n>]` | Bearer | 保留中のコメント → `ready` → ライブで届くコメント(`event: comment`、本文はエージェント向けのテキスト) |
| POST | `/api/agent/projects/<id>/tasks/<n>/log` | Bearer | 進捗ログの追加(text/plain) |
| POST | `/api/agent/projects/<id>/comments/<c>/reply[?resolve=1]` | Bearer | エージェントとしての返信。スレッドを対応済みにすることもできる |
| GET | `/api/projects/<id>` | Loopback | 各タスクの `listening` と、プロジェクトの `agentListening` |

CLI: `board next --project <id> [--limit <n>]` と、`board inbox --project <id> [--task <nn>] [--wait] [--timeout <secs>]`

## Critical flows

1. **割り振り**: リードが `board next` を実行し、各タスクを `In Progress` にして、worktree でサブエージェントを同時に起動する。
2. **接続**: サブエージェントは `inbox --task NN --wait` を実行する。`Client::connect` で GET events に接続し、Hub に登録されて、ボードに Agent バッジが出る。
3. **コメント**: ブラウザから POST すると、`comments::create` → `hub.publish` → 該当タスクのストリームへの送信 → 配信済みにする、の順に進む。`inbox` は最初のコメントのあと 300ms 待ってまとめて表示し、終了する。
4. **統合**: リードがマージして検証し、状態を更新してから、もう一度 `board next` を実行する。
5. **フォールバック**: ボードがない(状態ファイルがない、または接続できない)ときは、`inbox` はファイルを 0.5 秒ごとに確認し、`reply` と `log` はファイルに直接書く。

## Security and privacy

- トークンは起動ごとに生成した 128 ビットの値で、所有者だけが読めるファイルにある。ブラウザはこのファイルを読めないので、エージェント用 API を他サイトから使えない(NFR-001)。
- エージェント用のルートでも、Host と Origin がループバックであることを引き続き確認する。
- `SKILL.md` で、トークンを表示したり、ログに残したり、他に渡したりすることを禁止している。

## Reliability and failure handling

- 購読を先に登録してから保留中のコメントを読むので、間に投稿されたコメントも取りこぼさない。両方の経路で届いたコメントは、配信済みの ID の集合で重複を除く(NFR-003)。
- 配信済みにする前にコメントをファイルから読み直すので、同時に起きた解決などの変更を上書きしない。
- 一時ファイルの名前を書き込みごとに変えて、同時の保存がぶつからないようにする。
- 状態ファイルが古いままで、別のプロセスがポートを使っていても、トークンが一致しないため 401 になり、誤った相手には書き込まない。
- 接続が切れたことは、keepalive の送信に失敗した時点で検知し、登録を外す(NFR-004)。

## Observability and operations

ボードの Agent バッジとリード接続の表示で、誰が聞いているかが分かる。`inbox` の出力には、返信の方法と、もう一度待ち受けるコマンドが含まれる。

## Testing strategy

- **Rust の単体テスト**: スケジューラ(PLAN の順、グループの判定、グループなしは単独、待つ理由)、Hub の振り分けと解除、トークンの保存・読み込み・権限、エージェント用 API のトークン必須、エージェント用ストリーム(保留中 → ready → ライブ、ほかのタスクは除外、`listening` の表示)
- **Python の E2E テスト(起動スクリプト経由)**: `inbox --task 01 --wait` が接続されると `listening` になること、投稿から 5 秒未満で届くこと、`next` が PLAN の順で返すこと
- **手動での確認**: 仮のプロジェクトで 2 つのサブエージェントの接続を再現し、コメントが担当にだけ届くこと、切断するとバッジが消えることを確かめた。

## Deployment and rollout

プラグイン 0.7.0 で配布した。ボードを使わない利用者には影響がない(ファイル経由の動作は従来どおり)。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture and component boundaries(schedule.rs); Data model and state(plan_order) | 単体テスト `follows_plan_order_rather_than_numbers` と、E2E の `test_next_lists_parallel_tasks_in_plan_order` |
| REQ-002 | Architecture and component boundaries(schedule.rs) | 単体テスト `starts_the_first_ready_task_with_its_parallel_group`、`only_joins_the_running_group`、`ungrouped_tasks_run_alone_and_reasons_explain_idleness` |
| REQ-003 | Architecture and component boundaries(schedule.rs、agent.rs) | 単体テストで、待つ理由の文言(依存待ち、ブロック中、すべて完了、実行中の待ち)を確認する |
| REQ-004 | Interfaces and integrations(CLI) | CLI の単体テストで、`--limit 2` を解釈すること、`0` を拒否することを確認する |
| REQ-005 | Architecture and component boundaries(SKILL.md); Critical flows(割り振り) | SKILL.md の手順をレビューする。実際のサブエージェントでの検証は未実施(Open questions) |
| REQ-006 | Architecture and component boundaries(SKILL.md) | SKILL.md のブリーフ項目をレビューする |
| REQ-007 | Data model and state; Architecture and component boundaries(SKILL.md) | SKILL.md のルールをレビューする |
| REQ-008 | Critical flows(統合); Architecture and component boundaries(SKILL.md) | SKILL.md の手順をレビューする |
| REQ-009 | Architecture and component boundaries(hub.rs、events.rs); Critical flows(接続、コメント) | 単体テスト `streams_comments_to_a_listening_agent`、E2E の `test_subagent_receives_comments_live`、手動での 2 エージェントの再現 |
| REQ-010 | Interfaces and integrations(log); Critical flows(接続) | 単体テスト `agent_api_requires_the_token`(ログの追加)と、手動でボードに反映されることを確認する |
| REQ-011 | Architecture and component boundaries(hub.rs、UI) | 単体テスト `routes_comments_to_matching_subscribers`(解除で消える)、API の契約テスト(`listening`、`agentListening`)、手動での確認 |
| REQ-012 | Critical flows(フォールバック) | 単体テスト `delivers_each_comment_once_and_records_replies` と `logs_only_to_existing_tasks`(ボードなし) |
| NFR-001 | Security and privacy; Architecture and component boundaries(runtime.rs) | 単体テスト `saves_and_loads_server_info`(0600)と `agent_api_requires_the_token`(401) |
| NFR-002 | Technology choices(SSE); Critical flows(コメント) | E2E テストで、投稿から受け取るまで 5 秒未満であることを確認する |
| NFR-003 | Reliability and failure handling | `agent_stream` の実装レビューと、ストリームの単体テスト(保留中とライブの両方が 1 回ずつ届く) |
| NFR-004 | Reliability and failure handling; Architecture and component boundaries(events.rs) | 手動の確認で、切断から約 5 秒でバッジが消えた |

## Risks and tradeoffs

- 実際のサブエージェント(Claude Code の Agent ツール)での運用はまだ検証していない。動作は、CLI で再現した範囲でしか確認できていない。
- ホスト標準のサブエージェントを使うため、会話を閉じると並列実行も止まる。その代わり、権限の確認と費用の見え方は普段どおりになる。
- 全タスクを聞いているリードとサブエージェントが同時にいると、同じコメントが両方に届く。`SKILL.md` では、並列実行中はリードが全タスクを待ち受けない運用にしている。

## Open questions

- Claude Code のサブエージェントは、バックグラウンドのコマンドの完了通知を作業中に受け取れるか。受け取れない場合、各ステップ後に確認する手順を標準にするか。
- Codex のサブエージェントや worktree の扱いで、同じ手順がそのまま使えるか。
