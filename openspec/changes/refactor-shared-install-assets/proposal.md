## Why
- Dependency checks, default config template, and dlib model download logic are duplicated across `scripts/install-chissu.sh`, `build/package-deb.sh`, and `build/package-rpm.sh`, leading to drift (different defaults, package lists) and extra maintenance.
- We need a single source of truth for installer assets so packaging scripts and the standalone installer stay consistent with specs (installer-scripts, packaging-deb, packaging-rpm).

## What Changes
- Introduce a shared installer asset library (shell) that exposes: distro detection, prerequisite lists, default config rendering, and dlib model download routines.
- Refactor `install-chissu.sh` and both packaging scripts to consume the shared library/templates instead of embedding their own copies.
- Align the default `config.toml` and model URLs across installer and packaging assets; generate package assets from the same templates.
- Add simple verification hooks (dry-run/lint) to catch divergence during CI or local builds.

## Impact
- Affected specs: installer-scripts, packaging-deb, packaging-rpm.
- Affected code: scripts/install-chissu.sh, build/package-deb.sh, build/package-rpm.sh, build/package/assets/* (config + postinst helpers), new shared library path under scripts/ or build/package/.
- No behavior regressions intended; runtime behaviour should remain as currently specified but sourced from a single template.
