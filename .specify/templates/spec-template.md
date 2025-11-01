# Feature Specification: [FEATURE NAME]

**Feature Branch**: `[###-feature-name]`  
**Created**: [DATE]  
**Status**: Draft  
**Input**: User description: "$ARGUMENTS"

## User Scenarios & Testing *(mandatory)*

> すべてのストーリーはCLI操作(人間可読出力)と `--json` オプションの両方で検証可能であること。V4L2デバイスがない環境向けに録画済みフレームでの検証手段も記述すること。

### User Story 1 - [Brief Title] (Priority: P1)

[Describe this user journey in plain language]

**Why this priority**: [Explain the value and why it has this priority level]

**Independent Test**: `cargo test` または手動CLI手順で単独検証できる方法を記載

**Acceptance Scenarios**:

1. **Given** `/dev/video[ID]` が利用可能, **When** CLIで [action], **Then** 期待のIRフレームが `captures/` に保存される
2. **Given** `--json` オプション, **When** CLIで [action], **Then** JSON出力に [expected field] が含まれる

---

### User Story 2 - [Brief Title] (Priority: P2)

[Describe this user journey in plain language]

**Why this priority**: [Explain the value and why it has this priority level]

**Independent Test**: モック/録画フレームを使ったテスト、もしくは限定的な実機検証の手順

**Acceptance Scenarios**:

1. **Given** [initial state], **When** [action], **Then** [expected outcome]

---

### User Story 3 - [Brief Title] (Priority: P3)

[Describe this user journey in plain language]

**Why this priority**: [Explain the value and why it has this priority level]

**Independent Test**: [Describe how this can be tested independently]

**Acceptance Scenarios**:

1. **Given** [initial state], **When** [action], **Then** [expected outcome]

---

[Add more user stories as needed, each with an assigned priority]

### Edge Cases

- デバイスが接続されていない場合のエラー表示と終了コード
- 対応していないピクセルフォーマットを要求した場合の処理
- 長時間キャプチャでの保存容量上限やローテーション

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: CLI MUST 検出可能なV4L2デバイス一覧を提供する (`list` コマンド等)
- **FR-002**: CLI MUST 指定デバイスから赤外線フレームを取得し `captures/` に保存する
- **FR-003**: CLI MUST `--json` フラグ時に構造化したメタデータを標準出力する
- **FR-004**: システム MUST 取得したフレームに選択した露光/ゲインなどの設定値をメタデータとして記録する
- **FR-005**: CLI MUST ハードウェア依存の失敗時に非0終了コードと明確なエラーメッセージを返す

未確定事項は `NEEDS CLARIFICATION` を明示して追加し、後続タスクで解決すること。

### Key Entities *(include if feature involves data)*

- **CaptureSession**: デバイスID、フォーマット、露光設定、保存先パス
- **FrameMetadata**: タイムスタンプ、露光/ゲイン、平均IR輝度、保存ファイル名

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: CLIで `--json` 実行時に正規化されたJSONスキーマを返し、`jq` で検証可能
- **SC-002**: 30秒以内に10フレーム以上のIR画像を保存できる
- **SC-003**: 録画済みモックデータでの統合テストが `cargo test` で成功する
- **SC-004**: README/ドキュメントに新しい使用例と学習ノートが追記されレビューで承認される
