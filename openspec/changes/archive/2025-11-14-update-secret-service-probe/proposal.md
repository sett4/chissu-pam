## Summary
Root-executed `pam_chissu` currently invokes `ensure_secret_service_available` directly and fails whenever the GNOME Secret Service session is tied to the target user's D-Bus socket. We will introduce a user-impersonating helper process that forks from the PAM module, drops privileges to the target UID via `setuid`, and performs the Secret Service probe plus descriptor key lookup on that user's session bus. Parent and child will exchange JSON messages over a pipe so the helper can return success, key material, or typed error responses.

## Motivation
- Face-auth logins always abort when `require_secret_service = true` because the module runs as root without access to the user session bus.
- Operators currently disable Secret Service gating, defeating the purpose of AES-GCM descriptor protection.
- A fork+setuid helper keeps the core module privileged for camera access while letting the Secret Service call run with the same privileges D-Bus expects.

## Goals
- Run Secret Service probes and descriptor key retrieval in a child that has switched to the PAM target UID and environment.
- Communicate between parent and child using a JSON envelope so we can transmit keys and structured failure reasons.
- Return explicit outcomes: key found (with AES-GCM key bytes), key missing, Secret Service unavailable/locked, unexpected IPC failure.
- Map helper outcomes to PAM return codes per existing requirement (success → continue capture, unavailable → `PAM_IGNORE`, missing key → descriptor-missing flow, IPC failure → `PAM_SYSTEM_ERR`).

## Non-Goals
- Replacing the existing keyring crate; the helper will keep using `chissu-face-core` abstractions.
- Implementing asynchronous IPC or long-running daemons; the helper lives only for the authentication attempt.
- Changing how descriptors are stored or encrypted today.

## Risks & Mitigations
- **setuid failure or prohibited UIDs**: detect errors after fork, log them, return `PAM_SYSTEM_ERR` without touching the camera pipeline.
- **Deadlocks or hanging helpers**: use timeouts in the parent read loop and terminate/`SIGKILL` the child if it stalls; guard by closing the pipe.
- **JSON parsing errors**: validate with serde-containing integration tests to ensure round trips for every response type.

## Success Metrics
- PAM authenticates with `require_secret_service=true` on a GNOME session without forcing Secret Service to be world-accessible.
- Logs show successful helper impersonation before capture begins.
- Automated tests cover JSON protocol handling and helper outcome mapping.
