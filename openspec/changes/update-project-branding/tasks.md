1. [x] Rename the root crate/binary metadata to `chissu-pam` (Cargo package, Clap command, binary names) and adjust any build scripts if needed.
2. [x] Update default descriptor storage constants and environment variables to `/var/lib/chissu-pam/models` and `CHISSU_PAM_STORE_DIR`, ensuring both CLI and PAM module use the same values and adding coverage in docs.
3. [x] Refresh README, docs/pam-auth.md, and AGENTS.md to reference **chissu-pam** (CLI examples, constitution name, env vars).
4. [x] Modify specs (`infrared-capture`, `face-features`, `pam-face-auth`) so requirements and scenarios use the new binary name and default paths.
5. [x] Run `openspec validate update-project-branding --strict` plus `cargo fmt --check`/`cargo clippy -- -D warnings`/`cargo test` to confirm the rename is consistent.
