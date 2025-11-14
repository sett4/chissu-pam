1. [x] Extend `chissu-face-core::secret_service` with descriptor-key helpers (generate, store, delete) and expose AES-GCM utilities plus new AppError variants for encrypted store handling.
2. [x] Update face feature enrollment/removal flows to decrypt existing stores, rotate/register keys, encrypt rewritten stores, and ensure unit tests use a stubbed key backend.
3. [x] Teach `pam_chissu` to pass helper-provided keys into `load_enrolled_descriptors` and retry with the helper when encrypted stores are detected even if `require_secret_service` is false.
4. [x] Add AES-GCM encryption/decryption helpers for descriptor stores (including format migration, rollback on failure, and comprehensive tests).
5. [x] Document the new `faces enroll` behavior (README, docs/pam-auth) and run `cargo fmt`, `cargo clippy -- -D warnings`, and relevant `cargo test` commands.
