# study-rust-v4l2 Development Guidelines

Auto-generated from all feature plans. Last updated: [DATE]

## Active Technologies

- Rust 1.80+ (stable)
- `v4l`, `clap`, `serde`, `serde_json`, `image`

## Project Structure

```text
src/
├── cli/
├── capture/
├── infrared/
├── config/
└── utils/

tests/
├── unit/
├── integration/
└── snapshots/

docs/
└── guides/
```

## Commands

- `cargo fmt`
- `cargo clippy -- -D warnings`
- `cargo test`
- `cargo run -- list`
- `cargo run -- capture --device /dev/video0 --json`

## Code Style

- `cargo fmt --check` を必須化
- `cargo clippy -- -D warnings` で警告ゼロ
- `unsafe` 使用時は理由コメントとテストを同梱

## Recent Changes

[LAST 3 FEATURES AND WHAT THEY ADDED]

<!-- MANUAL ADDITIONS START -->
- V4L2デバイス互換性情報とIR撮影ノウハウをdocs/に集約すること
<!-- MANUAL ADDITIONS END -->
