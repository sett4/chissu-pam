## 1. Implementation
- [x] 1.1 Extend `install-chissu.sh` OS detection so `ID=fedora` and `ID_LIKE=fedora` flow into the RPM path (while keeping Rocky/RHEL as a fallback).
- [x] 1.2 Add Fedora-specific prerequisite package set in `scripts/lib/install_common.sh` and invoke it from the installer without EPEL/CRB enablement.
- [x] 1.3 Treat Fedora like other 64-bit RPM distros for PAM module placement and authselect wiring, keeping Rocky/RHEL behavior intact.
- [x] 1.4 Refresh README/installer docs and in-script help text to mention Fedora support.

## 2. Validation
- [x] 2.1 Run `openspec validate update-installer-fedora-support --strict`.
- [x] 2.2 Dry-run `install-chissu.sh` with `/etc/os-release` fixtures for Fedora and Rocky to confirm branching and logging (no system mutations).
