## MODIFIED Requirements
### Requirement: Face Feature Extraction Command
The CLI MUST provide a subcommand that loads an existing PNG image, detects faces, and computes descriptor vectors using dlib-based models.

#### Scenario: Successful extraction from PNG
- **GIVEN** a PNG file containing at least one detectable face
- **WHEN** the operator runs `chissu-pam faces extract <path>`
- **THEN** the command detects each face bounding box and computes a descriptor vector for every face
- **AND** the command exits with status code 0 after reporting the number of faces processed

### Requirement: Face Feature Comparison Command
The CLI MUST provide a `faces compare` subcommand that consumes descriptor JSON files produced by `faces extract` and reports cosine similarity scores between an input file and one or more comparison targets.

#### Scenario: Scores each comparison target
- **GIVEN** an input descriptor file containing at least one face
- **AND** two comparison descriptor files
- **WHEN** the operator runs `chissu-pam faces compare --input <file> --compare-target <target1> --compare-target <target2>`
- **THEN** the command computes cosine similarity for every face pair between the input file and each comparison file
- **AND** the command reports the highest similarity per comparison target in descending order
- **AND** the process exits with status code 0 after listing all scores

### Requirement: Face Feature Enrollment Command
The CLI MUST provide a `faces enroll` subcommand that associates descriptor vectors with a named operating system user.

#### Scenario: Successful enrollment from descriptor file
- **GIVEN** a descriptor JSON file produced by `faces extract`
- **AND** a target user name `alice`
- **WHEN** the operator runs `chissu-pam faces enroll --user alice <descriptor.json>`
- **THEN** the command validates the JSON structure, assigns unique IDs to each descriptor, appends them to Alice’s feature store, and exits with status code 0
- **AND** the command reports the descriptor IDs in both human-readable and `--json` structured output modes

### Requirement: User Feature Store
The system MUST persist enrolled descriptors in per-user JSON files stored under `/var/lib/chissu-pam/models/<user>.json` by default, supporting multiple descriptors per user with stable IDs and allowing operators to override the storage directory via CLI arguments.

#### Scenario: Store file created on first enrollment
- **WHEN** a user without prior enrollments is targeted
- **THEN** the command creates `/var/lib/chissu-pam/models/<user>.json` containing a JSON array of descriptor entries with metadata (ID, source file, created-at timestamp)

### Requirement: Face Feature Removal Command
The CLI MUST provide a `faces remove` subcommand that deletes descriptors from a user’s feature store by ID.

#### Scenario: Remove descriptor by ID
- **GIVEN** a user `alice` with enrolled descriptors
- **WHEN** the operator runs `chissu-pam faces remove --user alice --descriptor-id <id>`
- **THEN** the command removes the matching descriptor, rewrites the store atomically, and exits with status code 0 while reporting the removal
