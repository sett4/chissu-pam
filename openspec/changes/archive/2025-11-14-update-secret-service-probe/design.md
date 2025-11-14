## Overview
`pam_chissu` currently performs its Secret Service probe (via `chissu-face-core::secret_service`) inside the privileged PAM process. D-Bus denies the operation because PAM runs as root while the Secret Service session bus belongs to the target desktop user. The proposed design forks a short-lived helper that:
1. Inherits only the file descriptors required for IPC.
2. Drops privileges to the PAM user (`setgid`/`initgroups`/`setuid`).
3. Reuses `ensure_secret_service_available` and descriptor key lookup functions from `chissu-face-core`.
4. Serialises the result as JSON over the parent-provided pipe.

## Helper Lifecycle
- Parent creates a unidirectional pipe (or `socketpair` for bidirectional needs) before fork.
- After `fork()`, the child closes the read end, switches credentials, and loads the Secret Service entry (`Entry::new`).
- The child writes a JSON payload: `{ "status": "ok", "aes_gcm_key": "base64..." }` or `{ "status": "error", "kind": "secret_service_unavailable", "message": "..." }`.
- Parent closes the write end, waits for the child, and parses the JSON. Any malformed payload or abnormal exit maps to `secret_service_ipc_failure`.

## Message Schema
```json
{
  "status": "ok",
  "aes_gcm_key": "base64",
  "metadata": { "descriptor_modified": "2025-11-10T04:15:39Z" }
}
```
- `status` ∈ {`ok`, `missing`, `error`}.
- `kind` required when `status == "error"`; values include `secret_service_unavailable`, `keyring_locked`, `ipc_failure`.
- `message` contains a human-readable summary for logs.

## PAM Integration Flow
1. After config + user resolution, PAM checks `require_secret_service`.
2. PAM forks helper. Parent blocks on pipe read with timeout ≤ capture timeout.
3. `ok` ⇒ continue authentication using returned key; `missing` ⇒ translate to descriptor-missing failure reason; `secret_service_unavailable` ⇒ return `PAM_IGNORE` with log; `ipc_failure` ⇒ `PAM_SYSTEM_ERR`.
4. Parent ensures child exit status is reaped to avoid zombies.

## Testing
- Add unit tests for JSON serialization/deserialization of helper messages (both success and error variants).
- Add integration test with a stub helper implementation verifying PAM maps helper results to `PamReturnCode` outcomes without hitting hardware.
- Provide doc updates describing the helper and D-Bus constraints.
