## MODIFIED Requirements
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
