## Context
Install-time assets (dependency detection, default config, dlib model download helpers) are currently duplicated across `scripts/install-chissu.sh`, `build/package-deb.sh`, and `build/package-rpm.sh`. Defaults have already drifted (e.g., config.toml device/pixel format differences), and updating any list requires touching three locations. We want a single source of truth while preserving current behaviours and specs.

## Goals / Non-Goals
- Goals: unify defaults/templates/dependency lists; avoid copy-paste; make package assets generated from the same templates as the standalone installer; keep distro-specific behaviours intact.
- Non-Goals: change runtime behaviour or dependency policy; introduce a new build system; rewrite scripts into another language.

## Decisions
- Create `scripts/lib/install_common.sh` (path TBD) exporting: distro detection, prereq package arrays per distro, default config rendering function, dlib model URLs, and a helper to stage template assets into a given output directory.
- Add a small generator script (or function) to materialize `config.toml` and PAM snippets into `build/package/assets/` so Debian/RPM packaging reuse the same template files instead of maintaining their own copies.
- Keep scripts in POSIX bash; prefer sourced library over symlinks for portability with packaging tools.
- Preserve current command-line flags and behaviours in `install-chissu.sh`; library must be a drop-in source of values, not a behavioural change.

## Risks / Trade-offs
- If the library initialization changes environment expectations, existing packaging scripts could break; mitigate by keeping function signatures minimal and guarded by tests/check script.
- Shared defaults may require reconciling current mismatches (video device/pixel format). Need explicit decision to avoid surprising users; propose aligning on the more conservative defaults currently shipped in packages and document if we keep a difference.
- Packaging build environments may lack `bash` features; stick to `set -euo pipefail` and POSIX-compatible constructs already used.

## Migration Plan
1) Introduce the shared library and generator without altering existing behaviour; wire `install-chissu.sh` to use it.
2) Update packaging scripts to consume generated assets from the shared source; verify produced artifacts still pass existing specs.
3) Add a check script to compare generated assets against committed ones to prevent drift.

## Open Questions (resolved)
- Canonical config defaults: use `/dev/video2` with `GREY` pixel format (installer’s current values), keep `warmup_frames = 4`, `jitters = 1`, and world-writable embedding store only where required; propagate these into the shared template so packages and installer match.
- Dependency policy: build-time scripts may require `-dev`/`-devel` headers and toolchains; runtime package manifests should continue to declare only runtime libs. The shared library will expose separate build-prereq arrays (for installer and packaging build helpers) and runtime dependency metadata for generated control/spec files.
