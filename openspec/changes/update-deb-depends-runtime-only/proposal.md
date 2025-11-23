## Why
Current .deb packages declare -dev packages as runtime Depends, forcing users to install development headers even though only shared libraries are needed at runtime.

## What Changes
- Align packaging spec so runtime Depends rely on shlibs detection and exclude -dev packages.
- Move required -dev libraries to Build-Depends in the Debian control template.
- Regenerate control template (no binary rebuild in this change).

## Impact
- Affects spec: packaging-deb
- Affects files: build/package/debian/control.in
- No API or CLI surface changes; packaging metadata only.
