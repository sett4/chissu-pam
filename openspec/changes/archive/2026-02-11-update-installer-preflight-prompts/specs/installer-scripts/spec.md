## MODIFIED Requirements
### Requirement: Distro-Aware Installer Entry Point

The project SHALL provide an installer script that supports Ubuntu/Debian, Fedora (Workstation/Server), Rocky Linux (8/9), and Arch Linux by selecting the appropriate package manager and prerequisite set before deploying artifacts.

#### Scenario: Ubuntu/Debian prerequisites validated via apt

- **WHEN** `/etc/os-release` reports `ID=ubuntu` or `ID_LIKE=debian`
- **THEN** the installer checks required packages (`pkg-config`, dlib, OpenBLAS/LAPACK, GTK3, udev headers, compiler toolchain, curl, bzip2, rust toolchain when needed)
- **AND** if any are missing it exits non-zero, printing the full `apt-get install` command the operator should run; it must not install automatically.

#### Scenario: Fedora prerequisites validated via dnf without EPEL/CRB

- **WHEN** `/etc/os-release` reports `ID=fedora` or `ID_LIKE` includes `fedora`
- **THEN** the installer checks required packages (dlib, OpenBLAS/LAPACK, GTK3, udev/systemd headers, compiler toolchain, `pkgconf`, `curl`, `bzip2`)
- **AND** if any are missing it exits non-zero with a suggested `dnf install` command, without enabling EPEL/CRB or installing automatically.

#### Scenario: Rocky/RHEL prerequisites validated via dnf with EPEL/CRB guidance

- **WHEN** `/etc/os-release` reports `ID=rocky` or `ID_LIKE=rhel` (excluding Fedora)
- **THEN** the installer checks required packages (dlib, OpenBLAS/LAPACK, GTK3, udev/systemd headers, compiler toolchain, `pkgconfig`, `curl`, `bzip2`) and whether EPEL/CRB are enabled if needed
- **AND** if any are missing it exits non-zero with the `dnf config-manager --set-enabled ...; dnf install ...` command the operator should run; it must not install automatically.

#### Scenario: Arch prerequisites validated via pacman

- **WHEN** `/etc/os-release` reports `ID=arch` or `ID_LIKE` includes `arch`
- **THEN** the installer checks required packages (dlib, OpenBLAS/LAPACK, GTK3 headers, udev/systemd headers, compiler toolchain `base-devel`, `pkgconf`, `curl`, `bzip2`)
- **AND** if any are missing it exits non-zero with a `pacman -S --needed ...` hint (and an AUR note for dlib if absent), without installing automatically.

### Requirement: Idempotent And Observable Execution

The installer SHALL provide safety switches and logging so operators can rehearse actions and rerun it without unintended changes.

#### Scenario: Dry-run and explicit overwrite controls

- **WHEN** invoked with `--dry-run`
- **THEN** the installer prints the dependency checks, copy, and download steps it would take without modifying the system
- **AND** outside of dry-run it refuses to overwrite existing binaries/configs/models unless `--force` (with backups) is specified, emitting clear success/error messages and returning non-zero on failures, and it must never perform package installations automatically (only report missing ones).
