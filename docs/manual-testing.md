# Manual Infrared Capture Verification

Follow this checklist to validate the CLI against a real infrared-capable webcam.

## Setup

1. Confirm the device node with `v4l2-ctl --list-devices` and note the `/dev/videoX` path.
2. Ensure no other process is streaming from the camera.
3. Prepare a low-light or IR-illuminated scene so that the frame shows contrast.

## Capability sanity check

```bash
cargo run -- capture --device /dev/video0 --pixel-format Y16 --width 640 --height 480 --json
```

- Expect a JSON payload with `"success": true` and the negotiated format fields populated.
- If the device lacks the requested pixel format, the CLI exits with code `2` and an explanatory error on `stderr`.

## Exposure and gain tuning

```bash
cargo run -- capture \
  --device /dev/video0 \
  --pixel-format Y16 \
  --width 640 --height 480 \
  --exposure 160 --gain 8
```

- The logs should include messages such as `Set exposure to 160` or `Exposure control not supported`.
- Inspect the saved PNG under `./captures/` to confirm brightness adjustments.

To defer to the camera's automatic controls when available:

```bash
cargo run -- capture \
  --device /dev/video0 \
  --pixel-format Y16 \
  --auto-exposure \
  --auto-gain
```

- Expect logs like `Enabled auto exposure` / `Enabled auto gain`. If a control is missing, the CLI prints `Auto exposure control not supported` and continues with manual values.
- When auto is active, manual `--exposure`/`--gain` settings are skipped to avoid conflicting changes.

## Failure modes

- Intentionally request an unsupported pixel format (e.g. `--pixel-format MJPG`). Verify the CLI exits with code `2`, prints the unsupported-format error, and does **not** create a file.
- Disconnect the camera mid-run to confirm the CLI reports an I/O error and exits with code `1`.

## Cleanup

- Remove temporary captures if needed: `rm captures/*.png`.
- Re-run `cargo test` to ensure automated checks still pass after manual scenarios.
