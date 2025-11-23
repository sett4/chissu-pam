## ADDED Requirements
### Requirement: Tag-triggered RPM Packages
The GitHub Actions release workflow MUST produce RPM artifacts whenever a `v<MAJOR>.<MINOR>.<PATCH>` tag is pushed.

#### Scenario: Tag push builds RPMs
- **WHEN** the release workflow runs for tag `v1.2.3`
- **THEN** it installs the necessary RPM tooling (`rpm-build`, `createrepo_c`, etc.) and executes `build/package-rpm.sh` for each supported distro, storing the resulting `.rpm` files under `dist/`
- **AND** failures building the RPM cause the workflow to fail so releases are never missing RPM assets silently

### Requirement: RPM Release Assets
GitHub Releases MUST include the RPM artifacts beside the `.deb` files.

#### Scenario: RPM assets uploaded
- **WHEN** the workflow publishes assets for tag `v1.2.3`
- **THEN** it uploads each generated `.rpm` file (e.g., `chissu-pam-1.2.3.fedora.x86_64.rpm`) to the tag’s GitHub Release via the same step that publishes `.deb` files
- **AND** the workflow surfaces an error if any `.rpm` upload fails so maintainers can rerun the job
