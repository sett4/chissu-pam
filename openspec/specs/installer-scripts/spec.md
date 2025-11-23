# installer-scripts Specification

## Purpose
TBD - created by archiving change add-linux-install-scripts. Update Purpose after archive.
## Requirements
### Requirement: Distro-Aware Installer Entry Point

The project SHALL provide an installer script that supports Ubuntu/Debian and Rocky Linux (8/9) by selecting the appropriate package manager and prerequisite set before deploying artifacts.

#### Scenario: Ubuntu/Debian prerequisites installed via apt

- **WHEN** `/etc/os-release` reports `ID=ubuntu` or `ID_LIKE=debian`
- **THEN** the installer uses `apt` to ensure compiler toolchain, `pkg-config`, dlib, OpenBLAS/LAPACK, GTK3, and udev development packages are present (installing only when missing)
- **AND** it aborts with a helpful message if any dependency cannot be resolved.

#### Scenario: Rocky prerequisites installed via dnf with EPEL/CRB enabled

- **WHEN** `/etc/os-release` reports `ID=rocky` or `ID_LIKE=rhel`
- **THEN** the installer enables EPEL and CRB/PowerTools repositories, uses `dnf` to install packages providing dlib, OpenBLAS/LAPACK, GTK3, udev/systemd headers, compiler toolchain, and `pkgconfig`
- **AND** it exits non-zero with guidance when the distro is unsupported or repositories are missing.

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

