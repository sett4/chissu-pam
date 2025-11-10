# PAM Facial Authentication Module

The `pam-chissu` crate (located under `crates/pam-chissu/`) produces `pam_chissu.so`, a PAM authentication module that accepts a user only when a live camera capture matches facial descriptors previously enrolled with `chissu-cli faces enroll`.

## Build

```bash
# Build the shared library (generates pam_chissu.so plus legacy symlink)
cargo build --release -p pam-chissu

# Run unit tests (mocks only; no webcam required)
cargo test -p pam-chissu
```

The compiled module is located at `target/release/pam_chissu.so`. A compatibility symlink `libpam_chissuauth.so -> pam_chissu.so` is emitted in the same directory for one release cycle so existing deployment scripts continue to work.

## Installation overview

1. Copy the shared library into your PAM module directory (usually `/lib/security/` on Debian/Ubuntu):
   ```bash
   sudo install -m 0644 target/release/pam_chissu.so /lib/security/
   ```
2. Configure the service stack (example for `login`):
   ```pam
   # /etc/pam.d/login
   auth sufficient pam_chissu.so
   auth include system-local-login
   ```
   Place `pam_chissu.so` near the top so a successful match shortcuts the stack. Use `required` instead of `sufficient` if you prefer to keep password fallback.
3. Ensure `faces enroll` has populated `/var/lib/chissu-pam/models/<user>.json` for every user that should pass facial authentication.
4. Restart services or daemons that cache PAM state if necessary (e.g., `systemctl restart sshd`).

## Configuration

The module reads configuration from `/etc/chissu-pam/config.toml`. If the file is absent it falls back to `/usr/local/etc/chissu-pam/config.toml`. Both files are optional—defaults are used when neither exists. Available keys:

```toml
similarity_threshold = 0.75     # Float, default 0.7
capture_timeout_secs = 8        # Integer seconds, default 5
frame_interval_millis = 300     # Integer ms between samples, default 500
video_device = "/dev/video2"   # String, default "/dev/video0"
descriptor_store_dir = "/srv/face-store"  # Path, default "/var/lib/chissu-pam/models"
pixel_format = "Y16"            # V4L2 fourcc, default "Y16"
warmup_frames = 2               # Discarded per-sample warm-up frames, default 0
jitters = 2                     # Dlib jitter passes, default 1
landmark_model = "/opt/dlib/shape_predictor_68_face_landmarks.dat"
encoder_model = "/opt/dlib/dlib_face_recognition_resnet_model_v1.dat"
```

Configuration precedence: `/etc/chissu-pam/config.toml` → `/usr/local/etc/chissu-pam/config.toml` → built-in defaults.

If model paths are omitted, the module falls back to the `DLIB_LANDMARK_MODEL` and `DLIB_ENCODER_MODEL` environment variables (the same convention as the CLI).

`chissu-cli faces enroll` and `faces remove` read the same `descriptor_store_dir` key when `--store-dir` is not provided, so CLI enroll/remove operations automatically target the directory configured for the PAM module.

## Runtime behaviour

- The module opens the configured V4L2 device for each authentication attempt and captures frames until either:
  - A descriptor meets or exceeds `similarity_threshold` (returns `PAM_SUCCESS`).
  - `capture_timeout_secs` elapses (returns `PAM_AUTH_ERR`).
- Frames are sampled at intervals governed by `frame_interval_millis` (sleep is skipped when the remaining time is smaller than the interval).
- Descriptors are compared only against the file for the PAM target user (`/var/lib/.../<user>.json`). Missing or empty stores produce `PAM_AUTH_ERR`.
- All notable events are emitted via syslog (`AUTHPRIV` facility) with identifier `pam_chissu`. Inspect them with `journalctl -t pam_chissu`.
- Operational errors (configuration parse, camera I/O, model load) are reported as `PAM_SYSTEM_ERR`. The message includes the failing step for easier triage.

## Testing without hardware

Unit tests cover:
- Configuration defaults and parsing (`cargo test -p pam-chissu`).
- Descriptor length validation.
- Cosine similarity ranking.

Hardware-free integration tests are not included yet; the module expects a real camera for end-to-end verification. For CI, keep the PAM module out of the critical authentication path and rely on these mocked unit tests.

## Manual verification checklist

1. Enroll descriptors for a test user (`faces enroll --user testuser <descriptor.json>`).
2. Confirm `/var/lib/chissu-pam/models/testuser.json` exists and contains at least one descriptor.
3. Prepare `/etc/chissu-pam/config.toml` with the desired device path and threshold.
4. Enable the PAM module for a non-critical service (e.g., create `/etc/pam.d/chissu-test` referencing only `pam_chissu.so`).
5. Use `pamtester` or `su testuser -s /bin/bash` to initiate authentication. Watch `journalctl -f -t pam_chissu` for log entries:
   - `Starting face authentication...`
   - `Detected matching descriptor...` or `Authentication failed: ...`
6. Cover failure conditions by obscuring the camera or removing descriptors; the module should emit a warning and return `PAM_AUTH_ERR`.

## Troubleshooting

| Symptom | Likely cause | Suggested action |
|---------|--------------|------------------|
| `Authentication aborted: pam_get_user failed` | PAM conversation did not supply a user name | Verify the PAM stack order and ensure `pam_unix.so` precedes the module when user prompting is required. |
| `Descriptor length mismatch` | Enrolled descriptors were generated with a different model | Re-run `faces enroll` with consistent model versions. |
| `Failed to capture frame: device capability error` | Wrong `video_device` or insufficient permissions | Update the device path or adjust udev permissions so the PAM service can access the camera. |
| No syslog output | Syslog socket unavailable (e.g., chroot) | Check `/dev/log` availability or use `syslog::unix_custom` with a custom socket path. |

## Security notes

- Keep descriptor stores protected (`0600` is enforced during writes). Apply discretionary access controls if `/var/lib/chissu-pam/models` is relocated.
- Threshold tuning is critical: too low allows false positives, too high increases lockouts.
- Consider combining the module with a secondary factor (password, token) using the PAM control flags appropriate for your deployment.
- Monitor syslog for repeated failures—excessive timeouts may indicate camera faults or attempts to spoof the sensor.
