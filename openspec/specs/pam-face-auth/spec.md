# pam-face-auth Specification

## Purpose
TBD - created by archiving change add-pam-face-auth. Update Purpose after archive.
## Requirements
### Requirement: PAM Facial Authentication Module
The system MUST provide a shared library for PAM authentication named `libpam_chissu.so`, validating a live camera capture against embeddings enrolled for the target user and surfacing embedding terminology in logs/prompts while still accepting legacy descriptor stores.

#### Scenario: Missing user embeddings abort
- **WHEN** the target user's embedding file is absent in the configured store directory
- **THEN** the module returns `PAM_AUTH_ERR`
- **AND** emits a syslog message indicating embeddings are missing (or descriptor store missing, when a legacy file name is detected).

### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files via the shared `chissu-config` loader and honour documented defaults, preferring embedding-oriented keys while accepting legacy descriptor keys for compatibility.

#### Scenario: Shared loader keeps CLI and PAM aligned
- **GIVEN** both `chissu-cli` and `pam-chissu` import the `chissu-config` crate
- **WHEN** `/etc/chussu-pam/config.toml` defines `video_device = "/dev/video2"`, `warmup_frames = 6`, and `embedding_store_dir = "/srv/chissu/models"` (or the legacy `descriptor_store_dir`)
- **THEN** the PAM module resolves those values through the shared loader in the same order (primary path → secondary path → defaults) as the CLI
- **AND** any parse/read failure bubbles up from the shared loader so both binaries report the same error wording.

### Requirement: Target User Embedding Isolation
The module MUST restrict comparisons to embeddings belonging to the PAM target user and fail fast if none are available.

#### Scenario: Missing user embeddings abort
- **WHEN** the target user's embedding file is absent in the configured store directory
- **THEN** the module returns `PAM_AUTH_ERR`
- **AND** emits a syslog message indicating embeddings are missing.

### Requirement: Syslog Audit Logging
The module MUST emit syslog messages for notable events so administrators can inspect outcomes via `journalctl`.

#### Scenario: Error conditions logged with context
- **WHEN** a fatal error occurs during configuration loading, camera access, or embedding extraction
- **THEN** the module sends a syslog entry at error severity that includes the PAM service name and relevant error message before returning `PAM_SYSTEM_ERR`.

### Requirement: Interactive Conversation Feedback
The module MUST keep conversational prompts clear for users even when falling back to passwords after Secret Service gating.
#### Scenario: Secret Service fallback prompt stays concise
- **WHEN** the Secret Service helper reports it is unavailable or locked and the module returns `PAM_IGNORE`
- **THEN** the conversation callback sends a short `PAM_ERROR_MSG` that simply states face authentication is unavailable and that PAM will fall back to other factors, without echoing the helper's internal reason
- **AND** the module still logs the full helper message to syslog so administrators keep full diagnostics
- **SO** end users are not exposed to verbose Secret Service errors while operators retain detailed logs.

### Requirement: Secret Service Availability Gate
Secret Service probing MUST execute in a helper that can impersonate the target desktop user before the PAM module touches camera resources.
#### Scenario: Helper impersonates target user session
- **WHEN** `pam_sm_authenticate` prepares to probe the Secret Service for the PAM target user
- **THEN** it MUST fork a helper child that closes unused file handles, calls `setgid`/`initgroups`/`setuid` to adopt that user, and runs the probe inside the helper before any camera capture begins
- **AND** the parent process MUST consume the helper's structured IPC response and only proceed to capture work when the helper reports success.

#### Scenario: Helper outcome drives PAM return codes
- **WHEN** the helper reports Secret Service is locked, missing, or unreachable for the target user
- **THEN** the parent MUST log the helper's message and immediately return `PAM_IGNORE` without opening V4L2 devices, matching the earlier Secret Service gating behavior.

### Requirement: Embedding Key Helper Responses
The PAM module MUST exchange JSON messages with the helper so it can return embedding encryption keys or typed errors while understanding legacy descriptor-key field names for existing deployments.

#### Scenario: Successful key retrieval
- **WHEN** the helper reads the embedding AES-GCM key from the target user's Secret Service entry
- **THEN** it returns `{ "status": "ok", "embedding_key": "<base64>" }` (and continues to accept `descriptor_key` during transition)
- **AND** the parent parses the payload, validates the key length, and continues authentication using the decrypted embedding store.

#### Scenario: Missing key response
- **WHEN** no embedding key exists for the user in Secret Service
- **THEN** the helper returns `{ "status": "missing", "message": "..." }`
- **AND** the parent treats this as `FailureReason::DescriptorsMissing`/`EmbeddingsMissing`, surfacing the same PAM conversation/error flow as a missing embedding file.

