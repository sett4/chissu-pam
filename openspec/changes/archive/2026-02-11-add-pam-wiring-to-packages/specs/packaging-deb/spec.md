## ADDED Requirements
### Requirement: Debian PAM Wiring Via pam-auth-update
Debian/Ubuntu packages SHALL wire `libpam_chissu.so` into the `auth` stack using the distro-supported `pam-auth-update` mechanism and ship the required snippet.

#### Scenario: Postinst enables pam-auth-update entry
- **WHEN** `postinst configure` runs for the `.deb` package
- **THEN** it verifies `pam-auth-update` is available, installs or refreshes `/usr/share/pam-configs/chissu` from package assets, and executes `pam-auth-update --package --enable chissu`
- **AND** the resulting PAM order places `auth    sufficient    libpam_chissu.so` before the existing `pam_unix.so` entry
- **AND** if `pam-auth-update` is missing or reports an out-of-sync state, the script exits non-zero with a clear error so the install aborts instead of leaving partial wiring.

#### Scenario: Removal cleans pam-auth-update state
- **WHEN** the package is removed or purged
- **THEN** maintainer scripts call `pam-auth-update --package --remove chissu` and delete the snippet if present
- **AND** upgrades remain idempotent (no duplicate lines), while purge leaves other PAM entries untouched.
