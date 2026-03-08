## ADDED Requirements
### Requirement: RPM PAM Wiring Via authselect Profile
RPM packages SHALL wire `libpam_chissu.so` through an `authselect` custom profile derived from `sssd`, ensuring the module runs before `pam_unix.so` and can be reverted safely.

#### Scenario: %post applies custom authselect profile
- **WHEN** the `%post` script runs during RPM installation or upgrade
- **THEN** it checks for `authselect` availability and refuses to proceed if the current profile is unsynced, emitting a clear error
- **AND** it renders/updates a `custom/chissu` profile that injects `auth    sufficient    libpam_chissu.so` ahead of `pam_unix.so` in both `system-auth` and `password-auth` templates
- **AND** it selects the profile and applies changes while backing up the previous selection so upgrades are idempotent and ordering remains correct.

#### Scenario: %postun restores previous selection on erase
- **WHEN** the RPM is removed (erase, not upgrade)
- **THEN** `%postun` restores the previously saved authselect selection (if present) and removes the `custom/chissu` profile artifacts
- **AND** if `CHISSU_PAM_PURGE_MODELS=1` is set, model files are purged as already required, but PAM wiring removal is always performed regardless of the flag.
