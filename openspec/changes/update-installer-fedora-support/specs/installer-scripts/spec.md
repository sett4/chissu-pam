## MODIFIED Requirements
### Requirement: Distro-Aware Installer Entry Point

The project SHALL provide an installer script that supports Ubuntu/Debian, Fedora (Workstation/Server), Rocky Linux (8/9), and Arch Linux by selecting the appropriate package manager and prerequisite set before deploying artifacts.

#### Scenario: Ubuntu/Debian prerequisites installed via apt

- **WHEN** `/etc/os-release` reports `ID=ubuntu` or `ID_LIKE=debian`
- **THEN** the installer uses `apt` to ensure compiler toolchain, `pkg-config`, dlib, OpenBLAS/LAPACK, GTK3, and udev development packages are present (installing only when missing)
- **AND** it aborts with a helpful message if any dependency cannot be resolved.

#### Scenario: Fedora prerequisites installed via dnf without EPEL/CRB

- **WHEN** `/etc/os-release` reports `ID=fedora` or `ID_LIKE` includes `fedora`
- **THEN** the installer uses `dnf` (without enabling EPEL/CRB) to install packages providing dlib, OpenBLAS/LAPACK, GTK3, udev/systemd headers, compiler toolchain (`Development Tools`), `pkgconf`, `curl`, and `bzip2`
- **AND** it exits non-zero with guidance when the distro is unsupported or the package set cannot be resolved.

#### Scenario: Rocky/RHEL prerequisites installed via dnf with EPEL/CRB enabled

- **WHEN** `/etc/os-release` reports `ID=rocky` or `ID_LIKE=rhel` (excluding Fedora)
- **THEN** the installer enables EPEL and CRB/PowerTools repositories, uses `dnf` to install packages providing dlib, OpenBLAS/LAPACK, GTK3, udev/systemd headers, compiler toolchain, and `pkgconfig`
- **AND** it exits non-zero with guidance when the distro is unsupported or repositories are missing.

#### Scenario: Arch prerequisites installed via pacman

- **WHEN** `/etc/os-release` reports `ID=arch` or `ID_LIKE` includes `arch`
- **THEN** the installer uses `pacman -S --needed` to install packages providing dlib, OpenBLAS/LAPACK, GTK3 headers, udev/systemd headers, compiler toolchain (`base-devel`), `pkgconf`, `curl`, and `bzip2`
- **AND** it exits non-zero with a clear message if pacman is unavailable, the package set cannot be resolved, or the distro detection fails.

### Requirement: Correct Artifact Placement

The installer SHALL copy `chissu-cli` and `libpam_chissu.so` from a provided artifact directory into OS-appropriate locations with safe permissions.

#### Scenario: PAM module lands in distro path with 0644 mode

- **WHEN** the installer runs on Debian/Ubuntu
- **THEN** it places `libpam_chissu.so` into `/lib/security/` with mode `0644`
- **AND** when running on Fedora or Rocky/RHEL it places the library into `/usr/lib64/security/` with mode `0644`, invoking `restorecon` if available to set SELinux context.

#### Scenario: CLI installed to /usr/local/bin

- **WHEN** the installer resolves the source artifact directory (default `target/release` unless overridden)
- **THEN** it installs `chissu-cli` into `/usr/local/bin/chissu-cli` with mode `0755`
- **AND** it refuses to overwrite a different binary unless a `--force` flag is supplied, backing up the previous file when overwriting.

### Requirement: PAM Stack Wiring Is Automated Per Distro

The installer SHALL configure a single `auth` entry for `libpam_chissu.so` using each distribution's native mechanism instead of manual `/etc/pam.d` edits, MUST place it before `pam_unix.so` so face verification runs prior to password prompts, and MUST support preview and removal flows.

#### Scenario: Debian/Ubuntu uses pam-auth-update

- **WHEN** the installer detects `ID=debian`/`ubuntu` (or `ID_LIKE=debian`) and finds `pam-auth-update`
- **THEN** it writes a snippet to `/usr/share/pam-configs/chissu` that adds only an `auth` line for `libpam_chissu.so` with a priority that places it ahead of `pam_unix.so`
- **AND** runs `pam-auth-update --package --enable chissu` unless `--dry-run` is set
- **AND** `--uninstall` triggers `pam-auth-update --package --remove chissu` and removes the snippet.

#### Scenario: RHEL/Fedora families use authselect profile

- **WHEN** the installer detects `ID=fedora` or `ID_LIKE` includes `rhel`/`fedora` (e.g., Rocky/Fedora) and `authselect` is available
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
