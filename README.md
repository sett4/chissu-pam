# study-rust-v4l2

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
