## Why
- Contributors want to extract facial feature encodings from existing PNG captures to support downstream face recognition.
- The current CLI only captures infrared frames and does not process existing images.

## What Changes
- Add a new CLI subcommand that loads a PNG file, detects faces, and exports descriptor vectors using a dlib-based pipeline.
- Provide both human-readable and JSON outputs that include per-face metadata and the persisted feature file path.
- Persist extracted feature vectors to disk so they can be reused for enrollment or matching workflows.

## Impact
- Introduces a dependency on `dlib-rs` (and system-level dlib) for feature extraction.
- Requires additional configuration surface for selecting models and output locations.
- Extends testing with fixture-based runs that do not require a live camera.
