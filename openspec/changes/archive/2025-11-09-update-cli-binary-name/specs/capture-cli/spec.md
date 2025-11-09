## ADDED Requirements
### Requirement: CLI Binary Naming
The workspace MUST emit a `chissu-cli` binary for the capture tool whenever the root crate is built.

#### Scenario: Release build produces chissu-cli
- **WHEN** a maintainer runs `cargo build --release`
- **THEN** the build outputs `target/release/chissu-cli`
- **AND** no `chissu-pam` binary artifact remains in the `target` directory.

#### Scenario: Debug build produces chissu-cli
- **WHEN** a maintainer runs `cargo build`
- **THEN** the build outputs `target/debug/chissu-cli`
- **AND** the binary's `--help` banner introduces the tool as `chissu-cli`.

## MODIFIED Requirements
### Requirement: Config File Capture Defaults
The capture CLI MUST reuse the shared TOML configuration (`/etc/chissu-pam/config.toml`, `/usr/local/etc/chissu-pam/config.toml`) to resolve capture defaults whenever callers omit the corresponding CLI flags.
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
