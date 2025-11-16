## MODIFIED Requirements
### Requirement: Infrared Still Capture Command
- The `chissu-cli capture` capability MUST include infrared still capture behavior formerly scoped to `infrared-capture`, covering IR-specific persistence under `./captures/`, flag handling, and format negotiation.

#### Scenario: Command defers to shared CLI behavior
- **GIVEN** the operator runs `chissu-cli capture` without additional flags
- **WHEN** the command negotiates devices/formats and emits JSON
- **THEN** base defaults/logging come from shared capture behavior, while the IR flow ensures the frame saved to `./captures/<timestamp>.png` uses the negotiated infrared pixel format.

### Requirement: Infrared Device Capability Validation
- The capture capability MUST interrogate V4L2 capabilities and refuse infrared capture until device features and formats are confirmed.

#### Scenario: Capability check precedes capture
- **WHEN** the command starts
- **THEN** it queries device capabilities, supported formats, and frame sizes via the `v4l` crate
- **AND** logs the negotiated format/resolution before frame acquisition.

#### Scenario: Incompatible device reported clearly
- **GIVEN** the device lacks video capture capability or cannot stream infrared
- **WHEN** the command runs
- **THEN** it emits a structured error identifying the capability gap and exits non-zero without attempting to read frames.

### Requirement: Infrared Capture Parameters
- Infrared-specific flag handling (pixel formats, IR gains, filenames) MUST be described here while relying on shared flag precedence.

#### Scenario: Operator overrides IR-specific parameters
- **GIVEN** the operator supplies `--pixel-format GREY --gain 5 --output /tmp/ir.png`
- **WHEN** the capture negotiates
- **THEN** the spec records how these overrides affect infrared capture while flag resolution order follows shared behavior.

### Requirement: Infrared Dual Output Modes
- Infrared capture MUST reuse the shared CLI stdout/stderr contract and MAY only extend the JSON payload with IR-specific metadata.

#### Scenario: JSON payload extends shared schema
- **GIVEN** the operator runs `chissu-cli capture --json`
- **WHEN** the command succeeds
- **THEN** base JSON fields come from shared behavior, and IR-only fields (pixel format, frame path) are added without redefining logging rules.

### Requirement: Infrared Testable Capture Flow
- Infrared tests MUST validate IR-specific logic plus shared capture behavior using mocked/recorded frames.

#### Scenario: Mock test enforces cross-capability contract
- **GIVEN** `cargo test` runs an infrared capture test with recorded frames
- **WHEN** it verifies file writing and metadata
- **THEN** the test asserts warm-up discard and JSON schema per shared behavior, demonstrating extension without forking.

### Requirement: Infrared Mode Boundaries
- The infrared mode MUST be represented inside `chissu-cli-capture`, making clear that shared capture behaviors and IR-specific rules co-reside here.

#### Scenario: Contributors know where to edit shared vs IR logic
- **GIVEN** a maintainer needs to tweak warm-up logic or IR-specific validation
- **WHEN** they inspect capture specs
- **THEN** they see IR requirements within `chissu-cli-capture`, not a separate capability.
