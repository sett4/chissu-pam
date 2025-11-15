# docs-readme Specification

## Purpose
TBD - created by archiving change update-readme-toc. Update Purpose after archive.
## Requirements
### Requirement: README Table Of Contents
The README MUST expose a table of contents after the opening overview so readers can jump to the canonical sections called out below.

#### Scenario: Maintainer sees canonical sections
- **GIVEN** a maintainer opens README.md
- **WHEN** they scroll past the introduction
- **THEN** they see a Markdown list of links covering (at least) Overview, Why This Project, Getting Started, Usage, Configuration, Testing, and License anchors in that order.

### Requirement: Why This Project Highlights Secret Service Security
The README MUST explain why the project is secure by design, emphasizing Secret Service–backed embedding encryption and the reduced need for root.

#### Scenario: Why section sells security benefits
- **WHEN** a reader opens the "Why This Project" section
- **THEN** it states that embedding files are encrypted via GNOME Secret Service (AES-GCM) so leaked files remain unreadable
- **AND** it clarifies that everyday enrollment runs without `root` because Secret Service operates in the user session (only PAM wiring under `/etc/pam.d` needs elevated rights).

### Requirement: Prerequisites Detail Package Installs And Dlib Models
Getting Started MUST enumerate prerequisites with concrete installation guidance and reference the dlib model downloads/operators need.

#### Scenario: Prerequisites include install commands
- **WHEN** someone reads Getting Started → Prerequisites
- **THEN** they find example package commands (e.g., `sudo apt install libdlib-dev libopenblas-dev liblapack-dev`) plus the base Rust toolchain requirements.

#### Scenario: logind dependency noted
- **WHEN** a maintainer studies the prerequisites
- **THEN** it spells out that systemd-logind must be running so PAM can recover `$DISPLAY`/`$DBUS_SESSION_BUS_ADDRESS`/`$XDG_RUNTIME_DIR`, and it references `loginctl list-sessions`/`show-session` for validation.

#### Scenario: Dlib model downloads documented
- **WHEN** they continue through Getting Started
- **THEN** it lists the required dlib model filenames, download location (https://dlib.net/files/), and where to store or reference them for CLI runs.

### Requirement: Installation Explains File Placement And PAM Configuration
The README MUST spell out how to deploy binaries, config files, models, and PAM entries so operators can reproduce the setup.

#### Scenario: Installation paths documented
- **WHEN** a user reads Getting Started → Installation
- **THEN** it provides steps for installing `chissu-cli`, placing `libpam_chissu.so`, copying `/etc/chissu-pam/config.toml` (or `/usr/local/etc/...`), storing dlib weights, and wiring `/etc/pam.d/<service>` with `auth sufficient libpam_chissu.so`.

### Requirement: Usage Documents chissu-cli Enroll Flow
Usage MUST include examples for enrolling faces via the CLI, including elevated and non-elevated patterns, using embedding-oriented flags and outputs.

#### Scenario: Standard enroll example included
- **WHEN** someone reads Usage → Enrollment
- **THEN** they see a command example for `chissu-cli enroll` that references the landmark/encoder models, explains default target user behavior, and shows embedding terminology for outputs/IDs.

### Requirement: Configuration Section Explains chissu-pam TOML
A dedicated Configuration section MUST explain `chissu-pam` TOML files, precedence, and common keys using embedding-oriented names, while noting legacy descriptor key compatibility during transition.

#### Scenario: Config precedence documented
- **WHEN** an operator opens the Configuration section
- **THEN** it lists `/etc/chussu-pam/config.toml` and `/usr/local/etc/chussu-pam/config.toml`, describes how CLI/PAM fall back across them, and highlights important keys (device, pixel format, embedding_store_dir with legacy descriptor_store_dir alias, similarity thresholds, Secret Service toggles, etc.).
