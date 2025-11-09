# update-pam-chissu-target

## Why
- The PAM module is still published as the `pam-chissuauth` crate and produces `libpam_chissuauth.so`, while every documentation touchpoint (README.md:234, docs/pam-auth.md:3) tells operators to reference `pam_chissuauth.so`. The mismatch forces manual renaming after each build and causes confusion when wiring `/etc/pam.d/*`.
- Project branding elsewhere (CLI name `chissu-pam`, config paths `/etc/chissu-pam`, syslog identifier `pam_chissuauth`) is inconsistent. We want a single, memorable module name (`pam_chissu`) that matches what admins type inside PAM stacks and journalctl filters.
- Review checklists (AGENTS.md §Rust安全性) still mandate `cargo test -p pam-chissuauth`, so adopters can’t easily tell whether `pam_chissuauth` or `pam_chissu` is the canonical crate moving forward.

## What Changes
1. Rename the workspace crate (`pam-chissuauth/`) to `pam-chissu/` and update `Cargo.toml` entries so `cargo build -d pam_chissu` (i.e., `cargo build --release -p pam_chissu`) targets the new package. The shared library section will expose `name = "pam_chissu"` with `crate-type = ["cdylib"]`.
2. Ensure the build artefact placed under `target/<profile>/` is `pam_chissu.so` (without the `lib` prefix) so it can be dropped straight into `/lib/security/`. We will add a tiny `build.rs` (or workspace `xtask`) that copies/renames `libpam_chissu.so` to `pam_chissu.so` after linking and keep an optional compatibility symlink pointing to the new filename for one release.
3. Update every string constant, logging identifier, README snippet, and operator doc (e.g., `docs/pam-auth.md`, `AGENTS.md`, `openspec/specs/pam-face-auth/spec.md`) to reference the new crate/artefact name and `cargo` commands.
4. Refresh installation instructions to clarify: `cargo test -p pam_chissu`, `cargo build --release -p pam_chissu`, `sudo install -m 0644 target/release/pam_chissu.so /lib/security/pam_chissu.so`, and the expected PAM stack stanza `auth sufficient pam_chissu.so`.

## Impact
- **Builders**: one stable command (`cargo build -d pam_chissu`) yields the correctly named module, eliminating manual renames and associated CI scripting.
- **Operators**: docs, syslog identifiers, and PAM config snippets all point to `pam_chissu`, reducing drift between environments. Existing deployments can keep working via the temporary symlink.
- **QA**: test plans and the OpenSpec requirement for the PAM module shift to the new name, so reviewers know which crate/tests to run before merging.

## Open Questions
- Do we need to keep producing `libpam_chissuauth.so` beyond one release (e.g., for distro packaging scripts), or is a single-release symlink sufficient?
- Should we introduce an explicit installer command (`cargo xtask package-pam`) instead of doing the rename in `build.rs`, to give downstream packagers more control?
