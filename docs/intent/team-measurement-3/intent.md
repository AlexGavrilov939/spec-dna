---
status: accepted
class: routine
---
# The batch of protocol 2 of the model-team measurement

## Intent

Run the batch of protocol 2 (ADR 0033) with the cap that the owner set on 2026-11-23, and report 2 verdicts: `lean` against `lean-no-advisor`, and `lean` against no harness. Before the batch, fix the turn count of the probes, add a stricter leak column, and set the batch tools (the merge of the job results and the job scheduler).

## Why now

The gate stopped the batch after the 3 probes, because the projected cost was above the room. The owner raised the cap to 700 USD in total.

## Out of scope

- A change of the code of `src/`, a version change and a tag. The defect of the turn count in the runner stays open for 0.8.1.
- A change of the default harness level or of the advisor. The result goes to the owner.
- A change of a rule of ADR 0033 other than the cap.
