## ADDED Requirements
### Requirement: Shared Capture CLI Behavior
Every capture-oriented subcommand SHALL inherit a single set of CLI behaviors that live in the `capture-cli` capability: built-in defaults (device `/dev/video0`, pixel format `Y16`, four warm-up frames), config-file overrides, warm-up frame discarding, and dual output modes (`--json` vs human-readable).

#### Scenario: Any capture mode honors shared defaults
- **GIVEN** `chissu-cli capture --json` is invoked without explicit `--device`, `--pixel-format`, or `--warmup-frames`
- **WHEN** the capability referenced by the command needs those values
- **THEN** the CLI resolves them using the shared default/config logic defined in `capture-cli`
- **AND** any capability-specific spec (e.g., `infrared-capture`) may only override values it explicitly documents.

### Requirement: Capture CLI Capability Scope Declaration
The `capture-cli` spec MUST describe itself as the home for cross-cutting capture behaviors (controls, diagnostics, binary naming) so other capability specs can reference it instead of re-stating shared rules.

#### Scenario: Linked capability identifies shared owner
- **GIVEN** another capability (e.g., `infrared-capture`) needs the CLI logging, auto control toggles, or diagnostic subcommands
- **WHEN** contributors look up where to document or modify those behaviors
- **THEN** the `capture-cli` spec explicitly states it owns them and points to the relevant requirements (auto controls, config defaults, keyring diagnostics, binary naming).

## MODIFIED Requirements
### Requirement: Config File Capture Defaults
The shared configuration and built-in defaults MUST remain canonical inside `capture-cli` so that any capture capability referencing this requirement automatically inherits the same resolution order.

#### Scenario: Infrared capture references shared defaults
- **GIVEN** the `infrared-capture` capability defers to `capture-cli` for resolving devices and warm-up frames
- **WHEN** the operator runs `chissu-cli capture` without overriding these flags
- **THEN** the command applies `/dev/video0`, `Y16`, and four warm-up frames based on the shared behavior requirement and reports the resolved values in both human-readable and JSON outputs.

### Requirement: CLI Binary Naming
The workspace MUST continue to emit a `chissu-cli` binary name for all build profiles so capability-focused specs remain accurate regardless of how many capture modes exist.

#### Scenario: Future capture modes reuse binary naming
- **GIVEN** maintainers add a new capture capability (e.g., RGB capture)
- **WHEN** they build the workspace in debug or release mode
- **THEN** the resulting binary remains `chissu-cli`, ensuring documentation in sibling specs stays correct without additional edits.

### Requirement: Secret Service Diagnostics Command
The Secret Service diagnostic subcommand MUST remain defined in this capability even when other capture modes are introduced, and sibling specs SHALL reference it instead of redefining command semantics.

#### Scenario: Infrared spec links to diagnostics
- **GIVEN** operators follow the `infrared-capture` documentation to verify their environment
- **WHEN** they run `chissu-cli keyring check`
- **THEN** the diagnostic behavior is defined only once in `capture-cli`, and the infrared spec simply references it rather than redefining command semantics.
