## Tasks
1. [x] Move the CLI crate into `crates/chissu-cli/`: add a dedicated `Cargo.toml`, relocate `src/` and CLI `tests/`, and update `cargo run/test` documentation and scripts to reference `-p chissu-cli`.
2. [x] Relocate `pam-chissu/` to `crates/pam-chissu/`, keeping `pam_chissu` as the library target and updating any build scripts or include paths that assumed the old location.
3. [x] Convert the root `Cargo.toml` into a workspace-only manifest that lists `crates/chissu-cli`, `crates/chissu-face-core`, and `crates/pam-chissu` as members (plus any future crates), and adjust path dependencies in every crate to the new layout.
4. [x] Ensure each crate owns a local `tests/` directory while repository-level integration tests stay under `./tests/`; refresh README, docs, and `AGENTS.md` so contributors know where to place new tests and how to run `cargo test -p <crate>` vs `cargo test --workspace`.
5. [x] Update OpenSpec specs/docs that reference the old layout (e.g., capture CLI and PAM requirements) so they describe the new directories and workspace responsibilities.
6. [x] Validation: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace`, and targeted `cargo test -p pam_chissu`/`-p chissu-cli` to prove the restructured workspace still builds and passes tests.
