# packaging-deb Specification

## Purpose
TBD - created by archiving change add-deb-packaging-script. Update Purpose after archive.
## Requirements
### Requirement: Debian Packaging Script
The repository MUST ship a script that produces Debian-compatible `.deb` artifacts using the standard `dpkg-buildpackage` flow, and the resulting package MUST declare only runtime dependencies for end users while keeping build-time headers in `Build-Depends`.

#### Scenario: Maintainer builds Debian package
- **GIVEN** a Linux maintainer runs `build/package-deb.sh --distro debian --version <semver>` from the workspace root
- **THEN** the script sets `CARGO_HOME="$(pwd)/.cargo-home"`, runs `cargo build --release`, stages the CLI binary, PAM module, configs, docs, and install scripts under `build/package/debian`
- **AND** it invokes `dpkg-buildpackage -us -uc` (or equivalent debhelper tooling) to emit a `.deb` file into `dist/` whose filename embeds the distribution and version (e.g., `chissu-pam_<version>_debian_amd64.deb`)
- **AND** the package metadata declares runtime dependencies via `${shlibs:Depends}`/`${misc:Depends}` plus non-library tools (`curl`, `bzip2`, `ca-certificates`), without requiring any `-dev` packages at install time
- **AND** build-time headers and development libraries (e.g., `libdlib-dev`, `libopenblas-dev`, `liblapack-dev`, `libgtk-3-dev`, `libudev-dev`) are listed under `Build-Depends`.

### Requirement: Ubuntu Packaging Variant
The packaging script MUST also support Ubuntu builds without duplicating logic while honoring the same runtime-only dependency policy.

#### Scenario: Maintainer targets Ubuntu
- **WHEN** the maintainer passes `--distro ubuntu`
- **THEN** the script reuses the same staging workflow but writes Ubuntu-specific control metadata (e.g., `Distribution: ubuntu`, optional dependency versions) before calling `dpkg-buildpackage`
- **AND** the resulting artifact lands in `dist/` as `chissu-pam_<version>_ubuntu_amd64.deb`
- **AND** the package declares runtime dependencies via `${shlibs:Depends}`/`${misc:Depends}` plus non-library tools, with development libraries confined to `Build-Depends`.

### Requirement: Install-Time Dlib Download
The packages MUST stay lightweight by deferring dlib weight retrieval to installation time.

#### Scenario: Postinst downloads models
- **WHEN** the generated package is installed via `dpkg -i` or `apt`
- **THEN** the `postinst` script checks `/var/lib/chissu-pam/dlib-models/` (or a configurable path) for `shape_predictor_68_face_landmarks.dat` and `dlib_face_recognition_resnet_model_v1.dat`
- **AND** if either file is missing it downloads the `.bz2` archives from `https://dlib.net/files/`, uncompresses them, fixes permissions, and logs progress to stdout/stderr
- **AND** failures to reach the network or write the files cause `postinst` to exit non-zero so package installation aborts visibly

### Requirement: Idempotent Install Hooks
The install/uninstall scripts MUST be idempotent and respect offline scenarios.

#### Scenario: No-op when models exist
- **WHEN** `postinst` runs on a system that already hosts both dlib models
- **THEN** it logs that the files already exist and skips downloading them
- **AND** operators can pass an environment flag (documented in README) to skip download attempts entirely for offline mirrors
- **AND** `prerm`/`postrm` only remove files that the package created (config, symlinks) while leaving the dlib weights intact unless the operator opts-in via a documented flag

### Requirement: Debian PAM Wiring Via pam-auth-update
Debian/Ubuntu packages SHALL wire `libpam_chissu.so` into the `auth` stack using the distro-supported `pam-auth-update` mechanism and ship the required snippet.

#### Scenario: Postinst enables pam-auth-update entry
- **WHEN** `postinst configure` runs for the `.deb` package
- **THEN** it verifies `pam-auth-update` is available, installs or refreshes `/usr/share/pam-configs/chissu` from package assets, and executes `pam-auth-update --package --enable chissu`
- **AND** the resulting PAM order places `auth    sufficient    libpam_chissu.so` before the existing `pam_unix.so` entry
- **AND** if `pam-auth-update` is missing or reports an out-of-sync state, the script exits non-zero with a clear error so the install aborts instead of leaving partial wiring.

#### Scenario: Removal cleans pam-auth-update state
- **WHEN** the package is removed or purged
- **THEN** maintainer scripts call `pam-auth-update --package --remove chissu` and delete the snippet if present
- **AND** upgrades remain idempotent (no duplicate lines), while purge leaves other PAM entries untouched.

### Requirement: Deb Packaging Consumes Shared Installer Assets
The Debian/Ubuntu packaging workflow SHALL consume the shared installer templates/library for config defaults and model download hooks instead of maintaining separate copies.

#### Scenario: Deb build pulls shared config and hooks
- **WHEN** `build/package-deb.sh` stages package files
- **THEN** it copies the generated config template and any shared hook scripts from the common asset output
- **AND** it does not re-define prerequisite package lists or dlib download URLs independently (it reuses the shared library/templates).

