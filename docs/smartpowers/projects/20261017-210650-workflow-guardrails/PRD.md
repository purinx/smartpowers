# Product Requirements Document

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210650-workflow-guardrails

## Summary

Smartpowers のワークフローに、事故を防ぐための 3 つの安全柵を加える。作業開始時に git リポジトリを用意すること、Linear と連携しているときは進捗を常に同期すること、そして push は Crit で commit ログと差分を承認してもらってからだけ行うことである。

## Problem and outcome

- git 管理されていないディレクトリでワークフローを始めると、PRD や実装の変更履歴が残らず、取り消しもできない。
- Linear と連携していても、エージェントは作業の最後にまとめて更新しがちで、作業中の状況がチームから見えない。
- エージェントが確認なしに push すると、レビューしていない変更が外部に出てしまう。

これらにより、変更は最初から追跡でき、作業中の進捗はリアルタイムに共有され、外部に出る変更はすべてユーザーが確認したものだけになる。

## Target users

- Smartpowers で機能開発を進めるユーザー(Claude Code / Codex の利用者)
- Linear でタスクを追っているチームメンバー

## Scope

### In scope

- ワークフロー開始時の git リポジトリ確認と初期化
- Linear 連携タスクの進捗同期(着手、途中経過、ブロック、スコープ変更、完了)
- Crit を使った push 承認と、Crit がない環境での代替手順

### Out of scope

- Linear 以外のタスク管理ツール固有の実装(手順は「同等のツール」にも適用できる書き方にとどめる)
- push 以外の外部操作(リリース作成など)の承認フロー
- 自動テストによるエージェント行動の検証

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria |
| --- | --- | --- | --- |
| REQ-001 | Must | ワークフロー開始時、作業ディレクトリが git リポジトリでなければ、ほかのどの手順よりも先に `git init` し、そのことをユーザーに伝える。 | git 管理外のディレクトリで開始すると、最初の操作が `git init` になる。既存のリポジトリは再初期化も変更もされない。 |
| REQ-002 | Must | Linear と紐付いたタスクでは、着手・意味のある進捗・ブロック・スコープ変更・完了のたびに、その時点で Linear の課題を更新する。 | 各イベントの直後に、課題の状態またはコメントが更新されている。作業の最後にまとめて更新しない。 |
| REQ-003 | Must | Linear のワークフロー状態は推測せず、読み取り専用の呼び出しで取得したものを使う。新しい状態は作らない。 | 状態名をハードコードせず、チームに既存の状態だけを使っている。 |
| REQ-004 | Should | Linear の更新に失敗したら、どの更新が欠けたかをユーザーに伝え、次の状態変更までに再試行するか突き合わせる。 | 失敗のあとの報告に、欠けた更新内容が含まれている。 |
| REQ-005 | Must | push の前に、未 push の commit ログと差分を Crit で提示し、コメントなしで承認された(`approved: true`)場合だけ push する。 | Crit のレビューが承認で終わるまで push されない。コメントがあれば修正し、もう一度提示する。 |
| REQ-006 | Must | Crit がない環境では、commit ログと diff stat をチャットに示し、明示的な承認を得てから push する。 | `crit` が見つからないとき、チャットで承認を求めている。 |
| REQ-007 | Must | 承認したあとに commit が追加されたり書き換えられたりしたら、もう一度承認を得る。force push はユーザーが明示したときだけ行う。 | 承認された範囲と異なる範囲は push されない。 |

## Non-functional requirements

| ID | Area | Requirement | Measure |
| --- | --- | --- | --- |
| NFR-001 | Portability | 手順は Claude Code と Codex のどちらでも実行できる。 | SKILL.md の必須手順に、片方のホストでしか使えない機能を前提としたものがない。ホスト固有の機能は例示にとどめている。 |
| NFR-002 | Safety | 外部への書き込み(Linear の更新、push)は、ユーザーが承認した範囲に限る。 | Linear へ初めて書き込む前に対象プロジェクトの確認があり、push は REQ-005〜007 に従っている。 |

## User journeys

1. ユーザーが空のディレクトリで「新しいプロジェクトを始めて」と頼む。エージェントは `git init` してからプロジェクトを作る。
2. Linear と紐付いたタスクに着手すると課題が In Progress になり、テストが通るたびにコメントが付き、完了すると Done になる。
3. 作業が終わると、エージェントが Crit を開いて commit ログと差分を見せる。ユーザーが Finish Review を押すと push される。

## Assumptions and constraints

- Linear との連携は MCP などの接続ツールを通じて行う。接続がなければ、ローカルの計画を正とする。
- Crit は任意の依存で、ない環境では REQ-006 の代替手順を使う。
- 要件はコードではなく、エージェントへの指示(SKILL.md と references)で実現する。

## Success measures

- このリポジトリで行われた push が、すべて Crit の承認を経ている。
- Linear と連携したプロジェクトで、課題の更新時刻が作業の進み具合と一致している。

## Risks

- エージェントが指示を守らない可能性がある。手順は明確で短い命令文にし、例外条件も書いておく。
- Crit のレビュー待ちの間は push が止まる。これは意図した挙動とする。

## Open questions

なし。
