## Why
- Now that `pam_chissu` skips authentication when GNOME Secret Service is unavailable, operators need a simple way to verify keyring readiness before attempting PAM integration or CLI workflows that will depend on encrypted descriptor storage.
- Today there is no CLI entry point that exercises the same keyring probe logic—diagnosing missing DBus sessions or locked keyrings requires trial-and-error with PAM, which is slow and inconvenient.
- Providing a deterministic `chissu-cli` command keeps troubleshooting aligned with future DEK encryption stories and enables automated pre-flight checks in scripts or CI.

## What Changes
- Add a `chissu-cli keyring check` (or similar) subcommand that uses the same `keyring` crate probe as the PAM module to confirm whether Secret Service is reachable for the current user.
- The command emits human-readable status plus optional JSON (`--json`) showing success/failure, the service/user pair that was tested, and any underlying keyring error message.
- Exit zero when Secret Service is reachable (including the `NoEntry` case) and non-zero when the probe fails; ensure failures propagate the specific platform reason.
- Share probe logic between CLI and PAM by moving it into a reusable module or crate so both call paths stay consistent.
- Document the command in the README (`chissu-cli` section) with examples and troubleshooting guidance.

## Impact
- Introduces a new CLI dependency on the `keyring` crate (shared with `pam-chissu`) and a small amount of reusable helper code.
- Expands test coverage to include success/failure simulations for the keyring probe, ensuring CLI output/jump codes remain stable.
- Improves operator experience by allowing pre-flight verification without invoking PAM, helping future DEK encryption work rely on the same guard.
