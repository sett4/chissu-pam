# Implementation Plan: [FEATURE]

**Branch**: `[###-feature-name]` | **Date**: [DATE] | **Spec**: [link]
**Input**: Feature specification from `/specs/[###-feature-name]/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/commands/plan.md` for the execution workflow.

## Summary

[Extract from feature spec: primary requirement + technical approach from research]

## Technical Context

**Language/Version**: Rust 1.80+ (stable)
**Primary Dependencies**: `v4l`, `clap`, `serde`, `serde_json`, `image`
**Storage**: N/A (ローカルファイル保存のみ)
**Testing**: `cargo test` (必須), `cargo nextest` (任意)
**Target Platform**: Linux (x86_64) with V4L2 デバイス
**Project Type**: CLIツール (単一バイナリ)
**Performance Goals**: 30fps相当のフレーム取得、1回のキャプチャ実行時間は60秒以内
**Constraints**: 出力ファイルは500MB未満、`cargo clippy -- -D warnings` を常時通過
**Scale/Scope**: 単一リポジトリ・単一CLI、学習用ツール

必要に応じて実際の作業内容で数値や制約を上書きしてください。

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [ ] 原則I: `cargo fmt --check` / `cargo clippy -- -D warnings` をCIとローカルの両方で実行計画に含めた
- [ ] 原則II: 対応デバイスのV4L2機能検証(フォーマット・解像度)を設計に組み込んだ
- [ ] 原則III: 人間可読出力と `--json` 出力設計、エラーハンドリングと終了コードを定義した
- [ ] 原則IV: モック/録画済みフレームを用いたテスト戦略と実機テスト手順を書面化した
- [ ] 原則V: READMEやdocsへの知見反映タスクをPlan内に追加した

## Project Structure

### Documentation (this feature)

```text
specs/[###-feature]/
├── plan.md              # This file (/speckit.plan command output)
├── research.md          # Phase 0 output (/speckit.plan command)
├── data-model.md        # Phase 1 output (/speckit.plan command)
├── quickstart.md        # Phase 1 output (/speckit.plan command)
├── contracts/           # Phase 1 output (/speckit.plan command)
└── tasks.md             # Phase 2 output (/speckit.tasks command - NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
├── cli/            # clapベースのCLI定義
├── capture/        # V4L2デバイスとのやり取り
├── infrared/       # フレーム処理・フィルタ
├── config/         # 設定読み込みと検証
└── utils/          # 共通ヘルパ

tests/
├── integration/    # モック/録画データを用いた統合テスト
├── snapshots/      # 期待されるIRフレームのサンプル
└── unit/           # ピュアロジックのユニットテスト

docs/
└── guides/         # 学習ノート・手順

captures/           # 出力先(リポジトリには含めない)
```

**Structure Decision**: 単一CLI構成を維持し、`src/` 以下をコンポーネント単位で分割する。テストは`tests/` 以下で種類別に管理し、`docs/` に知見を追記する。

## Complexity Tracking

> **Fill ONLY if Constitution Check has violations that must be justified**

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| [e.g., 追加のcrate導入] | [current need] | [why現状構成では不足] |
| [e.g., unsafe使用] | [specific problem] | [safe実装では性能/互換性が不足] |
