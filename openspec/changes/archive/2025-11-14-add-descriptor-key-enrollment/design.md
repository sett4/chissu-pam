## Overview
We will introduce end-to-end encryption for user descriptor stores using AES-256-GCM with per-user keys stored in Secret Service. The CLI enrollment/removal flows will use a pluggable `DescriptorKeyBackend` so unit tests can stub Secret Service while production code relies on the keyring crate. Stores are serialized to JSON, encrypted, and written atomically with a new wrapper structure (`version`, `algorithm`, Base64 `nonce`/`ciphertext`).

## Key Lifecycle
1. `faces enroll` fetches the existing key (if any). When present, it decrypts the current store before appending descriptors.
2. The CLI generates a new 32-byte key via `OsRng`, registers it immediately (`Entry::set_password`) using the same service/user tuple the PAM helper expects, and re-encrypts the full store with that key.
3. On write failure, the logic attempts to restore the previous key (re-set it or delete the entry) to keep PAM and the store in sync.
4. `faces remove` reads/writes stores using the latest key but does not rotate; it simply reuses the stored key so descriptors remain decryptable without changing PAM state.

## Storage Format
Encrypted stores are saved as:
```json
{
  "version": 1,
  "algorithm": "AES-256-GCM",
  "nonce": "<base64>",
  "ciphertext": "<base64>"
}
```
Plaintext (legacy) arrays are still accepted; the first enrollment after the change will migrate them by reading plaintext, generating a key, and writing the encrypted wrapper.

## PAM Integration
`pam_chissu` now receives the raw AES-GCM key bytes from the helper and passes them into `load_enrolled_descriptors`. When configuration disables the Secret Service gate, descriptor loading first attempts without a key; if the store is encrypted, the module lazily invokes the helper to fetch a key and retries before surfacing an error. Helper outcomes remain unchanged (`ok`, `missing`, typed errors), but keys are finally used for decryption.

## Testing Strategy
- Unit tests in `faces.rs` use a stub backend to simulate key fetch/store/delete operations and verify that encrypted files round-trip, key rotation occurs, and rollback restores the previous key on failure.
- Additional tests cover `read_enrolled_store` detecting encrypted content and requiring a key.
- PAM tests exercise the helper fallback path by stubbing the descriptor loader to report `EncryptedStoreRequiresKey` once, ensuring the module retries with helper-provided bytes.
