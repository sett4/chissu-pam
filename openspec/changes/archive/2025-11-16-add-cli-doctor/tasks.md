## Tasks
- [x] Wire `chissu-cli doctor` subcommand with `--json` flag and summary exit codes.
- [x] Implement checks: config existence/parse (primary+secondary), video device availability/permissions, embedding store dir RW, model file readability, Secret Service availability via keyring, PAM module placement, PAM stack entry under /etc/pam.d.
- [x] Emit structured results (human + JSON) with pass/warn/fail per check and aggregate status.
- [x] Add unit/integration tests covering success, missing config, bad parse, missing device, and missing PAM module/config entries (using fixtures/mocks where hardware access is not available).
- [x] Update CLI docs/README with doctor usage and interpreting results.
