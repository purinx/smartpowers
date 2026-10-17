# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-18
Project: 20261018-001457-bash-scripts
PRD: ../PRD.md

## Design summary

`scripts/` の 2 つの Python スクリプトを、同じ名前の `.sh` に置き換える。引数、出力、終了コードは今のものをそのまま引き継ぐ。テストは次のように移す。

- ワークフローのスクリプトとマーケットプレイスのテスト: `tests/` の bash のテストにする。外部のツールに頼らない小さなテストランナー(`tests/run.sh`)で実行する。
- ボードの E2E テスト: ボードの crate の結合テスト(`board/tests/`)に移す。`cargo test` がビルドしたサーバーを、子プロセスとして直接起動して確かめる。

そのうえで、`SKILL.md`、README、CI から Python をなくし、0.8.0 としてリリースする。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| ワークフローのスクリプト | bash(3.2 互換)と POSIX の基本コマンド | PRD の方針(NFR-001、NFR-002)。macOS の標準の bash でも動く。 | Rust でボードのサブコマンドにする(初回にビルドが要り、最初の手順が重くなる) |
| bash のテスト | 自前の小さなランナー(`tests/run.sh` と `tests/lib.sh`) | 追加のツールが要らない。失敗したテストの名前と理由を出せれば足りる。 | bats(インストールが要る)、shunit2(取り込みが要る) |
| ボードの E2E テスト | Rust の結合テスト(`board/tests/board_e2e.rs`) | JSON の解析、5 秒以内の到着の計測、プロセスの停止を std だけで確実に書ける。`env!("CARGO_BIN_EXE_smartpowers-board")` でビルドしたサーバーを直接使える。 | bash と curl(JSON の解析に jq が要り、Windows でのプロセスの停止が不安定) |
| JSON の確認 | 値は bash のテストで、構文は Rust のテストで確かめる | bash では JSON の構文を確かめられない。Rust の結合テストには、レスポンスを読むための小さな JSON パーサーを書くので、それをマニフェストの構文の確認にも使う。 | jq、node(追加の依存になる) |
| Unicode からスラッグへの変換 | `iconv -f UTF-8 -t ASCII//TRANSLIT` のあと、英数字以外を `-` にする | ASCII のタイトルでは今と同じ結果になる。アクセント付きの文字はできる範囲で変換する。iconv がない、または失敗するときは ASCII 以外を捨てる。 | 変換しない(`Café` が `caf` になる) |

## System context

変更するのは `plugins/smartpowers/skills/smartpowers/scripts/`、`SKILL.md`、`tests/`、`board/tests/`(新規)、CI、README、`.gitignore`、バージョンの記載である。ボードのサーバーのソース(`board/src/`)とランチャーは変えない。プロジェクトの文書の形式も変えないので、既にあるプロジェクトはそのまま検証できる(NFR-003)。

## Architecture and component boundaries

- **`scripts/init_project.sh`**: 引数の解析、題名とスラッグの検証、スラッグの生成、プロジェクトの ID の決定、`PROJECT.md` の作成。
- **`scripts/validate_workflow.sh`**: プロジェクトの解決、`--list`、フェーズごとの検証(requirements、design、plan、implementation)。関数の分け方は Python 版に合わせる(`require_project`、`require_status`、`require_coverage`、`validate_requirements`、`validate_design`、`validate_plan`、`task_files`)。読み違えたときに、2 つの実装を見比べやすくするためである。
- **`tests/lib.sh`**: 次の 2 つを用意する。
  - アサーション(`assert_eq`、`assert_contains`、`assert_fails` など)
  - テストごとの一時ディレクトリ
- **`tests/run.sh`**: `tests/test_*.sh` を読み込み、`test_` で始まる関数をそれぞれサブシェルで実行する。最後に、成功と失敗の数を出す。
- **`tests/test_init_project.sh`、`tests/test_validate_workflow.sh`、`tests/test_marketplace.sh`**: 今の Python のテストの各ケースを移す。CRLF の文書と `--list` のケースを加える。
- **`board/tests/board_e2e.rs`**: 今の `test_board.py` の 5 ケースを移す。
- **`board/tests/support/mod.rs`**: 結合テスト用の部品。
  - 一時リポジトリ
  - サーバーの起動と停止
  - std だけで書いた最小限の HTTP クライアント
  - 最小限の JSON パーサー(テストのコードにだけ置き、配布物には入らない)
- **`board/tests/manifests.rs`**: マーケットプレイスと 2 つのプラグインのマニフェストを、上の JSON パーサーで読めることを確かめる。

