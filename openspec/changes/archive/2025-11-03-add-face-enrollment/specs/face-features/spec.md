## ADDED Requirements
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
