## Why
- PAM sessions triggered by polkit-1 (e.g., 1Password unlock) run without `$DISPLAY`/`DBUS_SESSION_BUS_ADDRESS`, so the Secret Service helper dies with `Secret Service platform failure: DBus error: Unable to autolaunch a dbus-daemon without a $DISPLAY for X11`.
- Without Secret Service the module returns `PAM_IGNORE`, forcing a password fallback and negating hands-free unlock—precisely the flow the project wants to showcase.
- logind already tracks each local user's active session, including exported environment variables (`loginctl show-session <id>` lists `DISPLAY`, `XDG_RUNTIME_DIR`, etc.). We can leverage the `org.freedesktop.login1` API to recover these variables inside PAM where the graphical environment is missing.

## What Changes
- Add a logind session inspector that, given a PAM target user, discovers their active session via `org.freedesktop.login1.Manager` (preferring the session attached to the current TTY/seat) and fetches that session's environment key/value pairs.
- Teach the Secret Service helper launcher to populate DISPLAY/DBUS/XDG variables from the inspector result before invoking libsecret, falling back to existing behavior when logind is unavailable or returns no active session.
- Expose configuration + logging so operators can see which session/environment was reused and why the helper still skipped Secret Service (e.g., no graphical session, session locked, logind denied access).
- Update docs/specs to describe the new dependency on logind when `require_secret_service = true`, plus troubleshooting instructions for headless or remote users.

## Impact
- Introduces a DBus dependency (likely through `zbus` or `dbus` crates) inside `pam_chissu`, so cross-compiling and minimum Rust version considerations must be reviewed.
- Forces careful privilege handling: the parent PAM process must read logind data as root but setenv()/exec inside the already unprivileged helper; this slightly complicates the helper IPC lifecycle and test plan.
- Adds new failure modes (logind unavailable, missing session), so PAM/CLI logging and spec requirements must cover these to keep operators informed.
- Documentation needs to clarify that graphical keyring unlock now depends on logind-provided session metadata and to outline how to test the flow via `loginctl env-status`/`show-session`.
