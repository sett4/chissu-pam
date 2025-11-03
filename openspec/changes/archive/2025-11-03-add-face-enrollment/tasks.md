## Implementation Checklist
- [x] Study existing `faces` CLI code and confirm option parsing + output patterns.
- [x] Implement per-user feature store abstraction with atomic read/modify/write and descriptor ID generation.
- [x] Wire new `faces enroll` subcommand that ingests extract JSON, validates descriptors, and appends them to the store with human + JSON output.
- [x] Implement `faces remove` subcommand that deletes descriptors by ID (or clears all) with matching output contracts.
- [x] Add unit / integration tests covering enroll and remove scenarios, including malformed input and concurrent write protection where practical.
- [x] Update documentation (README and relevant docs) to describe enrollment/removal workflows.
- [x] Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
