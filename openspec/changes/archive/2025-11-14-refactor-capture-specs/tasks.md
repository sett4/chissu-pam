## 1. Capture CLI spec cleanup
- [x] 1.1 Write the new "Shared Capture CLI Behavior" requirement describing defaults, warm-up handling, and dual output expectations for every capture subcommand.
- [x] 1.2 Add a "Capability Scope Declaration" requirement (and update the Purpose section text) so the `capture-cli` spec explains it owns shared controls, diagnostics, and binary naming.

## 2. Infrared capture spec alignment
- [x] 2.1 Update existing infrared requirements (command, configurable parameters, dual outputs) to reference the new shared behavior requirement where applicable.
- [x] 2.2 Add an "Infrared Mode Boundaries" requirement plus a real Purpose statement clarifying that this capability extends the shared CLI behavior with IR-only checks and tests.

## 3. Validation
- [x] 3.1 Run `openspec validate refactor-capture-specs --strict` and fix any issues.
