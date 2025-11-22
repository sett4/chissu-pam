## ADDED Requirements
### Requirement: Debian Packaging Script
The repository MUST ship a script that produces Debian-compatible `.deb` artifacts using the standard `dpkg-buildpackage` flow.

#### Scenario: Maintainer builds Debian package
- **GIVEN** a Linux maintainer runs `build/package-deb.sh --distro debian --version <semver>` from the workspace root
- **THEN** the script sets `CARGO_HOME="$(pwd)/.cargo-home"`, runs `cargo build --release`, stages the CLI binary, PAM module, configs, docs, and install scripts under `build/package/debian`
- **AND** it invokes `dpkg-buildpackage -us -uc` (or equivalent debhelper tooling) to emit a `.deb` file into `dist/` whose filename embeds the distribution and version (e.g., `chissu-pam_<version>_debian_amd64.deb`)
- **AND** the package metadata declares runtime dependencies on `libdlib-dev`, `libopenblas-dev`, `liblapack-dev`, `libgtk-3-dev`, `libudev-dev`, and `curl`

### Requirement: Ubuntu Packaging Variant
The packaging script MUST also support Ubuntu builds without duplicating logic.

#### Scenario: Maintainer targets Ubuntu
- **WHEN** the maintainer passes `--distro ubuntu`
- **THEN** the script reuses the same staging workflow but writes Ubuntu-specific control metadata (e.g., `Distribution: ubuntu`, optional dependency versions) before calling `dpkg-buildpackage`
- **AND** the resulting artifact lands in `dist/` as `chissu-pam_<version>_ubuntu_amd64.deb`

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
