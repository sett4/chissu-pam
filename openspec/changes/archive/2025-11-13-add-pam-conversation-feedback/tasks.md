## Implementation
- [x] Investigate current `pam_chissu` interaction with the PAM handle and add a safe wrapper for retrieving/invoking the conversation callback (gracefully handle null or failure cases).
- [x] Wire the wrapper into `pam_sm_authenticate` so each successful match sends a single `PAM_TEXT_INFO` message and each failure/retry path emits a `PAM_ERROR_MSG` that explains the reason or next action.
- [x] Add automated coverage (unit test with mocked `pam_conv` struct or integration harness) that asserts both message types are invoked with the right text under success and failure conditions.
- [x] Update PAM usage docs/README to describe the new interactive messages and how operators can expect them to appear in `login`/`sudo`.
- [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace`, and `cargo test -p pam-chissu`, capturing results for the change notes.
