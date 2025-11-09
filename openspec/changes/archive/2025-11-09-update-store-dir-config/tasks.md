1. Config resolution
   - [x] Introduce a CLI helper that loads `/etc/chissu-pam/config.toml` (fallback `/usr/local/...`) and exposes `descriptor_store_dir` when present, surfacing parse errors as CLI failures.
2. CLI defaults
   - [x] Update `faces enroll`/`faces remove` config wiring so `--store-dir` overrides take precedence, otherwise fall back to the config value, then `CHISSU_PAM_STORE_DIR`, then the built-in path.
   - [x] Add unit tests that cover the precedence chain and the parse failure path.
3. Documentation
   - [x] Update README/docs to describe the new defaulting order for the feature store directory.
4. Validation
   - [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` to prove the change is healthy.
