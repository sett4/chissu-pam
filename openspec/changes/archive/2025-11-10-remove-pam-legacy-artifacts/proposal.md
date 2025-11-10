# remove-pam-legacy-artifacts

## Why
- `cargo build -p pam-chissu` still emits a compatibility symlink `libpam_chissuauth.so` by routing every link through `.cargo/config.toml` and `scripts/pam_linker_wrapper.sh`. Those helpers existed only for the transition from `pam_chissuauth` to `pam_chissu`, and now just clutter `target/<profile>/` with stale names.
- Package scripts that glob for `libpam_*.so` occasionally pick the compatibility symlink, forcing reviewers to reiterate that `libpam_chissu.so` is the only supported artefact. Dropping the legacy name removes that confusion.
- The linker wrapper copies and symlinks files on every build, adding brittle logic to our toolchain and increasing the chance of stale symlinks when `target/` is reused. Removing it lets us rely on Cargo’s default linker path again.

## What Changes
1. Delete `.cargo/config.toml` and `scripts/pam_linker_wrapper.sh` so Cargo emits the standard `target/<profile>/libpam_chissu.so` without creating `libpam_chissuauth.so`.
2. Refresh documentation (`README.md`, `docs/pam-auth.md`, any setup guides) to describe copying `libpam_chissu.so` into `/lib/security/pam_chissu.so` and to call out that no compatibility symlink is produced anymore.
3. Extend the `pam-face-auth` capability spec with a scenario that specifically rejects `libpam_chissuauth.so`, ensuring future contributors do not reintroduce it.
4. Validation: run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test -p pam-chissu`, then demonstrate via `ls target/release` (or similar) that the build produces `libpam_chissu.so` without any legacy artefacts.

## Impact
- **Maintainers** see only `libpam_chissu.so` in `target/<profile>/` and follow a single copy/rename step when installing the PAM module.
- **Packagers** no longer have to strip `libpam_chissuauth.so` from release tarballs, reducing support churn for the deprecated module name.
- **Tooling** gets simpler: we remove the custom linker wrapper and rely on Cargo defaults, lowering the risk of stale symlinks or partial copies when incremental builds abort.
