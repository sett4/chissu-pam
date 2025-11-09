# Release Notes

## 2025-11-09
- `chissu-cli capture` now reads `video_device`, `pixel_format`, and `warmup_frames` from `/etc/chissu-pam/config.toml` (falling back to `/usr/local/etc/chissu-pam/config.toml`) whenever the corresponding CLI flags are omitted. The command logs when it falls back to the built-in `/dev/video0`, `Y16`, and 4 warm-up frame defaults so operators can tell when config values are missing.
