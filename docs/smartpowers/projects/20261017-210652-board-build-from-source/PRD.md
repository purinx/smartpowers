# Product Requirements Document

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210652-board-build-from-source

## Summary

進捗ボードのサーバーを、ビルド済みバイナリを配らずに、各マシンでソースからビルドして動かす。起動はコマンド 1 つで済み、Rust がないマシンでは専用の場所にツールチェーンを自動で入れる。macOS、Linux、Windows のいずれでも動くことを CI で確かめる。

## Problem and outcome

- 中身の分からないバイナリがリポジトリに commit されていると、プラグインの利用者は安心して実行できない。
- ソースからビルドする方式にすると、今度は Rust の導入が利用の壁になる。そのうえ、導入のために利用者の環境(PATH、シェル設定、`~/.cargo`)を書き換えるのも望ましくない。
- 開発者の手元(macOS)以外の OS で動くかどうかを、確かめる手段がない。

これにより、利用者は読めるソースだけを受け取り、追加の準備なしにボードを起動でき、どの OS でも同じ手順で動くようになる。

## Target users

- Smartpowers プラグインをインストールして進捗ボードを使うユーザー(macOS、Linux、Windows)
- ボードを起動するエージェント(Claude Code、Codex)
- プラグインの開発者(変更が 3 つの OS で動くか確かめたい)

## Scope

### In scope

- ソースから初回ビルドして起動する起動スクリプト(Unix 用の `scripts/board`、Windows 用の `scripts/board.cmd`)
- cargo がないときの、専用ディレクトリへの Rust ツールチェーンの自動導入
- C リンカーがないときの案内
- 3 つの OS でのテストと、Rust がない状態からの初回起動を確かめる CI

### Out of scope

- ビルド済みバイナリの配布(GitHub Releases への添付を含む)
- システム全体への Rust やビルドツールのインストール(C リンカーの導入は利用者が行う)
- 32 ビット環境や ARM 版 Windows 向けのツールチェーンの用意
- ボードの機能そのもの(進捗ボードのプロジェクトで扱う)

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria |
| --- | --- | --- | --- |
| REQ-001 | Must | プラグインとリポジトリにビルド済みバイナリを含めない。ボードは各マシンでソースからビルドする。 | リポジトリの履歴に実行ファイルがない。ビルドの出力先 `board/target/` が `.gitignore` に入っている。 |
| REQ-002 | Must | 1 つの起動コマンドで初回ビルドから起動までが終わる。2 回目以降は前回のビルドを再利用し、ソースが変わったときだけビルドし直す。 | 初回は `Building the Smartpowers board (first run only)...` と表示してから起動する。2 回目以降は再ビルドせずに起動する。プラグインを更新してソースが変わると、自動でビルドし直される。 |
| REQ-003 | Must | PATH にも `~/.cargo/bin` にも cargo がなければ、rustup で専用ディレクトリに Rust ツールチェーンを導入する。PATH、シェル設定、`~/.cargo` は変更しない。 | Rust のない環境で起動すると、`~/.cache/smartpowers/rust`(Windows では `%LOCALAPPDATA%\smartpowers\rust`、`SMARTPOWERS_RUST_HOME` で変更可)にツールチェーンが入り、ボードが起動する。ホームディレクトリには `.cargo` もシェル設定の変更もない。 |
| REQ-004 | Should | rustup は入っているがシェル設定が PATH に反映されていない場合は、`~/.cargo/bin` の cargo を使う。 | `~/.cargo/bin/cargo` があり、PATH に含まれていない環境で、専用ツールチェーンを導入せずに起動する。 |
| REQ-005 | Must | Windows では、Visual Studio のビルドツールなしでビルドして起動できる。 | Rust を隠した Windows 環境で `board.cmd --help` が成功する。 |
| REQ-006 | Should | Unix で C リンカー(`cc`)がなければ、OS に合った導入方法を示して止まる。 | macOS では `xcode-select --install` を、そのほかでは `build-essential` や `gcc` を案内し、0 以外の終了コードで終わる。 |
| REQ-007 | Must | CI で macOS、Linux、Windows それぞれのビルドとテストを行い、Rust がない状態からの初回起動も確かめる。 | push と PR ごとに、3 つの OS のテストジョブと「Launcher without Rust」ジョブがすべて成功する。 |

## Non-functional requirements

| ID | Area | Requirement | Measure |
| --- | --- | --- | --- |
| NFR-001 | Dependencies | サーバーは Rust の標準ライブラリだけで書き、サードパーティの crate を使わない。ツールチェーンがあればオフラインでビルドできる。 | `Cargo.toml` に `[dependencies]` がなく、`Cargo.lock` にはこの crate しかない。 |
| NFR-002 | Footprint | 専用ツールチェーンの導入は 1 回限りで、削除すれば容量を取り戻せる。 | minimal プロファイルの安定版で約 450MB(macOS での実測値 456MB)。2 回目以降はダウンロードしない。ディレクトリを消せば元に戻る。 |
| NFR-003 | Performance | 2 回目以降の起動で、ビルド確認にかかるオーバーヘッドが 1 秒未満。 | macOS の実測で、ビルド済みの状態から `board --help` が終わるまで 0.07 秒。 |
| NFR-004 | Transparency | 自動導入やビルドを始めるときは、何をどこに入れるかを標準エラー出力に表示する。 | 導入のときに `cargo not found; installing a private Rust toolchain into <dir> (one time)...` が出力される。 |
| NFR-005 | Compatibility | Unix 用の起動スクリプトは、macOS 標準の bash 3.2 と POSIX sh で動く。 | `scripts/board` は `#!/bin/sh` で、bash 固有の構文を使っていない。 |

## User journeys

1. Rust の入っていない Mac でエージェントがボードを起動する。起動スクリプトが専用のツールチェーンを入れ(13 秒ほど)、ボードをビルドしてブラウザを開く。次回からはすぐに起動する。
2. Windows のユーザーが `board.cmd` を実行する。Visual Studio がなくても windows-gnu のツールチェーンでビルドされ、起動する。
3. Linux で `cc` がない場合は、`build-essential` か `gcc` を入れるよう案内が出て止まる。
4. プラグインを更新すると、次の起動で 1 度だけビルドし直される。

## Assumptions and constraints

- 自動導入には、rust-lang.org への HTTPS 接続が必要。Unix では `curl` を使い、Windows では PowerShell を使う。
- Unix でのビルドには、システムの C リンカーが必要。
- Windows の自動導入は x86_64 だけを対象にする。

## Success measures

- 3 つの OS で、テストと Rust なしでの初回起動の CI ジョブが継続して成功している。
- リポジトリの履歴に、ビルド済みバイナリが含まれていない。

## Risks

- rustup や rust-lang.org の仕様が変わると、自動導入が壊れる可能性がある。CI の初回起動ジョブで早めに気づけるようにする。
- 初回は約 450MB をダウンロードするので、回線が遅いと時間がかかる。

## Open questions

なし。
