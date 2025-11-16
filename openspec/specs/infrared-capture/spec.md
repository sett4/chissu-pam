# infrared-capture Specification

## Purpose
TBD - created by archiving change add-ir-capture-cli. Update Purpose after archive.
## Requirements
### Requirement: Infrared Still Capture Command
The infrared command MUST defer to the `Shared Capture CLI Behavior` requirement for defaults and output mode semantics while guaranteeing IR-specific frame persistence under `./captures/`.

#### Scenario: Command defers to shared CLI behavior
- **GIVEN** the operator runs `chissu-cli capture` without additional flags
- **WHEN** the command negotiates devices/formats and emits JSON
- **THEN** base defaults/logging come from the shared behavior spec, while this requirement ensures the frame saved to `./captures/<timestamp>.png` uses the infrared pixel format negotiated earlier.

### Requirement: Device Capability Validation
The CLI MUST interrogate V4L2 capabilities and refuse capture until device features and formats are confirmed.

#### Scenario: Capability check precedes capture
- **WHEN** the command starts
- **THEN** it queries the device capabilities, supported formats, and frame sizes using the `v4l` crate
- **AND** it logs the negotiated format and resolution before frame acquisition

#### Scenario: Incompatible device reported clearly
- **GIVEN** the device lacks video capture capability or cannot stream infrared
- **WHEN** the command runs
- **THEN** it emits a structured error clearly identifying the capability gap (e.g., missing `VideoCapture`, `ReadWrite`, or desired format)
- **AND** it exits with a non-zero code without attempting to read frames

### Requirement: Configurable Capture Parameters
The infrared capability MUST only describe IR-specific flag handling (pixel formats, IR gains, filenames) and SHALL rely on the shared CLI requirement for generic flag precedence.

#### Scenario: Operator overrides IR-specific parameters
- **GIVEN** the operator supplies `--pixel-format GREY --gain 5 --output /tmp/ir.png`
- **WHEN** the capability negotiates the capture
- **THEN** the IR spec records how these overrides affect infrared capture, while flag resolution order defers to the shared CLI requirement.

### Requirement: Dual Output Modes
Infrared capture MUST reuse the shared CLI's stdout/stderr contract and MAY only extend the JSON payload with IR-specific metadata.

#### Scenario: JSON payload extends shared schema
- **GIVEN** the operator runs `chissu-cli capture --json`
- **WHEN** the command succeeds
- **THEN** the base JSON fields (device path, resolved defaults, success flag) come from the shared behavior spec, and the IR spec adds IR-only fields (pixel format, frame path) without redefining the shared logging rules.

### Requirement: Testable Capture Flow
Infrared tests MUST validate both the IR-specific logic and the shared CLI behavior they exercise, ensuring mocked captures prove the combined contract without hardware.

#### Scenario: Mock test enforces cross-capability contract
- **GIVEN** `cargo test` executes an infrared capture test using recorded frames
- **WHEN** it verifies file writing and metadata emission
- **THEN** the test also asserts compliance with the shared CLI behavior requirement (correct warm-up discard, JSON schema), demonstrating how infrared capture extends but does not fork the capture CLI contract.

### Requirement: Infrared Mode Boundaries
The infrared capture capability SHALL declare itself as a mode layered on top of the shared `chissu-cli-capture` behaviors, only adding IR-specific validation (device capabilities, pixel formats) and persistence rules.

#### Scenario: Contributors know where to edit shared vs IR logic
- **GIVEN** a maintainer needs to tweak warm-up logic or JSON structure for all capture modes
- **WHEN** they inspect the infrared spec
- **THEN** it states that such edits belong in `chissu-cli-capture`, while IR-only concerns (format validation, manual test guides) stay here.
