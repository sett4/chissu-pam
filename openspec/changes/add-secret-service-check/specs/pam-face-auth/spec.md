## ADDED Requirements
### Requirement: Secret Service Availability Gate
The PAM module MUST verify the GNOME Secret Service session via the `keyring` crate before starting any face-capture work and short-circuit when the keyring is not usable.

#### Scenario: Secret Service probe runs before capture
- **WHEN** `pam_sm_authenticate` resolves configuration and the PAM target user
- **THEN** it calls a helper that uses the `keyring` crate to access the user's default Secret Service collection
- **AND** only proceeds to open V4L2 devices if the helper confirms the collection can be reached without error.

#### Scenario: Missing Secret Service returns PAM_IGNORE
- **WHEN** the keyring helper reports that Secret Service is locked, missing, or otherwise unreachable
- **THEN** the module logs the failure reason (and optionally emits a PAM conversation message)
- **AND** immediately returns `PAM_IGNORE` so downstream PAM modules (e.g., password) continue handling the authentication attempt.

#### Scenario: Successful probe is logged
- **WHEN** the Secret Service probe succeeds
- **THEN** the module emits an info-level log noting Secret Service availability for the user/service pair
- **SO** operators can confirm the prerequisite was satisfied before face authentication begins.
