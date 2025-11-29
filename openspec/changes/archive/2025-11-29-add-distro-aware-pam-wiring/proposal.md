## Why
Operators currently edit `/etc/pam.d/*` by hand after running `scripts/install-chissu.sh`. This is error‑prone and varies across Debian/Ubuntu, RHEL/Fedora, and Arch families. We need a distro-aware, reversible way to place the PAM stack entries so installs are reproducible and safer.

## What Changes
- Add distro-specific PAM configuration handling to `install-chissu.sh` using native mechanisms (Debian `pam-auth-update`, RHEL/Fedora `authselect`, Arch includes in `system-local-login`/`login`) that wires **only** the `auth` facility for `libpam_chissu.so`.
- Ensure the face-auth `auth` entry is placed before `pam_unix.so` so face verification runs prior to password prompts on all supported distros.
- Provide dry-run and uninstall paths so PAM edits are previewable and reversible.
- Document the distro matrix, ordering expectations, and rollback steps in README/docs.

## Impact
- Spec: `installer-scripts` (add PAM wiring automation requirement).
- Code: `scripts/install-chissu.sh`, docs/README updates, potential templates for PAM snippets.
