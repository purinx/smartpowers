# Product Requirements Document

Status: Approved
Approved: 2026-10-18 by user
Last updated: 2026-10-17
Project: 20261017-210655-release-automation

## Summary

Smartpowers のリリースごとに `vX.Y.Z` タグと GitHub リリースを残し、リリースノートをリポジトリの `CHANGELOG.md` で管理する。宣言されたバージョンとタグが食い違っていたり、ノートがなかったりした場合は、公開や CI を止める。

## Problem and outcome

- 0.3.0〜0.6.0 はバージョンを上げても、タグもリリースノートも残っていなかった。そのため、利用者はどの版で何が変わったかを追えなかった。
- 0.4.0 より前は、バージョンの上げ忘れや食い違い(プラグイン設定とボードの crate の不一致など)を検出する仕組みがなかった。

これにより、リリースはタグ、GitHub リリース、CHANGELOG の 3 つで一貫して記録され、ノートの書き忘れやバージョンの不一致は公開前に機械的に検出される。

## Target users

- Smartpowers の利用者(GitHub リリースで変更点を確認する)
- Smartpowers の開発者とエージェント(リリース手順を実行する)

## Scope

### In scope

- `CHANGELOG.md` の書式と、0.1.0〜0.7.0 の記載
- タグの push を受けて GitHub リリースを作成するワークフロー
- 宣言されたバージョンの一致検査とノートの抽出を行うスクリプト
- CI でのノートの有無の検査と、テストでのバージョン一致の検査
- README の Releasing 節

### Out of scope

- バイナリなどリリース成果物の添付(ボードは各マシンでソースからビルドする)
- 0.6.0 以前のタグとリリースのさかのぼった作成(Open questions を参照)
- バージョンを上げるタイミングの自動判定

## Functional requirements

| ID | Priority | Requirement | Acceptance criteria |
| --- | --- | --- | --- |
| REQ-001 | Must | リリースごとに `vX.Y.Z` タグを付け、タグの push で GitHub リリースを公開する。 | `v0.7.0` を push すると Release ワークフローが成功し、「Smartpowers 0.7.0」のリリースが作られる。 |
| REQ-002 | Must | リリースノートは `CHANGELOG.md` の `## [X.Y.Z] - YYYY-MM-DD` 節に書き、バージョンを上げる commit と同じ commit で Crit のレビューを受ける。 | GitHub リリースの本文が、該当する CHANGELOG の節の本文と一致する。 |
| REQ-003 | Must | タグのバージョンが、プラグイン設定 2 か所とボードの `Cargo.toml` の宣言と一致しない場合はリリースを公開しない。 | 一致しないタグを渡すとスクリプトがエラーで終了し、食い違っているファイルと値を表示する。リリースは作られない。 |
| REQ-004 | Must | 現在の宣言バージョンに対応する CHANGELOG の節がなければ CI が失敗する。 | 節がないか本文が空の状態で push すると、CI の「Release notes exist for the current version」ステップが失敗する。 |
| REQ-005 | Must | プラグイン設定のバージョンと、ボードの crate の `Cargo.toml` と `Cargo.lock` のバージョンが一致する。 | 一致しないとテスト `test_board_crate_version_matches_plugin` が失敗する。 |
| REQ-006 | Should | リリース手順を README に書き、ノートをローカルで確認できるようにする。 | README の Releasing 節に手順があり、`scripts/release-notes.sh --tag vX.Y.Z` でノートが表示される。 |

## Non-functional requirements

| ID | Area | Requirement | Measure |
| --- | --- | --- | --- |
| NFR-001 | Tooling | リリース用のスクリプトは bash か Rust で書く。Python は使わない。 | `scripts/release-notes.sh` は bash で書かれている。既存の Python テスト(`tests/`)はこのルールより前からあるため対象外とする。 |
| NFR-002 | Review | タグを push するのは、Crit の push 承認を受けた commit だけにする。 | タグが指す commit は `main` に push 済みで、承認済みの範囲に含まれている。 |
| NFR-003 | Compatibility | スクリプトは macOS 標準の bash 3.2 と、Linux の CI(ubuntu-latest)で動く。 | bash 3.2 でも Linux の CI でも、同じ入力で同じ出力とエラーになる。 |

## User journeys

1. 開発者が機能を加え、「Release X.Y.Z」の commit でバージョンを上げ、CHANGELOG の節を書く。Crit で承認を受けて `main` に push する。
2. 同じ commit に `vX.Y.Z` タグを付けて push する。Release ワークフローが検査に通り、GitHub リリースを公開する。
3. 利用者が GitHub の Releases ページで変更点を確認し、`plugin update` で更新する。

## Assumptions and constraints

- GitHub Actions を使い、`GITHUB_TOKEN` に `contents: write` 権限を与える。
- バージョンは Semantic Versioning に従う。
- タグを push してもリリースを公開するまでは外部に変化はないが、push そのものはユーザーの承認範囲に含める(NFR-002)。

## Success measures

- 0.7.0 以降のすべてのバージョンに、タグ、GitHub リリース、CHANGELOG の節がそろっている。
- バージョンの不一致やノートの書き忘れが、公開前に CI か Release ワークフローで検出される。

## Risks

- Release ワークフローは CI の結果を待たない。タグを付ける commit は、`main` の CI が通っていることを手順で確認する必要がある。
- タグを付け直したり消したりすると、利用者の参照と食い違うことがある。

## Open questions

- 0.6.0 以前のバージョン(0.1.0〜0.6.0)にも、さかのぼってタグと GitHub リリースを作るか。作る場合、当時の commit には CHANGELOG がないので、ワークフローを使わず `gh release create` で個別に作る必要がある。
