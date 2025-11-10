## ADDED Requirements
### Requirement: Workspace Manifest Separation
The repository root MUST contain a workspace-only `Cargo.toml` that declares member crates and shared metadata without defining its own package.

#### Scenario: Root manifest exposes only workspace configuration
- **GIVEN** a maintainer opens `Cargo.toml` at the repository root
- **WHEN** they inspect the file
- **THEN** it contains `[workspace]` (and optional `[workspace.package]`/`[workspace.dependencies]`) but no `[package]` section
- **AND** the `members` list enumerates `crates/chissu-cli`, `crates/chissu-face-core`, and `crates/pam-chissu`.

#### Scenario: Workspace package shares metadata
- **WHEN** a maintainer inspects `[workspace.package]`
- **THEN** it defines the shared metadata (at minimum `edition`, `version`, `authors`, and `license`) so member crates inherit consistent defaults without duplicating the fields.

### Requirement: Crate Directory Placement
All Rust crates in this project MUST live under `crates/<name>/` with crate-specific manifests, including the CLI binary and PAM module.

#### Scenario: CLI crate resolved under crates/chissu-cli
- **WHEN** a maintainer runs `cargo metadata -p chissu-cli`
- **THEN** the package manifest path resolves to `crates/chissu-cli/Cargo.toml`
- **AND** the crate's `src/main.rs` and `tests/` live under the same directory.

#### Scenario: PAM crate resolved under crates/pam-chissu
- **WHEN** a maintainer runs `cargo metadata -p pam_chissu`
- **THEN** the package manifest path resolves to `crates/pam-chissu/Cargo.toml`
- **AND** the resulting build artefact (`pam_chissu.so`) originates from that directory without relying on files outside `crates/`.

### Requirement: Test Directory Split
Each crate MUST own a local `tests/` directory for component-scoped coverage while repository-level integration tests remain in the top-level `tests/` folder for cross-crate flows.

#### Scenario: Component tests live beside their crate
- **WHEN** a contributor adds CLI-specific integration tests
- **THEN** they place them under `crates/chissu-cli/tests/`
- **AND** `cargo test -p chissu-cli` runs those tests without needing the repository-level `tests/` folder.

#### Scenario: Cross-crate flows stay in top-level tests
- **WHEN** a test exercises both the CLI and PAM module together
- **THEN** it is added to the repository-level `tests/` directory
- **AND** `cargo test --workspace` executes it once against the workspace, independent of per-crate `tests/` folders.
