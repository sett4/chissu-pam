## 1. Implementation
- [x] 1.1 Mirror current PAM wiring logic from `scripts/install-chissu.sh` (pam-auth-update/authselect/Arch include) and identify reusable snippets for packages.
- [x] 1.2 Update Debian/Ubuntu maintainer scripts (`postinst`, `prerm`, `postrm`) to wire/unwire via `pam-auth-update` with idempotent snippet handling and clear logs.
- [x] 1.3 Update RPM spec `%post`/`%postun` to create/remove an authselect-derived profile that inserts `libpam_chissu.so` before `pam_unix.so`, with safe rollback on removal/upgrade.
- [x] 1.4 Adjust packaging helpers/templates as needed (control/spec metadata, bundled PAM snippet assets) to support the new hooks across `build/package-deb.sh` and `build/package-rpm.sh`.
- [x] 1.5 Document the new package behaviour in release notes/README packaging section if required by specs.

## 2. Validation
- [x] 2.1 `openspec validate add-pam-wiring-to-packages --strict`.
- [x] 2.2 Dry-run build scripts where possible (no install) to ensure templates render without shell errors.
- [x] 2.3 Align manual test notes for package installs/uninstalls per distro (pam-auth-update/authselect) for reviewers. (See notes in final summary.)
