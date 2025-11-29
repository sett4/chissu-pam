## ADDED Requirements

### Requirement: PAM Stack Wiring Is Automated Per Distro

The installer SHALL configure a single `auth` entry for `libpam_chissu.so` using each distribution's native mechanism instead of manual `/etc/pam.d` edits, MUST place it before `pam_unix.so` so face verification runs prior to password prompts, and MUST support preview and removal flows.

#### Scenario: Debian/Ubuntu uses pam-auth-update

- **WHEN** the installer detects `ID=debian`/`ubuntu` (or `ID_LIKE=debian`) and finds `pam-auth-update`
- **THEN** it writes a snippet to `/usr/share/pam-configs/chissu` that adds only an `auth` line for `libpam_chissu.so` with a priority that places it ahead of `pam_unix.so`
- **AND** runs `pam-auth-update --package --enable chissu` unless `--dry-run` is set
- **AND** `--uninstall` triggers `pam-auth-update --package --remove chissu` and removes the snippet.

#### Scenario: RHEL/Fedora families use authselect profile

- **WHEN** the installer detects `ID_LIKE=rhel` (e.g., Rocky/Fedora) and `authselect` is available
- **THEN** it creates/updates a custom profile (e.g., `custom/chissu`) derived from `sssd`, injecting a single `auth` line for `libpam_chissu.so` into `system-auth`/`password-auth` templates **before** the existing `pam_unix.so` entry
- **AND** it activates the profile via `authselect select ... && authselect apply-changes`, backing up previous selection so `--uninstall` can restore it and delete the custom profile
- **AND** if the profile is out of sync, the installer surfaces an actionable error instead of modifying files directly.

#### Scenario: Arch-based distros use include snippets

- **WHEN** the installer detects `ID=arch` or `ID_LIKE` containing `arch`
- **THEN** it writes `/etc/pam.d/chissu` with only an `auth` line for `libpam_chissu.so` and appends an include to `system-local-login` (falling back to `login` when absent) positioned before the `pam_unix.so` line without disturbing existing lines
- **AND** `--uninstall` removes the include and deletes the snippet, leaving other entries untouched.

#### Scenario: Dry-run and safety guards

- **WHEN** invoked with `--dry-run`
- **THEN** the installer prints the exact PAM operations (files, commands, profile names) without changing the system
- **AND** all PAM edits are idempotent, create backups before mutation, and abort with clear errors on missing required tools (`pam-auth-update`, `authselect`, `pacman`/`sed`).
