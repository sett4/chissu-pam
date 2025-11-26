## Why
Operators on Arch Linux cannot use the installer; they must translate package names manually. Adding first-class support reduces setup friction and keeps dependencies aligned.

## What Changes
- Detect Arch Linux in the installer entrypoint and select a pacman-based prerequisite path.
- Define Arch-specific package set and behaviors (e.g., base-devel, pkgconf, openblas/lapack, udev, gtk3).
- Update installer requirements/specs and usage docs to reflect Arch support and any caveats.

## Impact
- Affected specs: installer-scripts
- Affected code: scripts/install-chissu.sh, installer docs/README snippets
