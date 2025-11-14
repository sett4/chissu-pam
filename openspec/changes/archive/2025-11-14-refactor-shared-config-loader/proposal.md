## Why
- `chissu-cli` (`crates/chissu-cli/src/config.rs`) and the PAM module (`crates/pam-chissu/src/lib.rs`) both read `/etc/chissu-pam/config.toml` + `/usr/local/etc/chissu-pam/config.toml`, but each crate defines its own loader struct, constants, and error handling. Any new key must be copy/pasted twice, and past fixes (e.g., capture defaults, descriptor store dir) already duplicated tests.
- The PAM crate understands a superset of keys (thresholds, jitters, Secret Service gate) while the CLI trims the schema. Divergent schemas make it easy to introduce drift—one crate could reject a file the other accepts, or silently ignore a field that operators expect to apply everywhere.
- The OpenSpec constitution emphasizes "shared configuration" between CLI and PAM. Without a single loader, we cannot guarantee that both binaries enforce the same precedence, logging, and validation behaviour.

## What Changes
- Introduce a `chissu-config` crate that owns: the canonical `ConfigFile` struct (union of fields today), the ordered path search (`/etc/chissu-pam/config.toml`, `/usr/local/etc/chissu-pam/config.toml`), and error types for read/parse failures. Provide helpers like `ConfigFile::load()` returning `(Option<ConfigFile>, ConfigSource)` and `all_paths()` for tests.
- Refactor `chissu-cli` to drop its private loader and instead call into `chissu-config` for resolving descriptor store directory, capture defaults, and face model paths. CLI-specific structs such as `CaptureDefaults` stay in the CLI crate but are built from the shared config data.
- Refactor `pam-chissu` to replace `try_read_config`/`load_config` with the shared loader, keeping only the code that maps `ConfigFile` into the existing `ResolvedConfig` structure and logging the source path for observability.
- Update the capture CLI and pam-face-auth specs to require reuse of the shared loader so future contributors cannot reintroduce per-crate readers.
- Document the shared crate (README/docs) so operators know there is a single configuration schema and developers know where to add new keys.

## Impact
- Eliminates duplicate parsing logic and reduces the chance of config drift; adding new keys becomes a single-codepath change.
- Slightly increases compile times by introducing one tiny crate consumed by CLI and PAM, but no runtime regressions (the same two files are still read once per invocation).
- Unlocks future work (e.g., JSON output of effective config) because both binaries will have identical visibility into the config file contents.
