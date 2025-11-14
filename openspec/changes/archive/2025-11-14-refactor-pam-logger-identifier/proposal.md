# refactor-pam-logger-identifier

## Why
- `pam_chissu` appears as bare string literals inside `PamLogger`, both for the syslog identifier passed to `Formatter3164` and the fallback `eprintln!` branch. The duplication makes it trivial to introduce typos during future edits, which would silently diverge from the documented `journalctl -t pam_chissu` guidance.
- The `pam-face-auth` spec explicitly requires the syslog identifier to equal `pam_chissu`. Without a shared constant, we have no compile-time guard preventing new call sites from drifting away from that value.
- Reifying the identifier behind a constant also gives us a single place to document why the name cannot change (ABI compatibility with existing installations), simplifying future migrations.

## What Changes
1. Introduce a public (crate-visible) constant such as `const SYSLOG_IDENTIFIER: &str = "pam_chissu";` within `crates/pam-chissu/src/lib.rs` (or a dedicated module) and replace every existing literal with this constant.
2. Update `PamLogger::new` and the fallback stderr logging path to reference the constant so that syslog formatter setup and manual prints stay in lockstep.
3. Extend the `pam-face-auth` capability spec with a scenario that requires the module to centralize the identifier in a single constant, clarifying that any logging surface (syslog or stderr) must source that constant.
4. Validation: run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test -p pam-chissu`. Additionally, add or update a unit test (if feasible) that asserts the logger process field equals the constant to catch accidental drift.

## Impact
- **Operators** continue filtering via `journalctl -t pam_chissu` with reduced risk of future regressions because the identifier is now defined once.
- **Developers** gain a self-documenting constant that encodes the ABI requirement around the syslog identifier, making refactors (e.g., new logging sinks) safer.
- **Specs/Docs** explicitly call out the constant requirement, so future contributors know that any new logging code must depend on the shared identifier rather than duplicating string literals.
