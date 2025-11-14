# pam-face-auth Specification

## Purpose
TBD - created by archiving change add-pam-face-auth. Update Purpose after archive.
## Requirements
### Requirement: PAM Facial Authentication Module
The system MUST provide a shared library for PAM authentication named `libpam_chissu.so`, validating a live camera capture against descriptors enrolled for the target user.

#### Scenario: Build emits libpam_chissu artefact
- **WHEN** a maintainer runs `cargo build --release -p pam_chissu` (or the equivalent `cargo build -d pam_chissu` shortcut)
- **THEN** the build places `libpam_chissu.so` under `target/release/`
- **AND** maintainers copy that exact filename into `/lib/security/` without relying on auxiliary compatibility symlinks.

#### Scenario: Syslog identifier matches module name
- **WHEN** the PAM stack loads the module and it emits syslog events
- **THEN** each entry uses the identifier `pam_chissu`
- **SO** operators can follow the docs exactly when filtering events via `journalctl -t pam_chissu` or configuring PAM service stanzas like `auth sufficient pam_chissu.so`.

#### Scenario: Legacy libpam artefacts removed
- **WHEN** a maintainer inspects `target/<profile>/` after running `cargo build --release -p pam-chissu`
- **THEN** the directory contains `libpam_chissu.so` (and Cargo’s usual metadata files) but **NOT** `libpam_chissuauth.so`
- **SO** installation and packaging steps always pick the single supported module name without relying on deprecated symlinks.

#### Scenario: Identifier constant reused for every logging sink
- **WHEN** the module configures syslog (`Formatter3164.process`) or falls back to printing errors on stderr/stdout
- **THEN** all of those code paths pull the identifier from a single constant declared inside the crate (e.g., `SYSLOG_IDENTIFIER`)
- **SO** any future logging destination or refactor remains tied to the required `pam_chissu` value without duplicating literals that could drift.

### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files via the shared `chissu-config` loader (also used by the CLI) and honour documented defaults when no configuration file is present.

#### Scenario: Defaults applied when no config found
- **WHEN** neither configuration file is present
- **THEN** the module uses defaults of threshold `0.7`, timeout `5` seconds, store directory `/var/lib/chissu-pam/models`, and video device `/dev/video0`
- **AND** it logs the default usage at startup.

#### Scenario: Shared loader keeps CLI and PAM aligned
- **GIVEN** both `chissu-cli` and `pam-chissu` import the `chissu-config` crate
- **WHEN** `/etc/chissu-pam/config.toml` defines `video_device = "/dev/video2"`, `warmup_frames = 6`, and `descriptor_store_dir = "/srv/chissu/models"`
- **THEN** the PAM module resolves those values through the shared loader in the same order (primary path → secondary path → defaults) as the CLI
- **AND** any parse/read failure bubbles up from the shared loader so both binaries report the same error wording.

### Requirement: Target User Descriptor Isolation
The module MUST restrict comparisons to descriptors belonging to the PAM target user and fail fast if none are available.

#### Scenario: Missing user descriptors abort
- **WHEN** the target user's descriptor file is absent in the configured store directory
- **THEN** the module returns `PAM_AUTH_ERR`
- **AND** emits a syslog message indicating descriptors are missing.

### Requirement: Syslog Audit Logging
The module MUST emit syslog messages for notable events so administrators can inspect outcomes via `journalctl`.

#### Scenario: Error conditions logged with context
- **WHEN** a fatal error occurs during configuration loading, camera access, or descriptor extraction
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
- **THEN** it MUST fork a helper child that closes unused descriptors, calls `setgid`/`initgroups`/`setuid` to adopt that user, and runs the probe inside the helper before any camera capture begins
- **AND** the parent process MUST consume the helper's structured IPC response and only proceed to capture work when the helper reports success.

#### Scenario: Helper outcome drives PAM return codes
- **WHEN** the helper reports Secret Service is locked, missing, or unreachable for the target user
- **THEN** the parent MUST log the helper's message and immediately return `PAM_IGNORE` without opening V4L2 devices, matching the earlier Secret Service gating behavior.

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

