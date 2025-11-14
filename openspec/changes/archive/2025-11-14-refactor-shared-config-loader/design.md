## Overview
We will introduce a lightweight Rust crate named `chissu-config` under `crates/chissu-config/`. The crate centralizes everything about reading `/etc/chissu-pam/config.toml` (primary) and `/usr/local/etc/chissu-pam/config.toml` (secondary). CLI-only and PAM-only crates will depend on `chissu-config` instead of maintaining bespoke loaders.

Key contents:
- `pub const PRIMARY_CONFIG_PATH` / `SECONDARY_CONFIG_PATH` — moved from the existing crates so the precedence is defined once.
- `#[derive(Deserialize, Debug, Clone, Default)] pub struct ConfigFile` — union of fields currently parsed by both crates (descriptor store dir, capture defaults, face model paths, similarity threshold, capture timing, jitters, Secret Service requirement).
- `#[derive(Debug, Clone, PartialEq, Eq)] pub enum ConfigSource { Primary, Secondary }` to describe where config came from; `None` when no files existed.
- Error type `ConfigError` that mirrors the current error surfaces (IO failure vs parse failure) so each consumer can map it to its own error domain (`AppError` in CLI, `AuthError` in PAM).

## Loader API
```rust
pub fn load_config() -> Result<(Option<ConfigFile>, Option<ConfigSource>), ConfigError>;
pub fn load_from_paths(paths: &[PathBuf]) -> Result<(Option<ConfigFile>, Option<PathBuf>), ConfigError>;
```
- `load_config()` is the ergonomic call used by CLI/PAM; internally it just forwards to `load_from_paths` with the two canonical paths.
- `load_from_paths` provides determinism for tests; it returns the path that produced the config so callers can log it or build error messages.
- The loader only reads until the first file that successfully parses, matching the current behaviour in both crates.

## Integration Plan
1. `crates/chissu-cli`: keep `CaptureDefaults`, `FaceModelDefaults`, and existing public helpers but rewrite them to call `chissu_config::load_config`. Convert loader errors into `AppError::ConfigRead|ConfigParse` by inspecting the shared `ConfigError`.
2. `crates/pam-chissu`: delete `try_read_config`/`load_config` functions and replace them with a call to `chissu_config::load_config`. The returned `ConfigFile` is fed into `ResolvedConfig::from_raw` as today, and the optional source path is logged.
3. Tests: move shared loader tests (parse/read errors, precedence) into the new crate. Keep CLI/PAM unit tests for mapping/resolution logic (e.g., CLI still tests `CaptureDefaults`, PAM still tests `ResolvedConfig` defaults) but drop redundant file I/O tests from the leaf crates.
4. Docs: README/docs mention the single configuration schema and the shared crate name so contributors know where to change it.

## Assumptions / Open Questions
- No other crate reads the config file today; if additional crates need it later they also depend on `chissu-config`.
- We will keep the schema entirely optional (all `Option<_>` fields) to preserve non-breaking behaviour.
- CLI error mapping already differentiates between read vs parse; the new crate must carry enough detail to keep those variants intact.
