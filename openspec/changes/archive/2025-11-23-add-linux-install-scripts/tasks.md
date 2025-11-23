## 1. Installer

- [x] 1.1 Choose script entry point/location (e.g., `scripts/install.sh`) and supported flags/environment inputs.
- [x] 1.2 Implement OS detection (Ubuntu/Debian vs Rocky 8/9) and dependency installation, enabling EPEL/CRB when needed.
- [x] 1.3 Implement artifact staging: place `chissu-cli` into `/usr/local/bin` and `libpam_chissu.so` into the distro-correct PAM module directory with `0644` perms.
- [x] 1.4 Create data/config directories (`/etc/chissu-pam/`, `/usr/local/etc/chissu-pam/`, `/var/lib/chissu-pam/{models,dlib-models}`) with safe owners/modes and optional `--force` backups for existing files.
- [x] 1.5 Download or copy required dlib models (`shape_predictor_68_face_landmarks.dat`, `dlib_face_recognition_resnet_model_v1.dat`) and unpack `.bz2` archives; skip when already present.
- [x] 1.6 Add idempotency/dry-run logging so reruns don't clobber customized configs or models without explicit opt-in.

## 2. Validation & Docs

- [x] 2.1 Add README/INSTALL documentation for using the new installer on Ubuntu and Rocky Linux, including expected prerequisites and flags.
- [ ] 2.2 Run `shellcheck` (or equivalent) on the installer script. (Blocked: `sudo` lacks setuid; cannot install shellcheck here.)
- [x] 2.3 Run `openspec validate add-linux-install-scripts --strict` and fix any findings.
