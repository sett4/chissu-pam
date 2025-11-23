## MODIFIED Requirements
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
