## MODIFIED Requirements
### Requirement: Infrared Still Capture Command
The CLI MUST capture a single infrared frame from a V4L2 webcam and save it under `./captures/`.
#### Scenario: Successful capture to default path
- **GIVEN** a V4L2 device that supports the requested infrared pixel format and resolution
- **WHEN** the operator runs `chissu-cli capture` with no explicit output path
- **THEN** the command creates `./captures/<timestamp>.png` containing the captured frame
- **AND** the command exits with status code 0 after confirming the file path in stdout
