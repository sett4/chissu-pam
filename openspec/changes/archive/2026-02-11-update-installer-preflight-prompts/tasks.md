## 1. Implementation
- [x] 1.1 Add package preflight in `install-chissu.sh` that detects missing deps per distro without installing; exit with guidance and suggested command.
- [x] 1.2 Ensure Fedora/Rocky/Arch/Debian branches share the prompt-only behaviour; keep existing paths for PAM wiring/artifact placement when deps present.
- [x] 1.3 Update README installer section and script help to state dependencies are not auto-installed.

## 2. Validation
- [x] 2.1 Run `openspec validate update-installer-preflight-prompts --strict`.
- [x] 2.2 Dry-run the installer with os-release fixtures for Debian/Fedora/Rocky/Arch to confirm messaging and no package installs are attempted.
