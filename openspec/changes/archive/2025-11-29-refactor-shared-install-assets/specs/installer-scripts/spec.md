## ADDED Requirements
### Requirement: Shared Installer Asset Library
The installer SHALL source a shared shell library that centralizes distro detection, prerequisite package sets, dlib model URLs, and default config rendering so these values stay identical across installer and packaging flows.

#### Scenario: Installer uses shared defaults
- **WHEN** `install-chissu.sh` runs
- **THEN** it sources the shared library for distro detection, prerequisite package lists, model URLs, and default config content
- **AND** no duplicate hardcoded defaults remain in the script.

### Requirement: Asset Templates Generated From Common Source
The project SHALL provide a single canonical template set for install-time assets (at minimum `config.toml` and PAM snippets) plus a generator/verification step to materialize them for both the installer and packaging assets.

#### Scenario: Asset generation produces identical config
- **WHEN** the asset generator or check script runs
- **THEN** it writes `build/package/assets/etc/chissu-pam/config.toml` (and any related snippets) from the same template used by the installer
- **AND** drift detection fails (non-zero) if committed assets diverge from the template source.
