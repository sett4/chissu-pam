1. [x] Add a `SYSLOG_IDENTIFIER` constant inside `crates/pam-chissu/src/lib.rs` (or dedicated module) and refactor `PamLogger` plus any other logging call sites to reference it.
2. [x] Introduce a unit test (or extend existing tests) that verifies the logger formatter uses `SYSLOG_IDENTIFIER`, preventing future drift.
3. [x] Update `openspec/changes/refactor-pam-logger-identifier/specs/pam-face-auth/spec.md` to capture the constant-based logging requirement.
4. [x] Run `CARGO_HOME="$(pwd)/.cargo-home" cargo fmt`, `CARGO_HOME="$(pwd)/.cargo-home" cargo clippy -- -D warnings`, and `CARGO_HOME="$(pwd)/.cargo-home" cargo test -p pam-chissu`, sharing their outputs in the PR.
