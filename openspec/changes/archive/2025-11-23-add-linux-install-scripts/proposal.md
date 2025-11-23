## Why
Manual installation across Ubuntu and Rocky Linux requires many root-only steps (dependencies, PAM paths, config, model downloads) that are easy to misapply and currently undocumented for RPM-based distros.

## What Changes
- Add a distro-aware installer script that prepares prerequisites and deploys `chissu-cli`, `libpam_chissu.so`, default `chissu-pam/config.toml`, and dlib model assets.
- Encode correct file destinations and permissions for Debian/Ubuntu (`/lib/security`) and Rocky (`/usr/lib64/security`), plus `/usr/local/bin` and `/var/lib/chissu-pam/*` data paths.
- Provide defaults and flags for choosing artifact source paths, model download locations, and safe overwrite/backup behaviour so operators can re-run the installer idempotently.

## Impact
- Affected specs: installer-scripts (new capability); README/installation docs may need alignment.
- Affected code: new install script(s) under `scripts/` (or similar), sample config asset for `/etc/chissu-pam/config.toml`, and model download helper logic.
