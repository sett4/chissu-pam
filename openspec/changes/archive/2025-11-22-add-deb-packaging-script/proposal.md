# Proposal: Debian/Ubuntu Packaging Script

## Background
Operators currently install `chissu-cli` and `pam-chissu` from source. The repo already has a `build/deb/` directory with leftover artifacts, but there is no reproducible script or spec for generating Debian packages. Reviewers now expect a standard packaging workflow that produces `.deb` files for both Debian and Ubuntu so deployments can rely on APT-native installs.

## Goals
- Provide a documented script that builds `.deb` packages for both Debian and Ubuntu targets using standard Debian tooling (dpkg-buildpackage / debhelper style layout).
- Ensure the packages ship the CLI binary, PAM module, default config, and service assets without embedding the large dlib model binaries.
- Fetch the dlib landmark/encoder weights during package install (postinst) rather than during package build, so artifact creation stays lightweight and distribution-compliant.
- Place finished packages in `dist/` with metadata that distinguishes Debian vs Ubuntu builds.

## Non-Goals
- Publishing artifacts to an APT repo (out-of-scope; we only generate packages locally).
- Converting to other packaging formats (RPM, snap, etc.).
- Automating signing or uploading to Launchpad.

## High-Level Plan
1. Add a `build/package-deb.sh` helper that orchestrates cargo release builds (with `CARGO_HOME` set per repo rules), stages files under `build/package/debian`, renders control scripts, and invokes `dpkg-buildpackage -us -uc` per target distribution.
2. Provide Debian packaging metadata (`debian/control`, `rules`, `compat`, `copyright`, `postinst`, `prerm`) with knobs for Ubuntu vs Debian (e.g., `Distribution`, dependency versions) driven by env vars the script sets.
3. Embed a `postinst` hook that checks for the dlib model files and, when missing, downloads + decompresses them into `/var/lib/chissu-pam/dlib-models`, respecting offline installs by guarding with a flag.
4. Document script usage in `README`/docs plus add tests/CI hooks in future tasks as needed.

## Impact
- Simplifies install instructions for both distros, letting admins run a single script to get redistributable packages.
- Keeps `.deb` artifacts small by delegating dlib weight downloads to install time.
- Aligns packaging steps with the repo’s existing `build/` folder instead of ad-hoc manual commands.

## Open Questions
- Exact dependency versions (libdlib-dev, libopenblas-dev, liblapack-dev, libudev-dev) per target: confirm with maintainers during implementation.
- Whether we need separate packages for CLI vs PAM or one combined package that installs both. This proposal assumes a unified `chissu-pam` package containing CLI + PAM components.
