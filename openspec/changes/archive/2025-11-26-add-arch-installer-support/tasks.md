## 1. Spec & Planning
- [x] 1.1 Add installer-scripts spec delta documenting Arch support and pacman dependency list.

## 2. Implementation
- [x] 2.1 Extend scripts/install-chissu.sh OS detection to recognize Arch Linux.
- [x] 2.2 Add pacman prerequisite installation path with Arch-specific package names.
- [x] 2.3 Ensure directory/config/model handling remains distro-agnostic or adjust for Arch if needed.

## 3. Validation
- [x] 3.1 Run shellcheck or basic lint on scripts/install-chissu.sh. (shellcheck unavailable; ran `bash -n` as basic lint)
- [x] 3.2 Dry-run installer on Arch-like /etc/os-release fixture to confirm branch and messaging.
- [x] 3.3 Update README/installer docs to mention Arch support and prerequisites.
