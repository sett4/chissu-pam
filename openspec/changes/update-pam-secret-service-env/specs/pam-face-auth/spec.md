## MODIFIED Requirements
### Requirement: Secret Service Availability Gate
Secret Service probing MUST execute in a helper that can impersonate the target desktop user before the PAM module touches camera resources, and the helper MUST hydrate the per-user environment required to talk to Secret Service when PAM itself is missing those variables.

#### Scenario: Helper impersonates target user session
- **WHEN** `pam_sm_authenticate` prepares to probe the Secret Service for the PAM target user
- **THEN** it MUST fork a helper child that closes unused file handles, calls `setgid`/`initgroups`/`setuid` to adopt that user, and runs the probe inside the helper before any camera capture begins
- **AND** the parent process MUST consume the helper's structured IPC response and only proceed to capture work when the helper reports success.

#### Scenario: Helper outcome drives PAM return codes
- **WHEN** the helper reports Secret Service is locked, missing, or unreachable for the target user
- **THEN** the parent MUST log the helper's message and immediately return `PAM_IGNORE` without opening V4L2 devices, matching the earlier Secret Service gating behavior.

#### Scenario: Helper rehydrates session environment via logind
- **WHEN** the parent detects that `$DISPLAY`, `$DBUS_SESSION_BUS_ADDRESS`, or `$XDG_RUNTIME_DIR` are missing in the PAM service environment
- **THEN** it MUST query `org.freedesktop.login1` for the target user's active session, extract the session's `Display` value and runtime directory, and provide those values (plus a synthesized `unix:path=${XDG_RUNTIME_DIR}/bus` address) to the helper before it contacts Secret Service
- **SO THAT** Secret Service lookups succeed even when PAM is invoked from `polkit-1` or other non-graphical services where those environment variables are not inherited.

#### Scenario: Logind unreachable still surfaces structured errors
- **WHEN** logind rejects the query, no active session is exposed for the user, or the runtime directory is missing
- **THEN** the parent MUST log the structured reason (e.g., `no active logind session for user sett4`) and continue with the existing helper flow
- **AND** if Secret Service remains unreachable after the helper runs, PAM returns `PAM_IGNORE` with the logind context included in the syslog message so operators know why the face flow was skipped.
