<!-- OPENSPEC:START -->

# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:

- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:

- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# chissu-pam Constitution

## Core Principles

### I. Rust 安全性を最優先

- Rust 安定版(1.80 以降)でのビルドを前提とし、`cargo fmt --check` と `cargo clippy -- -D warnings` を常時通過させること。
- `unsafe` ブロックは原則禁止とし、必要な場合は局所化・理由コメント・対応テスト(ユニットもしくは統合)を同一ディレクトリに追加すること。
  安全な Rust 実装を前提にすることで、学習目的のコードでもハードウェア制御時の予期せぬ挙動を防止する。

### II. V4L2 互換性検証を徹底

- すべてのキャプチャ処理は `v4l` クレートを用いた V4L2 デバイス制御を行い、開始前にデバイス機能・フォーマット・解像度を検証すること。
- CLI は赤外線取得に必要な設定(例: フィルタ切替、露光、ゲイン)を引数または設定ファイルで明示的に受け取り、サポート外の場合は即座にエラー終了すること。
  ハードウェア互換性を確保することで、学習環境ごとの差異による失敗を早期に発見できる。

### III. 観測可能な CLI 体験を提供

- すべてのコマンドは人間可読な標準出力と、`--json` 指定時の構造化出力(JSON)を提供し、エラーは標準エラーに記録すること。
- 重要イベント(デバイス選択、フォーマット決定、フレーム保存先)はログレベル付きで出力し、終了コードで成功/失敗を明示すること。
  可視化された出力により、利用者と自動テストの両方が結果を検証できる。

### IV. テスト可能なキャプチャフローを維持

- `cargo test` で実行可能なユニットテストと、モックまたは録画済みフレームを用いた統合テスト(ハードウェア不要)を必ず用意すること。
- 実機依存の確認項目(例: 赤外線強度チェック)はリポジトリ直下の `tests/` (integration) に手動フラグ付きテストを配置し、実行手順を README に明記すること。
  テスト容易性を重視し、学習過程でも機能回帰を防ぐ。

### V. 学習成果と知見の記録

- 新規機能や探索結果は `docs/` または `openspec/project.md` に手順・注意点・参考資料として追記すること。
- CLI 使用例、依存パラメータ、ハードウェア前提条件をリリースノートと README に反映し、再現可能な学習環境を維持すること。
  知見を共有することで、学習目的プロジェクトとしての価値を継続的に高められる。

## 運用・技術制約

- 対象 OS は V4L2 をサポートする Linux ディストリビューション(推奨: Debian/Ubuntu 系)とする。
- Edition 2024 をサポートした Rust 1.85 以上、主要クレートは `v4l`, `clap`, `serde`, `serde_json`, `image` を採用し、ライセンス互換性を確認する。
- IR 対応 Web カメラを前提とし、必要に応じて IR フィルタの有無や外部照射装置をドキュメント化する。
- 出力ファイルは既定で `./captures/` に保存し、1 ファイルあたり最大 500MB を超えないようローテーションまたは圧縮を検討する。

## ワークスペース構成

- ルートの `Cargo.toml` はワークスペース専用で、`[workspace.package]` に共通メタデータ(edition/version など)を定義する。
- CLI は `crates/chissu-cli/`、共有ロジックは `crates/chissu-face-core/`、PAM モジュールは `crates/pam-chissu/` に配置する。
- 各 crate にはローカル `tests/` ディレクトリを用意し、crate 固有のテストは `cargo test -p <crate>` で実行する。
- 複数 crate を跨ぐ統合テストとフィクスチャはリポジトリ直下の `tests/` に置き、`cargo test --workspace` で実行する。
- CLI の実行例やドキュメントは `cargo run -p chissu-cli -- <subcommand>` 形式を採用する。

## 開発ワークフローとレビュー

- cargo を実行する場合は `CARGO_HOME="$(pwd)/.cargo-home" cargo build` のように CARGO_HOME を追加してください。なぜならデフォルトではワークスペースの外だから書き込みできません。
- すべての変更は `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace` をローカルで通し、変更対象 crate については `cargo test -p chissu-cli` / `cargo test -p pam-chissu` / `cargo test -p chissu-face-core` を追加実行したうえでレビューを申請する。
- PAM モジュール(`pam-chissu`)に関わる変更では、レビュー前に必ず `cargo test -p pam-chissu` を実行して結果を共有する。
- PR は CLI 操作例(人間可読 + JSON)と、テストの実行結果ログ(必要ならスクリーンショット)を添付する。
- ハードウェア依存の変更はレビュワーが再現可能な手順(デバイス設定、照明条件、期待結果)を PR 説明に含める。
- ドキュメント更新(README, docs/, openspec project)はコード変更と同一 PR で行い、知見の欠落を防ぐ。

## Governance

- 本憲章は開発ガイドラインの最上位ドキュメントとし、競合する場合は本憲章を優先する。
- 憲章の改訂は提案+影響調査+テンプレート反映を含む PR で行い、メンテナの承認後にマージすること。
- 変更内容が原則の追加・削除・名称変更を含む場合はマイナー以上のバージョン更新を行う。
- 各 PR レビューでは本憲章の遵守チェックリストを確認し、違反がある場合は是正計画を添える。

**Version**: 1.0.0 | **Ratified**: 2025-10-26 | **Last Amended**: 2025-10-26
