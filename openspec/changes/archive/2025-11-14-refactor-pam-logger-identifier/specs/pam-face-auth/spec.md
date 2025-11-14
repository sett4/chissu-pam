## MODIFIED Requirements
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
