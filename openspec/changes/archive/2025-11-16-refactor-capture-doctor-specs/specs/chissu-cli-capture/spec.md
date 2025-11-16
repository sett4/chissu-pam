## MODIFIED Requirements
### Requirement: Capture CLI Capability Scope Declaration
- The capability MUST be named `chissu-cli-capture` and MUST own only shared capture behaviors (defaults, auto controls, keyring diagnostics, binary naming), excluding doctor command requirements.

#### Scenario: Capture-only scope documented
- **WHEN** contributors look up where shared capture behaviors live
- **THEN** they see the capability called `chissu-cli-capture` and no longer find doctor command requirements mixed into this spec.

## REMOVED Requirements
### Requirement: Doctor Command Environment Diagnostics
- Doctor command requirements MUST be documented under the `chissu-cli-doctor` capability and SHALL be removed from `chissu-cli-capture`.

#### Scenario: Doctor behavior referenced elsewhere
- **WHEN** a contributor searches the capture spec for doctor behavior
- **THEN** they are directed to `chissu-cli-doctor` for diagnostics requirements, keeping the capture spec focused on capture flows.
