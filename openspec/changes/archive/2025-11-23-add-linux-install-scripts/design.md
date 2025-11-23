## Context

Operators currently follow manual README steps to install `chissu-cli`, `libpam_chissu.so`, configuration, and dlib models. We need a repeatable installer for Ubuntu and Rocky Linux that encodes correct paths and dependencies while avoiding accidental overwrites of customized setups.

## Goals / Non-Goals

- Goals: support Ubuntu/Debian and Rocky 8/9; install prerequisites; place CLI + PAM artifacts; seed `/etc/chissu-pam/config.toml`; provision model and embedding store directories; allow dry-run/idempotent use.
- Non-Goals: building artifacts from source (installer assumes prebuilt or provided paths); supporting distros beyond Ubuntu/Rocky; modifying `/etc/pam.d/*` stacks; producing .deb/.rpm packages.

## Decisions

- Detect OS via `/etc/os-release` and branch to `apt` vs `dnf`. On Rocky, enable EPEL and CRB before installing dlib/OpenBLAS/LAPACK/udev/gtk toolchain equivalents.
- Install `chissu-cli` to `/usr/local/bin` (0755) and `libpam_chissu.so` to the distro-correct PAM directory (`/lib/security` for Debian/Ubuntu, `/usr/lib64/security` for Rocky) with 0644 permissions.
- Create data/config roots under `/etc/chissu-pam/`, `/usr/local/etc/chissu-pam/`, and `/var/lib/chissu-pam/{models,dlib-models}` with restrictive modes; support `--force` to overwrite and `--backup`/timestamp copies by default.
- Seed a default `config.toml` that references `/dev/video0`, `Y16`, warmup frames, `/var/lib/chissu-pam/models`, and model paths pointing to `/var/lib/chissu-pam/dlib-models/*.dat`, leaving values commented where hardware-specific input is expected.
- Provide flags/env vars to override artifact source directory (default `target/release`), model directory, and whether to attempt model downloads (`curl` + `bunzip2`). Skip downloads when files already exist.
- Emit clear logging with a `--dry-run` mode that prints planned actions without writing, and exit non-zero on missing artifacts or unsupported distros.

## Risks / Trade-offs

- dlib/OpenBLAS packages may require EPEL/CRB on Rocky; failing to enable could leave dependencies unresolved. Provide detection and actionable errors.
- SELinux contexts on Rocky may need `restorecon` after copying the PAM module; the installer should invoke it when available or document the manual command.
- Model downloads (~100MB) can be slow; idempotent checks prevent repeated fetches but add branching.
- Overwrite safeguards may frustrate users needing to refresh configs; `--force` + backup mitigates while keeping defaults safe.

## Open Questions

- Should the installer optionally build the artifacts when `target/release` is missing? (default assumption: artifacts exist; flag could trigger `cargo build --release` if desired).
- Should we create a dedicated system group/owner for `/var/lib/chissu-pam/models` instead of world-writable? Current README example uses permissive mode; spec should decide ownership.
- Is SELinux relabeling mandatory for PAM modules on Rocky in this environment? Might need confirmation during implementation.
