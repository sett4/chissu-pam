# docs-readme Specification (Delta: rename-descriptors-to-embeddings)

## MODIFIED Requirements
### Requirement: Why This Project Highlights Secret Service Security
The README MUST explain why the project is secure by design, emphasizing Secret Service–backed embedding encryption and the reduced need for root.

#### Scenario: Why section sells security benefits
- **WHEN** a reader opens the "Why This Project" section
- **THEN** it states that embedding files are encrypted via GNOME Secret Service (AES-GCM) so leaked files remain unreadable
- **AND** it clarifies that everyday enrollment runs without `root` because Secret Service operates in the user session (only PAM wiring under `/etc/pam.d` needs elevated rights).

### Requirement: Usage Documents chissu-cli Enroll Flow
Usage MUST include examples for enrolling faces via the CLI, including elevated and non-elevated patterns, using embedding-oriented flags and outputs.

#### Scenario: Standard enroll example included
- **WHEN** someone reads Usage → Enrollment
- **THEN** they see a command example for `chissu-cli enroll` that references the landmark/encoder models, explains default target user behavior, and shows embedding terminology for outputs/IDs.

### Requirement: Configuration Section Explains chissu-pam TOML
A dedicated Configuration section MUST explain `chissu-pam` TOML files, precedence, and common keys using embedding-oriented names, while noting legacy descriptor key compatibility during transition.

#### Scenario: Config precedence documented
- **WHEN** an operator opens the Configuration section
- **THEN** it lists `/etc/chussu-pam/config.toml` and `/usr/local/etc/chussu-pam/config.toml`, describes how CLI/PAM fall back across them, and highlights important keys (device, pixel format, embedding_store_dir with legacy descriptor_store_dir alias, similarity thresholds, Secret Service toggles, etc.).
