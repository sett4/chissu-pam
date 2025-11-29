# chissu-pam

## Overview

chissu-pam is an open-source, face-recognition-driven Pluggable Authentication Module (PAM) that pairs a Rust CLI with shared libraries to enroll and verify users via infrared-friendly V4L2 webcams. The workspace explores a reproducible workflow that captures frames, derives reusable face embeddings, and wires those embeddings into PAM conversations for experimental login flows.

This repository is in an early, exploratory phase: interfaces move quickly, persistence formats may break, and the security surface has not been formally audited. Treat every component as pre-production, review the code before deploying to sensitive systems, and expect rough edges as the project evolves.

## Table of Contents

- [Overview](#overview)
- [Why This Project](#why-this-project)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Workspace Layout](#workspace-layout)
- [Usage](#usage)
- [Configuration](#configuration)
- [Testing](#testing)
- [Manual Verification with Hardware](#manual-verification-with-hardware)
- [License](#license)

## Why This Project

- **Secret Service–backed encryption.** Embedding stores are wrapped with AES-GCM keys managed by the GNOME Secret Service (`chissu-cli keyring ...`). Even if `/var/lib/chissu-pam/models/*.json` leaks, the ciphertext is unreadable until the legitimate user session unlocks the keyring.
- **Root privileges only for system wiring.** Daily capture, enrollment, and embedding store maintenance all run unprivileged inside the user’s desktop session so Secret Service is reachable. Elevated access is required only for installing binaries, copying `/etc/chissu-pam/config.toml`, or editing `/etc/pam.d/<service>`.

## Getting Started

### Prerequisites

- Linux with Video4Linux2 (V4L2) support and an infrared-capable webcam.
- Rust 1.85 or newer (Edition 2024) plus `cargo` in your `$PATH`.
- GNOME Secret Service (or another libsecret-compatible keyring) running in the target user session.
- systemd-logind (via `systemd-logind.service`) with an active desktop session for every user that expects face unlock. PAM uses logind's `Display`, `Type`, and runtime environment to reach Secret Service during non-graphical prompts (polkit-1, 1Password, etc.).
- Required kernel permissions to access `/dev/video*` devices.
- System libraries needed by the dlib face-recognition bindings.

Install the native dependencies on Debian/Ubuntu with:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libdlib-dev libopenblas-dev liblapack-dev libgtk-3-dev libudev-dev
```

#### Download the dlib models

The CLI and PAM module need the official dlib weights:

- `shape_predictor_68_face_landmarks.dat`
- `dlib_face_recognition_resnet_model_v1.dat`

Download them from https://dlib.net/files/ once, then store them in a shared location (for example `/var/lib/chissu-pam/dllib-models`). Point `chissu-cli` at the files via CLI flags or entries in `config.toml`. When you install the Debian/Ubuntu packages described below, the `postinst` hook handles these downloads automatically unless you export `CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` for offline deployments.

### Installation

#### Debian/Ubuntu packages (recommended)

1. **Build the package** (requires `debhelper`, `dpkg-dev`, and `curl`):

   ```bash
   build/package-deb.sh --distro debian   # or --distro ubuntu
   ```

   Pass `--version` to override the detected workspace version or `--arch` for non-`amd64` builds. Artifacts land in `dist/chissu-pam_<version>_<distro>_amd64.deb`.

2. **Install the package**:

   ```bash
   sudo dpkg -i dist/chissu-pam_0.3.0_debian_amd64.deb
   ```

   The CLI binary, PAM module, default config, and doc snippets are placed under the standard system paths.

3. **Automatic dlib weights**: during installation the `postinst` script downloads both dlib model files into `/var/lib/chissu-pam/dlib-models`. Skip downloads (for offline mirrors or air-gapped hosts) by setting `CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` before running `dpkg -i`. When purging the package, set `CHISSU_PAM_PURGE_MODELS=1` to remove the downloaded weights as well.

4. **PAM wiring + config**: the package registers `libpam_chissu.so` via `pam-auth-update --package --enable chissu`, inserting it ahead of `pam_unix.so`. Verify with `sudo pam-auth-update --list`. Adjust `/etc/chissu-pam/config.toml` as needed for your camera settings.

#### Automated releases

- Push a tag that matches `v<MAJOR>.<MINOR>.<PATCH>` (for example `git tag v0.3.0 && git push origin v0.3.0`).
- The `Release Debian Packages` workflow builds both Debian and Ubuntu `.deb` files via `build/package-deb.sh`, using the numeric portion of the tag as the package version.
- When the workflow finishes, GitHub Releases contains `chissu-pam_<version>_debian_amd64.deb`, `chissu-pam_<version>_ubuntu_amd64.deb`, and `chissu-pam_<version>_<distro>_x86_64.rpm` assets attached to that tag. Release notes are auto-generated; edit them manually if more detail is needed.
- If the workflow fails, fix the issue and click “Re-run jobs” for the tag; assets are replaced when uploads succeed.

#### RPM packages (Fedora/RHEL)

1. **Build the package** (requires `rpm-build`, `createrepo-c`, and the same native deps as the Debian flow):

   ```bash
   build/package-rpm.sh --distro fedora   # add --version <semver> to override
   ```

   Add `--skip-build` if you've already produced release binaries via another step. Artifacts land in `dist/chissu-pam_<version>_<distro>_x86_64.rpm`.

2. **Install the package**:

   ```bash
   sudo dnf install ./dist/chissu-pam_0.3.0_fedora_x86_64.rpm
   ```

3. **Automatic dlib weights**: `%post` mirrors the Debian behaviour—models are downloaded into `/var/lib/chissu-pam/dlib-models` unless `CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` is exported before running `dnf install`. Set `CHISSU_PAM_PURGE_MODELS=1` before uninstalling to remove the downloaded weights.

4. **PAM wiring**: `%post` uses `authselect` to create/refresh a `custom/chissu` profile and inserts `libpam_chissu.so` before `pam_unix.so` in `system-auth` and `password-auth`. On erase, `%postun` restores the previous profile. Verify with `authselect current` and adjust `/etc/chissu-pam/config.toml` for camera options.

##### Build RPMs via Docker (Ubuntu hosts)

Use the provided Fedora builder image when `rpmbuild` (or the necessary headers) are missing locally:

```bash
docker build -t chissu-rpm -f build/package/rpm/Dockerfile .
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  chissu-rpm \
  ./build/package-rpm.sh --distro fedora --version 0.3.0
```

The container writes artifacts back into `dist/` inside your working tree. Pass `--skip-build` if you already have release binaries before invoking the container.

#### Manual install from source

1. **Build release artifacts** (the workspace expects `CARGO_HOME` to be inside the repo):

   ```bash
   cargo build --release -p chissu-cli -p pam-chissu
   ```

2. **Install the CLI**:

   ```bash
   sudo install -m 0755 target/release/chissu-cli /usr/local/bin/chissu-cli
   ```

3. **Create Model dir**:

   ```bash
   sudo mkdir -p /var/lib/chissu-pam/models
   sudo chmod 0666 /var/lib/chissu-pam/models

   sudo mkdir -p /var/lib/chissu-pam/dlib-models
   sudo curl https://dlib.net/files/shape_predictor_68_face_landmarks.dat.bz2 -o /var/lib/chissu-pam/dlib-models/shape_predictor_68_face_landmarks.dat.bz2
   sudo curl https://dlib.net/files/dlib_face_recognition_resnet_model_v1.dat.bz2 -o /var/lib/chissu-pam/dlib-models/dlib_face_recognition_resnet_model_v1.dat.bz2
   sudo bunzip2 /var/lib/chissu-pam/dlib-models/shape_predictor_68_face_landmarks.dat.bz2 /var/lib/chissu-pam/dlib-models/dlib_face_recognition_resnet_model_v1.dat.bz2
   ```

   (Skip this step when installing via the `.deb` packages—the `postinst` script performs the same download.)

4. **Install the PAM module**:

   ```bash
   sudo install -m 0644 target/release/libpam_chissu.so /usr/lib/x86_64-linux-gnu/security/libpam_chissu.so
   ```

   (Use `/lib/security` on distributions that do not provide the multiarch PAM directory.)

5. **Provision configuration**: copy (or author) `/etc/chissu-pam/config.toml` and optionally `/usr/local/etc/chissu-pam/config.toml`. Specify `video_device`, `embedding_store_dir`, `landmark_model`, `encoder_model`, and PAM-related thresholds (see [Configuration](#configuration)).

6. **Store dlib weights** under a readable directory (for example `/var/lib/chissu-pam/dlib-models`) and update the config or environment variables so both CLI and PAM know where to load them.

7. **Wire PAM** by editing the relevant `/etc/pam.d/<service>` entry:

   ```text
   auth    sufficient      /usr/lib/x86_64-linux-gnu/security/libpam_chissu.so
   ```

   Place this `auth`-only entry **before** `pam_unix.so` so face verification runs ahead of password prompts. Keep your existing `auth` stack intact—this module augments, not replaces, other factors.

8. **Verify Secret Service access** for each user who will authenticate:

   ```bash
   chissu-cli keyring check --json || echo "Secret Service is locked"
   ```

9. **Test locally** before touching production logins:

   ```bash
   cargo test --workspace
   chissu-cli capture --json | jq
   ```

10. **sudo**:

    ```bash
    sudo echo test chissu-pam
    ```

#### Automated installer (Ubuntu/Fedora/Rocky/Arch)

If you already have release artifacts (or a downloaded bundle) you can let the repo script place files and dependencies for you:

```bash
sudo scripts/install-chissu.sh \
  --artifact-dir target/release \
  --model-dir /var/lib/chissu-pam/dlib-models \
  --store-dir /var/lib/chissu-pam/models
```

- Auto-detects Ubuntu/Debian vs Fedora vs Rocky Linux vs Arch Linux, **does not install packages automatically**, and instead prints a command to install any missing prerequisites before continuing. PAM module goes to `/lib/security` (Debian/Ubuntu/Arch) or `/usr/lib64/security` (Fedora/Rocky, with `restorecon` when available).
- On Arch it installs via `pacman -S --needed`: `base-devel`, `pkgconf`, `openblas`, `lapack`, `gtk3`, `systemd`, `curl`, `rust`, and `bzip2`. dlib is in AUR, so `yay -S dlib`.
- Wires PAM automatically per distro with a single `auth sufficient libpam_chissu.so` entry placed **before** `pam_unix.so`: Debian/Ubuntu via `pam-auth-update` snippet `/usr/share/pam-configs/chissu`, Fedora/RHEL/Rocky via an `authselect` custom profile, Arch by including a `/etc/pam.d/chissu` stack from `system-local-login`/`login`.
- Supports rollbacks with `--uninstall` (removes only the PAM wiring using distro-native tools) and `--dry-run` to preview all changes. Backups land in `/var/lib/chissu-pam/install/`.
- Seeds `/etc/chissu-pam/config.toml` if missing (honours `--force` to overwrite with a backup) and ensures `/var/lib/chissu-pam/{models,dlib-models}` exist. Defaults now set `warmup_frames = 4` and `require_secret_service = true` in the generated config.
- Downloads the dlib models only when the `.dat` files are absent; add `--skip-model-download` to prevent network calls or `--dry-run` to preview actions without changes.
- Override paths with `--artifact-dir`, `--model-dir`, `--store-dir`, or `--config-path` if your environment differs.

To roll back just the PAM wiring, run `sudo scripts/install-chissu.sh --uninstall`. Debian/Ubuntu use `pam-auth-update --remove chissu`, RHEL/Fedora restore the previously selected `authselect` profile (saved under `/var/lib/chissu-pam/install/authselect.previous`), and Arch removes the `auth include chissu` line plus `/etc/pam.d/chissu`.

### Secret Service + logind troubleshooting

`pam_chissu` now hydrates missing `$DISPLAY`, `$DBUS_SESSION_BUS_ADDRESS`, and `$XDG_RUNTIME_DIR` variables from systemd-logind whenever `require_secret_service = true`. This matters for PAM clients like polkit-1 (1Password unlock dialogs, GNOME Software updates, etc.) that invoke authentication without inheriting your desktop environment. Use these checks whenever the journal logs `Secret Service unavailable; skipping face authentication` or `No active logind session`:

1. **Confirm the session exists**

   ```bash
   loginctl list-sessions
   ```

   Ensure the target user shows an `active` session bound to the expected `seat` and `tty`.

2. **Inspect session properties**

   ```bash
   loginctl show-session <id> -p Display -p Type -p TTY -p Remote -p State
   ```

   The helper copies `Display` (e.g., `:0` or `wayland-0`) plus the runtime seat info before forking.

3. **Verify the runtime directory**

   ```bash
   loginctl show-user $(id -u $USER) -p RuntimePath
   ```

   A valid runtime dir allows the helper to synthesize `unix:path=$XDG_RUNTIME_DIR/bus` for Secret Service.

Successful hydration emits `Recovered session environment from logind for user 'alice': session=3 tty=tty2 seat=seat0 type=wayland ...` in syslog. If you instead see `No active logind session for user 'alice' (tty hint tty2)` the PAM stack will fall back to passwords until that desktop session is running/unlocked.

## Workspace layout

```
chissu-pam/
├── Cargo.toml            # Workspace-only manifest (no root package)
├── crates/
│   ├── chissu-cli/        # Binary crate (CLI entrypoint)
│   ├── chissu-face-core/  # Shared library crate
│   └── pam-chissu/        # PAM module crate (libpam_chissu.so)
└── tests/                # Cross-crate integration tests/fixtures
```

- Each crate owns a local `tests/` directory for component-scoped coverage (`cargo test -p <crate>`).
- Repository-level integration tests that touch multiple crates stay under the top-level `tests/` directory and run via `cargo test --workspace`.
- All crates inherit shared metadata (version, edition) from `[workspace.package]` in the root manifest, so changes only need to be made once.

## Building

```bash
cargo build
```

## Usage

`chissu-cli` exposes capture, feature extraction, enrollment, and maintenance commands. Run the installed binary directly (preferred) or invoke `cargo run -p chissu-cli -- …` during development. Detailed capture/extraction/compare walkthroughs now live in [docs/chissu-cli.md](docs/chissu-cli.md) while the sections below focus on enrollment and PAM integration flows.

### Environment doctor (`chissu-cli doctor`)

Run a quick, non-destructive diagnostic to confirm PAM + enrollment prerequisites before debugging deeper issues:

```bash
chissu-cli doctor            # human-readable
chissu-cli --json doctor | jq
```

Checks include config discovery/parse, video device access, embedding store permissions, dlib model readability, Secret Service availability, the PAM module location, and whether `/etc/pam.d/*` references `pam_chissu`. Exit code is `0` only when every check passes; warnings (e.g., both config files present) or failures return `1` with details per check.

### Enroll with live capture (`chissu-cli enroll`)

Automate the capture → extract → enroll pipeline with a single command that inherits capture defaults from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml` and finally `/dev/video0` + `Y16` + four warm-up frames). The command captures a frame, encodes embeddings, encrypts them via Secret Service–managed AES-GCM keys, and deletes the temporary capture + embedding files once enrollment succeeds.

```bash
chissu-cli enroll
```

- Target user defaults to the invoking account and does **not** require `sudo`. Because Secret Service runs in your desktop session, the CLI can request the embedding key, encrypt the updated store, and return status without elevated privileges.
- Use `--device /dev/video2` when you need to override the configured device, `--store-dir <path>` to bypass the config file, and `--jitters`, `--landmark-model`, `--encoder-model` to fine-tune extraction just like `faces extract`.
- Model paths (`landmark_model`, `encoder_model`) inherit from `/etc/chissu-pam/config.toml` when present, fall back to `DLIB_LANDMARK_MODEL` / `DLIB_ENCODER_MODEL`, and only then require explicit CLI flags.

Need to enroll another user’s embeddings? Elevate just for that command so you can reach their Secret Service session and embedding store:

```bash
sudo \
  chissu-cli enroll --user bob
```

`sudo` is required because `/var/lib/chissu-pam/models/bob.json` is root-owned. The helper still talks to Bob’s Secret Service instance and refuses to enroll if it cannot obtain the AES-GCM key or if the service is locked.

### PAM facial authentication

The repository now ships a PAM module (`libpam_chissu.so`) that authenticates Linux users by comparing a live camera capture with embeddings enrolled via `faces enroll`.

- Build the shared library with `cargo build --release -p pam-chissu` (or `cargo test -p pam-chissu` during development).
- Copy `target/release/libpam_chissu.so` into your PAM module directory (for example `sudo install -m 0644 target/release/libpam_chissu.so /usr/lib/x86_64-linux-gnu/security/libpam_chissu.so` on Debian/Ubuntu) and reference it from `/etc/pam.d/<service>` with `auth sufficient libpam_chissu.so`. The build no longer emits the historical `libpam_chissuauth.so` symlink, so there is a single canonical shared object to package.
- Configure the module via `/etc/chissu-pam/config.toml` (preferred) or `/usr/local/etc/chissu-pam/config.toml`. Each file is optional; when both are absent, the module falls back to:
  - `similarity_threshold = 0.9`
  - `capture_timeout_secs = 5`
  - `frame_interval_millis = 500`
  - `video_device = "/dev/video0"`
  - `embedding_store_dir = "/var/lib/chissu-pam/models"`
  - `pixel_format = "Y16"`
  - `warmup_frames = 0`
  - `jitters = 1`
  - `require_secret_service = false`
- Syslog (facility `AUTHPRIV`) records start, success, timeout, and error events. Review output with `journalctl -t pam_chissu` or `journalctl SYSLOG_IDENTIFIER=pam_chissu`.
- Interactive PAM conversations mirror those events on the terminal: successful matches trigger a `PAM_TEXT_INFO` banner, while retries and failures emit `PAM_ERROR_MSG` guidance ("stay in frame", "no embeddings", etc.) so operators see immediate feedback even without tailing syslog.
- Before opening the camera the module now forks a short-lived helper that switches to the PAM target user (`setuid`) and talks to the user's GNOME Secret Service session over D-Bus. The helper returns a JSON payload containing either the AES-GCM embedding key, a "missing" status, or a structured error. The parent logs the outcome and (a) continues capture when the key was returned, (b) surfaces the "no embeddings" flow when the key is missing, or (c) returns `PAM_IGNORE` when Secret Service is locked/unreachable so downstream modules can continue handling the login.
- Use `chissu-cli keyring check` to verify that Secret Service is reachable for the current user before wiring the PAM module into a stack. The command exits `0` on success, emits structured JSON when `--json` is supplied, and surfaces the underlying keyring error when the probe fails. Set `require_secret_service = true` to enforce the helper inside PAM; it defaults to `false` so you can opt in once the desktop session exposes Secret Service. Store a 32-byte AES-GCM embedding key (Base64-encoded) under `service=chissu-pam` and `user=<pam user>` so the helper can unlock embedding files during authentication.
- The module honours `DLIB_LANDMARK_MODEL` and `DLIB_ENCODER_MODEL` (or config entries with the same names) to locate dlib model files.

See [`docs/pam-auth.md`](docs/pam-auth.md) for installation walkthroughs, configuration examples, and troubleshooting tips.

## Configuration

Both `chissu-cli` and `pam-chissu` read configuration from:

1. `/etc/chissu-pam/config.toml`
2. `/usr/local/etc/chissu-pam/config.toml`
3. Built-in defaults (applied when neither file exists)

The first file that exists wins for each key; CLI flags or environment variables still override the resolved value. Common settings include:

| Key                                              | Purpose                                                                                    |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `video_device`                                   | Default V4L2 path (`/dev/video0` fallback).                                                |
| `pixel_format`                                   | Negotiated capture pixel format (`Y16` fallback).                                          |
| `warmup_frames`                                  | Number of frames to discard before saving.                                                 |
| `embedding_store_dir`                            | Directory for encrypted embedding files (`/var/lib/chissu-pam/models`).                    |
| `landmark_model` / `encoder_model`               | Paths to the dlib weights (overrideable via `DLIB_LANDMARK_MODEL` / `DLIB_ENCODER_MODEL`). |
| `similarity_threshold`                           | PAM acceptance threshold (default `0.9`).                                                  |
| `capture_timeout_secs` / `frame_interval_millis` | Live-auth capture timing knobs.                                                            |
| `jitters`                                        | Number of random jitters applied when encoding embeddings.                                 |
| `require_secret_service`                         | Fail fast when the Secret Service helper cannot obtain a key.                              |

For CLI operations, `chissu-config` also honours `CHISSU_PAM_STORE_DIR` for embedding storage overrides plus any immediate CLI flags. After editing the TOML file, re-run `chissu-cli keyring check` and a quick `chissu-cli capture --json` to verify the new settings.

## Testing

Automated tests exercise frame conversion, JSON serialization, and filesystem handling:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test --workspace
cargo test -p chissu-cli
cargo test -p pam_chissu
```

Run `cargo test -p chissu-face-core` when working on the shared library directly. Mocked frame data keeps tests independent of live hardware, but the dlib crates still require the native headers/libraries listed earlier. Without them `dlib-face-recognition` fails to compile.

## Manual verification with hardware

Run through the checklist in [`docs/manual-testing.md`](docs/manual-testing.md) when validating with a physical infrared camera. The document covers capability expectations, recommended exposure/gain values, and example commands for both human and JSON output modes.

## License

This project is distributed under the terms of the [GNU Lesser General Public License v2.1](LICENSE).
