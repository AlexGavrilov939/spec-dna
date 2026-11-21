---
status: draft
class: routine
---
# Build of 0.8.0 part B, slice S8b: the roll-up with finish, next, context and the area budget

## Intent

Build the rest of the roll-up ([design D54 to D56](../../design/0.8.0.md#d54-finish-next-and-mv-with-summaries)): the part of `finish` and `finish --holds`, the note of `next`, SD913 for summaries, `mv`, the tier `summary` of `context`, and the area budget SD705 to SD707.

## Why now

Slice S8a gives the pins and the codes. The flow of one change needs `finish` and `next`.

## Out of scope

- The other slices of part B, and the release of 0.8.0.
