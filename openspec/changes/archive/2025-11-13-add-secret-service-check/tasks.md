## Implementation
- [x] Add the `keyring` crate to `pam-chissu`, implement a `secret_service_available(user: &str) -> Result<bool, AuthError>` helper, and cover it with unit tests that mock keyring responses (success, locked/missing service) via trait abstraction or feature-gated shim.
- [x] Call the helper immediately after resolving configuration / determining the PAM target user; if it returns false or an error, log the reason, optionally notify via PAM conversation, and return `PAM_IGNORE` without touching the camera pipeline.
- [x] Ensure successful checks emit a debug/info log so operators can confirm the module verified Secret Service before attempting capture.
- [x] Update PAM documentation/config samples to explain the new guard, troubleshooting steps (ensure session bus and keyring daemon are running), and the expectation that the module may skip authentication when Secret Service is locked.
- [x] Run `CARGO_HOME="$(pwd)/.cargo-home" cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace`, and `cargo test -p pam-chissu`, capturing results for reviewers.
