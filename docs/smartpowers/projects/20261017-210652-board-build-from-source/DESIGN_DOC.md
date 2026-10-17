# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210652-board-build-from-source
PRD: ../PRD.md

## Design summary

ボードのサーバーは、標準ライブラリだけを使う Rust の crate(`skills/smartpowers/board/`)として配る。起動スクリプトを 2 つ用意する。Unix 用は POSIX sh の `scripts/board`、Windows 用は `scripts/board.cmd` である。どちらも、起動のたびに `cargo build --release --quiet` を実行してから、ビルドした実行ファイルを起動する。cargo が見つからなければ、rustup を使ってユーザー専用のディレクトリにツールチェーンを入れる。CI では 3 つの OS で、通常のテストに加えて、Rust を隠した状態からの初回起動を確かめる。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| サーバーの言語 | Rust(標準ライブラリのみ) | 1 つの実行ファイルにまとまり、依存がないのでオフラインでもビルドできる。ユーザーの要望に沿う。 | Python の標準ライブラリによるサーバー(最初の版。ユーザーの要望で Rust に置き換えた) |
| 配布の方法 | ソースだけを配り、各マシンでビルドする | 利用者は読めるソースだけを受け取り、実行するものを確かめられる。 | ビルド済みバイナリの commit(中身が分からず不安。いったん試したが、commit する前にやめた)、GitHub Releases へのバイナリ添付(結局、中身の分からないバイナリを配ることになる) |
| UI の実装 | ビルド工程のない素の JS(ES Modules) | ブラウザが読むコードが、そのまま人が読むコードになる。 | Rust から WASM にコンパイル(バイナリが再び増え、wasm-bindgen などのビルド工程も必要になるので却下) |
| Unix の起動スクリプト | POSIX sh | macOS 標準の bash 3.2 を含め、どこでも動く。 | bash 4 の機能を使う(macOS で動かない) |
| Windows の起動スクリプト | `.cmd`(CRLF) | 追加のシェルなしで動く。 | PowerShell スクリプト(実行ポリシーの制約がある) |
| ツールチェーンの導入 | rustup の `--no-modify-path --profile minimal`、専用の `RUSTUP_HOME` と `CARGO_HOME` | 利用者の環境を変えず、消せば元に戻る。 | 利用者に Rust の導入を任せる(利用の壁になる)、システム全体に入れる(環境を変えてしまう) |
| Windows のツールチェーン | `x86_64-pc-windows-gnu` を既定のホストにする | ツールチェーンにリンカーが含まれるので、Visual Studio のビルドツールが要らない。 | msvc(`link.exe` を入れるために VS Build Tools が必要) |

## System context

プラグインの中で関係する場所は次のとおり:

- `skills/smartpowers/board/`: crate のソース(`Cargo.toml`、`Cargo.lock`、`src/`)。ビルドの出力は `board/target/` に置き、`.gitignore` で除外する。
- `skills/smartpowers/assets/board/`: UI のファイル。`include_str!` で実行ファイルに埋め込む。
- `skills/smartpowers/scripts/board`、`board.cmd`: 起動スクリプト。

`.gitattributes` で、`*.cmd` を CRLF に、`scripts/board` を LF に固定する。どの OS でチェックアウトしても、改行の違いで起動スクリプトが壊れないようにするためである。SKILL.md は、エージェントがこの起動スクリプトを使うものとして書かれている。

## Architecture and component boundaries

- **`scripts/board`(POSIX sh)**:
  1. PATH に `cargo` があればそれを使う。
  2. なければ `~/.cargo/bin/cargo` を使う。
  3. それもなければ、`install_private_toolchain` で `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y -q --no-modify-path --profile minimal --default-toolchain stable` を実行する。`curl` がなければ止める。
  4. `cc` がなければ OS 別の案内を出して止める。
  5. ビルド済みでなければ「Building ... (first run only)」と表示し、`cargo build --release --quiet` のあと、`exec` で実行ファイルを起動する。
- **`scripts/board.cmd`**:
  1. `where cargo` で探し、なければ `%USERPROFILE%\.cargo\bin\cargo.exe` を使う。
  2. それもなければ `:private_toolchain` に進む。PowerShell の `Invoke-WebRequest` で `rustup-init.exe` をダウンロードし、`--default-host x86_64-pc-windows-gnu` を付けて実行する。
  3. ビルドして実行ファイルを起動し、終了コードを呼び出し元に返す。
- **CI(`.github/workflows/ci.yml`)**: `test` ジョブ(3 つの OS で rustfmt(Linux のみ)、clippy(`-D warnings`)、`cargo test`、Python のテスト)と、`bootstrap` ジョブ(3 つの OS で Rust を隠して初回起動)の 2 つ。

## Data model and state

| 状態 | 場所 | 消えたときの扱い |
| --- | --- | --- |
| ビルドの出力 | `<plugin>/skills/smartpowers/board/target/release/` | 次の起動でビルドし直す |
| 専用ツールチェーン | `${SMARTPOWERS_RUST_HOME:-${XDG_CACHE_HOME:-$HOME/.cache}/smartpowers/rust}`(Windows では `%SMARTPOWERS_RUST_HOME%` または `%LOCALAPPDATA%\smartpowers\rust`)。その下に `rustup/` と `cargo/` | 次の起動で入れ直す |

## Interfaces and integrations

