## Why
- Operators keep `/etc/chissu-pam/config.toml` in sync with the PAM module so both the CLI and PAM endpoint use the same device and format parameters. Today the `capture` command ignores those values, so operators must duplicate `--device`, `--pixel-format`, and `--warmup-frames` on every run.
- The PAM module already honors `video_device`, `pixel_format`, and `warmup_frames` from the same config file. Bringing the CLI in line removes surprises when manual captures are used to debug PAM behaviour.
- Falling back to the existing CLI defaults (index `0`, `Y16`, four warm-up frames) preserves backwards compatibility for users without config files.

## What Changes
- Extend the CLI configuration loader to read `video_device`, `pixel_format`, and `warmup_frames` from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml`).
- Update `chissu-pam capture` argument resolution so that each of the above flags is optional: CLI flag overrides config, config overrides built-in defaults.
- Document the new fallback order in the capture CLI spec and README so operators know how to preconfigure capture defaults.
- Add unit tests covering config precedence plus docs/changelog notes so future contributors do not regress the layering.

## Impact
- No breaking changes: supplying explicit CLI flags continues working; environments without config files continue to see the same defaults.
- Minimal runtime overhead (single config parse that we already perform for descriptor store resolution in other commands).
- Adds new config keys to document but reuses existing TOML file structure already consumed by the PAM module.
