## Overview
`pam_chissu` forks a helper that drops to the PAM target user before using `libsecret` via `chissu-face-core`. When PAM is triggered by `polkit-1` (1Password, GNOME Software, etc.) the service runs in a non-graphical environment so `$DISPLAY`, `DBUS_SESSION_BUS_ADDRESS`, and `XDG_RUNTIME_DIR` are unset. `libsecret` then tries to spawn a new session bus through `dbus-launch`, which fails without `$DISPLAY`. systemd-logind already tracks each logged-in session and exposes both the `Display` property and the user’s runtime directory through the `org.freedesktop.login1` API. We can interrogate logind before starting the helper and synthesize the missing environment variables.

## Session Discovery Strategy
1. Add a small `LogindInspector` (using `zbus` on the system bus) that queries `org.freedesktop.login1.Manager`.
2. Discovery flow:
   - Call `GetUser` with the target UID to obtain the `/org/freedesktop/login1/user/_UID` object.
   - Grab the `Sessions` property to enumerate `(session_id, object_path)` tuples.
   - For each session path, fetch `org.freedesktop.login1.Session` properties and pick the first session with `State == "active"` and `Class == "user"`. Prefer the session whose `TTY` matches the PAM conversation’s TTY (when provided via `pam_get_item(PAM_TTY)`), then any `Active=yes` session.
   - When no active session exists, record the reason so PAM can surface “Secret Service unavailable (no active logind session)” instead of a generic DBus failure.
3. Track enough metadata (session id, seat, type, display string, runtime path) for logging and environment materialization.

## Environment Mapping
- `Display` → `$DISPLAY` when non-empty; additionally set `$WAYLAND_DISPLAY` if `Type == "wayland"` and `Display` is non-empty.
- `User.RuntimePath` → `$XDG_RUNTIME_DIR`; also construct `$DBUS_SESSION_BUS_ADDRESS=unix:path=${runtime}/bus` if `bus` exists.
- Always preserve existing environment keys; only fill empty ones or override when config flag `prefer_logind_env` (default true) is set.
- Provide fallbacks:
  - If the session exposes no `Display`, keep current `$DISPLAY` (even if empty) and only set `$XDG_RUNTIME_DIR` / `$DBUS_SESSION_BUS_ADDRESS`.
  - If runtime dir missing, skip bus address synthesis but still attempt to set `$DISPLAY`.

## Helper Integration
- Extend `run_secret_service_helper` with an `EnvironmentSource` parameter.
- Parent:
  1. Resolves the target user, queries logind, and prepares a map of env pairs.
  2. Serializes the env map into the socketpair before fork or stores it in shared memory accessible to child (simplest: keep it in process memory and have child call `apply_logind_env(&map)` right after fork, before dropping privileges).
  3. Logs which session id/seat were used; on failure logs the structured reason.
- Child:
  1. Immediately calls `apply_logind_env` before `fetch_embedding_key`.
  2. Continues existing privilege drop + Secret Service fetch logic.
- All logind errors map to the existing Secret Service unavailable path so PAM can return `PAM_IGNORE`, but the syslog now states, e.g., `Secret Service unavailable; no active logind session for user sett4 (needed DISPLAY/DBUS vars)`.

## Error Handling and Security
- System bus access occurs before dropping privileges, so reads happen as root (PAM default). We must close the `zbus::Connection` before forking to avoid descriptor leaks.
- Timeouts: place a short (<200 ms) timeout on logind RPCs to avoid dragging authentication latency. If logind is unresponsive, skip environment hydration and keep existing helper behavior.
- Cache: avoid caching across requests to minimize stale data—PAM calls are infrequent and logind calls are cheap.
- Unit tests: stub the inspector trait so helper logic can be tested without D-Bus. Integration tests can fake a logind response to ensure `$DISPLAY`/`$DBUS_SESSION_BUS_ADDRESS` get set before invoking `fetch_embedding_key`.

## Testing & Observability
- Add unit tests for the inspector to ensure we pick the proper session ordering and environment bridging logic.
- Extend helper IPC tests to confirm the parent logs the session metadata and that helper inherits synthesized env vars (visible through `std::env::var`).
- Document manual verification: `loginctl list-sessions`, `busctl get-property org.freedesktop.login1 /org/.../session/_XX org.freedesktop.login1.Session Display`, and `env -i DISPLAY= DBUS_SESSION_BUS_ADDRESS= ... pamtester ...` to prove fallback works.
