## MODIFIED Requirements
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
