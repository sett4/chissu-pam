## Why
Operators expect the CLI and PAM module to honor the same descriptor store configuration. Today `faces enroll`/`faces remove` always fall back to `/var/lib/chissu-pam/models`, so a system that moved the store via `/etc/chissu-pam/config.toml` ends up writing to two different directories.

## What Changes
- Load `descriptor_store_dir` from `/etc/chissu-pam/config.toml` (or `/usr/local/etc/chissu-pam/config.toml` fallback) when `--store-dir` is omitted.
- Keep support for `--store-dir` and `CHISSU_PAM_STORE_DIR`, but make the config file the primary implicit default before falling back to the env var or built-in path.
- Document the new precedence so operators know how to align CLI runs with PAM deployments.

## Impact
- Affected specs: `face-features` (user feature store requirement)
- Affected code: CLI argument handling (`src/cli.rs`, `src/faces.rs`), shared path resolution helpers, unit tests, README/docs references.
