# PAM Facial Authentication Module

The `pam-chissu` crate (located under `crates/pam-chissu/`) produces `libpam_chissu.so`, a PAM authentication module that accepts a user only when a live camera capture matches facial embeddings previously enrolled with `chissu-cli faces enroll`.

## Build

```bash
# Build the shared library
cargo build --release -p pam-chissu

# Run unit tests (mocks only; no webcam required)
cargo test -p pam-chissu
```

The compiled module is located at `target/release/libpam_chissu.so`; copy it directly into `/lib/security/`:

```bash
sudo install -m 0644 target/release/libpam_chissu.so /lib/security/libpam_chissu.so
```

## Installation overview

1. Copy the shared library into your PAM module directory (usually `/lib/security/` on Debian/Ubuntu):
   ```bash
   sudo install -m 0644 target/release/libpam_chissu.so /lib/security/libpam_chissu.so
   ```
2. Configure the service stack (example for `login`):
   ```pam
   # /etc/pam.d/login
   auth sufficient libpam_chissu.so
   auth include system-local-login
   ```
   Place `libpam_chissu.so` near the top so a successful match shortcuts the stack. Use `required` instead of `sufficient` if you prefer to keep password fallback.
3. Ensure `faces enroll` has populated `/var/lib/chissu-pam/models/<user>.json` for every user that should pass facial authentication. You can now run `chissu-cli enroll` to capture a frame, extract embeddings, and store them in one command. It inherits the configured video device/pixel format *and* `landmark_model`/`encoder_model` paths, defaults the target user to the invoking account, and only honors `--user <name>` overrides when executed as `root`.
4. Restart services or daemons that cache PAM state if necessary (e.g., `systemctl restart sshd`).

## Configuration

The module reads configuration from `/etc/chissu-pam/config.toml`. If the file is absent it falls back to `/usr/local/etc/chissu-pam/config.toml`. Both files are optional—defaults are used when neither exists. Developers adding new keys or validations must update `crates/chissu-config`, which is the shared loader used by both the PAM module and `chissu-cli`. Available keys:

```toml
similarity_threshold = 0.75     # Float, default 0.7
capture_timeout_secs = 8        # Integer seconds, default 5
frame_interval_millis = 300     # Integer ms between samples, default 500
video_device = "/dev/video2"   # String, default "/dev/video0"
embedding_store_dir = "/srv/face-store"  # Path, default "/var/lib/chissu-pam/models"
pixel_format = "Y16"            # V4L2 fourcc, default "Y16"
warmup_frames = 2               # Discarded per-sample warm-up frames, default 0
jitters = 2                     # Dlib jitter passes, default 1
require_secret_service = false  # Opt-in to enforcing keyring availability before capture
landmark_model = "/opt/dlib/shape_predictor_68_face_landmarks.dat"
encoder_model = "/opt/dlib/dlib_face_recognition_resnet_model_v1.dat"
```

Configuration precedence: `/etc/chissu-pam/config.toml` → `/usr/local/etc/chissu-pam/config.toml` → built-in defaults.

If model paths are omitted, the module falls back to the `DLIB_LANDMARK_MODEL` and `DLIB_ENCODER_MODEL` environment variables (the same convention as the CLI).

`chissu-cli faces enroll` and `faces remove` read `embedding_store_dir` when `--store-dir` is not provided, so CLI enroll/remove operations automatically target the directory configured for the PAM module.

## Secret Service prerequisite

`pam_chissu` now verifies Secret Service access by forking a helper child that drops privileges to the PAM target user (`initgroups` + `setgid` + `setuid`) and talks to the user's D-Bus session. The helper exchanges JSON with the parent over a pipe/socketpair and returns one of three statuses:

- `{"status":"ok","embedding_key":"<base64>"}` — a 32-byte AES-GCM embedding key encoded as Base64. The parent logs success and proceeds to camera capture.
- `{"status":"missing","message":"..."}` — no key is stored for that user/service. The parent maps this to the existing "no embeddings" flow so PAM returns `PAM_AUTH_ERR` with the usual messaging.
- `{"status":"error","kind":"secret_service_unavailable","message":"..."}` — Secret Service is locked, missing, or refused the request. The parent logs the helper message, notifies the terminal, and returns `PAM_IGNORE` so downstream modules (password, hardware tokens, etc.) can continue.

Populate the key with any Secret Service frontend (for example `secret-tool store --label 'Chissu embedding key' service chissu-pam user alice` followed by pasting a 32-byte Base64 string). The helper trims whitespace, accepts padded or unpadded Base64, and rejects other encodings.

