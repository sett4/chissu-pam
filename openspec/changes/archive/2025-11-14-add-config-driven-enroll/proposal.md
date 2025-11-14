## Summary

`chissu-cli enroll` will be a new one-shot command that loads the existing PAM/capture configuration, captures an infrared frame, runs face detection + descriptor extraction, and immediately invokes the encrypted enrollment flow. It defaults to enrolling the invoking Unix user, only honoring `--user <name>` when the process runs as root so unattended scripts can provision other accounts safely.

## Motivation

- Operators must currently chain `capture`, `faces extract`, and `faces enroll` manually, forcing temporary files, repeated flags, and error-prone sequencing.
- Many deployments already describe capture parameters inside `/etc/chissu-pam/config.toml`; duplicating those flags on every CLI invocation leads to drift between PAM and enrollment settings.
- Help-desk or kiosk scenarios need a turnkey “look at the camera to enroll yourself” experience without exposing descriptor files on disk.

## Goals

- Provide a config-driven enrollment command that captures a frame, extracts descriptors, and saves them through the AES-GCM workflow defined for `faces enroll`.
- Reuse the capture defaults/config order from `capture-cli` (config → built-ins) so video device, pixel format, and warm-up frames stay consistent with PAM expectations.
- Default the target user to the invoking account; allow `--user <name>` only for UID 0 so root automation can still seed other users.
- Produce both human-readable and `--json` summaries that mention the captured frame location (if preserved), descriptor count, generated IDs, and feature-store path.

## Non-Goals

- Replacing the existing `faces enroll` command; it remains available for workflows that already have descriptor JSON files.
- Changing the descriptor format, encryption schema, or PAM helper protocol.
- Providing a GUI or automated face selection heuristics beyond the current detector.

## Risks & Mitigations

- **Capture failures or no detected faces**: abort before touching the store and optionally keep the failed capture path in logs/JSON for troubleshooting.
- **Temporary capture artifacts leak sensitive data**: store captures under OS temporary directory with `0600` permissions and delete them after successful enrollment unless `--keep-image` (future extension) is used.
- **Privilege confusion**: enforce UID checks inside the command so only root can override the target user, and log the resolved user in both output modes.

## Success Metrics

- Running `chissu-cli enroll` as an unprivileged user captures a frame using the configured device, extracts descriptors, and appends them to the user’s encrypted store without any intermediate JSON files.
- Running `sudo chissu-cli enroll --user alice` updates `/var/lib/chissu-pam/models/alice.json` while rotating the Secret Service key as defined in the existing requirement.
- CI/integration tests cover the pipeline end-to-end with fixture captures, including failures for non-root `--user` attempts and missing faces.
