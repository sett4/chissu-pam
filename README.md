# chissu-pam

A teaching-oriented CLI written in Rust that captures a single infrared frame from a V4L2-compatible webcam and now extracts reusable facial descriptors from existing PNG images. The tool validates device capabilities, negotiates an infrared-friendly pixel format, and saves the frame under `./captures/`. It supports both human-readable logging and a JSON summary for automated workflows.

## Prerequisites

- Linux with Video4Linux2 (V4L2) support and an infrared-capable webcam.
- Rust 1.80 or newer.
- Required kernel permissions to access `/dev/video*` devices.
- System libraries needed by the dlib face-recognition bindings (`libdlib-dev`, `libopenblas-dev`, and `liblapack-dev` on Debian/Ubuntu).
- Pretrained dlib face models (see [Face feature extraction](#face-feature-extraction)).

## Building

```bash
cargo build
```

## Usage

Capture a frame using default settings:

```bash
cargo run -- capture
```

Override device path, pixel format, and frame size:

```bash
cargo run -- capture \
  --device /dev/video2 \
  --pixel-format Y16 \
  --width 1280 \
  --height 720 \
  --exposure 120 \
  --gain 4
```

Let the camera negotiate exposure/gain automatically when the device supports it:

```bash
cargo run -- capture \
  --auto-exposure \
  --auto-gain
```

Request JSON output suitable for scripting:

```bash
cargo run -- capture --json
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

Operators who already maintain `/etc/chissu-pam/config.toml` for the PAM module can reuse the same file to provide CLI defaults. When `chissu-pam capture` is invoked without `--device`, `--pixel-format`, or `--warmup-frames`, the command now consults the config file (falling back to `/usr/local/etc/chissu-pam/config.toml`) before applying the built-in `/dev/video0`, `Y16`, and 4-frame defaults. Built-in defaults are logged explicitly so it is obvious when no config values were found.

Example snippet:

```toml
video_device = "/dev/video2"
pixel_format = "GREY"
warmup_frames = 10
```

With this file in place you can simply run `cargo run -- capture` and the CLI will capture from `/dev/video2` using the GREY pixel format while discarding 10 warm-up frames. Supplying CLI flags still wins over config values when you need to override a setting temporarily.

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

cargo run -- faces extract captures/sample.png --output captures/features/sample.json
```

You can override the model resolution per-invocation:

```bash
cargo run -- faces extract captures/sample.png \
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
cargo run -- faces compare \
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
cargo run -- faces compare --input reference.json --compare-target candidate.json --json
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
cargo run -- faces enroll --user alice captures/features/reference.json
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
auth_id=$(cargo run -- faces enroll --user alice captures/features/reference.json --json | jq -r ".added[0].id")
cargo run -- faces remove --user alice --descriptor-id "$auth_id"

# Remove every descriptor for a user
cargo run -- faces remove --user alice --all

# Work against a non-default store directory
cargo run -- faces enroll --user alice --store-dir ./captures/enrolled captures/features/reference.json
cargo run -- faces remove --user alice --descriptor-id "$auth_id" --store-dir ./captures/enrolled
```

The command reports the IDs that were deleted and the number of descriptors that remain. With `--json` it emits a structured summary containing `removed_ids`, `remaining`, and the target store path. Attempting to delete an unknown ID exits with status code `4`, leaving the store unchanged. Using `--all` deletes the store file entirely (or treats the operation as a no-op when the user has no enrolled descriptors).

When neither command receives `--store-dir`, they inherit the same precedence chain described for enrollment (config files, then `CHISSU_PAM_STORE_DIR`, then the built-in path), keeping CLI operations aligned with the PAM module configuration.

### PAM facial authentication

The repository now ships a PAM module (`pam_chissu.so`) that authenticates Linux users by comparing a live camera capture with descriptors enrolled via `faces enroll`.

- Build the shared library with `cargo build --release -p pam-chissu` (or `cargo test -p pam-chissu` during development).
- Place the resulting `target/release/pam_chissu.so` under `/lib/security/` (or your distribution’s PAM module directory), then update `/etc/pam.d/<service>` to include `auth sufficient pam_chissu.so` in the desired stack. A compatibility symlink `libpam_chissuauth.so -> pam_chissu.so` is produced under `target/<profile>/` for packagers that still expect the historical name.
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
cargo test
```

Mocked frame data is used so tests do not require live hardware.

`cargo test` requires the dlib headers and libraries listed in the prerequisites. Without them the build for `dlib-face-recognition` will fail during compilation.

## Manual verification with hardware

Run through the checklist in [`docs/manual-testing.md`](docs/manual-testing.md) when validating with a physical infrared camera. The document covers capability expectations, recommended exposure/gain values, and example commands for both human and JSON output modes.
