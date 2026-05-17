# chissu-pam Specs

This directory contains the current project Specs. They are the canonical behavior references for CLI, PAM, packaging, release, documentation, and workspace structure decisions.

## Spec Index

- [chissu-cli-capture](chissu-cli-capture.md): shared capture command behavior, infrared still capture, V4L2 negotiation, defaults, warm-up frames, and capture output contracts.
- [chissu-cli-doctor](chissu-cli-doctor.md): non-mutating environment diagnostics for CLI, PAM, Secret Service, camera, model, and optional polkit checks.
- [chissu-cli-faces](chissu-cli-faces.md): face extraction, comparison, enrollment, removal, embedding persistence, and config-driven live enrollment flows.
- [docs-readme](docs-readme.md): README structure and required project documentation coverage.
- [installer-scripts](installer-scripts.md): distro-aware installer behavior, artifact placement, config seeding, model provisioning, and PAM wiring.
- [packaging-deb](packaging-deb.md): Debian/Ubuntu package build, install hooks, model download, and pam-auth-update integration.
- [packaging-rpm](packaging-rpm.md): RPM package build, install hooks, model download, and authselect integration.
- [pam-face-auth](pam-face-auth.md): PAM facial authentication, configuration, Secret Service gating, helper IPC, and syslog behavior.
- [release-automation](release-automation.md): tag-triggered release package automation and GitHub Release asset publication.
- [workspace-structure](workspace-structure.md): Cargo workspace layout, crate placement, shared metadata, and test directory boundaries.

## Maintenance

- Keep each Spec focused on externally visible behavior and testable requirements.
- Update the relevant Spec in the same change as code or documentation that changes behavior.
- Use `Requirement` and `Scenario` sections for requirements that should be verified by tests, manual procedures, or release checks.
