# chissu-cli-faces Specification

## Purpose
Defines the CLI `faces` subcommands (extract, compare, enroll, remove) including embedding terminology and storage rules.
## Requirements
### Requirement: Capability Naming Aligns With CLI
- The capability MUST be named `chissu-cli-faces` to match the `chissu-cli faces` command and the naming pattern used by other CLI capabilities (e.g., `chissu-cli-capture`).

#### Scenario: Spec name matches CLI command
- **WHEN** contributors search specs for the `chissu-cli faces` subcommands
- **THEN** they find the `chissu-cli-faces` capability and no longer see `face-features` as an active capability name.

### Requirement: Face Feature Extraction Command
The CLI MUST provide a subcommand that loads an existing PNG image, detects faces, and computes embedding vectors (previously called descriptors) using dlib-based models while still accepting legacy terminology on the command line.

#### Scenario: Successful extraction from PNG
- **GIVEN** a PNG file containing at least one detectable face
- **WHEN** the operator runs `chissu-cli faces extract <path>`
- **THEN** the command detects each face bounding box and computes an embedding vector for every face
- **AND** the command exits with status code 0 after reporting the number of faces processed

#### Scenario: Legacy flag remains accepted during transition
- **WHEN** the operator invokes the command with the legacy `--descriptor-output` (or equivalent descriptor-named flag)
- **THEN** the CLI treats it as the embedding output flag, emits a deprecation notice, and continues successfully.

### Requirement: Feature Persistence Format
The CLI MUST persist extracted embeddings and face metadata to disk in a structured format that downstream tools can reuse, defaulting to embedding-oriented field names while still parsing legacy descriptor fields.

#### Scenario: Default output location is used
- **WHEN** the operator does not provide an explicit output path
- **THEN** the command writes a JSON file under `./captures/features/<timestamp>.json` containing an array of embedding vectors and associated bounding boxes
- **AND** the command logs the saved file path before exiting

#### Scenario: Operator overrides output path
- **WHEN** the operator supplies `--output <file>`
- **THEN** the command saves the embedding data to that path, creating parent directories as needed
- **AND** the command emits the effective path in its human-readable or JSON output

### Requirement: Structured Outputs
The CLI MUST honor the global `--json` flag for the extraction command and surface human-readable logs by default, preferring embedding terminology in both modes while preserving legacy descriptor fields for inbound compatibility.

#### Scenario: Human-readable extraction run
- **WHEN** the operator runs the command without `--json`
- **THEN** stdout lists detected faces, embedding vector length, and the saved feature file path in plain text
- **AND** errors continue to flow to stderr

#### Scenario: JSON extraction run
- **WHEN** the operator runs the command with `--json`
- **THEN** stdout emits a single JSON object containing detector metadata, face bounding boxes, embedding vectors, and saved file path, using embedding field names (e.g., `embedding_vectors`)
- **AND** verbose human-oriented logs are suppressed from stdout

### Requirement: Testable Feature Pipeline
The project MUST include automated tests and documentation that validate embedding extraction without requiring a live camera.

#### Scenario: Fixture-based automated test
- **WHEN** `cargo test` executes
- **THEN** at least one test uses a fixture PNG image and a stubbed model to validate embedding generation and persistence logic

#### Scenario: Manual model setup guidance
- **WHEN** contributors read the documentation
- **THEN** they find instructions for acquiring the required dlib model weights and running the extraction command against sample images

### Requirement: Face Feature Comparison Command
The CLI MUST provide a `faces compare` subcommand that consumes embedding JSON files produced by `faces extract` and reports cosine similarity scores while accepting legacy descriptor inputs.

#### Scenario: Scores each comparison target
- **GIVEN** an input embedding file containing at least one face
- **AND** two comparison embedding files
- **WHEN** the operator runs `chissu-cli faces compare --input <file> --compare-target <target1> --compare-target <target2>`
- **THEN** the command computes cosine similarity for every face pair between the input file and each comparison file
- **AND** the command reports the highest similarity per comparison target in descending order
- **AND** the process exits with status code 0 after listing all scores

### Requirement: Face Feature Enrollment Command
The CLI MUST manage embedding encryption keys via Secret Service when enrolling embeddings, rotating them on every run, and writing the per-user store in encrypted form. Legacy descriptor CLI flags or JSON fields MUST still be parsed but marked deprecated.

