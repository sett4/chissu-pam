# installer-scripts Specification

## Purpose
TBD - created by archiving change add-linux-install-scripts. Update Purpose after archive.
## Requirements
### Requirement: Distro-Aware Installer Entry Point

The project SHALL provide an installer script that supports Ubuntu/Debian, Rocky Linux (8/9), and Arch Linux by selecting the appropriate package manager and prerequisite set before deploying artifacts.

#### Scenario: Ubuntu/Debian prerequisites installed via apt

- **WHEN** `/etc/os-release` reports `ID=ubuntu` or `ID_LIKE=debian`
- **THEN** the installer uses `apt` to ensure compiler toolchain, `pkg-config`, dlib, OpenBLAS/LAPACK, GTK3, and udev development packages are present (installing only when missing)
- **AND** it aborts with a helpful message if any dependency cannot be resolved.

#### Scenario: Rocky prerequisites installed via dnf with EPEL/CRB enabled

- **WHEN** `/etc/os-release` reports `ID=rocky` or `ID_LIKE=rhel`
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
- **AND** when running on Rocky it places the library into `/usr/lib64/security/` with mode `0644`, invoking `restorecon` if available to set SELinux context.

#### Scenario: CLI installed to /usr/local/bin

- **WHEN** the installer resolves the source artifact directory (default `target/release` unless overridden)
- **THEN** it installs `chissu-cli` into `/usr/local/bin/chissu-cli` with mode `0755`
- **AND** it refuses to overwrite a different binary unless a `--force` flag is supplied, backing up the previous file when overwriting.

### Requirement: Config Seeding And Directory Layout

The installer SHALL create the standard configuration and data directories, seeding a default `config.toml` without clobbering existing operator-provided files.

#### Scenario: Default config written with backup protection

- **WHEN** `/etc/chissu-pam/config.toml` is absent
- **THEN** the installer writes a template that reflects current defaults (`video_device = "/dev/video0"`, `pixel_format = "Y16"`, `warmup_frames = 0`, `embedding_store_dir = "/var/lib/chissu-pam/models"`, commented `landmark_model` and `encoder_model` pointing to `/var/lib/chissu-pam/dlib-models/*.dat`)
- **AND** if the file already exists, the installer leaves it untouched unless `--force` is provided, in which case it saves a timestamped backup before overwriting.

#### Scenario: Data directories created with restrictive modes

- **WHEN** the installer prepares state directories
- **THEN** it ensures `/etc/chissu-pam/`, `/usr/local/etc/chissu-pam/`, `/var/lib/chissu-pam/models`, and `/var/lib/chissu-pam/dlib-models` exist with owner `root:root` (or configurable) and modes no more permissive than `0755` for directories and `0644` for files.

### Requirement: Dlib Model Provisioning

The installer SHALL provision the required dlib model files in an operator-specified or default directory, downloading them only when missing.

#### Scenario: Model downloads are idempotent

- **WHEN** either model (`shape_predictor_68_face_landmarks.dat`, `dlib_face_recognition_resnet_model_v1.dat`) is absent in the target directory (default `/var/lib/chissu-pam/dlib-models`)
- **THEN** the installer fetches the corresponding `.bz2` archives from `https://dlib.net/files/`, unpacks them, and leaves the `.dat` files readable by `chissu-cli` and `pam_chissu`
- **AND** if the `.dat` files already exist, the installer skips download and extraction.

### Requirement: Idempotent And Observable Execution

The installer SHALL provide safety switches and logging so operators can rehearse actions and rerun it without unintended changes.

#### Scenario: Dry-run and explicit overwrite controls

- **WHEN** invoked with `--dry-run`
- **THEN** the installer prints the dependency, copy, and download steps it would take without modifying the system
- **AND** outside of dry-run it refuses to overwrite existing binaries/configs/models unless `--force` (with backups) is specified, emitting clear success/error messages and returning non-zero on failures.

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

