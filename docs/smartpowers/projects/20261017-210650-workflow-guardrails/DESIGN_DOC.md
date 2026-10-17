# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210650-workflow-guardrails
PRD: ../PRD.md

## Design summary

3 つの安全柵は、どれもエージェントへの指示として実装する。git の初期化と push 承認は `SKILL.md` に新しい節として追加し、Linear の同期は `references/task-manager.md` に「Progress sync」節を追加して、Phase 4 から参照する。コードは追加しない。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| 実装手段 | `SKILL.md` と references への指示 | ホストをまたいで同じ手順を使え、ワークフローの他の部分と同じ仕組みで配布できる。 | Claude Code の hooks(Codex で使えない) |
| push の承認 | `crit --range <upstream>..HEAD` | commit ログと差分をブラウザで見て、行にコメントでき、承認状態(`approved: true`)を機械的に判定できる。 | チャットにまとめて表示(Crit がないときの代替として残す) |
| Linear の操作 | 接続済みの MCP ツール | 既存の task-manager ガイドと同じ経路を使える。 | 専用 CLI(依存が増える) |

## System context

既存の 4 フェーズ(要件、設計、計画、実装)のワークフローはそのまま使う。変更するのは `SKILL.md` に追加する 2 節と、Phase 4 からの参照、それに `references/task-manager.md` の新しい節だけである。検証スクリプトとプロジェクトの構造は変えない。

## Architecture and component boundaries

- **`SKILL.md` › Initialize git first**: 最初の節に置く。`git rev-parse --is-inside-work-tree` で確認し、git 管理外なら `git init` してユーザーに伝える。既存のリポジトリには触れない。
- **`SKILL.md` › Phase 4**: 状態が変わるたびに、紐付いたリモート課題へ反映するよう求め、詳細は task-manager ガイドの Progress sync に任せる。
- **`references/task-manager.md` › Progress sync**: 着手、進捗、ブロック、スコープ変更、完了のそれぞれで何をするかを定める。状態は既存のものを読み取り専用の呼び出しで取得し、失敗したときの扱いも書く。
- **`SKILL.md` › Push approval**: upstream との差分の取り方、Crit でのレビューの流れ、Crit がないときの代替、承認の範囲、force push の禁止を定める。

## Data model and state

- タスクファイルの `Status:` と Linear の課題の状態を、常に一致させる。
- タスクファイルの `Remote task:` に、課題のキーと URL を記録する。
- push の承認は、その時点の `<upstream>..HEAD` の範囲に対して与えられる。

## Interfaces and integrations

- Linear: 課題の状態変更、コメントの追加、ワークフロー状態の取得
- Crit: `crit --range` をバックグラウンドで起動し、終了時に標準エラー出力の `approved: true/false` を読む。
- git: `rev-parse`、`init`、`fetch`、`log <upstream>..HEAD`、`push`

## Critical flows

1. **開始**: リポジトリかを確認し、違えば `git init` して報告してから、プロジェクトを選ぶか作る。
2. **状態の変更**: タスクファイルの `Status` を更新し、Linear の状態を更新し、コメントを付ける。失敗したら報告して突き合わせる。
3. **push**: `git fetch` → 未 push の範囲を確認 → Crit で提示 → 承認されたら範囲が変わっていないことを確かめて push する。コメントがあれば修正して、もう一度提示する。

## Security and privacy

- 外部への書き込みは、承認された範囲に限る(NFR-002)。
- force push は既定で禁止する。

## Reliability and failure handling

- Linear の更新に失敗したら、そのことを報告し、次の状態変更までに突き合わせる。
- push の前にもう一度 fetch し、範囲が承認時と同じことを確かめる。違っていたら承認を取り直す。

## Observability and operations

Linear の課題の履歴と、Crit のレビュー記録(承認時刻、レビューしたファイル数)が作業の記録になる。

## Testing strategy

エージェントの振る舞いは自動テストの対象外とする。指示の内容はレビューで確認し、実際の運用で検証する。このリポジトリで行われたすべての push(0.3.0〜0.7.0)は、Crit の承認を経ている。

## Deployment and rollout

プラグイン 0.3.0 で配布した。既存のプロジェクトにも、次のセッションから適用される。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture and component boundaries(Initialize git first); Critical flows(開始) | git 管理外のディレクトリでワークフローを始め、最初の操作を確認する |
| REQ-002 | Architecture and component boundaries(Progress sync); Data model and state | Linear と紐付いたプロジェクトで、各イベント後に課題の更新を確認する |
| REQ-003 | Architecture and component boundaries(Progress sync); Interfaces and integrations | ガイドの記述をレビューし、運用中に新しい状態が作られていないことを確認する |
| REQ-004 | Reliability and failure handling; Critical flows(状態の変更) | 更新の失敗を想定したガイドの記述をレビューする |
| REQ-005 | Architecture and component boundaries(Push approval); Critical flows(push) | このリポジトリの各 push が Crit の承認後に行われていることを確認する |
| REQ-006 | Architecture and component boundaries(Push approval) | Crit がない環境を想定したガイドの記述をレビューする |
| REQ-007 | Critical flows(push); Security and privacy | 承認後に範囲が変わった場合に再提示されることを運用で確認する |
| NFR-001 | Technology choices; Architecture and component boundaries | SKILL.md の必須手順に、ホスト固有の機能への依存がないことをレビューする |
| NFR-002 | Security and privacy; Critical flows | 外部へ書き込む前に確認や承認があることをレビューと運用で確認する |

## Risks and tradeoffs

- 指示で実装したため、守られることは保証できない。その代わり、ホストに依存せず導入の手間もない。
- push ごとに承認を求めるので、速さよりも安全を優先している。

## Open questions

なし。
