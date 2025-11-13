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

### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files and honour documented defaults when no configuration file is present.

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

### Requirement: Interactive Conversation Feedback
The PAM module MUST leverage the `pam_conv` callback to surface success, retry, and failure status messages to the invoking PAM client while still logging to syslog.

#### Scenario: Success message uses PAM_TEXT_INFO
- **WHEN** `pam_sm_authenticate` is about to return `PAM_SUCCESS` because a descriptor match was found
- **THEN** the module calls the PAM conversation function with message style `PAM_TEXT_INFO`
- **AND** the message clearly states that face authentication succeeded (optionally including the PAM service name).

#### Scenario: Failures send PAM_ERROR_MSG
- **WHEN** an authentication attempt ends without a matching descriptor (because descriptors are missing, no face was detected, or the threshold was not met)
- **THEN** before returning `PAM_AUTH_ERR` the module invokes the conversation callback with style `PAM_ERROR_MSG`
- **AND** the text explains why the attempt failed and that PAM may offer another retry depending on the stack configuration.

#### Scenario: Retry instructions use PAM_ERROR_MSG
- **WHEN** the module needs the user to adjust while it keeps capturing (e.g., no face detected but timeout not reached)
- **THEN** it emits a single `PAM_ERROR_MSG` via the conversation callback to instruct the user to stay in frame or adjust lighting while the module retries within the same `pam_sm_authenticate` call.

#### Scenario: Missing conversation handler handled gracefully
- **WHEN** PAM does not supply a conversation structure or invoking it fails
- **THEN** the module logs a warning and continues without crashing, still returning the correct PAM code for the authentication result.

