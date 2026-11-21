---
status: draft
class: routine
---
# Build of 0.8.0 part B, slice S4c: one pass for the agent

## Intent

Merge the skill `accept` into `review`, give `next` one item `review` with the prompt line and the item of a person after 2 rejected records, close an open acceptance task with `finish`, add rule D of the hook `protect-approvals`, write the agent `acceptor` with `review`, and keep the skill texts of `lite` as in 0.7.1 with a golden test ([design D71, D75, D76](../../design/0.8.0.md#d70-to-d79-the-acceptance-in-one-pass)).

## Why now

Slice S4b gives the core. The agent still needs one step to learn, and `lite` must not change.

## Out of scope

- The other slices of part B, and the release of 0.8.0.
