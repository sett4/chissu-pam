## Implementation Tasks
- [ ] Review existing Secret Service helper result handling in `crates/pam-chissu/src/lib.rs` (conversation + syslog flows).
- [ ] Update the PAM conversation call for helper-driven `PAM_IGNORE` responses to send a fixed short message without internal reason details.
- [ ] Keep syslog logging unchanged and confirm helper reason still appears there.
- [ ] Add or update unit tests (or helper-focused tests) to verify the conversation text and syslog logging behavior when Secret Service is unavailable.
- [ ] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test -p pam-chissu`.
