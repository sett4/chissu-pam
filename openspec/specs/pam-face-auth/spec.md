# pam-face-auth Specification

## Purpose
TBD - created by archiving change add-pam-face-auth. Update Purpose after archive.
## Requirements
### Requirement: PAM Facial Authentication Module
The system MUST provide a shared library `pam_chissu.so` that implements PAM authentication by validating a live camera capture against descriptors enrolled for the target user.

#### Scenario: Build target renamed to pam_chissu
- **WHEN** a maintainer runs `cargo build --release -p pam_chissu` (or the equivalent `cargo build -d pam_chissu` shortcut)
- **THEN** the build places `pam_chissu.so` under `target/release/`
- **AND** the library can be copied directly into `/lib/security/pam_chissu.so` without any manual renaming.

#### Scenario: Syslog identifier matches module name
- **WHEN** the PAM stack loads the module and it emits syslog events
- **THEN** each entry uses the identifier `pam_chissu`
- **SO** operators can follow the docs exactly when filtering events via `journalctl -t pam_chissu` or configuring PAM service stanzas like `auth sufficient pam_chissu.so`.

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

