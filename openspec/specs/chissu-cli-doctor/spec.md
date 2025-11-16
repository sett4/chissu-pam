# chissu-cli-doctor Specification

## Purpose

Defines the `chissu-cli doctor` diagnostics subcommand that validates runtime prerequisites for capture/enrollment flows and the PAM module without mutating system state.

## Requirements

### Requirement: Doctor Command Environment Diagnostics

The doctor subcommand MUST validate runtime prerequisites, emit human-readable and `--json` structured reports, and return a non-zero exit code when any check fails.

#### Scenario: Doctor runs complete suite

- **WHEN** an operator runs `chissu-cli doctor` (optionally with `--json`)
- **THEN** the command executes all defined checks (config discovery/parse, video device access, embedding store directory, landmark/encoder model files, Secret Service availability, PAM module presence, PAM stack configuration)
- **AND** prints per-check statuses (`pass`/`warn`/`fail`) with reasons
- **AND** returns exit code 0 only when all checks are `pass`, exits 1 otherwise.

#### Scenario: Config files discovered and validated

- **GIVEN** the shared config loader that searches `/etc/chissu-pam/config.toml` then `/usr/local/etc/chissu-pam/config.toml`
- **WHEN** `doctor` runs
- **THEN** it reports which path was used
- **AND** emits a `warn` if both files exist
- **AND** emits a `fail` if neither exists or if TOML parsing fails, including the parse error message.

#### Scenario: Video device path and permissions checked

- **WHEN** `doctor` resolves `video_device` via the shared loader defaults/overrides from `chissu-cli-capture`
- **THEN** it verifies the device node exists and is a readable/writable V4L2 character device
- **AND** reports `fail` with the OS error when the node is missing or inaccessible
- **AND** reports `pass` when the device can be opened for capture negotiation.

#### Scenario: Embedding store directory accessibility

- **WHEN** `doctor` resolves `embedding_store_dir` (or legacy descriptor directory) from config/defaults
- **THEN** it checks the directory exists and is readable & writable by the current user
- **AND** reports `fail` if missing or lacking permissions, otherwise `pass` with the resolved path.

#### Scenario: Model file readability

- **WHEN** `doctor` reads `landmark_model` and `encoder_model` paths from config/defaults
- **THEN** it verifies each file exists and can be opened for read
- **AND** reports individual `fail` statuses when missing or unreadable, otherwise `pass` with the canonical path.

#### Scenario: Secret Service availability via keyring

- **WHEN** `doctor` probes Secret Service using the existing `keyring` crate
- **THEN** it reports `pass` when the default collection can be opened
- **AND** reports `fail` with the underlying keyring error when Secret Service is locked, unreachable, or unsupported.

#### Scenario: PAM module installation check

- **WHEN** `doctor` inspects PAM module locations
- **THEN** it reports `pass` if `libpam_chissu.so` exists in `/lib/security` or `/lib64/security`
- **AND** reports `fail` when neither path contains the library, including the searched paths.

#### Scenario: PAM stack entry check

- **WHEN** `doctor` scans `/etc/pam.d/` for chissu-pam integration (e.g., an `auth` line invoking `libpam_chissu.so` in a service file)
- **THEN** it reports `pass` when a matching entry is present
- **AND** reports `warn` when no entry exists so operators know PAM is not yet wired
- **AND** includes the inspected files/services in the message.

#### Scenario: JSON output shape

- **WHEN** `doctor` runs with `--json`
- **THEN** it emits a single JSON object containing an array of check results with `name`, `status`, `message`, and relevant `path`/`device` fields
- **AND** includes an aggregate `ok` boolean matching the exit code so automation can parse outcomes.
