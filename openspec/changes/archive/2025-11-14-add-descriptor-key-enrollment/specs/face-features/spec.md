## MODIFIED Requirements
### Requirement: Face Feature Enrollment Command
The CLI MUST manage descriptor encryption keys via Secret Service when enrolling descriptors, rotating them on every run, and writing the per-user store in encrypted form.

#### Scenario: Enrollment creates or rotates AES-GCM key
- **WHEN** `chissu-cli faces enroll --user <name>` runs
- **THEN** the command fetches the existing AES-GCM descriptor key for `<name>` from Secret Service (service `chissu-pam`)
- **AND** if a key exists it decrypts the user’s current store before appending descriptors
- **AND** the command generates a new 32-byte AES-256-GCM key, registers it in Secret Service, and encrypts the updated store with that key before exiting.

#### Scenario: Secret Service errors abort enrollment
- **WHEN** Secret Service is locked/unavailable or returns an invalid/malformed key
- **THEN** the enroll command logs the failure, exits non-zero, and leaves both the feature store and the previously registered key untouched so operators can retry safely.

#### Scenario: Legacy plaintext store migration
- **WHEN** a user’s feature store is still plaintext (no AES key registered)
- **THEN** the first enrollment run accepts the legacy file, generates/registers a new key, and re-writes the store using the encrypted format so subsequent PAM authentications can decrypt it via the helper key.

### Requirement: Face Feature Removal Command
The removal flow MUST reuse the encrypted store format and Secret Service key so descriptors remain protected when entries are deleted.

#### Scenario: Removal decrypts and re-encrypts store
- **WHEN** `chissu-cli faces remove` deletes descriptors for a user with an encrypted store
- **THEN** it fetches the user’s AES-GCM key from Secret Service, decrypts the store, removes the requested descriptors, and rewrites the store encrypted with the same key before exiting.

#### Scenario: Missing key detection
- **WHEN** the removal command encounters an encrypted store but cannot obtain the Secret Service key
- **THEN** it fails with a descriptive error instructing the operator to unlock Secret Service (or rerun enroll) instead of corrupting or rewriting the store.
