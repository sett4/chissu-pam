# face-features Specification

## Purpose
TBD - created by archiving change add-face-feature-extraction. Update Purpose after archive.
## Requirements
### Requirement: Face Feature Extraction Command
The CLI MUST provide a subcommand that loads an existing PNG image, detects faces, and computes descriptor vectors using dlib-based models.

#### Scenario: Successful extraction from PNG
- **GIVEN** a PNG file containing at least one detectable face
- **WHEN** the operator runs `study-rust-v4l2 faces extract <path>`
- **THEN** the command detects each face bounding box and computes a descriptor vector for every face
- **AND** the command exits with status code 0 after reporting the number of faces processed

#### Scenario: Invalid image path aborts fast
- **WHEN** the operator provides a missing or unreadable PNG path
- **THEN** the command emits an error describing the filesystem issue to stderr
- **AND** no output feature file is written
- **AND** the process exits with status code 2

### Requirement: Feature Persistence Format
The CLI MUST persist extracted descriptors and face metadata to disk in a structured format that downstream tools can reuse.

#### Scenario: Default output location is used
- **WHEN** the operator does not provide an explicit output path
- **THEN** the command writes a JSON file under `./captures/features/<timestamp>.json` containing an array of descriptor vectors and associated bounding boxes
- **AND** the command logs the saved file path before exiting

#### Scenario: Operator overrides output path
- **WHEN** the operator supplies `--output <file>`
- **THEN** the command saves the descriptor data to that path, creating parent directories as needed
- **AND** the command emits the effective path in its human-readable or JSON output

### Requirement: Structured Outputs
The CLI MUST honor the global `--json` flag for the extraction command and surface human-readable logs by default.

#### Scenario: Human-readable extraction run
- **WHEN** the operator runs the command without `--json`
- **THEN** stdout lists detected faces, descriptor vector length, and the saved feature file path in plain text
- **AND** errors continue to flow to stderr

#### Scenario: JSON extraction run
- **WHEN** the operator runs the command with `--json`
- **THEN** stdout emits a single JSON object containing detector metadata, face bounding boxes, descriptor vectors, and saved file path
- **AND** verbose human-oriented logs are suppressed from stdout

### Requirement: Testable Feature Pipeline
The project MUST include automated tests and documentation that validate descriptor extraction without requiring a live camera.

#### Scenario: Fixture-based automated test
- **WHEN** `cargo test` executes
- **THEN** at least one test uses a fixture PNG image and a stubbed model to validate descriptor generation and persistence logic

#### Scenario: Manual model setup guidance
- **WHEN** contributors read the documentation
- **THEN** they find instructions for acquiring the required dlib model weights and running the extraction command against sample images

