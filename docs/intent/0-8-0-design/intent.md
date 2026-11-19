---
status: draft
class: higher-risk
---
# 0.8.0 design

## Intent

Design spec-dna 0.8.0, "trust in the gate". Today the gate proves links, not facts. A `covers:` tag is a comment, the author of the code closes the change, and nothing keeps true what is above the requirements. The design covers the 10 items of the owner brief:

- A test report as evidence, with 2 proof levels and a declared proof debt.
- An independent acceptance of each change, with a record that stays after `finish`.
- Approval through the code host.
- Owners by path, invariants with ids, and the write-back of deviations.
- A roll-up of summaries, with a stale check and an area budget.
- The measurement of the model team.
- The carry-over of 0.7.1.

The core stays deterministic: no model and no network in `check`. The work is design only: [the design page](../../design/0.8.0.md), ADRs 0024 to 0031 and the slice plan. No product code changes and the version does not change.

## Why now

The owner closed the scope on 2026-11-18. 0.7.1 is released, and its carry-over is known. The owner reviews the design before the build starts.

## Out of scope

- Product code, tests and the release of 0.8.0.
- The paid eval runs. The design gives their protocol and their caps.
- A change to the locked text of an accepted ADR. The new ADRs amend them.
