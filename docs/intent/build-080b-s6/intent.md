---
status: draft
class: routine
---
# Build of 0.8.0 part B, slice S6: the agent roles of [owners]

## Intent

Build the agent roles of `[owners]`: one record entry answers each matched role, a matched role needs a record, a person in `[owners]` gives a warning and not exit code 3, and the acknowledgement of a deviation with `branch` (`approve <slug> acceptance`, SD932, SD934) ([design D3 item 10, D5, D77](../../design/0.8.0.md#d70-to-d79-the-acceptance-in-one-pass)).

## Why now

Slice S4b gives the record of format 2 with `roles`. The roles need the match of paths and the check.

## Out of scope

- The other slices of part B, and the release of 0.8.0.
