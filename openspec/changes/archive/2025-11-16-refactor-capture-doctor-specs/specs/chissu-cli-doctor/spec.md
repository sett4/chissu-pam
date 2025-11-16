## ADDED Requirements
### Requirement: Doctor Command Environment Diagnostics
- The `chissu-cli doctor` subcommand MUST validate config, video device access, model files, embedding store directory, Secret Service availability, PAM module presence, and PAM stack entries; MUST report `pass`/`warn`/`fail` with reasons; and MUST align JSON output/exit code (`ok` boolean) with overall status.

#### Scenario: Diagnostics suite executes
- **WHEN** `chissu-cli doctor` runs with or without `--json`
- **THEN** it performs all checks without mutating state, returns exit code 0 only when all checks pass, and emits structured output usable by automation.
