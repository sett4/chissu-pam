# Release Notes

## Unreleased
- `pam-chissu` now recovers Secret Service helper environment for both X11 and Wayland sessions. The new `secret_service_session = "auto"` config default detects the session type from logind, with `"x11"` and `"wayland"` available as explicit overrides for unusual desktop stacks.
- Added a user guide for troubleshooting `polkit-agent-helper@.service` sandbox issues when 1Password or other polkit prompts cannot reach the user's Secret Service bus or configured camera device.

## 2025-11-09
- `chissu-cli capture` now reads `video_device`, `pixel_format`, and `warmup_frames` from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml`) whenever the corresponding CLI flags are omitted. The command logs when it falls back to the built-in `/dev/video0`, `Y16`, and 4 warm-up frame defaults so operators can tell when config values are missing.
