---
status: draft
class: routine
---
# 0.8.0 part B design: acceptance in one pass

## Intent

Change the design of the acceptance record of 0.8.0 after the use of the record in part A. One acceptance run took about 25 minutes and 200 to 250 thousand tokens. Slice S4 needed 8 runs, and all were `rejected`. The new design needs a record only for a `higher-risk` change in `standard`, and it uses one pass of one agent that reviews and writes the record. It adds the findings `blocker` and `note`, a short follow-up for a note fix, and a stop after 2 `rejected` records. It also gives the roles of `[owners]` and the fixes of 5 open notes of S5. [The design page](../../design/0.8.0.md#15-acceptance-in-one-pass) holds the decisions D70 to D79. [ADR 0032](../../adr/0032-acceptance-in-one-pass.md) records them.

## Why now

The owner decided the change on 2026-11-21, after the report of part A. The build of part B waits for the design.

## Out of scope

- Product code and tests. The slices of part B build them.
- The paid eval runs.
- A change to the locked text of an accepted ADR. ADR 0032 amends them.
