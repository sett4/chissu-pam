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

