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

### Requirement: CLI Binary Naming
The workspace MUST continue to emit a `chissu-cli` binary name for all build profiles so capability-focused specs remain accurate regardless of how many capture modes exist.

#### Scenario: Future capture modes reuse binary naming
- **GIVEN** maintainers add a new capture capability (e.g., RGB capture)
- **WHEN** they build the workspace in debug or release mode
- **THEN** the resulting binary remains `chissu-cli`, ensuring documentation in sibling specs stays correct without additional edits.

### Requirement: Secret Service Diagnostics Command
The Secret Service diagnostic subcommand MUST remain defined in this capability even when other capture modes are introduced, and sibling specs SHALL reference it instead of redefining command semantics.

#### Scenario: Infrared spec links to diagnostics
- **GIVEN** operators follow the `infrared-capture` documentation to verify their environment
- **WHEN** they run `chissu-cli keyring check`
- **THEN** the diagnostic behavior is defined only once in `capture-cli`, and the infrared spec simply references it rather than redefining command semantics.

### Requirement: Shared Capture CLI Behavior
Every capture-oriented subcommand SHALL inherit a single set of CLI behaviors that live in the `capture-cli` capability: built-in defaults (device `/dev/video0`, pixel format `Y16`, four warm-up frames), config-file overrides, warm-up frame discarding, and dual output modes (`--json` vs human-readable).

#### Scenario: Any capture mode honors shared defaults
- **GIVEN** `chissu-cli capture --json` is invoked without explicit `--device`, `--pixel-format`, or `--warmup-frames`
- **WHEN** the capability referenced by the command needs those values
- **THEN** the CLI resolves them using the shared default/config logic defined in `capture-cli`
- **AND** any capability-specific spec (e.g., `infrared-capture`) may only override values it explicitly documents.

### Requirement: Capture CLI Capability Scope Declaration
The `capture-cli` spec MUST describe itself as the home for cross-cutting capture behaviors (controls, diagnostics, binary naming) so other capability specs can reference it instead of re-stating shared rules.

#### Scenario: Linked capability identifies shared owner
- **GIVEN** another capability (e.g., `infrared-capture`) needs the CLI logging, auto control toggles, or diagnostic subcommands
- **WHEN** contributors look up where to document or modify those behaviors
- **THEN** the `capture-cli` spec explicitly states it owns them and points to the relevant requirements (auto controls, config defaults, keyring diagnostics, binary naming).

