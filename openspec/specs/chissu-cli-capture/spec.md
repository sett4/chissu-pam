# chissu-cli-capture Specification

## Purpose
Defines the shared capture behaviors for `chissu-cli` capture subcommands (device defaults, auto exposure/gain, warm-up handling, diagnostics, and binary naming) that other capabilities reference.
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
- **THEN** the diagnostic behavior is defined only once in `chissu-cli-capture`, and the infrared spec simply references it rather than redefining command semantics.

### Requirement: Shared Capture CLI Behavior
Every capture-oriented subcommand SHALL inherit a single set of CLI behaviors that live in the `chissu-cli-capture` capability: built-in defaults (device `/dev/video0`, pixel format `Y16`, four warm-up frames), config-file overrides, warm-up frame discarding, and dual output modes (`--json` vs human-readable).

#### Scenario: Any capture mode honors shared defaults
- **GIVEN** `chissu-cli capture --json` is invoked without explicit `--device`, `--pixel-format`, or `--warmup-frames`
- **WHEN** the capability referenced by the command needs those values
- **THEN** the CLI resolves them using the shared default/config logic defined in `chissu-cli-capture`
- **AND** any capability-specific spec (e.g., `infrared-capture`) may only override values it explicitly documents.

### Requirement: Capture CLI Capability Scope Declaration
- The capability MUST be named `chissu-cli-capture` and MUST own shared capture behaviors (defaults, auto controls, keyring diagnostics, binary naming) plus infrared capture rules, excluding doctor command requirements.

#### Scenario: Capture-only scope documented
- **WHEN** contributors look up where shared capture behaviors live
- **THEN** they see the capability called `chissu-cli-capture` and find both shared and infrared capture requirements here, with doctor behavior separated.

### Requirement: Infrared Still Capture Command
- The `chissu-cli capture` capability MUST include infrared still capture behavior formerly scoped to `infrared-capture`, covering IR-specific persistence under `./captures/`, flag handling, and format negotiation.

#### Scenario: Command defers to shared CLI behavior
- **GIVEN** the operator runs `chissu-cli capture` without additional flags
- **WHEN** the command negotiates devices/formats and emits JSON
- **THEN** base defaults/logging come from shared capture behavior, while the IR flow ensures the frame saved to `./captures/<timestamp>.png` uses the negotiated infrared pixel format.

### Requirement: Infrared Device Capability Validation
- The capture capability MUST interrogate V4L2 capabilities and refuse infrared capture until device features and formats are confirmed.

#### Scenario: Capability check precedes capture
- **WHEN** the command starts
- **THEN** it queries device capabilities, supported formats, and frame sizes via the `v4l` crate
- **AND** logs the negotiated format/resolution before frame acquisition.

#### Scenario: Incompatible device reported clearly
- **GIVEN** the device lacks video capture capability or cannot stream infrared
- **WHEN** the command runs
- **THEN** it emits a structured error identifying the capability gap and exits non-zero without attempting to read frames.

### Requirement: Infrared Capture Parameters
- Infrared-specific flag handling (pixel formats, IR gains, filenames) MUST be described here while relying on shared flag precedence.

#### Scenario: Operator overrides IR-specific parameters
- **GIVEN** the operator supplies `--pixel-format GREY --gain 5 --output /tmp/ir.png`
- **WHEN** the capture negotiates
- **THEN** the spec records how these overrides affect infrared capture while flag resolution order follows shared behavior.

### Requirement: Infrared Dual Output Modes
- Infrared capture MUST reuse the shared CLI stdout/stderr contract and MAY only extend the JSON payload with IR-specific metadata.

#### Scenario: JSON payload extends shared schema
- **GIVEN** the operator runs `chissu-cli capture --json`
- **WHEN** the command succeeds
- **THEN** base JSON fields come from shared behavior, and IR-only fields (pixel format, frame path) are added without redefining logging rules.

### Requirement: Infrared Testable Capture Flow
- Infrared tests MUST validate IR-specific logic plus shared capture behavior using mocked/recorded frames.

#### Scenario: Mock test enforces cross-capability contract
- **GIVEN** `cargo test` runs an infrared capture test with recorded frames
- **WHEN** it verifies file writing and metadata
- **THEN** the test asserts warm-up discard and JSON schema per shared behavior, demonstrating extension without forking.

### Requirement: Infrared Mode Boundaries
- The infrared mode MUST be represented inside `chissu-cli-capture`, making clear that shared capture behaviors and IR-specific rules co-reside here.

#### Scenario: Contributors know where to edit shared vs IR logic
- **GIVEN** a maintainer needs to tweak warm-up logic or IR-specific validation
- **WHEN** they inspect capture specs
- **THEN** they see IR requirements within `chissu-cli-capture`, not a separate capability.

