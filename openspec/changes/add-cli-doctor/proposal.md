## Why
Operators need a single `chissu-cli doctor` command to validate PAM + capture prerequisites (config files, models, Secret Service, PAM install) before attempting enroll/auth. Today troubleshooting requires manual steps.

## What Changes
- Add a `doctor` subcommand to `chissu-cli` that runs a suite of checks across config, devices, model files, Secret Service, PAM module install, and PAM stack configuration.
- Produce human-readable and `--json` outputs with pass/warn/fail statuses per check plus overall exit code.
- Reuse the shared `chissu-config` loader for resolving config paths/settings when performing checks.

## Impact
- Faster diagnose of misconfiguration without touching hardware.
- Reduces support load by surfacing missing files/permissions early.
- Aligns CLI and PAM expectations by exercising both config loader and Secret Service plumbing.
