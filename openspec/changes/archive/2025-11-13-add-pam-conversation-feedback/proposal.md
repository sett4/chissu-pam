## Why
- The PAM module currently logs start/success/failure events to syslog only; terminal or display managers never surface them to the user because the module does not call the PAM conversation callback (`pam_conv`).
- Authenticators want immediate, human-readable hints about what is happening (e.g., when capture retries occur or when the match succeeds) to avoid silent waits or unexplained failures.
- PAM best practices expect success notifications (`PAM_TEXT_INFO`) and retry/failure notices (`PAM_ERROR_MSG`) so front-ends like `login`, `sudo`, and display managers can show context-sensitive prompts.

## What Changes
- Teach `pam_chissu` to acquire the conversation function from the PAM handle and send informational messages whenever authentication succeeds, fails, or schedules another capture attempt.
- Emit `PAM_TEXT_INFO` with a short success string after the module determines a descriptor match, and emit `PAM_ERROR_MSG` when an attempt fails (frames exhausted, descriptors missing) or when the module is about to retry after a transient issue.
- Update the `pam-face-auth` specification with an explicit "Interactive Conversation Feedback" requirement that covers both success and retry/failure scenarios, plus minimum wording expectations.
- Add tests (unit or integration harness) that stub the PAM conversation structure to verify both message types are invoked with the expected sequencing and contents.
- Document the new UX behavior (including how service stacks can suppress/forward the messages) in README or PAM usage notes if gaps exist today.

## Impact
- Requires new helper(s) for safely formatting and sending conversation messages without panicking even if the PAM stack lacks a conversation function.
- Slight behavior change for existing deployments: users will now see textual feedback when authentication succeeds or fails; syslog logging remains but terminal UX improves.
- Encourages additional tests around PAM handle plumbing, which may surface previously untested error paths (e.g., missing `pam_conv`).
