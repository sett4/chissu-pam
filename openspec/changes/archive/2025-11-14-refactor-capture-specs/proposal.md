# Proposal: Refactor Capture-CLI Capability Specs

## Problem
- `capture-cli` and `infrared-capture` currently overlap on the same `chissu-cli capture` command but do not describe where shared behavior (defaults, `--json` output, warm-up behavior) lives, so contributors duplicate language or edit the wrong spec.
- Both specs still have placeholder `Purpose` text from their originating changes, so reviewers cannot quickly tell which capability should host new requirements.
- The CLI requirements include tooling-only behavior (keyring diagnostics, binary naming) mixed with infrared-only capture logic, making rename/refactor discussions hard to ground in the specs.

## Goals
1. Document a shared "Capture CLI behavior" requirement inside `capture-cli` so other capture modes can reference it instead of repeating flag/output semantics.
2. Clarify capability boundaries by explicitly stating that `infrared-capture` extends (not replaces) the shared CLI behavior and only adds IR-specific constraints; introduce purpose statements that spell this out.
3. Evaluate whether the `capture-cli` capability name still matches its contents by creating a requirement that records the scope (controls + diagnostics) and referencing it from other specs.

## Non-Goals
- Implementing any CLI code changes; this proposal only restructures specs.
- Deciding final terminology for future capture modes (e.g., RGB capture) beyond establishing where those rules would live.

## Approach
1. **Capture shared behavior requirement**: Add a new `Shared Capture CLI Behavior` requirement under `capture-cli` covering default resolution, warm-up frames, and dual output expectations so other capture specs can reference a single source.
2. **Purpose and scope clarification**: Replace TBD Purpose text in both specs with summaries that highlight their roles (shared CLI surface vs. IR mode). Add a `Capability Scope Declaration` requirement in `capture-cli` that records the expectation to host diagnostics and reusable controls, and a complementary `Infrared Mode Boundaries` requirement in `infrared-capture`.
3. **Cross-spec references**: Update `infrared-capture` requirements (dual output, configurable params) to explicitly reference the shared behavior requirement, ensuring future refactors or renames remain coherent.

## Risks & Mitigations
- *Risk*: Future capture modes might still fork behavior. *Mitigation*: codify references so new specs must deliberately override shared requirements.
- *Risk*: Contributors may ignore the new requirement. *Mitigation*: tasks include surfacing the reference in README/docs when implementing the change.

## Timeline
This is a documentation-only spec refactor and can be completed within the current planning cycle.
