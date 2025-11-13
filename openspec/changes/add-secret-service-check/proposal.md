## Why
- `pam_chissu` depends on GNOME Secret Service to eventually store and retrieve descriptor encryption keys (DEKs), but today it never verifies whether the DBus-backed Secret Service for the user session is actually available.
- When a Linux login stack invokes `pam_chissu` before the graphical/keyring session unlocks, Secret Service APIs fail; at that point the user is still required to type their password, so performing face authentication has no benefit and only slows the login path.
- We need a deterministic, logged signal that Secret Service is missing so future work (DEK storage) can depend on the guarantee that any successful authentication ran with an unlocked keyring.

## What Changes
- Introduce the `keyring` crate to `pam_chissu` and add a helper that attempts to connect to the session Secret Service using the configured PAM target user before any face capture begins.
- Wire the helper into the configuration/initialization flow so that a missing DBus session, locked keyring, or other `keyring` error causes the module to log the reason and immediately return `PAM_IGNORE`, allowing the remaining PAM modules (password, etc.) to continue.
- Add structured logging plus (optional) PAM conversation feedback that explains the module skipped because Secret Service was unavailable.
- Extend the `pam-face-auth` spec with a "Secret Service Availability Gate" requirement covering both the success (proceed) and failure (return `PAM_IGNORE`) paths, including the expectation that the check happens before any capture work.
- Document the new behavior in the PAM README/configuration notes so operators know why the module may be skipped and how to troubleshoot (e.g., ensure the session bus and keyring are started).

## Impact
- Adds a new dependency (`keyring`) and introduces an early-return path in authentication, but keeps hardware capture untouched when Secret Service is ready.
- Users whose sessions do not provide Secret Service will simply fall back to password modules; no additional prompts appear beyond a single informational log/conversation message.
- Future DEK encryption work can rely on this guard, reducing the risk of partially initialized state or plaintext descriptor files.
