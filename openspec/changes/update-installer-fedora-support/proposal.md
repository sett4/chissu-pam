## Why
Rocky Linux support in `install-chissu.sh` assumes RHEL-style server hosts, but we want an installer path that matches Fedora workstations where most contributors test. Fedora currently mis-detects as unsupported (ID=fedora) and needs a package set that does not rely on EPEL/CRB.

## What Changes
- Update OS detection and prerequisite handling in `install-chissu.sh` to explicitly support Fedora (ID=fedora or ID_LIKE=fedora) while keeping Rocky/RHEL fallback.
- Adjust dependency lists and PAM/library placement logic to treat Fedora like other 64-bit RPM distros, without EPEL/CRB enablement.
- Refresh installer spec scenarios to include Fedora coverage and note the Fedora vs Rocky differences.

## Impact
- Affected specs: installer-scripts
- Affected code: scripts/install-chissu.sh, scripts/lib/install_common.sh, shared assets/docs if references mention Rocky-only support
