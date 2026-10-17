# Product Requirements Document

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-18
Project: 20261018-001457-bash-scripts

## Summary

Smartpowers に残っている Python のスクリプトとテストをなくす。ワークフローの 2 つのスクリプト(プロジェクトの作成と検証)と、その単体テスト、マーケットプレイスのテストは bash に書き直す。ボードの E2E テストは、bash で書くと壊れやすいため Rust のテストに移す。これで、プラグインを使うのにも開発するのにも Python が要らなくなる。

## Problem and outcome

- プラグインを使うには、ボードのための Rust(必要なら自動で入る)に加えて、`python3` も必要になっている。Python の入っていない環境では、ワークフローの最初の手順(プロジェクトの作成)から失敗する。
- リポジトリには「新しいスクリプトは bash か Rust で書く」という方針があるが、古いスクリプトとテストが Python のまま残っていて、方針と食い違っている。CI も Python のセットアップを必要としている。

移行後は、プラグインの利用に必要なのが bash と一般的なコマンドだけになり、リポジトリのスクリプトとテストがすべて bash か Rust にそろう。

## Target users

- Claude Code / Codex で Smartpowers を使うユーザー(macOS、Linux、Windows の Git Bash)
- Smartpowers の開発者(このリポジトリのメンテナーとコントリビューター)

## Scope

### In scope

- `init_project.py` と `validate_workflow.py` の bash への置き換え
- `tests/` の Python テスト 4 本の置き換え(bash 3 本、Rust 1 本)
- `SKILL.md`、README、CI から Python への言及と依存をなくすこと
- 0.8.0 としてのリリース

### Out of scope

- ボードのランチャー(`scripts/board`、`board.cmd`)と Rust のサーバーの機能変更
- 既にある bash のスクリプト(`scripts/release-notes.sh`)の変更
- Windows の `cmd.exe` や PowerShell から bash なしでワークフローのスクリプトを動かすこと(Windows では Git Bash を前提とする)
- 検証ルールの追加や変更(今の検証内容をそのまま移す)

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria |
| --- | --- | --- | --- |
| REQ-001 | Must | プロジェクト作成スクリプトを bash で提供する。引数(`--root`、`--title`、`--slug`)、作られる `PROJECT.md` の内容、標準出力(`Project:` と `Path:`)は今と同じにする。 | 同じ引数で実行すると、今のスクリプトと同じ名前のディレクトリと同じ内容の `PROJECT.md` ができる。同じスラッグでも時刻が違えば別のプロジェクトになる。既にあるプロジェクトは上書きしない。 |
| REQ-002 | Must | タイトルからスラッグを作れないとき(日本語だけのタイトルなど)や、スラッグや題名が不正なときは、理由を示してエラーで終了する。 | 日本語だけのタイトルで `--slug` を省くと、`--slug` を指定するよう求めるエラーになる。64 文字を超えるスラッグや大文字を含むスラッグは拒否される。改行を含む題名や空の題名は拒否される。 |
| REQ-003 | Must | 検証スクリプトを bash で提供する。引数(`--root`、`--project`、`--list`、`--phase`)、検証の内容、成功時とエラー時のメッセージ、終了コードは今と同じにする。 | 今の Python のテストが確かめている 7 つの場合(正常、要件の対応漏れ、タスクのファイル名の誤り、承認記録の欠落、下書きの計画、ほかのプロジェクトの文書、ほかのプロジェクトが壊れている場合)で、同じ結果になる。このリポジトリの全プロジェクトで、今と同じ判定になる。 |
| REQ-004 | Must | `--list` は、プロジェクトごとに ID、状態、最終更新時刻、題名をタブ区切りで出す。 | このリポジトリで実行すると、全プロジェクトが今と同じ形式で並ぶ。`PROJECT.md` のないプロジェクトは `Invalid (missing PROJECT.md)` と出る。 |
| REQ-005 | Must | `SKILL.md` と README の手順は、新しいスクリプトを呼ぶ。`python3` を前提とした記述は残さない。 | `SKILL.md`、README、CI のどこにも `python` が出てこない。 |
| REQ-006 | Must | ワークフローのスクリプトとマーケットプレイスの整合性のテストを bash で書き、1 つのコマンドで全部を実行できるようにする。 | `tests/run.sh` を実行すると、すべてのテストが走り、1 つでも失敗すれば 0 以外で終わる。失敗したテストの名前と理由が表示される。 |
| REQ-007 | Must | ボードの E2E テスト(配信、コメントの往復、SSE でのその場の配信、`next` の順序、ループバック以外の拒否)を Rust のテストに移す。 | `cargo test` で 5 つの場合がすべて確かめられる。 |
| REQ-008 | Must | CI は Python を使わずに、3 つの OS で bash のテストと Rust のテストを実行する。 | CI の設定から `setup-python` がなくなり、macOS、Linux、Windows で緑になる。 |
| REQ-009 | Must | リポジトリから Python のファイルをなくす。 | `git ls-files '*.py'` が何も返さない。 |
| REQ-010 | Must | 0.8.0 としてリリースする。 | CHANGELOG に 0.8.0 の節があり、`v0.8.0` のタグと GitHub のリリースがある。 |

## Non-functional requirements

| ID | Area | Requirement | Measure |
| --- | --- | --- | --- |
| NFR-001 | Portability | macOS の標準の bash 3.2 と BSD のコマンド、Linux の GNU のコマンド、Windows の Git Bash のどれでも動く。 | CI の 3 つの OS で bash のテストが通る。手元の macOS(bash 3.2)でも通る。 |
| NFR-002 | Dependencies | bash と、どの環境にもある基本的なコマンド(`sed`、`awk`、`grep`、`find`、`date` など)だけを使う。`jq`、`bats`、Python などの追加のツールは要らない。 | スクリプトとテストが呼ぶコマンドが、この範囲に収まっている。 |
| NFR-003 | Compatibility | 既にあるプロジェクトの文書は、書き換えずにそのまま新しいスクリプトで扱える。 | このリポジトリの 7 プロジェクトで、移行前と同じ判定になる。 |

## User journeys

1. Python の入っていない環境で、ユーザーが新しいプロジェクトを始める。エージェントは bash のスクリプトでプロジェクトを作り、PRD を検証する。
2. コントリビューターが変更を加え、`tests/run.sh` と `cargo test` を実行して確かめる。Python の準備は要らない。

## Assumptions and constraints

- ワークフローを動かすホスト(Claude Code、Codex)は、Windows でも bash を使える(Claude Code は Windows で Git Bash を使う)。
- タイトルからスラッグを作る処理は、ASCII の英数字だけのタイトルでは今と同じ結果にする。アクセント付きの文字(`é` など)の扱いは環境によって変わってもよい。どの場合でも、`--slug` を指定すれば確実に決まる。
- ボードの E2E テストはランチャーを通さずに、ビルドしたサーバーを直接起動して確かめる。ランチャーそのものは、CI の「Launcher without Rust」のジョブが引き続き 3 つの OS で確かめる。

## Success measures

- リポジトリに Python のファイルがなく、CI が Python なしで緑になる。
- 移行の前後で、このリポジトリの全プロジェクトの検証結果が変わらない。

## Risks

- bash 3.2 と BSD のコマンドは機能が限られる(連想配列や `sed -i` の書き方の違いなど)。テストを 3 つの OS で回して確かめる。
- Windows の Git Bash では、改行コード(CRLF)の文書を読むと結果が変わるおそれがある。CRLF の文書もテストで確かめる。
- ボードの E2E テストを Rust に移すと、ランチャーを通した確認は CI の起動ジョブだけになる。

## Open questions

なし。