## Data model and state

- 文書の読み込み: ファイルを読むときに `\r` を取り除く。Python の `read_text` が CRLF を LF として扱うのと同じ結果にするためである。
- `Status:` と `Project:`: 先頭からの最初の一致を使う。大文字と小文字は区別しない。値の前後の空白は取り除く。
- 要件 ID: `REQ-` か `NFR-` に 3 桁の数字が続き、前後が英数字や `_` でないものを数える(Python の `\b` と同じ)。抜き出した ID は `sort -u` で集合にし、`comm` で差を取る。
- 状態の比較: 両方を小文字にしてから比べる。
- `--list` の更新時刻: プロジェクトの中で一番新しいファイルの時刻を使う。
  - 時刻の取得: `stat -f %m`(BSD)、だめなら `stat -c %Y`(GNU)
  - 書式: `date -r`(BSD)、だめなら `date -d @`(GNU)で、`+09:00` のようなコロン付きのオフセットにする。
- 作成時刻: 1 回の `date` の呼び出しから、ID 用(`YYYYMMDD-HHMMSS`)と `Created:` 用(ISO 8601、コロン付きのオフセット)の両方を作る。テストでは `SMARTPOWERS_NOW`(UNIX 時刻)と `TZ` を与えて時刻を固定する。

## Interfaces and integrations

- `init_project.sh --root <dir> --title <title> [--slug <slug>]`
  - 成功時: `Project: <id>` と `Path: <path>` を出して 0 で終わる。
  - 引数や入力の誤り: `init_project.sh: error: <理由>` を標準エラー出力に出して 2 で終わる(Python の argparse の `parser.error` と同じ)。
- `validate_workflow.sh [--root <dir>] (--list | --project <id>) [--phase requirements|design|plan|implementation]`
  - 成功時: `OK: <id> <phase> artifacts are structurally valid` を出して 0 で終わる。
  - 検証の失敗: `ERROR: <理由>` を出して 1 で終わる。
  - 引数の誤り: 2 で終わる。
  - エラーの文言は、Python 版の文言をそのまま使う。
- `SKILL.md` と README からは、`bash <plugin-root>/skills/smartpowers/scripts/<name>.sh` の形で呼ぶ。実行権限がなくても、Windows でも、同じ書き方で動く。
- `bash tests/run.sh [テストファイル...]`: 引数がなければ全部を実行する。
- CI: `Python tests` のステップを `Shell tests`(`shell: bash` で `bash tests/run.sh`)に置き換え、`setup-python` を消す。ボードの E2E テストは、既にある `cargo test` のステップで走る。

## Critical flows

1. **プロジェクトの作成**: 引数を解析 → 題名を検証 → スラッグを決めて検証 → 時刻から ID を作る → 親ディレクトリを作る → `mkdir` でプロジェクトのディレクトリを作る(既にあれば失敗するので、上書きは起きない) → `PROJECT.md` を書く → ID とパスを出す。
2. **検証**: ID の形式を確かめる → `PROJECT.md` の `Project:` を確かめる → フェーズに応じて PRD、DESIGN_DOC、PLAN とタスクの順に、所属、状態、承認記録、要件の対応を確かめる → 最初に見つかった誤りで終わる。
3. **ボードの E2E テスト**: 一時リポジトリを作る → `SMARTPOWERS_STATE_HOME` を一時ディレクトリにしてサーバーを `--port 0 --no-open` で起動する → 標準出力の 1 行目からポートを読む → HTTP で確かめる → `inbox`、`reply`、`log`、`next` は同じバイナリをサブコマンド付きで実行する → 終わったら子プロセスを kill する。

## Security and privacy

- スクリプトは `--root` の下の `docs/smartpowers/projects/` 以外には書き込まない。
- 題名は `PROJECT.md` の 1 行目にそのまま書くが、ファイル名には使わない。ファイル名に使うのは、検証済みのスラッグだけである。
- 変数は常に `"..."` で囲み、題名にスペースや `*` が入っていても、意図しない展開が起きないようにする。

## Reliability and failure handling

- 両方のスクリプトは `set -eu` で動かす。誤りは、意図したメッセージと終了コードで返す。
- iconv がない、または変換に失敗したときは、ASCII 以外を捨てる方法に切り替える。
- `stat` と `date` は BSD 版を試して、だめなら GNU 版を使う。
- テストのランナーは、テストごとにサブシェルと一時ディレクトリを使う。そのため、1 つのテストが失敗しても、ほかのテストに影響しない。

