## Implementation
- [x] Survey existing face descriptor utilities and identify reusable comparison logic for PAM path.
- [x] Define configuration loader that searches `/etc/chissu-pam/config.toml` then `/usr/local/etc/chissu-pam/config.toml`, supporting threshold, timeout, store directory, and video device keys with documented defaults.
- [x] Implement Rust PAM module crate producing `libpam_chissuauth.so` that wires configuration, camera capture, descriptor extraction, and cosine-similarity matching for `pam_sm_authenticate`.
- [x] Add syslog logging pathway for notable events (start, captured frame count, match result, errors) respecting PAM conventions.
- [x] Provide integration or unit tests that mock feature stores and descriptor comparisons, plus documentation covering installation, configuration, and operational caveats.
- [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, and record results in the change notes.
