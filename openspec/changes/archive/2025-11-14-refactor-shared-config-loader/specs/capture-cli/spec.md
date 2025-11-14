## MODIFIED Requirements
### Requirement: Config File Capture Defaults
The capture CLI MUST reuse the shared TOML configuration loader provided by the `chissu-config` crate (the same loader consumed by the PAM module) to resolve `device`, `pixel_format`, and `warmup_frames` whenever callers omit the corresponding CLI flags, falling back to the existing built-in defaults only when no config value is present.

#### Scenario: Config file supplies capture defaults
- **GIVEN** `/etc/chissu-pam/config.toml` defines `video_device = "/dev/video2"`, `pixel_format = "GREY"`, and `warmup_frames = 10`
- **AND** the operator runs `chissu-cli capture` without `--device`, `--pixel-format`, or `--warmup-frames`
- **THEN** the CLI uses `/dev/video2`, `GREY`, and `10` during capture negotiation
- **AND** the human and JSON outputs report those resolved values.

#### Scenario: CLI flags override config
- **GIVEN** the config file defines `video_device = "/dev/video2"`
- **WHEN** the operator runs `chissu-cli capture --device /dev/video4`
- **THEN** the CLI captures from `/dev/video4` regardless of the config value and records that override in its logs/output.

#### Scenario: Built-in defaults still apply
- **WHEN** neither configuration file exists or the relevant keys are absent
- **AND** the operator omits the corresponding CLI flags
- **THEN** the CLI defaults to `/dev/video0` (index 0), pixel format `Y16`, and four warm-up frames
- **AND** it logs that the built-in defaults were used.

#### Scenario: Shared loader prevents drift
- **GIVEN** both `chissu-cli` and `pam-chissu` depend on the `chissu-config` crate
- **WHEN** a new config key or validation rule is added to `chissu-config`
- **THEN** the capture CLI automatically observes the same precedence, parse failures, and logging semantics as the PAM module without duplicating loader code.
