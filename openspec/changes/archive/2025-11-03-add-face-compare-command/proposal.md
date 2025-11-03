## Why
Operators need to score facial feature files exported via `faces extract` against multiple candidates without re-running extraction.

## What Changes
- Add a `faces compare` CLI subcommand that reads descriptor JSON files and reports cosine similarity scores.
- Extend face-features spec with requirements for comparison inputs, outputs, and error handling.

## Impact
- Introduces read-only processing of existing feature files; no new models required.
- Requires documentation and tests covering similarity scoring and JSON output.
