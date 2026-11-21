---
status: accepted
class: routine
---
# Build of 0.8.0 part B, slice S5n: the open notes of S5

## Intent

Fix the 5 open notes of slice S5 ([design D78](../../design/0.8.0.md#d70-to-d79-the-acceptance-in-one-pass)): unique ids of the deviations and the invariants of a record, part 5.3 of `finish` that prints the deviations of the record, code spans in `Delta:`, `Follow-up:` and `Invariants`, a paragraph in `Invariants`, and the comment of `skip_early_sections`.

## Why now

The owner asked for the fix on 2026-11-21. The later slices of part B build on the record code that these notes touch.

## Out of scope

- The format 2 of the record and the other decisions of D70 to D77. Slice S4b builds them.
