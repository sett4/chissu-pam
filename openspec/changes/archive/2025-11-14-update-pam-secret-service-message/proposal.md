# Update PAM Secret Service Messaging

## Why
Secret Service gating currently surfaces detailed helper failure reasons directly to the PAM conversation. Operators reported that those messages appear noisy or confusing to end users, even though syslog already contains the diagnostics. We need to keep syslog rich but make PAM prompts minimal.

## What Changes
- Keep syslog logging exactly as today, including helper error reasons.
- When returning `PAM_IGNORE` because Secret Service is unavailable or locked, send a short, reasonless PAM conversation message (e.g., "Face unlock unavailable. Falling back to password.").
- Ensure all other failure and success flows keep their existing messaging.

## Impact
- Users receive a concise PAM prompt while the module still exits with `PAM_IGNORE`.
- Administrators still see detailed helper errors via syslog for troubleshooting.
