## Overview
We will add a Rust-based PAM authentication module that validates the invoking Unix user by comparing a live camera capture against descriptors previously enrolled via `study-rust-v4l2 faces enroll`. The module will be built as `libpam_chissuauth.so` and expose the standard PAM entry points.

## Key Components
- **Configuration Loader**: Reads TOML configuration from `/etc/chissu-pam/config.toml` with a fallback to `/usr/local/etc/chissu-pam/config.toml`. Supports fields:
  - `video_device` (string, default `/dev/video0`)
  - `descriptor_store_dir` (string, default `/var/lib/study-rust-v4l2/models`)
  - `similarity_threshold` (float, default `0.7`)
  - `capture_timeout_secs` (integer, default `5`)
  - `frame_interval_millis` (optional integer to pace sampling)
- **Descriptor Store Loader**: Resolves the PAM target user (`PAM_USER`) and loads their descriptors from the configured store directory, failing fast if none exist.
- **Camera Sampler**: Uses the existing V4L2 capture pipeline (refactored into a reusable component) to stream frames until timeout. Each frame is converted into the descriptor representation used in enrollment. Sampling stops as soon as a descriptor crosses the similarity threshold.
- **Similarity Engine**: Reuses the cosine-similarity routines from the `faces compare` command. It compares every captured descriptor against all enrolled descriptors for the user.
- **Syslog Logging**: Utilises `libc::openlog` / `syslog` bindings (via `syslog` crate or `tracing-subscriber` integration) to report module start, configuration used, match success, match failure, and errors. All logs include the PAM service name for easier audit filtering.

## PAM Flow
1. `pam_sm_authenticate` loads configuration, resolves `PAM_USER`, and opens syslog context.
2. It loads the enrolled descriptor file for the user; if missing, authentication fails with `PAM_AUTH_ERR`.
3. The sampler captures frames until timeout, producing descriptors per frame.
4. After each descriptor is computed, we compute cosine similarity against the enrolled data. If any value ≥ threshold, we return `PAM_SUCCESS`.
5. If timeout expires without a match or errors occur, we log the issue and return the appropriate PAM error code (`PAM_AUTH_ERR` for mismatch, `PAM_SYSTEM_ERR` for fatal errors).
6. `pam_sm_setcred` simply returns `PAM_SUCCESS` because credentials are unchanged.

## Reuse & Refactors
- Extract descriptor encoding and similarity code from `faces.rs` into a shared module so both the CLI and PAM module avoid duplication.
- Consider factoring camera capture helpers to reuse the existing warm-up and control logic without pulling in the entire CLI stack.
- Ensure that the new crate does not require CLI-only dependencies like `clap`.

## Testing Strategy
- Introduce unit tests for configuration fallback ordering and descriptor matching.
- Provide integration tests that feed synthetic descriptors into the sampler and bypass actual camera access (e.g., trait-based sampler allowing mocked frames).
- Document a manual testing workflow that requires physical hardware, while automated tests rely on mocks to satisfy CI.

## Deployment Notes
- Update documentation with build instructions (`cargo build --release -p pam-chissuauth`), library installation steps, PAM service configuration snippet, and configuration file schema.
- Consider future packaging (Debian/Ubuntu `.so` placement under `/lib/security`).
