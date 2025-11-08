# pam-face-auth Specification

## Purpose
TBD - created by archiving change add-pam-face-auth. Update Purpose after archive.
## Requirements
### Requirement: PAM Facial Authentication Module
The system MUST provide a shared library `libpam_chissuauth.so` that implements PAM authentication by validating a live camera capture against descriptors enrolled for the target user.

#### Scenario: Successful match returns PAM success
- **GIVEN** the invoking PAM stack calls `pam_sm_authenticate`
- **AND** the target user has at least one descriptor in the configured store directory
- **AND** a captured face descriptor meets or exceeds the configured cosine-similarity threshold against any enrolled descriptor
- **THEN** the module returns `PAM_SUCCESS` to PAM
- **AND** logs the success to syslog with the service name and similarity score.

#### Scenario: No matching descriptor fails authentication
- **WHEN** the module captures frames until the timeout elapses without any descriptor meeting the threshold
- **THEN** it returns `PAM_AUTH_ERR`
- **AND** records a syslog warning noting the timeout condition and observed peak similarity.

### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files and honour documented defaults when no configuration file is present.

#### Scenario: Primary configuration file preferred
- **GIVEN** `/etc/chissu-pam/config.toml` exists
- **THEN** the module loads similarity threshold, capture timeout (seconds), descriptor store directory, and video device path from this file
- **AND** only falls back to `/usr/local/etc/chissu-pam/config.toml` when the primary file is absent.

#### Scenario: Defaults applied when no config found
- **WHEN** neither configuration file is present
- **THEN** the module uses defaults of threshold `0.7`, timeout `5` seconds, store directory `/var/lib/chissu-pam/models`, and video device `/dev/video0`
- **AND** it logs the default usage at startup.

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

