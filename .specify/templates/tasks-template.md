---

description: "Task list template for feature implementation"
---

# Tasks: [FEATURE NAME]

**Input**: Design documents from `/specs/[###-feature-name]/`
**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: 原則IVにより、ユニットテストとモック/録画フレームを用いた統合テストは必須。実機確認が必要な場合は `manual` ラベル付きタスクで明示する。

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- タスク記述には正確なファイルパスとCLIコマンド例を含めること

## Path Conventions

- `src/cli/` CLI定義(`clap`)
- `src/capture/` V4L2入出力
- `src/infrared/` フレーム後処理
- `tests/unit/`, `tests/integration/`, `tests/snapshots/`
- `docs/` 学習ノート・ガイド

## Phase 0: 憲章チェック

- [ ] T000 Constitution: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` の実行計画を確認
- [ ] T001 Constitution: V4L2互換性調査とテストデータ準備方針を確認

---

## Phase 1: Setup (Shared Infrastructure)

- [ ] T010 プロジェクト設定更新 (`Cargo.toml`, 依存クレート追加)
- [ ] T011 `src/config/` に設定バリデーション追加
- [ ] T012 [P] ロガー/構造化出力の基盤実装 (`tracing` 等)

---

## Phase 2: Foundational (Blocking Prerequisites)

- [ ] T020 `src/capture/` にV4L2デバイス検出ロジック実装
- [ ] T021 `tests/unit/` にデバイス情報モックテスト追加
- [ ] T022 `tests/integration/` に録画済みフレーム読み込みテスト追加
- [ ] T023 ドキュメントにデバイス前提条件を追記

**Checkpoint**: 原則I〜IVの基盤準備完了

---

## Phase 3: User Story 1 - [Title] (Priority: P1) 🎯 MVP

**Goal**: [Brief description of what this story delivers]

**Independent Test**: `cargo test --test [name]` またはCLIデモ

### Tests for User Story 1 ⚠️

- [ ] T030 [P] [US1] ユニットテスト追加 (`tests/unit/`)
- [ ] T031 [P] [US1] 統合テスト (録画フレーム利用)

### Implementation for User Story 1

- [ ] T032 [US1] CLIサブコマンド実装 (`src/cli/`)
- [ ] T033 [US1] キャプチャ制御実装 (`src/capture/`)
- [ ] T034 [US1] 出力フォーマット(JSON+人間可読)実装 (`src/cli/output.rs` 等)
- [ ] T035 [US1] ログと終了コード整備
- [ ] T036 [US1] docs/usage.md に使用例追記

**Checkpoint**: ストーリー単体でCLI実行→フレーム保存まで検証済み

---

## Phase 4: User Story 2 - [Title] (Priority: P2)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 2 ⚠️

- [ ] T040 [P] [US2] ユニット/統合テスト追加

### Implementation for User Story 2

- [ ] T041 [US2] 設定/構成ファイル対応
- [ ] T042 [US2] `--json` 拡張 or メタデータ記録更新
- [ ] T043 [US2] docs/ に学習ノート追記

---

## Phase 5: User Story 3 - [Title] (Priority: P3)

**Goal**: [Brief description of what this story delivers]

**Independent Test**: [How to verify this story works on its own]

### Tests for User Story 3 ⚠️

- [ ] T050 [P] [US3] テスト追加

### Implementation for User Story 3

- [ ] T051 [US3] 追加処理実装
- [ ] T052 [US3] ログ/エラーハンドリング調整
- [ ] T053 [US3] ドキュメント更新

---

## Phase N: Polish & Cross-Cutting Concerns

- [ ] T060 `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` の最終実行
- [ ] T061 生成物サイズと保存場所の確認
- [ ] T062 README, openspec/project.md, docs/ の更新
- [ ] T063 変更内容の学習メモを追記

---

## Dependencies & Execution Order

- Phase 0 完了後に初めてPlanを進行する
- Phase 2 完了前にユーザーストーリー着手不可
- 実機テストは`manual`ラベルで明示し、レビュー時に結果を記録

### Parallel Opportunities

- [P] マークのタスクは並行実行可能
- 異なるユーザーストーリーはFoundational完了後に並行可能

### 完了条件

- すべてのテストタスクがパスしていること
- 文書更新タスクが完了していること
- 憲章チェック項目に未完了がないこと
