## Why
The installer script now wires PAM stacks per distro (`pam-auth-update`, `authselect`, Arch includes), but `.deb`/`.rpm` packages still leave PAM unchanged, forcing manual steps after package installs. We need package-native post-install/uninstall wiring so package users get the same automated experience and removal safety without running `scripts/install-chissu.sh` separately.

## What Changes
- Add distro-aware PAM wiring/unwiring to Debian/Ubuntu maintainer scripts (`postinst`, `prerm`/`postrm`) using `pam-auth-update` snippets.
- Add equivalent PAM wiring for RPM-based installs via `%post`/`%postun` using `authselect` custom profiles, aligned with the CLI installer logic.
- Keep model download hooks intact while making the PAM steps idempotent, preview-friendly (logs), and safe on uninstall/upgrade.

## Impact
- Affected specs: `packaging-deb`, `packaging-rpm` (new PAM wiring requirements; align with installer-scripts behaviour).
- Affected code: `build/package-deb.sh` templates (`postinst`, `prerm`, `postrm`, control metadata if needed), `build/package-rpm.sh` and `build/package/rpm/chissu-pam.spec.in` hooks/templates, possibly shared PAM snippets under `scripts/pam/` for reuse.
- Users installing from packages get PAM stack wired automatically and removed cleanly per distro defaults without manual edits.
