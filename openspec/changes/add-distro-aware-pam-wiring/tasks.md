## 1. Implementation

- [x] 1.1 Add distro detection branch in `install-chissu.sh` for PAM config handling (Debian/Ubuntu via pam-auth-update, RHEL/Fedora via authselect, Arch via include file) wiring only the `auth` facility and placing it before `pam_unix.so`.
- [x] 1.2 Add dry-run/uninstall switches that no-op or revert PAM changes safely.
- [x] 1.3 Add PAM snippet templates to `scripts/` (or embed) with tests for auth-only mode/placement logic.
- [x] 1.4 Update README/docs with distro-specific install + rollback steps.
- [ ] 1.5 Run `openspec validate` and existing linters/tests.

## 2. Validation

- [x] 2.1 `openspec validate add-distro-aware-pam-wiring --strict`
- [ ] 2.2 `cargo fmt && cargo clippy -- -D warnings`
- [ ] 2.3 Targeted script/unit tests (if added) or dry-run script invocation per distro matrix.
