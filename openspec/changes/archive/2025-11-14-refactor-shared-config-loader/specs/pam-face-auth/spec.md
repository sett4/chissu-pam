## MODIFIED Requirements
### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files via the shared `chissu-config` loader (also used by the CLI) and honour documented defaults when no configuration file is present.

#### Scenario: Defaults applied when no config found
- **WHEN** neither configuration file is present
- **THEN** the module uses defaults of threshold `0.7`, timeout `5` seconds, store directory `/var/lib/chissu-pam/models`, and video device `/dev/video0`
- **AND** it logs the default usage at startup.

#### Scenario: Shared loader keeps CLI and PAM aligned
- **GIVEN** both `chissu-cli` and `pam-chissu` import the `chissu-config` crate
- **WHEN** `/etc/chissu-pam/config.toml` defines `video_device = "/dev/video2"`, `warmup_frames = 6`, and `descriptor_store_dir = "/srv/chissu/models"`
- **THEN** the PAM module resolves those values through the shared loader in the same order (primary path → secondary path → defaults) as the CLI
- **AND** any parse/read failure bubbles up from the shared loader so both binaries report the same error wording.
