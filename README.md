# chissu-pam

chissu-pam is an open-source, face-recognition-driven Pluggable Authentication Module (PAM) that pairs a Rust CLI with shared libraries to enroll and verify users via infrared-friendly V4L2 webcams. The workspace explores a reproducible workflow that captures frames, derives reusable feature vectors, and wires those descriptors into PAM conversations for experimental login flows.

This repository is in an early, exploratory phase: interfaces move quickly, persistence formats may break, and the security surface has not been formally audited. Treat every component as pre-production, review the code before deploying to sensitive systems, and expect rough edges as the project evolves.

## Prerequisites

- Linux with Video4Linux2 (V4L2) support and an infrared-capable webcam.
- Rust 1.80 or newer.
- Required kernel permissions to access `/dev/video*` devices.
- System libraries needed by the dlib face-recognition bindings (`libdlib-dev`, `libopenblas-dev`, and `liblapack-dev` on Debian/Ubuntu).
- Pretrained dlib face models (see [Face feature extraction](#face-feature-extraction)).

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

Capture a frame using default settings:

```bash
cargo run -p chissu-cli -- capture
```

Override device path, pixel format, and frame size:

```bash
cargo run -p chissu-cli -- capture \
  --device /dev/video2 \
  --pixel-format Y16 \
  --width 1280 \
  --height 720 \
  --exposure 120 \
  --gain 4
```

Let the camera negotiate exposure/gain automatically when the device supports it:

```bash
cargo run -p chissu-cli -- capture \
  --auto-exposure \
  --auto-gain
```

Request JSON output suitable for scripting:

```bash
cargo run -p chissu-cli -- capture --json
```

Example JSON payload:

```json
{
  "success": true,
  "output_path": "captures/ir-frame-20251026T120305.123Z.png",
  "device": {
    "driver": "uvcvideo",
    "card": "IR Cam",
    "bus_info": "usb-0000:00:14.0-3",
    "path": "/dev/video0"
  },
  "format": {
    "pixel_format": "Y16",
    "width": 640,
    "height": 480
  },
  "exposure": 120,
  "auto_exposure": "applied"
}
```

By default the CLI discards a handful of warm-up frames so auto exposure can settle before saving. Adjust this behavior with `--warmup-frames` if your device needs more (or fewer) frames to stabilize.

### Config-driven capture defaults

Operators who already maintain `/etc/chissu-pam/config.toml` for the PAM module can reuse the same file to provide CLI defaults. When `chissu-cli capture` is invoked without `--device`, `--pixel-format`, or `--warmup-frames`, the command now consults the config file (falling back to `/usr/local/etc/chissu-pam/config.toml`) before applying the built-in `/dev/video0`, `Y16`, and 4-frame defaults. Built-in defaults are logged explicitly so it is obvious when no config values were found.

Example snippet:

```toml
video_device = "/dev/video2"
pixel_format = "GREY"
warmup_frames = 10
```

With this file in place you can simply run `cargo run -p chissu-cli -- capture` and the CLI will capture from `/dev/video2` using the GREY pixel format while discarding 10 warm-up frames. Supplying CLI flags still wins over config values when you need to override a setting temporarily.

On failures the command prints a descriptive message to `stderr`. With `--json`, a structured error is emitted on `stdout` and diagnostic hints remain on `stderr`.

### Face feature extraction

Supply a PNG that contains one or more faces and the command will produce descriptor vectors suitable for downstream face recognition. The dlib models can be provided via CLI flags or environment variables.

Download the official models from https://dlib.net/files/ and keep track of their locations:

- `shape_predictor_68_face_landmarks.dat`
- `dlib_face_recognition_resnet_model_v1.dat`

Run the extractor and direct the descriptors to a file:

```bash
export DLIB_LANDMARK_MODEL=$HOME/models/shape_predictor_68_face_landmarks.dat
export DLIB_ENCODER_MODEL=$HOME/models/dlib_face_recognition_resnet_model_v1.dat

cargo run -p chissu-cli -- faces extract captures/sample.png --output captures/features/sample.json
```

You can override the model resolution per-invocation:

```bash
cargo run -p chissu-cli -- faces extract captures/sample.png \
  --landmark-model $HOME/models/shape_predictor_68_face_landmarks.dat \
  --encoder-model $HOME/models/dlib_face_recognition_resnet_model_v1.dat \
  --jitters 2
```

Human-readable output lists the detected faces, descriptor length, and the saved feature file. Structured runs honour the global `--json` switch and emit a payload similar to:

```json
{
  "success": true,
  "image_path": "captures/sample.png",
  "output_path": "captures/features/face-features-20251101T235959.123Z.json",
  "num_faces": 1,
  "faces": [
    {
      "bounding_box": { "left": 120, "top": 80, "right": 320, "bottom": 360 },
      "descriptor": [0.0123, 0.1042, 0.0831, 0.0987]
    }
  ],
  "landmark_model": "/home/user/models/shape_predictor_68_face_landmarks.dat",
  "encoder_model": "/home/user/models/dlib_face_recognition_resnet_model_v1.dat",
  "num_jitters": 1
}
```

The default output path is `./captures/features/face-features-<timestamp>.json`.

If you encounter build failures referencing `dlib/dnn.h`, install the system development headers mentioned above before running `cargo build` or `cargo test`.

### Face feature comparison

Re-use previously exported descriptor files to compute similarity scores without re-extracting features. Provide one input file and any number of comparison targets:

```bash
cargo run -p chissu-cli -- faces compare \
  --input captures/features/reference.json \
  --compare-target captures/features/candidate-01.json \
  --compare-target captures/features/candidate-02.json
```

Human-oriented output ranks the targets by cosine similarity and highlights the face indices that produced the best match:

```
Loaded 1 face(s) from captures/features/reference.json
Similarity metric: cosine
Target captures/features/candidate-01.json => cosine similarity 0.9234 (input face #0, target face #0)
Target captures/features/candidate-02.json => cosine similarity 0.8120 (input face #0, target face #1)
```

Pass `--json` to receive a machine-friendly array:

```bash
cargo run -p chissu-cli -- faces compare --input reference.json --compare-target candidate.json --json
```

```json
[
  {
    "target_path": "candidate.json",
    "best_similarity": 0.9234,
    "input_face_index": 0,
    "target_face_index": 0
  }
]
```

If any descriptor file is missing, unreadable, or contains no faces, the command aborts, prints an error to `stderr`, and exits with status code `2`.

### Face feature enrollment

Register descriptor vectors with a specific Linux user so the planned PAM module can perform facial authentication. Point the command at a descriptor JSON exported by `faces extract`:

```bash
cargo run -p chissu-cli -- faces enroll --user alice captures/features/reference.json
```

Each descriptor receives a unique identifier and is appended to `/var/lib/chissu-pam/models/alice.json` by default (created automatically with `0600` permissions). The store is a JSON array containing the descriptor vector, bounding box, source file, creation timestamp, and stable ID:

```json
[
  {
    "id": "4ac00b41-5f0d-4d2b-9a65-1bf01cb6cb4c",
    "descriptor": [0.0123, 0.1042, 0.0831, 0.0987],
    "bounding_box": { "left": 120, "top": 80, "right": 320, "bottom": 360 },
    "source": "captures/features/reference.json",
    "created_at": "2025-11-03T20:15:11.204Z"
  }
]
```

Pass `--json` to receive a payload that lists the generated descriptor IDs and the feature-store path. Use `--store-dir <path>` to override the storage directory explicitly. When the flag is omitted, the CLI reads `descriptor_store_dir` from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml`), then consults the `CHISSU_PAM_STORE_DIR` environment variable, and finally falls back to the built-in `/var/lib/chissu-pam/models/` location.

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

The command reports the IDs that were deleted and the number of descriptors that remain. With `--json` it emits a structured summary containing `removed_ids`, `remaining`, and the target store path. Attempting to delete an unknown ID exits with status code `4`, leaving the store unchanged. Using `--all` deletes the store file entirely (or treats the operation as a no-op when the user has no enrolled descriptors).

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
- Syslog (facility `AUTHPRIV`) records start, success, timeout, and error events. Review output with `journalctl -t pam_chissu` or `journalctl SYSLOG_IDENTIFIER=pam_chissu`.
- The module honours `DLIB_LANDMARK_MODEL` and `DLIB_ENCODER_MODEL` (or config entries with the same names) to locate dlib model files.

See [`docs/pam-auth.md`](docs/pam-auth.md) for installation walkthroughs, configuration examples, and troubleshooting tips.

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
