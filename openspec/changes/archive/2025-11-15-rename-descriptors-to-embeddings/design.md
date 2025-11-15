## Scope
Rename the primary face feature artifact from "descriptor" to "embedding" in user-facing surfaces (CLI flags, config keys, JSON/log output, docs) while keeping a backward-compatible path for existing users. No algorithmic change; storage format remains the same vector of floats.

## Proposed Naming Map
- Concept: descriptor -> embedding (default wording across code, docs, logs)
- Config keys: `descriptor_store_dir` -> `embedding_store_dir` (accept both; `embedding_store_dir` wins; emit warning when legacy key used)
- CLI flags/args: `--descriptor`/`--descriptors`/`--descriptor-id` -> `--embedding`/`--embeddings`/`--embedding-id` (accept legacy aliases, prefer new names in help)
- JSON fields: `descriptor_ids`, `descriptor_key`, `descriptor_vectors` -> `embedding_ids`, `embedding_key`, `embedding_vectors`; continue to accept legacy field names on input where applicable; outputs prefer new names.
- Types/structs: `Descriptor`/`EnrolledDescriptor` -> keep storage structs but rename public-facing types/aliases to `Embedding`/`EnrolledEmbedding` where non-breaking; add `type Descriptor = Embedding` aliases as needed to preserve code stability during transition.
- Files/paths: keep existing file extensions/locations; only change human-facing labels. Do NOT rename on-disk JSON property names in existing files automatically; loader should understand both.

## Compatibility Strategy
- Config loader: look for new key first, then legacy; log deprecation notice when legacy used.
- CLI parsing: prefer embedding flags in clap definitions; add hidden aliases for descriptor versions; deprecate via help text or warnings.
- IPC (PAM helper): accept both field sets; include both names when it is cheap or emit new names only and translate in parent where needed.
- Tests: add coverage for mixed new/old inputs to prevent regression during deprecation period.

## Out of Scope
- No change to crypto mechanics (AES-GCM keys) or Secret Service service names.
- No automatic migration of existing JSON stores; compatibility layer must read old field names.
