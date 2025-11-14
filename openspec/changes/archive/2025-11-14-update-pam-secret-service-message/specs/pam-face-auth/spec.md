## MODIFIED Requirements
### Requirement: Interactive Conversation Feedback
The module MUST keep conversational prompts clear for users even when falling back to passwords after Secret Service gating.
#### Scenario: Secret Service fallback prompt stays concise
- **WHEN** the Secret Service helper reports it is unavailable or locked and the module returns `PAM_IGNORE`
- **THEN** the conversation callback sends a short `PAM_ERROR_MSG` that simply states face authentication is unavailable and that PAM will fall back to other factors, without echoing the helper's internal reason
- **AND** the module still logs the full helper message to syslog so administrators keep full diagnostics
- **SO** end users are not exposed to verbose Secret Service errors while operators retain detailed logs.
