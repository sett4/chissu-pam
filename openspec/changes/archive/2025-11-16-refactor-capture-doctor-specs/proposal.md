# Proposal: Refactor capture/doctor specs

## Why
- Current `capture-cli` spec mixes capture behaviors with the `doctor` diagnostics flow, making ownership unclear.
- Capability naming conflicts with the CLI binary (`chissu-cli`) and other mode-specific specs.

## What Changes
- Rename the shared capture capability to `chissu-cli-capture` and scope it to capture behaviors only.
- Move `doctor` command requirements into a dedicated `chissu-cli-doctor` capability.
- Update dependent specs to reference the new capture capability name.

## Impact
- Clearer spec boundaries between capture flows and diagnostics.
- Future CLI subcommands can reference shared capture behavior without inheriting doctor semantics.
- Documentation and tooling point to capabilities that match CLI naming.
