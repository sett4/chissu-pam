# Proposal: Merge infrared capture spec into chissu-cli-capture

## Why
- Reduce duplication by housing both shared and infrared-specific capture requirements in a single capability.
- Simplify contributor discovery by aligning all capture behavior under `chissu-cli-capture`.

## What Changes
- Move the requirements currently in `infrared-capture` into `chissu-cli-capture`.
- Leave `infrared-capture` as a stub that points to the merged capability.

## Impact
- One canonical place for capture requirements, including IR specifics.
- Future changes to capture flows need only touch `chissu-cli-capture`.
