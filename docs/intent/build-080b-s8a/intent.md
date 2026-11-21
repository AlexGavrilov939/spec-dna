---
status: accepted
class: routine
---
# Build of 0.8.0 part B, slice S8a: the roll-up, type, pins and refresh

## Intent

Build the roll-up core ([design D50 to D53](../../design/0.8.0.md#d50-the-type-summary-and-the-layer-summaries)): the type `summary`, the layer `summaries`, the pins `rests-on`, SD641 to SD645 in the group `rollup`, `new summary` and `refresh` with its guard.

## Why now

The owner asked for the roll-up in part B. It uses the one function "is stale" of part A.

## Out of scope

- The other slices of part B, and the release of 0.8.0.