#### Scenario: Enrollment creates or rotates AES-GCM key
- **WHEN** `chissu-cli faces enroll --user <name>` runs
- **THEN** the command fetches the existing AES-GCM embedding key for `<name>` from Secret Service (service `chissu-pam`)
- **AND** if a key exists it decrypts the user’s current store before appending embeddings
- **AND** the command generates a new 32-byte AES-256-GCM key, registers it in Secret Service, and encrypts the updated store with that key before exiting.

#### Scenario: Legacy descriptor input remains valid
- **WHEN** a user supplies a JSON file that uses legacy descriptor field names
- **THEN** enrollment succeeds by mapping those fields to embeddings and emits a deprecation warning.

### Requirement: User Feature Store
The system MUST persist enrolled embeddings in per-user JSON files under a configurable base directory, aligned with PAM configuration defaults, preferring embedding-oriented config keys while accepting legacy descriptor keys for compatibility.

#### Scenario: Default base directory comes from config
- **GIVEN** `/etc/chissu-pam/config.toml` contains `embedding_store_dir = "/srv/face-store"` (or the legacy `descriptor_store_dir`)
- **AND** the operator runs `chissu-cli faces enroll --user alice <embedding.json>` without specifying `--store-dir`
- **THEN** the CLI loads the configuration file, resolves `/srv/face-store/alice.json` as the feature store path, and logs which key was used.

### Requirement: Face Feature Removal Command
The removal flow MUST reuse the encrypted store format and Secret Service key so embeddings remain protected when entries are deleted.

#### Scenario: Removal decrypts and re-encrypts store
- **WHEN** `chissu-cli faces remove` deletes embeddings for a user with an encrypted store
- **THEN** it fetches the user’s AES-GCM key from Secret Service, decrypts the store, removes the requested embeddings, and rewrites the store encrypted with the same key before exiting.

### Requirement: Config-Driven Auto Enrollment Command
The CLI MUST expose a top-level `chissu-cli enroll` command that captures an infrared frame using the same configuration/default order defined in the `chissu-cli-capture` capability, runs face detection + embedding extraction, and immediately reuses the encrypted enrollment flow without requiring intermediate embedding files.

#### Scenario: End-to-end capture, extract, and enroll
- **GIVEN** `/etc/chissu-pam/config.toml` defines `video_device = "/dev/video2"`, `pixel_format = "Y16"`, and `embedding_store_dir = "/srv/chissu/models"` (or the legacy `descriptor_store_dir`)
- **AND** the operator has dlib landmark/encoder model paths available via flags or environment variables
- **WHEN** they run `chissu-cli enroll --json`
- **THEN** the command loads the config, captures a frame using the shared defaults (honoring warm-up frames), detects faces, and encodes embeddings using the same logic as `faces extract`
- **AND** it appends those embeddings to the resolved store via the AES-GCM workflow already defined for `faces enroll`, logging/returning the generated embedding IDs, user, and store path in both human-readable and JSON outputs (JSON MUST include `"captured_image"`, `"target_user"`, `"embedding_ids"`, and `"store_path"`).

### Requirement: Enrollment User Resolution Controls
The automated enrollment command MUST infer the target user from the invoking Unix account, only permitting `--user <name>` overrides when the effective UID is 0 so that unprivileged operators cannot inject embeddings into other accounts.

#### Scenario: Default to invoking user
- **WHEN** an unprivileged user runs `chissu-cli enroll`
- **THEN** the command resolves the target user to the invoking account (e.g., via `getuid`/`getlogin`), logs it in human output, includes it in the JSON payload, and writes embeddings under that user’s encrypted store without requiring a flag.

#### Scenario: Root override succeeds
- **WHEN** root runs `sudo chissu-cli enroll --user alice`
- **THEN** the command accepts the override, records that `alice` is the target, and appends embeddings to Alice’s encrypted store via the same AES-GCM workflow.

#### Scenario: Non-root override is rejected
- **WHEN** a non-root user runs `chissu-cli enroll --user bob`
- **THEN** the command fails validation before any capture occurs, explaining that only root may override the user, and it exits with a non-zero status without touching any store files.

