# Polkit Agent Helper Troubleshooting

This guide covers `pam-chissu` failures that appear only when authentication is
started through polkit-based desktop prompts, such as 1Password system unlock.
The usual pattern is:

- lock screen authentication works
- `sudo` authentication works
- 1Password or another polkit prompt falls back to password or fails

On newer Linux desktop systems, `polkit-agent-helper@.service` may run with a
strict systemd sandbox. That sandbox can hide the user session bus and camera
device from the PAM module even after `pam_chissu` correctly drops privileges to
the target user.

## Confirm The Failure

Inspect the PAM logs:

```bash
journalctl -t pam_chissu --since today --no-pager
```

You can also inspect the polkit helper unit:

```bash
systemctl cat polkit-agent-helper@.service
```

Look for sandboxing options such as:

```ini
ProtectHome=yes
PrivateDevices=yes
DevicePolicy=strict
```

These options are useful hardening, but they can block `pam_chissu` from reaching
resources it needs for face authentication.

## Secret Service Bus Is Blocked

### Symptom

The log contains a Secret Service or DBus error like:

```text
DBus session bus preflight failed for /run/user/1000/bus: Permission denied
```

or:

```text
Failed to connect to socket /run/user/1000/bus: Permission denied
```

This means `pam_chissu` recovered the correct session bus address, but the
polkit helper service cannot access `/run/user/<uid>/bus`.

### Recommended Override

Create a systemd drop-in:

```bash
sudo systemctl edit polkit-agent-helper@.service
```

Add:

```ini
[Service]
ProtectHome=tmpfs
BindReadOnlyPaths=/run/user
```

Then reload systemd:

```bash
sudo systemctl daemon-reload
systemctl cat polkit-agent-helper@.service
```

`ProtectHome=tmpfs` keeps `/home`, `/root`, and `/run/user` hidden by default.
`BindReadOnlyPaths=/run/user` adds back the runtime directory needed to reach
the user's session bus. The bus socket remains protected by normal Unix
permissions, so the helper must still drop to the target user before it can use
that user's bus.

## Camera Device Is Hidden

### Symptom

After the Secret Service step succeeds, the log contains:

```text
Secret Service helper returned embedding key ... proceeding
Failed to capture frame: failed to open video device /dev/videoX: No such file or directory
```

This usually means `PrivateDevices=yes` or `DevicePolicy=strict` hides the V4L2
device from `polkit-agent-helper@.service`.

### Recommended Override

Add the configured camera device to the same drop-in. Replace `/dev/video2` with
the `video_device` from `/etc/chissu-pam/config.toml`.

```ini
[Service]
ProtectHome=tmpfs
BindReadOnlyPaths=/run/user
BindPaths=/dev/video2
DeviceAllow=/dev/video2 rw
```

Reload systemd:

```bash
sudo systemctl daemon-reload
systemctl cat polkit-agent-helper@.service
```

Retry the polkit flow, such as unlocking 1Password.

## Broader Fallback

If the device is still unavailable after `BindPaths=` and `DeviceAllow=`, the
service may need a broader device namespace exception:

```ini
[Service]
ProtectHome=tmpfs
BindReadOnlyPaths=/run/user
PrivateDevices=no
DeviceAllow=/dev/video2 rw
```

Use this only after the narrower override fails. `PrivateDevices=no` exposes
more of `/dev` to the polkit helper service.

## Verification Checklist

1. Confirm the effective unit contains your drop-in:

   ```bash
   systemctl cat polkit-agent-helper@.service
   ```

2. Confirm the camera path matches your config:

   ```bash
   rg '^video_device' /etc/chissu-pam/config.toml
   ls -l /dev/video2
   ```

3. Retry the desktop prompt.

4. Inspect `pam_chissu` logs:

   ```bash
   journalctl -t pam_chissu --since today --no-pager
   ```

Expected successful flow:

```text
Recovered Secret Service session environment from logind ...
Secret Service helper returned embedding key ...
Captured frame 1 from /dev/videoX ...
Authentication success ...
```

## Security Notes

These overrides relax the sandbox for `polkit-agent-helper@.service`.

- `BindReadOnlyPaths=/run/user` lets the helper see user runtime directories.
  Per-user directory permissions still restrict access to the target user's
  session bus.
- `BindPaths=/dev/videoX` and `DeviceAllow=/dev/videoX rw` expose the configured
  camera device to the helper.
- `PrivateDevices=no` is broader and should be treated as a last resort.

Keep the override as narrow as your deployment allows, and prefer one specific
camera device over exposing all video devices.
