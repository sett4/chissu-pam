# refactor-workspace-layout

## Why
- The root `Cargo.toml` mixes `[workspace]` and `[package]`, so the CLI binary is defined at `.` (see `Cargo.toml:1-27`). That prevents us from treating the repository root as a pure workspace controller and forces every dependency/change to rebuild the CLI even when touching only leaf crates.
- Only `crates/chissu-face-core` lives under `crates/`; the CLI code still resides in `src/` and `pam-chissu/` sits beside `crates/`. This contradicts the intended multi-project layout and keeps tests/config files coupled to legacy paths.
- The documentation (README diagram, `AGENTS.md` governance, and reviewer workflow) already assumes each component ships as its own crate with co-located `tests/`. Without the layout split we keep accumulating relative-path hacks (e.g., `use chissu_face_core` via `path = "crates/..."`) and make it harder to share fixtures or enforce per-crate `cargo test` gates.

## What Changes
1. Turn the repository root into a workspace-only manifest: keep `[workspace]` + `resolver = "2"`, drop `[package]`, and declare member crates explicitly (`crates/chissu-cli`, `crates/chissu-face-core`, `crates/pam-chissu`). Populate `[workspace.package]` with the shared metadata (version, edition, license, authors) so leaf crates inherit consistent defaults.
2. Move the CLI crate into `crates/chissu-cli/`:
   - Create `Cargo.toml` mirroring the current root manifest dependencies/features.
   - Relocate `src/` and CLI-specific `tests/` under the new directory (plus `tests/` for subcommands).
   - Update documentation, scripts, and references (`README`, `docs/*`, `openspec`) to reference `cargo run -p chissu-cli`.
3. Move the PAM crate under `crates/pam-chissu/` so workspace-relative tooling, docs, and CI can address all crates through the same prefix. Ensure the crate metadata (`name = "pam_chissu"`, `crate-type`) remains intact after the move.
4. Keep `crates/chissu-face-core/` as-is but update dependency paths throughout (CLI + PAM + tests) to use the new relative location if any paths change.
5. Ensure repository-level `tests/` continues to host integration tests that exercise multiple crates together, while every crate has a local `tests/` directory for component-scoped checks. Document the intended split in README/docs so contributors know where to place new tests.
6. Refresh contributor workflow instructions (README, `AGENTS.md`, `docs/`) to spell out `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --workspace` expectations after the move.

## Impact
- **Build + tooling**: `cargo build --workspace` will compile each crate independently, enabling incremental rebuilds (e.g., touching PAM no longer recompiles CLI bins). CI scripts can explicitly target `-p chissu-cli` or `-p pam_chissu` without relying on relative paths.
- **Contributors**: Clearer directory ownership lowers onboarding cost. Each crate gains a `tests/` folder for targeted coverage while the top-level `tests/` remains for cross-crate flows.
- **Docs/ops**: Documentation references align with the physical layout, avoiding ambiguity when instructing operators where binaries/libraries originate.

## Decisions
- We will switch consumers directly to the `crates/` layout without keeping a legacy `src/` at the repository root.
- Shared metadata lives in `[workspace.package]` so all crates inherit the same version, edition, and license.
