## Tasks
1. [x] Remove the custom linker configuration/wrapper so `cargo build --release -p pam-chissu` emits the standard `libpam_chissu.so` output while no longer producing the legacy `libpam_chissuauth.so` symlink.
2. [x] Update documentation (`README.md`, `docs/pam-auth.md`, related guides) to explain that builders install `libpam_chissu.so` into `/lib/security/pam_chissu.so` and that only the canonical artefact remains.
3. [x] Modify `openspec/specs/pam-face-auth/spec.md` via this change to require the `libpam_chissu.so` build output and forbid `libpam_chissuauth.so` from appearing.
4. [x] Validation: run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test -p pam-chissu`, and capture an `ls target/release` (or `debug`) output showing that the build produces `libpam_chissu.so` with no compatibility artefacts.
