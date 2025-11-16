## MODIFIED Requirements
### Requirement: Capability Naming Aligns With CLI
- The capability MUST be named `chissu-cli-faces` to match the `chissu-cli faces` command and the naming pattern used by other CLI capabilities (e.g., `chissu-cli-capture`).

#### Scenario: Spec name matches CLI command
- **WHEN** contributors search specs for the `chissu-cli faces` subcommands
- **THEN** they find the `chissu-cli-faces` capability and no longer see `face-features` as an active capability name.
