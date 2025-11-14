1. Shared config crate
   - [x] Scaffold `crates/chissu-config` with `ConfigFile`, `ConfigError`, and loader helpers that search the primary + fallback paths.
   - [x] Add unit tests covering precedence, parse failures, IO failures, and empty-path behaviour using temp files.
2. CLI integration
   - [x] Replace `crates/chissu-cli/src/config.rs` file I/O with `chissu_config::load_config`, keeping the public `resolve_store_dir`, `load_capture_defaults`, and `load_face_model_defaults` APIs intact.
   - [x] Adapt CLI error mapping/tests so they lean on the shared crate while retaining coverage for CLI-specific structs.
3. PAM integration
   - [x] Remove `try_read_config`/`load_config` from `crates/pam-chissu/src/lib.rs` and call the shared loader instead, wiring its error into `AuthError::Config`.
   - [x] Keep the PAM-side logging of which file sourced the config (if any) so operators continue seeing the same diagnostics.
4. Documentation & validation
   - [x] Update README/docs to state that CLI and PAM share the `chissu-config` loader and that every new config key must be added there.
   - [x] Run `cargo fmt`, `CARGO_HOME="$(pwd)/.cargo-home" cargo clippy -- -D warnings`, and `CARGO_HOME="$(pwd)/.cargo-home" cargo test --workspace` to prove the refactor is safe.
