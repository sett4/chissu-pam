## Why
- Operators want Pluggable Authentication Module (PAM) support that grants access only when the camera detects a face matching previously enrolled descriptors (`faces enroll`).
- Existing tooling extracts and stores descriptors but there is no authentication integration for login flows.
- A dedicated PAM module enables on-device facial verification without duplicating enrollment or feature storage logic.

## What Changes
- Deliver a shared library `libpam_chissuauth.so` implementing PAM authentication (at minimum `pam_sm_authenticate` and `pam_sm_setcred`).
- Load configurable parameters (descriptor store directory, capture device, similarity threshold, sample timeout) from `/etc/chissu-pam/config.toml` with a fallback to `/usr/local/etc/chissu-pam/config.toml` and sane defaults.
- Capture live frames from the configured V4L2 video device, derive descriptors, and compare each against the target PAM user’s enrolled descriptors.
- Accept authentication when any captured sample is above the configured cosine-similarity threshold; otherwise return a PAM authentication failure.
- Emit structured syslog messages for auditing (start, success, failure, error conditions).
- Document configuration, integration steps, and testing guidance in project docs.

## Impact
- Introduces a Rust crate/binary artifact for the PAM module alongside existing CLI code.
- Adds configuration parsing, camera sampling logic, and descriptor comparison routines shared (or refactored) from existing `faces` functionality.
- Requires packaging guidance and potentially new build targets for producing `libpam_chissuauth.so`.
- Demands integration tests or harnesses that simulate PAM calls without physical hardware.
