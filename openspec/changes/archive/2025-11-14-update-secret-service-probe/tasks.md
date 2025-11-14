1. [x] Implement a helper abstraction that forks, switches credentials to the PAM target user, and communicates via JSON over a pipe/socketpair.
2. [x] Extend `chissu-face-core` secret service utilities to return AES-GCM descriptor keys plus typed error variants for missing keys and unavailable services.
3. [x] Integrate the helper results into `pam_chissu` so that `require_secret_service` uses the helper, logs structured outcomes, and maps each error to the documented PAM codes.
4. [x] Add unit/integration tests covering JSON IPC message handling, helper failure mapping, and the Secret Service unavailable/missing key flows without requiring hardware.
5. [x] Update README/docs to describe the new helper behavior, D-Bus constraints, and how operators can verify functionality.
6. [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --workspace` (plus crate-specific tests) to validate the change.
