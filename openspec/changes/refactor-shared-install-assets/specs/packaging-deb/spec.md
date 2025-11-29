## ADDED Requirements
### Requirement: Deb Packaging Consumes Shared Installer Assets
The Debian/Ubuntu packaging workflow SHALL consume the shared installer templates/library for config defaults and model download hooks instead of maintaining separate copies.

#### Scenario: Deb build pulls shared config and hooks
- **WHEN** `build/package-deb.sh` stages package files
- **THEN** it copies the generated config template and any shared hook scripts from the common asset output
- **AND** it does not re-define prerequisite package lists or dlib download URLs independently (it reuses the shared library/templates).
