# chissu-pam

## Overview

chissu-pam is an open-source, experimental Pluggable Authentication Module
(PAM) for Linux face authentication. It pairs a Rust CLI with a PAM module to
capture frames from infrared-friendly V4L2 webcams, enroll face embeddings, and
compare a live capture during PAM authentication.

The project is intended for learning, experimentation, and careful local
evaluation. It is not a replacement for a production-grade biometric
authentication system.

## Table of Contents

- [Overview](#overview)
- [Project Status / Security Notice](#project-status--security-notice)
- [Features](#features)
- [Supported Platforms / Requirements](#supported-platforms--requirements)
- [Quick Install](#quick-install)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Documentation](#documentation)
- [Development](#development)
- [License](#license)

## Project Status / Security Notice

This repository is in an early, exploratory phase: interfaces move quickly,
persistence formats may break, and the security surface has not been formally
audited. Review the code before enabling the module on any sensitive system.

PAM configuration mistakes can lock you out. Before wiring `libpam_chissu.so`
into a real service, keep a password-based fallback, maintain an active root or
recovery shell, and test against a non-critical PAM service first.

## Features

- **V4L2 capture pipeline.** Captures infrared-friendly frames from Linux
  `/dev/video*` devices through the shared Rust capture layer.
- **Encrypted embedding stores.** Face embeddings are encrypted with AES-GCM
  keys managed through GNOME Secret Service or another libsecret-compatible
  keyring.
- **Unprivileged enrollment.** Daily capture, enrollment, and embedding
  maintenance run in the user's desktop session; elevated access is reserved
  for installation and PAM wiring.
- **PAM integration.** `libpam_chissu.so` compares a live camera capture against
  the target user's enrolled embeddings and reports authentication events via
  PAM conversations and syslog.
- **Human and JSON CLI output.** CLI commands support readable terminal output
  and structured `--json` output for scripting.

## Supported Platforms / Requirements

- Linux with Video4Linux2 (V4L2) support.
- An infrared-capable webcam and permission to access the relevant
  `/dev/video*` device.
- Rust 1.85 or newer plus `cargo` for source builds.
- GNOME Secret Service, or another libsecret-compatible keyring, running in the
  target user's session.
- systemd-logind with an active desktop session for users who expect face
  unlock.
- Native development libraries for dlib, OpenBLAS/LAPACK, libclang, GTK, udev,
  and PAM when building locally.

On Debian/Ubuntu, the native build dependencies are:

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libclang-dev libdlib-dev libopenblas-dev liblapack-dev libgtk-3-dev libudev-dev libpam0g-dev
```

The CLI and PAM module also need the official dlib model files:

- `shape_predictor_68_face_landmarks.dat`
- `dlib_face_recognition_resnet_model_v1.dat`

Packages and installer scripts can download these models into
`/var/lib/chissu-pam/dlib-models`. For manual installs, download them from
<https://dlib.net/files/> and point `config.toml` or the relevant CLI flags at
the `.dat` files.

## Quick Install

Download the package for your distribution from the
[GitHub Releases page](https://github.com/sett4/chissu-pam/releases), then
install it with your system package manager:

```bash
# Debian/Ubuntu
sudo dpkg -i ./chissu-pam_<version>_<distro-release>_amd64.deb

# Fedora/RHEL-family systems
sudo dnf install ./chissu-pam_<version>_<distro>_x86_64.rpm
```

The packages install `chissu-cli`, `libpam_chissu.so`, a default
`/etc/chissu-pam/config.toml`, PAM integration snippets, and documentation
under standard system paths. Package hooks download the dlib model files unless
`CHISSU_PAM_SKIP_MODEL_DOWNLOAD=1` is set before installation.

If you are working from source, build the release artifacts and use the
installer script:

```bash
CARGO_HOME="$(pwd)/.cargo-home" cargo build --release -p chissu-cli -p pam-chissu

sudo scripts/install-chissu.sh \
  --artifact-dir target/release \
  --model-dir /var/lib/chissu-pam/dlib-models \
  --store-dir /var/lib/chissu-pam/embeddings
```

The installer supports Debian/Ubuntu, Fedora, Rocky Linux, and Arch Linux. It
prints missing prerequisite packages instead of installing them automatically,
supports `--dry-run`, and can remove only its PAM wiring with `--uninstall`.

Package build and release-maintenance details live in
[docs/packaging.md](docs/packaging.md) and [docs/releasing.md](docs/releasing.md).

## Quick Start

Run the environment doctor first:

```bash
chissu-cli doctor
chissu-cli doctor --polkit
chissu-cli --json doctor | jq
```

`doctor` checks configuration, video device access, model readability, Secret
Service availability, embedding store permissions, PAM module placement, and
PAM stack references. Use `--polkit` when debugging desktop prompts such as
1Password or GNOME Software.

Enroll your own face embeddings from a live capture:

```bash
chissu-cli enroll
```

The command captures a frame, extracts embeddings, stores them encrypted through
Secret Service-managed AES-GCM keys, and removes temporary capture artifacts.
It defaults to the invoking user and normally does not require `sudo`.

To verify PAM behavior, start with a non-critical service or a controlled
`pamtester` setup before changing login, sudo, or screen-unlock stacks. A minimal
PAM entry looks like this:

```pam
auth sufficient libpam_chissu.so
```

Place the entry before `pam_unix.so` only after confirming password fallback and
recovery access. See [docs/pam-auth.md](docs/pam-auth.md) for full installation,
verification, and rollback guidance.

## Configuration

Both `chissu-cli` and `pam-chissu` read configuration from:

1. `/etc/chissu-pam/config.toml`
2. `/usr/local/etc/chissu-pam/config.toml`
3. Built-in defaults when neither file exists

CLI flags and supported environment variables override resolved file values.
Common settings include:

| Key | Purpose |
| --- | --- |
| `video_device` | Default V4L2 device path. |
| `pixel_format` | Capture pixel format, commonly `Y16`. |
| `warmup_frames` | Frames discarded before saving or evaluating a capture. |
| `embedding_store_dir` | Directory for encrypted per-user embedding stores. |
| `landmark_model` / `encoder_model` | dlib model file paths. |
| `similarity_threshold` | PAM acceptance threshold. |
| `capture_timeout_secs` / `frame_interval_millis` | Live-auth timing controls. |
| `jitters` | dlib embedding jitter count. |
| `require_secret_service` | Whether PAM requires keyring access before capture. |
| `secret_service_session` | Secret Service session mode: `auto`, `x11`, or `wayland`. |

After editing configuration, run:

```bash
chissu-cli keyring check
chissu-cli capture --json
```

## Documentation

- [CLI usage reference](docs/chissu-cli.md)
- [PAM setup, runtime behavior, and troubleshooting](docs/pam-auth.md)
- [Manual infrared camera verification](docs/manual-testing.md)
- [polkit-agent-helper troubleshooting](docs/users-guide/polkit-agent-helper-troubleshooting.md)
- [Packaging guide](docs/packaging.md)
- [Release process](docs/releasing.md)
- [Behavior specs](docs/specs/index.md)

## Development

Workspace layout:

```text
chissu-pam/
├── Cargo.toml
├── crates/
│   ├── chissu-cli/
│   ├── chissu-config/
│   ├── chissu-face-core/
│   └── pam-chissu/
└── tests/
```

Use a repository-local `CARGO_HOME` when running cargo commands:

```bash
CARGO_HOME="$(pwd)/.cargo-home" cargo build
CARGO_HOME="$(pwd)/.cargo-home" cargo fmt --check
CARGO_HOME="$(pwd)/.cargo-home" cargo clippy -- -D warnings
CARGO_HOME="$(pwd)/.cargo-home" cargo test --workspace
CARGO_HOME="$(pwd)/.cargo-home" cargo test -p chissu-cli
CARGO_HOME="$(pwd)/.cargo-home" cargo test -p chissu-face-core
CARGO_HOME="$(pwd)/.cargo-home" cargo test -p pam-chissu
```

Tests use mocked frame data where possible, but native dlib dependencies are
still required to compile the face-recognition bindings.

## License

This project is distributed under the terms of the
[GNU Lesser General Public License v2.1](LICENSE).
