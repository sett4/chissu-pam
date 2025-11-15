## 1. Specification
- [x] 1.1 Update face-features, pam-face-auth, and docs-readme specs to use "embedding" terminology and note backward-compatible aliases for legacy descriptor inputs.

## 2. Terminology & Interfaces
- [x] 2.1 Audit code and docs for "descriptor" naming (functions, structs, config keys, CLI flags, JSON fields, log messages, tests) and draft a rename matrix -> embedding equivalents.
- [x] 2.2 Update CLI flags/args (e.g., compare/enroll/remove) and config keys to prefer embedding names; preserve descriptor aliases with deprecation warnings where practical.
- [x] 2.3 Adjust JSON/human-readable outputs and PAM helper payload field names to emit embedding terminology while accepting legacy descriptors on input.
- [x] 2.4 Refresh README and docs/ usage/configuration sections to describe embeddings, the new keys/flags, and the temporary compatibility behavior.

## 3. Validation
- [x] 3.1 Add/adjust tests covering embedding-preferring flows plus the descriptor compatibility paths (CLI unit/integration, PAM helper IPC where feasible).
- [x] 3.2 Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --workspace` to confirm the rename is safe.
- [x] 3.3 Update any recorded sample outputs/fixtures to reflect embedding wording.
