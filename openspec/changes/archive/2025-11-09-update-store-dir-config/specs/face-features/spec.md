## MODIFIED Requirements
### Requirement: User Feature Store
The system MUST persist enrolled descriptors in per-user JSON files under a configurable base directory, keeping CLI defaults aligned with PAM configuration and still allowing operators to override the location when necessary.

#### Scenario: Store file created on first enrollment
- **WHEN** a user without prior enrollments is targeted
- **THEN** the command creates `<base-dir>/<user>.json` containing a JSON array of descriptor entries with metadata (ID, source file, created-at timestamp)

#### Scenario: Default base directory comes from config
- **GIVEN** `/etc/chissu-pam/config.toml` contains `descriptor_store_dir = "/srv/face-store"`
- **AND** the operator runs `chissu-pam faces enroll --user alice <descriptor.json>` without specifying `--store-dir`
- **THEN** the CLI loads the configuration file, resolves `/srv/face-store/alice.json` as the feature store path, and logs the configured location

#### Scenario: Override precedence is documented
- **WHEN** the operator sets `--store-dir /tmp/store`
- **THEN** the CLI writes to `/tmp/store/<user>.json` regardless of the configuration file or environment
- **AND** if the flag is absent, the CLI next consults the configuration file, then `CHISSU_PAM_STORE_DIR`, and finally the built-in `/var/lib/chissu-pam/models` default
