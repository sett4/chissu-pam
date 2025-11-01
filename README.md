# study-rust-v4l2

A teaching-oriented CLI written in Rust that captures a single infrared frame from a V4L2-compatible webcam. The tool validates device capabilities, negotiates an infrared-friendly pixel format, and saves the frame under `./captures/`. It supports both human-readable logging and a JSON summary for automated workflows.

## Prerequisites

- Linux with Video4Linux2 (V4L2) support and an infrared-capable webcam.
- Rust 1.80 or newer.
- Required kernel permissions to access `/dev/video*` devices.

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

## Testing

Automated tests exercise frame conversion, JSON serialization, and filesystem handling:

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

Mocked frame data is used so tests do not require live hardware.

## Manual verification with hardware

Run through the checklist in [`docs/manual-testing.md`](docs/manual-testing.md) when validating with a physical infrared camera. The document covers capability expectations, recommended exposure/gain values, and example commands for both human and JSON output modes.
