# chissu-pam

## Overview

chissu-pam is an open-source, face-recognition-driven Pluggable Authentication Module (PAM) that pairs a Rust CLI with shared libraries to enroll and verify users via infrared-friendly V4L2 webcams. The workspace explores a reproducible workflow that captures frames, derives reusable feature vectors, and wires those descriptors into PAM conversations for experimental login flows.

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

- **Secret Service–backed encryption.** Descriptor stores are wrapped with AES-GCM keys managed by the GNOME Secret Service (`chissu-cli keyring ...`). Even if `/var/lib/chissu-pam/models/*.json` leaks, the ciphertext is unreadable until the legitimate user session unlocks the keyring.
- **Root privileges only for system wiring.** Daily capture, enrollment, and descriptor maintenance all run unprivileged inside the user’s desktop session so Secret Service is reachable. Elevated access is required only for installing binaries, copying `/etc/chissu-pam/config.toml`, or editing `/etc/pam.d/<service>`.

## Getting Started

### Prerequisites

- Linux with Video4Linux2 (V4L2) support and an infrared-capable webcam.
- Rust 1.85 or newer (Edition 2024) plus `cargo` in your `$PATH`.
- GNOME Secret Service (or another libsecret-compatible keyring) running in the target user session.
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

Download them from https://dlib.net/files/ once, then store them in a shared location (for example `/var/lib/chissu-pam/dllib-models`). Point `chissu-cli` at the files via CLI flags or entries in `config.toml`.

### Installation

1. **Build release artifacts** (the workspace expects `CARGO_HOME` to be inside the repo):

   ```bash
   cargo build --release -p chissu-cli -p pam-chissu
   ```

2. **Install the CLI**:

   ```bash
   sudo install -m 0755 target/release/chissu-cli /usr/local/bin/chissu-cli
   ```

3. **Install the PAM module**:

   ```bash
   sudo install -m 0644 target/release/libpam_chissu.so /lib/security/libpam_chissu.so
   ```

