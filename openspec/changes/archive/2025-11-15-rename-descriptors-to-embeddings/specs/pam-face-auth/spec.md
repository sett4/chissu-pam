# pam-face-auth Specification (Delta: rename-descriptors-to-embeddings)

## MODIFIED Requirements
### Requirement: PAM Facial Authentication Module
The system MUST provide a shared library for PAM authentication named `libpam_chissu.so`, validating a live camera capture against embeddings enrolled for the target user and surfacing embedding terminology in logs/prompts while still accepting legacy descriptor stores.

#### Scenario: Missing user embeddings abort
- **WHEN** the target user's embedding file is absent in the configured store directory
- **THEN** the module returns `PAM_AUTH_ERR`
- **AND** emits a syslog message indicating embeddings are missing (or descriptor store missing, when a legacy file name is detected).

### Requirement: Configurable Similarity And Capture Parameters
The module MUST load operational parameters from TOML configuration files via the shared `chissu-config` loader and honour documented defaults, preferring embedding-oriented keys while accepting legacy descriptor keys for compatibility.

#### Scenario: Shared loader keeps CLI and PAM aligned
- **GIVEN** both `chissu-cli` and `pam-chissu` import the `chissu-config` crate
- **WHEN** `/etc/chussu-pam/config.toml` defines `video_device = "/dev/video2"`, `warmup_frames = 6`, and `embedding_store_dir = "/srv/chissu/models"` (or the legacy `descriptor_store_dir`)
- **THEN** the PAM module resolves those values through the shared loader in the same order (primary path → secondary path → defaults) as the CLI
- **AND** any parse/read failure bubbles up from the shared loader so both binaries report the same error wording.

### Requirement: Embedding Key Helper Responses
The PAM module MUST exchange JSON messages with the helper so it can return embedding encryption keys or typed errors while understanding legacy descriptor-key field names for existing deployments.

#### Scenario: Successful key retrieval
- **WHEN** the helper reads the embedding AES-GCM key from the target user's Secret Service entry
- **THEN** it returns `{ "status": "ok", "embedding_key": "<base64>" }` (and continues to accept `descriptor_key` during transition)
- **AND** the parent parses the payload, validates the key length, and continues authentication using the decrypted embedding store.

#### Scenario: Missing key response
- **WHEN** no embedding key exists for the user in Secret Service
- **THEN** the helper returns `{ "status": "missing", "message": "..." }`
- **AND** the parent treats this as `FailureReason::DescriptorsMissing`/`EmbeddingsMissing`, surfacing the same PAM conversation/error flow as a missing embedding file.