## Observability and operations

標準出力と標準エラー出力の内容は Python 版と同じなので、エージェントへの指示や運用は変わらない。テストのランナーは、失敗したテストの名前、期待した値、実際の値を出す。

## Testing strategy

- 移すテスト: 今の Python のテストのすべてのケースを、bash か Rust のテストに 1 対 1 で移す。
  - プロジェクトの作成: 3 ケース
  - 検証: 7 ケース
  - マーケットプレイス: 3 ケース
  - ボード: 5 ケース
- 加えるテスト:
  - CRLF の文書が LF の文書と同じに判定されること
  - `--list` の形式と、`PROJECT.md` のないプロジェクトの扱い
  - スラッグの長さと文字種の誤り
  - 既にあるプロジェクトを上書きしないこと
  - マニフェストの JSON の構文
- 移行前後の比較: このリポジトリの 7 プロジェクトを、Python 版と bash 版の両方でフェーズごとに検証し、出力が一致することを確かめる。`--list` も比べる。比べ終えてから Python のファイルを消す。
- CI: 3 つの OS で `bash tests/run.sh` と `cargo test` が通ること。手元の macOS の `/bin/bash`(3.2)でも、`bash tests/run.sh` を実行する。

## Deployment and rollout

0.8.0 として出す。CHANGELOG の Changed に「ワークフローのスクリプトが bash になり、`python3` が要らなくなった」ことを書く。`SKILL.md` の呼び出し方が変わるので、古いバージョンのスキルが新しいスクリプトを呼ぶことはない。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture and component boundaries(init_project.sh); Interfaces and integrations; Critical flows(プロジェクトの作成) | bash のテストで、ID、`PROJECT.md` の内容、出力、別の時刻で別のプロジェクトになること、上書きしないことを確かめる |
| REQ-002 | Technology choices(スラッグへの変換); Interfaces and integrations | bash のテストで、日本語だけの題名、長すぎるスラッグ、大文字、改行を含む題名がそれぞれ終了コード 2 になることを確かめる |
| REQ-003 | Architecture and component boundaries(validate_workflow.sh); Data model and state; Critical flows(検証) | bash のテストで Python 版の 7 ケースを確かめる。移行前後の比較で、全プロジェクトの出力が一致することを確かめる |
| REQ-004 | Data model and state(`--list` の更新時刻); Interfaces and integrations | bash のテストで形式と欠けた `PROJECT.md` を確かめる。移行前後で `--list` の出力を比べる |
| REQ-005 | Interfaces and integrations(SKILL.md と README からの呼び出し) | `git grep -i python` が `SKILL.md`、README、CI で何も返さないことを確かめる |
| REQ-006 | Architecture and component boundaries(run.sh、lib.sh、各テスト) | わざと失敗するテストで、名前と理由が表示され、0 以外で終わることを手元で確かめる |
| REQ-007 | Architecture and component boundaries(board_e2e.rs); Critical flows(ボードの E2E テスト) | `cargo test` で 5 ケースが通ることを確かめる |
| REQ-008 | Interfaces and integrations(CI) | CI の 3 つの OS が緑になることを確かめる |
| REQ-009 | Testing strategy(移行前後の比較のあとに削除) | `git ls-files '*.py'` が空であることを確かめる |
| REQ-010 | Deployment and rollout | CI の changelog の確認と、リリースのワークフローが `v0.8.0` のリリースを作ることを確かめる |
| NFR-001 | Data model and state(BSD と GNU の切り替え); Reliability and failure handling | CI の 3 つの OS と、手元の bash 3.2 で bash のテストを実行する |
| NFR-002 | Technology choices | スクリプトとテストが呼ぶコマンドを見直す。CI に追加のツールのインストールがないことを確かめる |
| NFR-003 | System context; Data model and state(CRLF、最初の一致) | 移行前後の比較で、7 プロジェクトの判定が一致することを確かめる |

## Risks and tradeoffs

- Python 版と bash 版を並べて持つ期間は作らない。比べ終えたら、すぐに消す。食い違いを見落とすと気づきにくいので、比較は全フェーズで行う。
- bash のテストは JSON の構文を確かめられない。構文は Rust のテストで確かめる。Rust の CI が通らないと、このチェックも走らない。
- ボードの E2E テストはランチャーを通さなくなる。ランチャーの確認は、CI の「Launcher without Rust」のジョブと、手元で起動したときの確認に頼る。
- `SMARTPOWERS_NOW` はテストのための入り口で、利用者には案内しない。

## Open questions

なし。
