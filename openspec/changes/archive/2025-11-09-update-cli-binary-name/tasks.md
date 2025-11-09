1. [x] Rename the root crate binary target from `chissu-pam` to `chissu-cli` (`Cargo.toml`, `src/cli.rs`, build metadata) so `cargo build` outputs the new artifact.
2. [x] Update CLI help text, README, docs (`docs/pam-auth.md`, release notes) and any config references that mention the executable so operators invoke `chissu-cli` consistently.
3. [x] Adjust specs/tests or fixture scripts that shell out to the CLI so they point to `chissu-cli`, ensuring automated coverage stays green.
4. [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` to confirm the rename is build- and lint-clean.
