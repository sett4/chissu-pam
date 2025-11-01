## Why
- Manual exposure/gain tuning is burdensome when hardware already offers auto controls; learners want a quick "just work" option for IR capture.
- Defaulting to unused manual parameters can produce unusable frames under varying lighting, conflicting with the constitution's emphasis on safe, observable CLI behavior.

## What Changes
- Add CLI flags to request automatic exposure and gain so the tool toggles corresponding V4L2 controls before capture.
- Detect and set the device's auto controls when present; fall back to manual parameters when auto is unavailable.
- Extend logging/JSON summary to record whether auto controls were requested/applied and integrate test coverage for the new logic.

## Impact
- Touches `src/cli.rs`, `src/capture.rs`, and related tests to support auto toggles and capability detection.
- Requires README/manual docs updates describing the new flags and expected behavior when hardware lacks auto controls.
- No breaking changes: existing manual flags continue to work; auto toggles are opt-in.
