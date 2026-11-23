---
status: closed
class: routine
---
# Protocol 2 of the model-team measurement

## Intent

Measure the model team of `init` on 5 real slices from the history of this repository, in place of the synthetic tasks of protocol 1. Each slice had defects that a later review found. The measure is the share of the later-defect tests that an arm passes. The owner gets 2 verdicts (`lean` against `lean-no-advisor`, and `lean` against no harness), the cost of each arm, and the decision data for the default harness level, the advisor rule and an advisor gate.

## Why now

Protocol 1 ended with no result: the tasks were at the ceiling, and `lean` cost 4 to 6 times more with no gain. The owner decided on 2026-11-23 to run a new protocol on harder tasks, with a cap of 380 USD.

## Out of scope

- A change of the code of `src/`, a version change and a tag.
- A change of the default harness level or of the advisor. The result goes to the owner.
- The rule "the advisor only on higher-risk changes": it enters the note of 0.8.1 as a decision, with no code.
