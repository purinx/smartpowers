# First Build Design

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210655-release-automation
PRD: ../PRD.md

## Design summary

`CHANGELOG.md` を、リリースノートの唯一の正とする。タグを push すると `.github/workflows/release.yml` が `scripts/release-notes.sh` を実行し、宣言されたバージョンの一致を検査したうえで該当する節を抜き出し、`gh release create` で公開する。CI では同じスクリプトを `--current` で実行してノートの有無を検査し、Python テストでボードの crate とのバージョン一致を検査する。

## Technology choices

| Area | Choice | Rationale | Alternatives considered |
| --- | --- | --- | --- |
| ノートの置き場所 | リポジトリの `CHANGELOG.md`(Keep a Changelog 風) | バージョンを上げる commit と一緒に Crit でレビューできる。 | GitHub の自動生成ノート(レビューを経ない)、リリース画面での手書き |
| 検査と抽出 | `scripts/release-notes.sh`(bash、sed と awk) | 依存がなく、CI と手元の両方で動く。 | `scripts/release_notes.py`(Python は使わない方針により置き換えた) |
| 公開 | GitHub Actions + `gh release create --verify-tag` | ランナーに標準で入っている `gh` だけで済む。タグが存在しない場合は失敗する。 | サードパーティのリリース用アクション(依存が増える) |
| crate のバージョン一致 | `tests/test_marketplace.py` のテスト | 既存のマニフェスト一致テストと同じ場所にまとめられる。 | スクリプトによる検査に統合(このルールより前に実装済み) |

## System context

既存の CI(`ci.yml`: Rust と Python のテストを 3 つの OS で実行)とプラグインのマニフェストを前提にする。バージョンが宣言されているのは `plugins/smartpowers/.claude-plugin/plugin.json`、`plugins/smartpowers/.codex-plugin/plugin.json`、`plugins/smartpowers/skills/smartpowers/board/Cargo.toml`(と、それに追従する `Cargo.lock`)の 3 か所。プラグインのコードは変えない。

## Architecture and component boundaries

- **`CHANGELOG.md`**: バージョンごとに `## [X.Y.Z] - YYYY-MM-DD` の節を置き、その下に Added / Changed / Fixed / Security などの小見出しを書く。0.1.0〜0.7.0 は git の履歴から書き起こした。
- **`scripts/release-notes.sh`**: 入力は `--tag vX.Y.Z` または `--current`。`--output` を指定するとファイルに書き出す。
  1. タグの形式(`^v[0-9]+\.[0-9]+\.[0-9]+$`)を検査する。
  2. 宣言されている 3 か所のバージョンを sed で取り出し、一致しないものをすべて挙げて失敗する。
  3. awk で `## [X.Y.Z]` から次の `## [` までを取り出し、前後の空行を除く。節がないか本文が空なら失敗する。
- **`.github/workflows/release.yml`**: `push: tags: ["v*"]` で起動し、`contents: write` 権限で `ubuntu-latest` 上で実行する。スクリプトでノートを作り、`gh release create "$GITHUB_REF_NAME" --title "Smartpowers X.Y.Z" --notes-file ... --verify-tag` を実行する。
- **`.github/workflows/ci.yml`**: Linux のジョブで `scripts/release-notes.sh --current > /dev/null` を実行する。
- **`tests/test_marketplace.py`**: `test_board_crate_version_matches_plugin` で、`Cargo.toml` の `version = "..."` と、`Cargo.lock` の `smartpowers-board` パッケージのバージョンが、プラグインのバージョンと一致することを検査する。
- **README › Releasing**: 3 つの手順(「Release X.Y.Z」の commit、`main` への push、タグの作成と push)と、ローカルでのプレビュー方法。

## Data model and state

- タグは `vX.Y.Z` の注釈付きタグ(`git tag -a vX.Y.Z -m "Smartpowers X.Y.Z"`)とする。
- GitHub リリースのタイトルは「Smartpowers X.Y.Z」、本文は CHANGELOG の節の本文。
- ワークフローは、タグが指す commit の中の `CHANGELOG.md` を読む。

## Interfaces and integrations

- `scripts/release-notes.sh [--tag vX.Y.Z | --current] [--output FILE]`: 成功すると終了コード 0 でノートを出力し、失敗すると `ERROR: ...` を標準エラー出力に出して終了コード 1 を返す。
- GitHub Actions: `GITHUB_REF_NAME`、`RUNNER_TEMP`、`github.token`
- GitHub CLI: `gh release create`

