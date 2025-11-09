# capture-cli Specification

## Purpose
TBD - created by archiving change add-auto-exposure-gain. Update Purpose after archive.
## Requirements
### Requirement: Auto Exposure And Gain Controls
The CLI SHALL let callers opt into device-provided automatic exposure and gain adjustments before capturing a frame.

#### Scenario: Auto Control Available
- **GIVEN** a V4L2 device that reports `Exposure, Auto` and/or `Gain, Auto` controls
- **WHEN** the user passes `--auto-exposure` and/or `--auto-gain`
- **THEN** the CLI enables the corresponding auto control(s) prior to capture
- **AND** it records in logs/JSON summary that auto adjustment is active.

#### Scenario: Auto Control Missing
- **GIVEN** a V4L2 device that does not expose the requested auto control
- **WHEN** the user passes `--auto-exposure` or `--auto-gain`
- **THEN** the CLI emits a debug log explaining the control is unavailable
- **AND** it proceeds with any manual exposure/gain parameters that were supplied.
#### Scenario: Auto Control Needs Warm-Up
- **GIVEN** the user enables auto exposure and/or gain
- **WHEN** the CLI captures a frame
- **THEN** it SHALL discard a configurable number of warm-up frames (default 4) before saving the final image to allow the device controls to converge.

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

