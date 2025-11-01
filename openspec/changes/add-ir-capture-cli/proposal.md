## Why
- Provide a single-command workflow for capturing a still infrared frame from a V4L2-compatible webcam so learners can validate hardware setup quickly.
- Establish baseline interfaces (human readable + `--json`) required by the study-rust-v4l2 constitution for future capture features.
- Ensure device capability checks and error surfacing are explicit so unexpected hardware differences can be diagnosed without guesswork.

## What Changes
- Introduce a `study-rust-v4l2` CLI that negotiates an infrared-capable format with a selected video device, captures one frame, and persists it under `./captures/`.
- Add configuration flags for device selection, format override, exposure/gain tuning, output filename, and `--json` structured output.
- Implement compatibility verification (query device capabilities, supported formats/resolutions) before capture, failing fast when unsupported.
- Emit structured logging covering device choice, negotiated format, capture path, and any recoverable fallbacks.
- Provide tests using mocked frame sources or recorded sample data so the capture flow is verifiable without hardware, plus documentation updates with usage examples and manual test guidance.

## Impact
- New Rust binary crate (or binary target) introducing dependencies including `clap`, `v4l`, `serde`, `serde_json`, and `image`.
- Adds integration test assets (sample frame file or mock adapter) increasing repository size modestly; ensure assets stay << 500MB per the constitution.
- Requires updates to README and docs to describe CLI usage, JSON schema, and manual hardware validation steps.
- Establishes groundwork for future multi-frame capture or video streaming features by defining initial logging and output conventions.
