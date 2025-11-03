# Proposal: Face Feature Enrollment CLI

## Summary
- Add a `faces enroll` subcommand that ingests descriptor JSON files produced by `faces extract` and attaches them to a user-scoped feature store.
- Introduce a `faces remove` subcommand that deletes previously enrolled descriptors for a user.
- Persist descriptors in a secure, append-only format that supports multiple descriptors per user and works with the planned PAM module.

## Motivation
Operators need a safe way to associate extracted face descriptors with individual Linux users before the PAM module can rely on them. Today there is no officially supported workflow to register or delete descriptors, so implementers must manage raw JSON files manually, risking mistakes and corrupting authentication data. Dedicated CLI flows reduce that risk and set expectations for storage layout and validation.

## Scope
### In Scope
- CLI argument parsing, validation, and JSON ingestion for `faces enroll` and `faces remove`.
- Definition of per-user storage layout (directory, filename convention) under the existing captures or state directory.
- Input validation, error reporting, and `--json` aware structured output for success and failure cases.
- Tests that cover enrolling single and multiple descriptors, duplicate handling, and removal paths without requiring PAM integration.

### Out of Scope
- Direct integration with PAM or system-wide privilege escalations.
- Cryptographic signing or encryption of the descriptor files (future enhancement if required by security review).
- Remote synchronization or distribution of enrolled descriptors.

## Success Criteria
- Running `faces enroll --user <name> <descriptor.json>` appends descriptors to that user’s store and exits 0, with both human-readable and JSON outputs available.
- Invalid input (missing file, malformed JSON, user store write failures) exits with non-zero status and surfaces actionable error messages.
- Running `faces remove --user <name> --descriptor-id <id>` removes the targeted descriptor, gracefully handling unknown IDs and empty stores.
- Automated tests exercise enroll and remove flows, ensuring descriptor stores remain valid JSON arrays and no data races occur.

## Risks and Open Questions
- Need to confirm storage path convention (e.g., `captures/enrolled/<user>.json`) and file permissions. Proposal assumes per-user JSON files owned by CLI user.
- Descriptor identity strategy: rely on feature vector hashes or generated UUIDs. This proposal leans toward deterministic UUID v4 assigned at ingestion.
- Concurrency: CLI may be invoked concurrently; must consider atomic file writes (temp file + rename) or file locking.

## Timeline
We expect implementation and tests within a single iteration once the proposal is approved.
