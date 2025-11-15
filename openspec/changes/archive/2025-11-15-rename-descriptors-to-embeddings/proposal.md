## Why
"Descriptor" aligns with older feature-extraction terminology and is confusing versus modern face-recognition vocabulary. We want the workspace to speak in terms of "embeddings" so docs, config, CLI, and PAM output match common practice and are clearer to contributors.

## What Changes
- Rename the primary face feature artifact from "descriptor" to "embedding" across README, docs, CLI/PAM user-facing messages, JSON/log output, and code identifiers.
- Update configuration keys and CLI flags/fields to prefer embedding-oriented names while providing backward-compatible aliases for existing descriptor keys during the transition.
- Refresh specs and tests to assert the new terminology and ensure the compatibility layer is covered.

## Impact
- Affected specs: face-features, docs-readme, pam-face-auth.
- Affected code: chissu-cli (faces extract/compare/enroll/auto_enroll), chissu-face-core (models, errors, secret_service), pam-chissu (helper responses, logging), config schemas/validation, docs under README and docs/.
