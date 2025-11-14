## ADDED Requirements
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
