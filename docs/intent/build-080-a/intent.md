---
status: accepted
class: routine
---
# Build of 0.8.0, part A: the carry-over, the runner, the test report and the proof levels

## Intent

Build slices S0 to S3 of the 0.8.0 design ([design page](../../design/0.8.0.md), section 12) with the owner answers of 2026-11-19. S0 is the carry-over of 0.7.1: the release check (D63), the label `By: person` (D64) and the base of the `upgrade` check (D65). S1 is the change of the eval runner (D57 to D60). S2 is the test report as evidence (D1). S3 is the proof levels and the proof debt (D2).

Each slice is one pull request. The design page and ADRs 0024 to 0031 are the spec. A slice that departs from the design says so in its pull request and in the report of the part.

Profile defaults (answer 4): in `lite` each new mechanism is off. In `standard` only the acceptance and the roll-up are on, and the other mechanisms need their key. A project of 0.7.1 that runs `upgrade` alone gets no new error.

## Why now

The owner decided the design on 2026-11-19. Part A builds the half of 0.8.0 that does not depend on the acceptance record, and it stops before the tag.

## Out of scope

- The acceptance record, the invariants and the deviations: the changes `build-080-s4` and `build-080-s5`.
- The roll-up, the owners, the reruns and the release: part B.
- The version, the tag, the release and any paid eval run.
