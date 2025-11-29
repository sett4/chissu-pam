## Overview

We will extend `scripts/install-chissu.sh` to manage PAM stack wiring in a distro-aware, reversible way. The installer already detects Debian/Ubuntu, RHEL/Rocky, and Arch. We will add a PAM handler per family that uses native tooling instead of editing `system-auth` directly. The PAM module `libpam_chissu.so` only implements the `auth` facility, so we must wire a single `auth` entry and ensure it runs **before** `pam_unix.so` to keep face verification ahead of password prompts.

## Approach

- **Debian/Ubuntu**: drop a snippet at `/usr/share/pam-configs/chissu` with a single `auth` line for `libpam_chissu.so`, flagged to appear before `pam_unix.so` when `pam-auth-update --package` rebuilds `common-auth`. Use `pam-auth-update --remove` for uninstall. Avoid touching `/etc/pam.d/common-*` directly.
- **RHEL/Fedora/Rocky**: create an `authselect` custom profile `chissu` based on `sssd`. Inject one `auth` line for `libpam_chissu.so` into `system-auth` and `password-auth` templates **before** the existing `pam_unix.so` entry, then run `authselect select custom/chissu && authselect apply-changes`. For uninstall, restore previous profile (`authselect select sssd` or cached previous selection) and delete the custom profile.
- **Arch**: write `/etc/pam.d/chissu` with a single `auth` line and include it from `system-local-login` (preferred) or `login` if the former is missing; the include must be positioned before the `pam_unix.so` line in the target file. Uninstall removes the include line and deletes the snippet.
- **Safety**: guard with `--dry-run` and `--uninstall`. Back up touched files before editing. Fail fast on unsupported distros or missing tooling (`pam-auth-update`, `authselect`).

## Data/Artifacts

- Static PAM snippet templates shipped in `scripts/pam/` (auth block only) to avoid inline heredocs duplication.
- State files under `/var/lib/chissu-pam/install/` for backups and previous authselect profile tracking.

## Alternatives Considered

- Editing `/etc/pam.d/system-auth` directly: rejected due to distro divergence and fragility.
- Packaging-only PAM hooks: insufficient for users installing from source tarballs.

## Risks

- `authselect` must run as root and will refuse custom edits if profile unsynced; handle by detecting `authselect check` and prompting to sync.
- On Arch, upstream configs may be locally customized; includes must append without clobbering existing content.
