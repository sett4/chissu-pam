## ADDED Requirements
### Requirement: Secret Service Diagnostics Command
The capture CLI MUST provide a subcommand that verifies Secret Service availability via the `keyring` crate, mirroring the PAM module's behavior.

#### Scenario: Human-readable success output
- **WHEN** an operator runs `chissu-cli keyring check`
- **AND** the keyring probe reaches the default Secret Service collection for the invoking user (even if no entry exists yet)
- **THEN** the command exits with status `0`
- **AND** it prints a confirmation message that includes the probed user/service.

#### Scenario: JSON output for automation
- **WHEN** the operator passes `--json` to the check command
- **THEN** the CLI emits a JSON object containing the service, user, `status` field (`"ok"` or `"error"`), and an `error` message when applicable
- **SO** scripts can parse the result without scraping text.

#### Scenario: Failures propagate reason and non-zero exit
- **WHEN** the keyring probe encounters a locked keyring, missing DBus session, or other error
- **THEN** the command exits with a non-zero status (e.g., `2`)
- **AND** it surfaces the underlying keyring error message in both human-readable and JSON modes.
