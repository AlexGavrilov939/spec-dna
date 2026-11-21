---
status: accepted
class: routine
---
# Build of 0.8.0 part B, slice S11: the release candidate of 0.8.0

## Intent

Prepare the release candidate: version 0.8.0, the pin, the CHANGELOG with the upgrade notes from 0.7.1, the proof of `upgrade` on a 0.7.1 fixture and on this repository, the paired benchmark against v0.7.1, the final `checker` review since v0.7.1, and `scripts/verify-release.sh`. Create no tag.

## Why now

The owner checks the acceptance gate on the release candidate before the tag.

## Out of scope

- The other slices of part B, and the release of 0.8.0.
