## 1. Analysis & Design
- [x] 1.1 Compare current defaults/dependency lists/config templates across installer and packaging scripts; choose canonical values (decision: `/dev/video2` + `GREY`, installer defaults win; build prereqs include `-dev/-devel` toolchains, runtime deps remain runtime-only in packages).
- [x] 1.2 Finalize shared asset layout (library path, template location, generated outputs) and update design.md.

## 2. Implementation
- [x] 2.1 Add shared shell library for distro detection, prereq lists, config rendering, and dlib model download helpers.
- [x] 2.2 Refactor `scripts/install-chissu.sh` to consume the shared library for defaults, model URLs, and config rendering.
- [x] 2.3 Refactor `build/package-deb.sh` to pull config/postinst assets from the shared template outputs; drop duplicated dependency checks.
- [x] 2.4 Refactor `build/package-rpm.sh` to reuse the shared assets/template outputs; drop duplicated dependency checks.
- [x] 2.5 Provide a small verification target (e.g., `scripts/install-assets-check.sh` or Make target) that asserts generated assets match templates and/or runs shellcheck on the shared library.

## 3. Validation
- [x] 3.1 Run `openspec validate refactor-shared-install-assets --strict`.
- [x] 3.2 Run `shellcheck` on touched shell scripts (or document if unavailable).
- [x] 3.3 Build smoke: `CARGO_HOME="$(pwd)/.cargo-home" cargo build --release -p chissu-cli -p pam-chissu` (to ensure packaging inputs exist) if time permits.
- [x] 3.4 Optionally dry-run installer `DRY_RUN=1 SKIP_DOWNLOAD=1 ./scripts/install-chissu.sh --artifact-dir target/release --config-path /tmp/config.toml` to confirm behaviour unchanged.
