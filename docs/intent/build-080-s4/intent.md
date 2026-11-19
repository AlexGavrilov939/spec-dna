---
status: accepted
class: routine
---
# Build of 0.8.0, slice S4: the acceptance record

## Intent

Build slice S4 of the 0.8.0 design (decision D3, ADR 0025) with the owner answers of 2026-11-19: the record `acceptance.toml`, `spec-dna accept <slug>` with `--brief`, `finish` steps 5.2 and 5.5, the codes SD927 to SD931, SD933 and SD935, the skill `accept`, the agent `acceptor`, the `protect-approvals` rule and the `APPROVAL_RULE` sentence.

This slice dogfoods itself. Its pull request holds the acceptance record of the slice, made by the mechanism of the slice. The report of the part tells the cost in turns and time, and each friction point.

## Why now

S4 follows S2 and S3, and S5 needs it.

## Out of scope

- The invariants and the deviations: the change `build-080-s5`.
- The acknowledgement by a person and the review mode: 0.8.1.
- The version, the tag and the release.
