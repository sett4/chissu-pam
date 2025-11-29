## Context
Package installs currently drop binaries/configs and download dlib models, but leave PAM stack wiring manual unless operators run `scripts/install-chissu.sh`. Debian/Ubuntu should use `pam-auth-update` snippets; RHEL/Fedora should use `authselect` custom profiles. We need parity with the CLI installer while keeping package post scripts safe and idempotent.

## Goals / Non-Goals
- Goals: automate PAM wiring/unwiring during package install/remove; ensure ordering before `pam_unix.so`; reuse distro-native tools; keep hooks idempotent and logged.
- Non-Goals: change PAM behaviour for Arch packages (Arch packaging not in scope), redesign installer CLI, or add new runtime dependencies beyond tooling already expected on each distro.

## Decisions
- Debian/Ubuntu: ship a `/usr/share/pam-configs/chissu` snippet from the package and run `pam-auth-update --package --enable chissu` in `postinst`; `prerm`/`postrm` use `pam-auth-update --package --remove chissu` and delete the snippet on purge; guard with presence check for pam-auth-update and exit non-zero with a clear message if missing.
- RPM (RHEL/Fedora): `%post` creates/updates a custom `authselect` profile derived from `sssd` (name `custom/chissu`), injects `auth sufficient libpam_chissu.so` before `pam_unix.so` in `system-auth` and `password-auth` templates, backs up prior selection, then runs `authselect select` + `apply-changes`; `%postun` restores previous profile on erase and removes the custom profile when safe.
- Shared: keep model download hooks unchanged; PAM steps should be no-ops on upgrade when already wired; log operations to stdout/stderr for package manager visibility.

## Risks / Trade-offs
- authselect changes can brick PAM if misapplied: mitigate by backing up previous selection and failing fast if `authselect current` shows unsynced state.
- pam-auth-update may be absent on minimal installs: decide whether to depend on it explicitly or emit actionable error; initial plan is to check and abort install so user can install `pam-auth-update` package.
- Package scripts run as root under maintainer-scripts constraints; keep shell minimal (POSIX sh) to avoid bashisms.

## Migration Plan
- Add PAM assets to package templates, update maintainer scripts/spec hooks, and validate with dry-run builds.
- Document uninstall behaviour so operators know how to remove PAM wiring.

## Open Questions
- Should Debian package declare `pam-auth-update` in `Depends` or handle missing command with a warning but continue? (lean: hard requirement to avoid partially wired PAM).
- For RPM, should we support systems without `authselect` by falling back to direct `/etc/pam.d` edits? (lean: fail with guidance to install authselect, matching distro defaults.)
