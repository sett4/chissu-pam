## ADDED Requirements
### Requirement: Interactive Conversation Feedback
The PAM module MUST leverage the `pam_conv` callback to surface success, retry, and failure status messages to the invoking PAM client while still logging to syslog.

#### Scenario: Success message uses PAM_TEXT_INFO
- **WHEN** `pam_sm_authenticate` is about to return `PAM_SUCCESS` because a descriptor match was found
- **THEN** the module calls the PAM conversation function with message style `PAM_TEXT_INFO`
- **AND** the message clearly states that face authentication succeeded (optionally including the PAM service name).

#### Scenario: Failures send PAM_ERROR_MSG
- **WHEN** an authentication attempt ends without a matching descriptor (because descriptors are missing, no face was detected, or the threshold was not met)
- **THEN** before returning `PAM_AUTH_ERR` the module invokes the conversation callback with style `PAM_ERROR_MSG`
- **AND** the text explains why the attempt failed and that PAM may offer another retry depending on the stack configuration.

#### Scenario: Retry instructions use PAM_ERROR_MSG
- **WHEN** the module needs the user to adjust while it keeps capturing (e.g., no face detected but timeout not reached)
- **THEN** it emits a single `PAM_ERROR_MSG` via the conversation callback to instruct the user to stay in frame or adjust lighting while the module retries within the same `pam_sm_authenticate` call.

#### Scenario: Missing conversation handler handled gracefully
- **WHEN** PAM does not supply a conversation structure or invoking it fails
- **THEN** the module logs a warning and continues without crashing, still returning the correct PAM code for the authentication result.
