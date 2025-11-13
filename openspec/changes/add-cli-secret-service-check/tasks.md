## Implementation
- [x] Extract the existing Secret Service probe logic into a shared helper (e.g., a new module in `chissu-face-core` or a small internal crate) so both PAM and CLI reuse the same code path and error types.
- [x] Add a `keyring` dependency and implement a `chissu-cli keyring check` subcommand that invokes the shared probe, prints human-readable status, returns exit code `0` on success, and preserves the specific error message on failure.
- [x] Support `--json` output for the new command, including fields like `service`, `user`, `status`, and `error` (when applicable) so scripts can parse the result.
- [x] Write unit/integration tests covering success, missing entry (still success), and representative failure cases by mocking the probe layer.
- [x] Update README/docs to describe the new command, usage examples, and how it relates to PAM/DEK workflows.
- [x] Run `CARGO_HOME="$(pwd)/.cargo-home" cargo fmt`, `cargo clippy -- -D warnings`, `cargo test --workspace`, and `cargo test -p chissu-cli`, capturing results for reviewers.
