# infrared-capture Specification

## Purpose
TBD - created by archiving change add-ir-capture-cli. Update Purpose after archive.
## Requirements
### Requirement: Infrared Still Capture Command
The CLI MUST capture a single infrared frame from a V4L2 webcam and save it under `./captures/`.

#### Scenario: Successful capture to default path
- **GIVEN** a V4L2 device that supports the requested infrared pixel format and resolution
- **WHEN** the operator runs `study-rust-v4l2 capture` with no explicit output path
- **THEN** the command creates `./captures/<timestamp>.png` containing the captured frame
- **AND** the command exits with status code 0 after confirming the file path in stdout

#### Scenario: Unsupported format aborts fast
- **GIVEN** the selected device lacks the requested infrared pixel format
- **WHEN** the operator runs the command
- **THEN** the command emits an error explaining the missing format to stderr
- **AND** no file is written
- **AND** the process exits with status code 2

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
The CLI MUST expose arguments for device path/index, pixel format, resolution, exposure, gain, and output filename.

#### Scenario: Operator overrides defaults
- **WHEN** the operator supplies flags for device path, pixel format, resolution, exposure, gain, or output filename
- **THEN** the command applies those values during capability negotiation and frame capture
- **AND** the settings are echoed back in logs or JSON output

### Requirement: Dual Output Modes
The CLI MUST support human-readable logging by default and structured JSON output when `--json` is provided.

#### Scenario: Human-readable run
- **WHEN** the operator runs the command without `--json`
- **THEN** logs include device selection, negotiated format, applied IR settings, and the saved file path in plain text

#### Scenario: JSON run
- **WHEN** the operator runs the command with `--json`
- **THEN** stdout emits a single JSON object containing device metadata, negotiated settings, output file path, and success state
- **AND** human-readable logs are suppressed from stdout but critical errors still surface on stderr

### Requirement: Testable Capture Flow
The project MUST provide automated tests validating the capture pipeline without requiring live hardware.

#### Scenario: Mocked frame capture test
- **WHEN** `cargo test` runs
- **THEN** at least one test uses a mock frame source or recorded frame data to validate frame conversion and file writing logic

#### Scenario: Manual hardware test guidance
- **WHEN** contributors consult the documentation
- **THEN** they find a reproducible manual test procedure covering device setup, command invocation, and expected outputs for infrared capture

