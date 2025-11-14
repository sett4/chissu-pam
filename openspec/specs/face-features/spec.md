# face-features Specification

## Purpose
TBD - created by archiving change add-face-feature-extraction. Update Purpose after archive.
## Requirements
### Requirement: Face Feature Extraction Command
The CLI MUST provide a subcommand that loads an existing PNG image, detects faces, and computes descriptor vectors using dlib-based models.
#### Scenario: Successful extraction from PNG
- **GIVEN** a PNG file containing at least one detectable face
- **WHEN** the operator runs `chissu-cli faces extract <path>`
- **THEN** the command detects each face bounding box and computes a descriptor vector for every face
- **AND** the command exits with status code 0 after reporting the number of faces processed

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
The CLI MUST provide a `faces compare` subcommand that consumes descriptor JSON files produced by `faces extract` and reports cosine similarity scores.
#### Scenario: Scores each comparison target
- **GIVEN** an input descriptor file containing at least one face
- **AND** two comparison descriptor files
- **WHEN** the operator runs `chissu-cli faces compare --input <file> --compare-target <target1> --compare-target <target2>`
- **THEN** the command computes cosine similarity for every face pair between the input file and each comparison file
- **AND** the command reports the highest similarity per comparison target in descending order
- **AND** the process exits with status code 0 after listing all scores

### Requirement: Face Feature Enrollment Command
The CLI MUST manage descriptor encryption keys via Secret Service when enrolling descriptors, rotating them on every run, and writing the per-user store in encrypted form.

#### Scenario: Enrollment creates or rotates AES-GCM key
- **WHEN** `chissu-cli faces enroll --user <name>` runs
- **THEN** the command fetches the existing AES-GCM descriptor key for `<name>` from Secret Service (service `chissu-pam`)
- **AND** if a key exists it decrypts the user’s current store before appending descriptors
- **AND** the command generates a new 32-byte AES-256-GCM key, registers it in Secret Service, and encrypts the updated store with that key before exiting.

#### Scenario: Secret Service errors abort enrollment
- **WHEN** Secret Service is locked/unavailable or returns an invalid/malformed key
- **THEN** the enroll command logs the failure, exits non-zero, and leaves both the feature store and the previously registered key untouched so operators can retry safely.

#### Scenario: Legacy plaintext store migration
- **WHEN** a user’s feature store is still plaintext (no AES key registered)
- **THEN** the first enrollment run accepts the legacy file, generates/registers a new key, and re-writes the store using the encrypted format so subsequent PAM authentications can decrypt it via the helper key.

### Requirement: User Feature Store
The system MUST persist enrolled descriptors in per-user JSON files under a configurable base directory, aligned with PAM configuration defaults.
#### Scenario: Default base directory comes from config
- **GIVEN** `/etc/chissu-pam/config.toml` contains `descriptor_store_dir = "/srv/face-store"`
- **AND** the operator runs `chissu-cli faces enroll --user alice <descriptor.json>` without specifying `--store-dir`
- **THEN** the CLI loads the configuration file, resolves `/srv/face-store/alice.json` as the feature store path, and logs the configured location

### Requirement: Face Feature Removal Command
The removal flow MUST reuse the encrypted store format and Secret Service key so descriptors remain protected when entries are deleted.

#### Scenario: Removal decrypts and re-encrypts store
- **WHEN** `chissu-cli faces remove` deletes descriptors for a user with an encrypted store
- **THEN** it fetches the user’s AES-GCM key from Secret Service, decrypts the store, removes the requested descriptors, and rewrites the store encrypted with the same key before exiting.

#### Scenario: Missing key detection
- **WHEN** the removal command encounters an encrypted store but cannot obtain the Secret Service key
- **THEN** it fails with a descriptive error instructing the operator to unlock Secret Service (or rerun enroll) instead of corrupting or rewriting the store.

### Requirement: Config-Driven Auto Enrollment Command
The CLI MUST expose a top-level `chissu-cli enroll` command that captures an infrared frame using the same configuration/default order defined in the `capture-cli` capability, runs face detection + descriptor extraction, and immediately reuses the encrypted enrollment flow without requiring intermediate descriptor files.

#### Scenario: End-to-end capture, extract, and enroll
- **GIVEN** `/etc/chissu-pam/config.toml` defines `video_device = "/dev/video2"`, `pixel_format = "Y16"`, and `descriptor_store_dir = "/srv/chissu/models"`
- **AND** the operator has dlib landmark/encoder model paths available via flags or environment variables
- **WHEN** they run `chissu-cli enroll --json`
- **THEN** the command loads the config, captures a frame using the shared defaults (honoring warm-up frames), detects faces, and encodes descriptors using the same logic as `faces extract`
- **AND** it appends those descriptors to the resolved store via the AES-GCM workflow already defined for `faces enroll`, logging/returning the generated descriptor IDs, user, and store path in both human-readable and JSON outputs (JSON MUST include `"captured_image"`, `"target_user"`, `"descriptor_ids"`, and `"store_path"`).

#### Scenario: Config falls back to shared defaults
- **GIVEN** neither `/etc/chissu-pam/config.toml` nor `/usr/local/etc/chissu-pam/config.toml` exists
- **WHEN** an operator runs `chissu-cli enroll`
- **THEN** the command reuses the `capture-cli` shared defaults (device `/dev/video0`, pixel format `Y16`, warm-up frames `4`) and logs the resolved values before attempting capture.

#### Scenario: No faces abort the enrollment
- **GIVEN** the command successfully captures a frame but the detector finds zero faces
- **WHEN** `chissu-cli enroll` completes processing
- **THEN** it exits with a non-zero status, emits an error explaining that no faces were detected, and leaves the encrypted descriptor store untouched (any temporary capture that was produced may be retained only for troubleshooting logs).

#### Scenario: Store directory override remains available
- **WHEN** the operator supplies `--store-dir ./captures/enrolled`
- **THEN** the command enrolls descriptors into that directory instead of the config-derived path while still following the same AES-GCM key rotation behavior.

### Requirement: Enrollment User Resolution Controls
The automated enrollment command MUST infer the target user from the invoking Unix account, only permitting `--user <name>` overrides when the effective UID is 0 so that unprivileged operators cannot inject descriptors into other accounts.

#### Scenario: Default to invoking user
- **WHEN** an unprivileged user runs `chissu-cli enroll`
- **THEN** the command resolves the target user to the invoking account (e.g., via `getuid`/`getlogin`), logs it in human output, includes it in the JSON payload, and writes descriptors under that user’s encrypted store without requiring a flag.

#### Scenario: Root override succeeds
- **WHEN** root runs `sudo chissu-cli enroll --user alice`
- **THEN** the command accepts the override, records that `alice` is the target, and appends descriptors to Alice’s encrypted store via the same AES-GCM workflow.

#### Scenario: Non-root override is rejected
- **WHEN** a non-root user runs `chissu-cli enroll --user bob`
- **THEN** the command fails validation before any capture occurs, explaining that only root may override the user, and it exits with a non-zero status without touching any store files.

