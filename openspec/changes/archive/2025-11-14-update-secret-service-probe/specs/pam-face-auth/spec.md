## MODIFIED Requirements
### Requirement: Secret Service Availability Gate
Secret Service probing MUST execute in a helper that can impersonate the target desktop user before the PAM module touches camera resources.
#### Scenario: Helper impersonates target user session
- **WHEN** `pam_sm_authenticate` prepares to probe the Secret Service for the PAM target user
- **THEN** it MUST fork a helper child that closes unused descriptors, calls `setgid`/`initgroups`/`setuid` to adopt that user, and runs the probe inside the helper before any camera capture begins
- **AND** the parent process MUST consume the helper's structured IPC response and only proceed to capture work when the helper reports success.

#### Scenario: Helper outcome drives PAM return codes
- **WHEN** the helper reports Secret Service is locked, missing, or unreachable for the target user
- **THEN** the parent MUST log the helper's message and immediately return `PAM_IGNORE` without opening V4L2 devices, matching the earlier Secret Service gating behavior.

## ADDED Requirements
### Requirement: Descriptor Key Helper Responses
The PAM module MUST exchange JSON messages with the helper so it can return descriptor encryption keys or typed errors.

#### Scenario: Successful key retrieval
- **WHEN** the helper reads the descriptor AES-GCM key from the target user's Secret Service entry
- **THEN** it returns `{ "status": "ok", "aes_gcm_key": "<base64>" }`
- **AND** the parent parses the payload, validates the key length, and continues authentication using the decrypted descriptor store.

#### Scenario: Missing key response
- **WHEN** no descriptor key exists for the user in Secret Service
- **THEN** the helper returns `{ "status": "missing", "message": "..." }`
- **AND** the parent treats this as `FailureReason::DescriptorsMissing`, surfacing the same PAM conversation/error flow as a missing descriptor file.

#### Scenario: Secret Service or IPC failure reporting
- **WHEN** the helper cannot reach Secret Service (locked session, DBus refusal) or the IPC channel breaks
- **THEN** it returns `{ "status": "error", "kind": "secret_service_unavailable" | "ipc_failure", "message": "..." }` (or the parent synthesizes the `ipc_failure` error if parsing fails)
- **AND** the parent maps `secret_service_unavailable` to `PAM_IGNORE` and `ipc_failure` to `PAM_SYSTEM_ERR`, logging the helper message before exiting.
