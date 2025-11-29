# packaging-rpm Specification

## Purpose
TBD - created by archiving change add-rpm-package-build. Update Purpose after archive.
## Requirements
### Requirement: RPM Packaging Script
The repository MUST ship a helper that produces RPM packages via the standard `rpmbuild` flow.

#### Scenario: Maintainer builds RPM package
- **GIVEN** a maintainer runs `build/package-rpm.sh --distro fedora --version <semver>` (or another supported RPM-based distro) from the workspace root
- **THEN** the script sets `CARGO_HOME="$(pwd)/.cargo-home"`, runs `cargo build --release` for `chissu-cli` and `pam-chissu`, stages the binaries/config/docs under `build/package/rpm/<distro>`
- **AND** it renders a `.spec` file plus `%post`/`%postun` hooks, then invokes `rpmbuild -bb` so that an `.rpm` is emitted into `dist/` with the distro + architecture encoded in the filename (e.g., `chissu-pam-<version>.<distro>.x86_64.rpm`)
- **AND** runtime dependencies include `dlib`, `openblas`, `lapack`, `gtk3`, `libudev`, `curl`, and `bzip2`

### Requirement: Install-Time Model Download
RPM packages MUST avoid bundling the dlib weights and instead download them during installation.

#### Scenario: %post downloads models
- **WHEN** the RPM is installed via `dnf rpm install` or `rpm -i`
- **THEN** the `%post` script checks `/var/lib/chissu-pam/dlib-models/` for `shape_predictor_68_face_landmarks.dat` and `dlib_face_recognition_resnet_model_v1.dat`
- **AND** missing weights are fetched from `https://dlib.net/files/` using `curl`, decompressed with `bzip2`, and stored with `0644` permissions
- **AND** an environment variable (e.g., `CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1`) skips downloads for offline installs
- **AND** install fails loudly when downloads are requested but cannot complete

### Requirement: Idempotent RPM Hooks
RPM lifecycle scripts MUST be idempotent and preserve operator-owned assets.

#### Scenario: %postun preserves models unless requested
- **WHEN** the RPM is removed
- **THEN** `%postun` only deletes files the package created (config template, empty directories)
- **AND** it leaves dlib weights untouched unless `CHISSU_PAM_PURGE_MODELS=1` is set, mirroring the Debian behaviour
- **AND** rerunning `%post` on upgrades logs that the files already exist instead of redownloading them

### Requirement: RPM PAM Wiring Via authselect Profile
RPM packages SHALL wire `libpam_chissu.so` through an `authselect` custom profile derived from `sssd`, ensuring the module runs before `pam_unix.so` and can be reverted safely.

#### Scenario: %post applies custom authselect profile
- **WHEN** the `%post` script runs during RPM installation or upgrade
- **THEN** it checks for `authselect` availability and refuses to proceed if the current profile is unsynced, emitting a clear error
- **AND** it renders/updates a `custom/chissu` profile that injects `auth    sufficient    libpam_chissu.so` ahead of `pam_unix.so` in both `system-auth` and `password-auth` templates
- **AND** it selects the profile and applies changes while backing up the previous selection so upgrades are idempotent and ordering remains correct.

#### Scenario: %postun restores previous selection on erase
- **WHEN** the RPM is removed (erase, not upgrade)
- **THEN** `%postun` restores the previously saved authselect selection (if present) and removes the `custom/chissu` profile artifacts
- **AND** if `CHISSU_PAM_PURGE_MODELS=1` is set, model files are purged as already required, but PAM wiring removal is always performed regardless of the flag.

