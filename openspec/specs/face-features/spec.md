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

### Requirement: Face Feature Comparison Command
The CLI MUST provide a `faces compare` subcommand that consumes descriptor JSON files produced by `faces extract` and reports cosine similarity scores between an input file and one or more comparison targets.

#### Scenario: Scores each comparison target
- **GIVEN** an input descriptor file containing at least one face
- **AND** two comparison descriptor files
- **WHEN** the operator runs `study-rust-v4l2 faces compare --input <file> --compare-target <target1> --compare-target <target2>`
- **THEN** the command computes cosine similarity for every face pair between the input file and each comparison file
- **AND** the command reports the highest similarity per comparison target in descending order
- **AND** the process exits with status code 0 after listing all scores

#### Scenario: JSON comparison run
- **WHEN** the operator runs the command with `--json`
- **THEN** stdout emits a single JSON array where each element contains the comparison path and the reported similarity score
- **AND** informational logs are suppressed from stdout while stderr still carries errors

#### Scenario: Missing comparison file aborts
- **WHEN** any specified input or comparison descriptor file is unreadable or missing
- **THEN** the command emits an error describing the filesystem issue to stderr
- **AND** no similarity scores are emitted
- **AND** the process exits with status code 2

### Requirement: Face Feature Enrollment Command
The CLI MUST provide a `faces enroll` subcommand that associates descriptor vectors with a named operating system user.

#### Scenario: Successful enrollment from descriptor file
- **GIVEN** a descriptor JSON file produced by `faces extract`
- **AND** a target user name `alice`
- **WHEN** the operator runs `study-rust-v4l2 faces enroll --user alice <descriptor.json>`
- **THEN** the command validates the JSON structure, assigns unique IDs to each descriptor, appends them to Alice’s feature store, and exits with status code 0
- **AND** the command reports the descriptor IDs in both human-readable and `--json` structured output modes

#### Scenario: Missing descriptor file aborts
- **WHEN** the operator specifies a descriptor file path that does not exist or is unreadable
- **THEN** the command emits an actionable error to stderr, makes no modifications to any feature store, and exits with status code 2

#### Scenario: Invalid descriptor payload aborts
- **WHEN** the provided JSON cannot be parsed into descriptor vectors of the expected dimension
- **THEN** the command emits a validation error to stderr, leaves existing feature stores untouched, and exits with status code 3

### Requirement: User Feature Store
The system MUST persist enrolled descriptors in per-user JSON files stored under `/var/lib/study-rust-v4l2/models/<user>.json` by default, supporting multiple descriptors per user with stable IDs and allowing operators to override the storage directory via CLI arguments.

#### Scenario: Store file created on first enrollment
- **WHEN** a user without prior enrollments is targeted
- **THEN** the command creates `/var/lib/study-rust-v4l2/models/<user>.json` containing a JSON array of descriptor entries with metadata (ID, source file, created-at timestamp)

#### Scenario: Operator overrides store directory
- **GIVEN** a writable directory `/tmp/store`
- **WHEN** the operator passes `--store-dir /tmp/store`
- **THEN** the command persists descriptors under `/tmp/store/<user>.json` instead of the default location

#### Scenario: Store write robustness
- **WHEN** concurrent or repeated enrollments occur
- **THEN** the CLI writes updates atomically (using temporary files and rename or equivalent) so that the store file is never left in a partially written state

### Requirement: Face Feature Removal Command
The CLI MUST provide a `faces remove` subcommand that deletes descriptors from a user’s feature store by ID.

#### Scenario: Remove descriptor by ID
- **GIVEN** a user `alice` with enrolled descriptors
- **WHEN** the operator runs `study-rust-v4l2 faces remove --user alice --descriptor-id <id>`
- **THEN** the command removes the matching descriptor, rewrites the store atomically, and exits with status code 0 while reporting the removal

#### Scenario: Unknown descriptor ID
- **WHEN** the operator targets an ID not present in the user’s store
- **THEN** the command exits with status code 4 and informs the operator that no descriptor matched, leaving the store unchanged

#### Scenario: Remove all descriptors
- **WHEN** the operator supplies `--all`
- **THEN** the command clears the user’s feature store (deleting the file or replacing it with an empty array) and exits with status code 0