4. **Provision configuration**: copy (or author) `/etc/chissu-pam/config.toml` and optionally `/usr/local/etc/chissu-pam/config.toml`. Specify `video_device`, `descriptor_store_dir`, `landmark_model`, `encoder_model`, and PAM-related thresholds (see [Configuration](#configuration)).

5. **Store dlib weights** under a readable directory (for example `/etc/chissu-pam/models`) and update the config or environment variables so both CLI and PAM know where to load them.

6. **Wire PAM** by editing the relevant `/etc/pam.d/<service>` entry:

   ```text
   auth    sufficient    libpam_chissu.so
   ```

   Keep your existing `auth` stack intact—this module augments, not replaces, other factors.

7. **Verify Secret Service access** for each user who will authenticate:

   ```bash
   chissu-cli keyring check --json || echo "Secret Service is locked"
   ```

8. **Test locally** before touching production logins:

   ```bash
   cargo test --workspace
   chissu-cli capture --json | jq
   ```

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

### Enroll with live capture (`chissu-cli enroll`)

Automate the capture → extract → enroll pipeline with a single command that inherits capture defaults from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml` and finally `/dev/video0` + `Y16` + four warm-up frames). The command captures a frame, encodes descriptors, encrypts them via Secret Service–managed AES-GCM keys, and deletes the temporary capture + descriptor files once enrollment succeeds.

```bash
chissu-cli enroll \
  --landmark-model /etc/chissu-pam/models/shape_predictor_68_face_landmarks.dat \
  --encoder-model /etc/chissu-pam/models/dlib_face_recognition_resnet_model_v1.dat
```

- Target user defaults to the invoking account and does **not** require `sudo`. Because Secret Service runs in your desktop session, the CLI can request the descriptor key, encrypt the updated store, and return status without elevated privileges.
- Use `--device /dev/video2` when you need to override the configured device, `--store-dir <path>` to bypass the config file, and `--jitters`, `--landmark-model`, `--encoder-model` to fine-tune extraction just like `faces extract`.
- Model paths (`landmark_model`, `encoder_model`) inherit from `/etc/chissu-pam/config.toml` when present, fall back to `DLIB_LANDMARK_MODEL` / `DLIB_ENCODER_MODEL`, and only then require explicit CLI flags.
- `--json` mirrors the `faces enroll` payload (`user`, `store_path`, `added`) and appends capture metadata so automation can persist auditing data:

```json
{
  "user": "alice",
  "target_user": "alice",
  "store_path": "/var/lib/chissu-pam/models/alice.json",
  "added": [
    {
      "id": "7ae5d0e0-76d6-46f1-9ff4-c0cfd83a9a5a",
      "descriptor_len": 128,
      "source": "captures/auto-enroll/features-20251114T180102.101Z.json",
      "created_at": "2025-11-14T18:01:02.134Z"
    }
  ],
  "descriptor_ids": ["7ae5d0e0-76d6-46f1-9ff4-c0cfd83a9a5a"],
  "captured_image": "captures/auto-enroll/capture-20251114T180102.101Z.png",
  "captured_image_deleted": true,
  "descriptor_file_deleted": true,
  "faces_detected": 1
}
```

Need to enroll another user’s descriptors? Elevate just for that command so you can reach their Secret Service session and descriptor store:

```bash
sudo --preserve-env=DLIB_LANDMARK_MODEL,DLIB_ENCODER_MODEL \
  chissu-cli enroll --user bob \
  --landmark-model /etc/chissu-pam/models/shape_predictor_68_face_landmarks.dat \
  --encoder-model /etc/chissu-pam/models/dlib_face_recognition_resnet_model_v1.dat
```

`sudo` is required because `/var/lib/chissu-pam/models/bob.json` is root-owned. The helper still talks to Bob’s Secret Service instance and refuses to enroll if it cannot obtain the AES-GCM key or if the service is locked.

### Face feature enrollment

Register descriptor vectors with a specific Linux user so the planned PAM module can perform facial authentication. Point the command at a descriptor JSON exported by `faces extract`:

```bash
cargo run -p chissu-cli -- faces enroll --user alice captures/features/reference.json
```

Each descriptor receives a unique identifier and is appended to `/var/lib/chissu-pam/models/alice.json` by default (created automatically with `0600` permissions). The CLI now generates or rotates a 32-byte AES-256-GCM key for the user, registers it with Secret Service (`service = chissu-pam`, `user = <pam user>`), decrypts any existing store, and rewrites the updated store in encrypted form. Legacy plaintext stores are migrated automatically the first time you run the command after upgrading. The decrypted content remains the same JSON array you are used to (descriptor vector, bounding box, source file, creation timestamp, and stable ID), but it now lives inside an encrypted wrapper on disk.

Pass `--json` to receive a payload that lists the generated descriptor IDs and the feature-store path. Use `--store-dir <path>` to override the storage directory explicitly. When the flag is omitted, the CLI reads `descriptor_store_dir` from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml`), then consults the `CHISSU_PAM_STORE_DIR` environment variable, and finally falls back to the built-in `/var/lib/chissu-pam/models/` location. Remember that the Secret Service daemon for the target user must be reachable during enrollment so the key rotation succeeds.

- Missing or unreadable descriptor files exit with status code `2`.
- Malformed payloads or empty descriptor lists exit with status code `3` and leave the store untouched.
- Descriptor length mismatches between the payload and the existing store also exit with status code `3`.

### Face feature removal

Remove descriptors from the store when they are no longer valid:

```bash
# Remove a specific descriptor by ID
auth_id=$(cargo run -p chissu-cli -- faces enroll --user alice captures/features/reference.json --json | jq -r ".added[0].id")
cargo run -p chissu-cli -- faces remove --user alice --descriptor-id "$auth_id"

# Remove every descriptor for a user
cargo run -p chissu-cli -- faces remove --user alice --all

# Work against a non-default store directory
cargo run -p chissu-cli -- faces enroll --user alice --store-dir ./captures/enrolled captures/features/reference.json
cargo run -p chissu-cli -- faces remove --user alice --descriptor-id "$auth_id" --store-dir ./captures/enrolled
```

The command reports the IDs that were deleted and the number of descriptors that remain. With `--json` it emits a structured summary containing `removed_ids`, `remaining`, and the target store path. Attempting to delete an unknown ID exits with status code `4`, leaving the store unchanged. Using `--all` deletes the store file entirely (or treats the operation as a no-op when the user has no enrolled descriptors). Behind the scenes the CLI fetches the user’s AES-GCM key from Secret Service, decrypts the store, removes the requested records, and re-encrypts the result with the same key so PAM continues to read the file.

When neither command receives `--store-dir`, they inherit the same precedence chain described for enrollment (config files, then `CHISSU_PAM_STORE_DIR`, then the built-in path), keeping CLI operations aligned with the PAM module configuration.

### PAM facial authentication

The repository now ships a PAM module (`libpam_chissu.so`) that authenticates Linux users by comparing a live camera capture with descriptors enrolled via `faces enroll`.

- Build the shared library with `cargo build --release -p pam-chissu` (or `cargo test -p pam-chissu` during development).
- Copy `target/release/libpam_chissu.so` into your PAM module directory (for example `sudo install -m 0644 target/release/libpam_chissu.so /lib/security/libpam_chissu.so`) and reference it from `/etc/pam.d/<service>` with `auth sufficient libpam_chissu.so`. The build no longer emits the historical `libpam_chissuauth.so` symlink, so there is a single canonical shared object to package.
- Configure the module via `/etc/chissu-pam/config.toml` (preferred) or `/usr/local/etc/chissu-pam/config.toml`. Each file is optional; when both are absent, the module falls back to:
  - `similarity_threshold = 0.7`
  - `capture_timeout_secs = 5`
  - `frame_interval_millis = 500`
  - `video_device = "/dev/video0"`
  - `descriptor_store_dir = "/var/lib/chissu-pam/models"`
  - `pixel_format = "Y16"`
  - `warmup_frames = 0`
  - `jitters = 1`
  - `require_secret_service = false`
- Syslog (facility `AUTHPRIV`) records start, success, timeout, and error events. Review output with `journalctl -t pam_chissu` or `journalctl SYSLOG_IDENTIFIER=pam_chissu`.
- Interactive PAM conversations mirror those events on the terminal: successful matches trigger a `PAM_TEXT_INFO` banner, while retries and failures emit `PAM_ERROR_MSG` guidance ("stay in frame", "no descriptors", etc.) so operators see immediate feedback even without tailing syslog.
- Before opening the camera the module now forks a short-lived helper that switches to the PAM target user (`setuid`) and talks to the user's GNOME Secret Service session over D-Bus. The helper returns a JSON payload containing either the AES-GCM descriptor key, a "missing" status, or a structured error. The parent logs the outcome and (a) continues capture when the key was returned, (b) surfaces the "no descriptors" flow when the key is missing, or (c) returns `PAM_IGNORE` when Secret Service is locked/unreachable so downstream modules can continue handling the login.
- Use `chissu-cli keyring check` to verify that Secret Service is reachable for the current user before wiring the PAM module into a stack. The command exits `0` on success, emits structured JSON when `--json` is supplied, and surfaces the underlying keyring error when the probe fails. Set `require_secret_service = true` to enforce the helper inside PAM; it defaults to `false` so you can opt in once the desktop session exposes Secret Service. Store a 32-byte AES-GCM descriptor key (Base64-encoded) under `service=chissu-pam` and `user=<pam user>` so the helper can unlock descriptor files during authentication.
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
| `descriptor_store_dir`                           | Directory for encrypted descriptor files (`/var/lib/chissu-pam/models`).                   |
| `landmark_model` / `encoder_model`               | Paths to the dlib weights (overrideable via `DLIB_LANDMARK_MODEL` / `DLIB_ENCODER_MODEL`). |
| `similarity_threshold`                           | PAM acceptance threshold (default `0.7`).                                                  |
| `capture_timeout_secs` / `frame_interval_millis` | Live-auth capture timing knobs.                                                            |
| `jitters`                                        | Number of random jitters applied when encoding descriptors.                                |
| `require_secret_service`                         | Fail fast when the Secret Service helper cannot obtain a key.                              |

For CLI operations, `chissu-config` also honours `CHISSU_PAM_STORE_DIR` for descriptor storage overrides plus any immediate CLI flags. After editing the TOML file, re-run `chissu-cli keyring check` and a quick `chissu-cli capture --json` to verify the new settings.

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