Run `chissu-cli keyring check` (add `--json` for machine parsing) to confirm the current shell session can reach Secret Service before enabling the PAM guard. Flip `require_secret_service = true` once keys are provisioned; it defaults to `false` for compatibility with headless or console-only setups.

`chissu-cli faces enroll` now performs the full key lifecycle: it decrypts any existing store, generates a fresh 32-byte AES-256-GCM key, registers the key in Secret Service, and writes the updated embedding store in encrypted form. Each subsequent enrollment repeats the rotation so compromised keys cannot decrypt newly written data. `faces remove` and `faces remove --all` reuse the currently registered key when they rewrite the store, keeping PAM and the helper in sync without unnecessary rotations.

Troubleshooting tips:

- Ensure a session bus and `gnome-keyring-daemon` (or compatible Secret Service implementation) are running for the target user before PAM attempts begin.
- When testing via `pamtester` or SSH, forward the DBus session variables (e.g., `DBUS_SESSION_BUS_ADDRESS`) or rely on a display manager that exports them automatically.
- Review `journalctl -t pam_chissu` for messages such as `Secret Service helper returned embedding key (...)` or `Embedding key missing for user ...` to confirm the helper outcome. Errors prefixed with `Secret Service unavailable` indicate the guard short-circuited with `PAM_IGNORE`.

## Runtime behaviour

- The module opens the configured V4L2 device for each authentication attempt and captures frames until either:
  - An embedding meets or exceeds `similarity_threshold` (returns `PAM_SUCCESS`).
  - `capture_timeout_secs` elapses (returns `PAM_AUTH_ERR`).
- Frames are sampled at intervals governed by `frame_interval_millis` (sleep is skipped when the remaining time is smaller than the interval).
- Embeddings are compared only against the file for the PAM target user (`/var/lib/.../<user>.json`). Missing or empty stores produce `PAM_AUTH_ERR`.
- All notable events are emitted via syslog (`AUTHPRIV` facility) with identifier `pam_chissu`. Inspect them with `journalctl -t pam_chissu`.
- When the PAM stack exposes a conversation callback, the module mirrors those events interactively: successful matches emit a `PAM_TEXT_INFO` banner, while retries (no face yet) and failures send `PAM_ERROR_MSG` guidance so terminal users know whether to stay in frame or re-run the command.
- Operational errors (configuration parse, camera I/O, model load) are reported as `PAM_SYSTEM_ERR`. The message includes the failing step for easier triage.

## Testing without hardware

Unit tests cover:
- Configuration defaults and parsing (`cargo test -p pam-chissu`).
- Embedding length validation.
- Cosine similarity ranking.

Hardware-free integration tests are not included yet; the module expects a real camera for end-to-end verification. For CI, keep the PAM module out of the critical authentication path and rely on these mocked unit tests.

## Manual verification checklist

1. Enroll embeddings for a test user (`faces enroll --user testuser <embedding.json>`).
2. Confirm `/var/lib/chissu-pam/models/testuser.json` exists and contains at least one embedding.
3. Prepare `/etc/chissu-pam/config.toml` with the desired device path and threshold.
4. Enable the PAM module for a non-critical service (e.g., create `/etc/pam.d/chissu-test` referencing only `libpam_chissu.so`).
5. Use `pamtester` or `su testuser -s /bin/bash` to initiate authentication. Watch `journalctl -f -t pam_chissu` for log entries:
   - `Starting face authentication...`
   - `Detected matching embedding...` or `Authentication failed: ...`
6. Cover failure conditions by obscuring the camera or removing embeddings; the module should emit a warning and return `PAM_AUTH_ERR`.

## Troubleshooting

| Symptom | Likely cause | Suggested action |
|---------|--------------|------------------|
| `Authentication aborted: pam_get_user failed` | PAM conversation did not supply a user name | Verify the PAM stack order and ensure `pam_unix.so` precedes the module when user prompting is required. |
| `Embedding length mismatch` | Enrolled embeddings were generated with a different model | Re-run `faces enroll` with consistent model versions. |
| `Failed to capture frame: device capability error` | Wrong `video_device` or insufficient permissions | Update the device path or adjust udev permissions so the PAM service can access the camera. |
| No syslog output | Syslog socket unavailable (e.g., chroot) | Check `/dev/log` availability or use `syslog::unix_custom` with a custom socket path. |

## Security notes

- Keep embedding stores protected (`0600` is enforced during writes). Apply discretionary access controls if `/var/lib/chissu-pam/models` is relocated.
- Threshold tuning is critical: too low allows false positives, too high increases lockouts.
- Consider combining the module with a secondary factor (password, token) using the PAM control flags appropriate for your deployment.
- Monitor syslog for repeated failures—excessive timeouts may indicate camera faults or attempts to spoof the sensor.
