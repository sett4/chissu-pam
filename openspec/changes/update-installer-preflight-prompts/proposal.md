## Why
Running `scripts/install-chissu.sh` currently begins installing prerequisite packages automatically. This is risky on target systems; we want the script to remain read-only for dependencies by default and instead list what is missing so the user can install manually.

## What Changes
- Replace automatic package installation in `install-chissu.sh` with a preflight that detects missing packages per distro and prints the exact install command without executing it.
- Keep PAM wiring, binary/config/model steps unchanged once prerequisites are satisfied.
- Update docs/help text to note the prompt/exit behaviour instead of auto-install.

## Impact
- Affected specs: installer-scripts
- Affected code: scripts/install-chissu.sh, scripts/lib/install_common.sh (if package lists adjusted), README.md
