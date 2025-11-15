# chissu-cli Usage Reference

This document expands on the core `chissu-cli` workflows referenced in the README. It covers capture, configuration-driven defaults, face feature extraction, and similarity comparison commands with both human-readable and JSON output illustrations.

## Capture infrared frames

Capture a frame using default settings:

```bash
chissu-cli capture
```

Override device path, pixel format, and frame size:

```bash
chissu-cli capture \
  --device /dev/video2 \
  --pixel-format Y16 \
  --width 1280 \
  --height 720 \
  --exposure 120 \
  --gain 4
```

Let the camera negotiate exposure/gain automatically when the device supports it:

```bash
chissu-cli capture \
  --auto-exposure \
  --auto-gain
```

Request JSON output suitable for scripting:

```bash
chissu-cli capture --json
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

## Config-driven capture defaults

Operators who already maintain `/etc/chissu-pam/config.toml` for the PAM module can reuse the same file to provide CLI defaults. When `chissu-cli capture` is invoked without `--device`, `--pixel-format`, or `--warmup-frames`, the command consults the config file (and falls back to `/usr/local/etc/chissu-pam/config.toml`) before applying the built-in `/dev/video0`, `Y16`, and 4-frame defaults. Built-in defaults are logged explicitly so it is obvious when no config values were found.

Example snippet:

```toml
video_device = "/dev/video2"
pixel_format = "GREY"
warmup_frames = 10
```

With this file in place you can simply run `chissu-cli capture` and the CLI will capture from `/dev/video2` using the GREY pixel format while discarding 10 warm-up frames. Supplying CLI flags still wins over config values when you need to override a setting temporarily.

> Both `chissu-cli` and the PAM module load this file via `crates/chissu-config`, so add any new keys or validation to that crate to keep every binary in sync.

On failures the command prints a descriptive message to `stderr`. With `--json`, a structured error is emitted on `stdout` and diagnostic hints remain on `stderr`.

## Face feature extraction

Supply a PNG that contains one or more faces and the command will produce embedding vectors suitable for downstream face recognition. The dlib models can be provided via CLI flags or environment variables.

Download the official models from https://dlib.net/files/ and keep track of their locations:

- `shape_predictor_68_face_landmarks.dat`
- `dlib_face_recognition_resnet_model_v1.dat`

Run the extractor and direct the embeddings to a file:

```bash
export DLIB_LANDMARK_MODEL=$HOME/models/shape_predictor_68_face_landmarks.dat
export DLIB_ENCODER_MODEL=$HOME/models/dlib_face_recognition_resnet_model_v1.dat

chissu-cli faces extract captures/sample.png --output captures/features/sample.json
```

You can override the model resolution per invocation:

```bash
chissu-cli faces extract captures/sample.png \
  --landmark-model $HOME/models/shape_predictor_68_face_landmarks.dat \
  --encoder-model $HOME/models/dlib_face_recognition_resnet_model_v1.dat \
  --jitters 2
```

Human-readable output lists the detected faces, embedding length, and the saved feature file. Structured runs honour the global `--json` switch and emit a payload similar to:

```json
{
  "success": true,
  "image_path": "captures/sample.png",
  "output_path": "captures/features/face-features-20251101T235959.123Z.json",
  "num_faces": 1,
  "faces": [
    {
      "bounding_box": { "left": 120, "top": 80, "right": 320, "bottom": 360 },
      "embedding": [0.0123, 0.1042, 0.0831, 0.0987]
    }
  ],
  "landmark_model": "/home/user/models/shape_predictor_68_face_landmarks.dat",
  "encoder_model": "/home/user/models/dlib_face_recognition_resnet_model_v1.dat",
  "num_jitters": 1
}
```

The default output path is `./captures/features/face-features-<timestamp>.json`.

If you encounter build failures referencing `dlib/dnn.h`, install the system development headers mentioned in the README before running `cargo build` or `cargo test`.

## Face feature comparison

Re-use previously exported embedding files to compute similarity scores without re-extracting features. Provide one input file and any number of comparison targets:

```bash
chissu-cli faces compare \
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
chissu-cli faces compare --input reference.json --compare-target candidate.json --json
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

If any embedding file is missing, unreadable, or contains no faces, the command aborts, prints an error to `stderr`, and exits with status code `2`.
