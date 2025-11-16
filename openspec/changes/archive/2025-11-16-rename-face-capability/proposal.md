# Proposal: Rename face capability to chissu-cli-faces

## Why
- Align spec naming with the `chissu-cli faces` command and other CLI-scoped capabilities (e.g., `chissu-cli-capture`).
- Reduce ambiguity between face feature extraction and the CLI surface that hosts those subcommands.

## What Changes
- Rename the `face-features` capability to `chissu-cli-faces`.
- Update the spec title and any capability self-references accordingly.

## Impact
- Consistent capability naming across CLI specs.
- Easier discoverability for contributors mapping CLI subcommands to specs.