- 起動スクリプトの引数は、すべてそのまま実行ファイルに渡す(`"$@"`、`%*`)。
- 環境変数 `SMARTPOWERS_RUST_HOME` で、専用ツールチェーンの置き場所を変えられる。
- 外部への接続は `https://sh.rustup.rs`(Unix)と `https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe`(Windows)。導入の 1 回だけ使う。

## Critical flows

1. **初回起動(Rust なし)**: cargo を探して見つからない → 専用ツールチェーンを導入(stderr に説明を出す)→ ビルド → 起動。macOS での実測では約 13 秒、456MB だった。
2. **2 回目以降**: cargo を見つける → `cargo build` はビルドが最新なので何もしない → 起動。オーバーヘッドは約 0.07 秒。
3. **プラグインの更新後**: ソースが変わっているので `cargo build` が 1 度だけビルドし直す。
4. **Windows での導入**: rustup は、自分のファイル名が `rustup-init` で始まるときだけインストーラーとして動く。そのため一意な名前の一時ディレクトリを作り、その中に `rustup-init.exe` という名前で保存して実行し、終わったらディレクトリごと消す。

## Security and privacy

- 実行するのは、読めるソースからその場でビルドしたものだけにする(REQ-001)。
- ダウンロードには HTTPS だけを使い、Unix では `--proto '=https' --tlsv1.2` で古い TLS への格下げを防ぐ。
- 利用者の PATH、シェル設定、`~/.cargo` は変更しない(`--no-modify-path` と専用の HOME)。

## Reliability and failure handling

- `curl` も C リンカーもない、ダウンロードに失敗した、導入に失敗した、のいずれの場合も、理由と https://rustup.rs への案内を出して 0 以外で終わる。
- ビルドに失敗した場合は、cargo のエラーを表示して止まる(`set -eu`、`if errorlevel 1 exit /b 1`)。
- Windows では、導入に失敗しても一時ディレクトリを消す。

## Observability and operations

導入とビルドの進み具合は標準エラー出力に出す。ボードの URL を出力する標準出力(`Smartpowers board: http://...`)は汚さない。導入したツールチェーンを消したいときは、そのディレクトリを削除すればよい。

## Testing strategy

- **CI の test ジョブ**: macOS、Linux、Windows それぞれで、clippy と cargo test、それに起動スクリプト経由の Python のテスト(`tests/test_board.py`。Windows では `board.cmd`)を実行する。
- **CI の bootstrap ジョブ**: PATH から `.cargo/bin` を取り除き、HOME(Windows では USERPROFILE)と `SMARTPOWERS_RUST_HOME` を一時ディレクトリに向けて `--help` を実行する。そのうえで、専用ツールチェーンが入ったこと、`~/.cargo` が作られていないことを確かめる。
- **手元での確認**: macOS で、一時的な HOME と cargo のない PATH を使って初回の導入と 2 回目の起動を測り、ホームディレクトリに `.cache/smartpowers` しか作られないことを確かめた。

## Deployment and rollout

0.3.0 で、ソースからビルドする方式の起動スクリプトとともにボードを出した。0.4.0 で専用ツールチェーンの自動導入と、その Windows 向け修正(インストーラーのファイル名)を加えた。CI(2794341)は 0.3.0 のときに追加し、Rust なしでの初回起動のジョブは自動導入と同時に追加した。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Technology choices(配布の方法); System context; Security and privacy | 履歴と `.gitignore` を確認する。リポジトリに実行ファイルがないこと |
| REQ-002 | Architecture and component boundaries(起動スクリプト); Critical flows 1〜3 | CI の Python テストが起動スクリプト経由で起動すること。2 回目以降に再ビルドされないことを手元で確かめる |
| REQ-003 | Architecture and component boundaries; Data model and state; Critical flows 1 | CI の bootstrap ジョブで、専用ツールチェーンが入り、`~/.cargo` がないことを確かめる |
| REQ-004 | Architecture and component boundaries(cargo の探し方) | `~/.cargo/bin` だけに cargo がある環境(このリポジトリの開発環境)で起動を確かめる |
| REQ-005 | Technology choices(Windows のツールチェーン); Critical flows 4 | CI の Windows の bootstrap ジョブが成功すること |
| REQ-006 | Architecture and component boundaries(`cc` の確認); Reliability and failure handling | 起動スクリプトの分岐をレビューする |
| REQ-007 | Architecture and component boundaries(CI); Testing strategy | push と PR ごとに CI の 6 ジョブが成功すること |
| NFR-001 | Technology choices(サーバーの言語) | `Cargo.toml` に依存がないこと。CI の 3 つの OS でビルドが通ること |
| NFR-002 | Data model and state; Critical flows 1 | 手元で導入したときのサイズを測る(456MB) |
| NFR-003 | Critical flows 2 | 手元で 2 回目の起動時間を測る(0.07 秒) |
| NFR-004 | Observability and operations | 導入時に出るメッセージを CI のログで確かめる |
| NFR-005 | Technology choices(Unix の起動スクリプト) | `#!/bin/sh` で書いていることをレビューする。macOS の CI で実行されること |

## Risks and tradeoffs

- 利用者の手元でビルドするので、初回は時間がかかり、C リンカーも必要になる。その代わり、中身の分からないバイナリを配らずに済む。
- `cargo build` を毎回呼ぶので、起動のたびに少しオーバーヘッドがある(約 0.07 秒)。その代わり、プラグインを更新したときにビルドし直し忘れることがない。
- Windows の専用ツールチェーンを gnu にしたので、利用者がすでに msvc で入れている Rust とは別物になる。利用者自身の cargo がある場合はそちらを優先する。

## Open questions

なし。
