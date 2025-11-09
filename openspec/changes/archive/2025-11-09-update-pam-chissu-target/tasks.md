## Tasks
1. [x] Rename the crate directory to `pam-chissu/`, update the workspace member list (`Cargo.toml#L2`), and change the package name/lib target (`pam-chissuauth/Cargo.toml`) to `pam-chissu`/`pam_chissu`.
2. [x] Update source-level identifiers (syslog `process`, error prefixes, doc comments) so every runtime message and PAM stack snippet uses `pam_chissu`.
3. [x] Introduce a lightweight post-build step (e.g., `build.rs` or `xtask pam-artifact`) that renames `target/<profile>/libpam_chissu.so` to `target/<profile>/pam_chissu.so` and, for one release, emits a compatibility symlink for `libpam_chissuauth.so`.
4. [x] Refresh documentation (`README.md`, `docs/pam-auth.md`, `AGENTS.md`, any scripts) to instruct `cargo build --release -p pam_chissu`, `cargo test -p pam_chissu`, and copying `pam_chissu.so` into `/lib/security/`.
5. [x] Update `openspec/specs/pam-face-auth/spec.md` (and any dependent specs) via this change delta so the requirement names the new artefact and logging identifier.
6. [x] Validation: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test -p pam_chissu`, and (if added) the packaging/rename helper to prove it produces `pam_chissu.so` plus the temporary compatibility symlink.
