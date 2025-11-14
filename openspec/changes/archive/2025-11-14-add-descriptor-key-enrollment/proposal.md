## Summary
`chissu-cli faces enroll` currently appends descriptors directly to plaintext JSON feature stores. We need the command to create/rotate a per-user AES-GCM key in Secret Service, decrypt any existing store with the prior key, and re-encrypt the updated descriptor store with a freshly generated key that is immediately written back to Secret Service.

## Motivation
- PAM helper now expects Secret Service to return a 32-byte AES-GCM key, but the enrollment workflow never provisions one.
- Feature stores remain plaintext at rest, so even if the PAM module enforces key availability the descriptor data stays unprotected.
- Operators need an automated way to rotate keys on every enrollment so compromised keys cannot decrypt newly written descriptors.

## Goals
- `faces enroll` MUST fetch an existing descriptor key (if present), use it to decrypt the user’s store, generate a brand-new 32-byte key, register it with Secret Service, and encrypt the updated store with that key.
- When no key exists yet, the command should create one, register it, and encrypt the resulting store while still accepting plaintext legacy stores for migration.
- Surface actionable errors when Secret Service is unavailable or when previously stored keys are malformed.
- Ensure PAM can decrypt encrypted stores by passing helper-provided keys into the descriptor loading path even when `require_secret_service = false`.

## Non-Goals
- Changing the descriptor extraction format (`faces extract`) or how descriptors are serialized before encryption.
- Introducing partial updates or incremental encryption; each enrollment rewrites the full store as it already does.
- Modifying PAM’s helper IPC schema — it already returns AES-GCM keys.

## Risks & Mitigations
- **Secret Service update succeeds but store write fails**: best effort rollback by restoring the previous key (or deleting the new entry) when encryption/write errors occur.
- **Legacy plaintext stores without keys**: detect the format automatically, accept plaintext for the first enrollment, then write the encrypted form.
- **Headless/CI environments lacking Secret Service**: commands fail with explicit guidance rather than silently writing plaintext; unit tests will rely on a stubbed backend to avoid DBus dependencies.

## Success Metrics
- Running `faces enroll` on a user with an existing store results in a new Secret Service entry (Base64 key) and an encrypted store file that PAM can decrypt using the helper key.
- CLI logs mention key rotation, and PAM can load descriptors when `require_secret_service=true`; when `false`, it auto-escalates to the helper if the store is encrypted.
- Tests cover encryption/decryption, key rotation rollback, and helper fallback in PAM.
