1. Config loader updates
   - [x] Teach `src/config.rs` to deserialize `video_device`, `pixel_format`, and `warmup_frames` from the existing TOML file (primary + fallback path) while keeping the descriptor store resolution logic intact.
   - [x] Add targeted unit tests that cover CLI flag > config > built-in fallback ordering and error surfacing for parse/read failures.
2. Capture command wiring
   - [x] Update `CaptureArgs` -> `CaptureConfig` conversion so that missing `device`, `pixel_format`, or `warmup_frames` fields consult the new config helper before falling back to built-in defaults.
   - [x] Ensure JSON/human summaries continue to reflect the resolved values and add coverage in CLI-level tests or new unit tests for the conversion logic.
3. Documentation
   - [x] Extend README/docs to describe the new config keys, fallback order, and an example `config.toml` snippet.
   - [x] Note the behaviour change in the CHANGELOG or release notes (if present) so operators know that capture now honors config values.
4. Validation
   - [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` to confirm the change meets the repo's quality gates.