## Critical flows

1. **リリースの準備**: バージョンを 3 か所で上げ、`cargo build` で `Cargo.lock` を更新し、CHANGELOG の節を追加して「Release X.Y.Z」として commit する。CI が通り、Crit の承認を受けたら `main` に push する。
2. **公開**: 承認済みの commit にタグを付けて push すると、Release ワークフローが検査、抽出、公開を行う。エージェントはワークフローの成功とリリースの URL を確認する。
3. **検査で失敗した場合**: タグとバージョンが一致しないか、ノートがなければ、ワークフローは失敗しリリースは作られない。原因を直した commit に、改めてタグを付ける。

## Security and privacy

- ワークフローの権限は `contents: write` だけにする。
- タグを push するのは、Crit の push 承認を受けた commit だけにする(NFR-002)。リリース commit の承認には、同じ commit へのタグの push も含まれるものとして扱う。

## Reliability and failure handling

- スクリプトは `set -euo pipefail` で実行し、検査の失敗はすべて理由を表示して終了する。
- `--verify-tag` により、タグが存在しない場合は公開しない。
- Release ワークフローは CI の完了を待たないため、タグは `main` の CI が通った commit にだけ付ける。

## Observability and operations

Release ワークフローの実行履歴と、GitHub の Releases ページが記録になる。エージェントは公開後にワークフローの結果とリリースの URL を報告する。

## Testing strategy

- 手元で確認した内容:
  - 0.7.0 のノートの抽出(標準出力とファイル)
  - 架空のリポジトリでの、途中の節(0.3.0)と最後の節(0.1.0)の抽出
  - 存在しない節、形式の誤ったタグ、バージョンの不一致、引数の不足、未知のオプションでの失敗
- CI: `--current` の実行と、`test_board_crate_version_matches_plugin`
- 実際の公開: `v0.7.0` で Release ワークフローが成功し、リリースの本文が CHANGELOG と一致することを確認した。

## Deployment and rollout

d962bcb で導入した。`v0.7.0` タグはこの commit を指し、最初の GitHub リリースになった(バージョンの宣言はこの時点でも 0.7.0)。以後のリリースはすべてこの手順に従う。

## Requirement traceability

| Requirement | Design sections | Verification approach |
| --- | --- | --- |
| REQ-001 | Architecture and component boundaries(release.yml); Critical flows(公開) | `v0.7.0` の push で Release ワークフローが成功し、リリースが作られたことを確認した |
| REQ-002 | Architecture and component boundaries(CHANGELOG.md、release-notes.sh); Data model and state | リリースの本文と CHANGELOG の 0.7.0 の節を比べた |
| REQ-003 | Architecture and component boundaries(release-notes.sh); Reliability and failure handling | 手元で一致しないタグを渡し、エラーと終了コード 1 を確認した |
| REQ-004 | Architecture and component boundaries(ci.yml) | CI のステップ「Release notes exist for the current version」と、架空のリポジトリでの節がない場合の失敗 |
| REQ-005 | Architecture and component boundaries(test_marketplace.py) | `test_board_crate_version_matches_plugin`(意図的にバージョンをずらして失敗することを確認済み) |
| REQ-006 | Architecture and component boundaries(README › Releasing); Interfaces and integrations | README の手順を確認し、`scripts/release-notes.sh --tag vX.Y.Z` でノートが表示されることを確認した |
| NFR-001 | Technology choices | `scripts/` に Python のスクリプトがないことを確認する |
| NFR-002 | Security and privacy; Critical flows(公開) | タグが Crit の承認を経て push 済みの commit を指していることを確認する |
| NFR-003 | Architecture and component boundaries(release-notes.sh); Testing strategy | macOS(bash 3.2)と CI(ubuntu-latest)で実行して確認した |

## Risks and tradeoffs

- ノートを手書きにしたので、書く手間はかかる。その代わりにレビューを経た記述になり、CI で書き忘れを防げる。
- 宣言されているバージョンの場所が増えた場合は、スクリプトとテストの両方を更新する必要がある。
- crate のバージョン一致の検査はまだ Python のテストにある。bash への移行は未決(既存の Python ファイルの扱いと合わせて判断する)。

## Open questions

- 0.6.0 以前のバージョンに、さかのぼってタグとリリースを作るか(PRD の Open questions と同じ)。
