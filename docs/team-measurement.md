# Measurement of the model team: pilot of 0.8.0

This page gives the result of the pilot of the measurement in [ADR 0029](adr/0029-protocol-of-the-model-team-measurement.md). It is for the owner, who decides the default harness level and the advisor gate of 0.8.1. The main batch did not run. This page gives no verdict.

## What ran

- 12 tasks in `bench/team/tasks/`. Each task has a hidden oracle that the agent never sees. The self-test `bench/team/selftest.py` proves each task: the reference passes, and each wrong solution fails.
- Models: Claude Opus 5.5 as the main model, Claude Fable 5.2 as the advisor, Claude Code 2.1.330, spec-dna 0.8.0.
- A run passes only when `claude` exits 0 and all 3 checks pass: tests unchanged, visible tests, hidden tests. A run that hits 40 turns or 4 USD counts as a fail.
- The runs used a clean environment, so the settings of the host session did not reach the agent.

## Step 2: the manipulation check

3 tasks (`team-01` to `team-03`), 1 run for each setup and task. The setups are `none` (no harness), `lean-no-advisor` and `lean`. The cost of this step was 5.26 USD.

| Setup | Passed | Cost of a run (USD) | Turns | Minutes | Advisor calls |
| --- | --- | --- | --- | --- | --- |
| `none` | 3 of 3 | 0.18 to 0.23 | 6 to 7 | 0.6 to 0.9 | 0 |
| `lean-no-advisor` | 3 of 3 | 0.24 to 0.35 | 7 to 8 | 1.0 to 1.6 | 0 |
| `lean` | 3 of 3 | 1.04 to 1.45 | 8 to 11 | 2.3 to 3.7 | 7 in 3 runs |

The transcripts prove the setups. The counts are:

- `lean-no-advisor`: 0 advisor calls in 27 runs.
- `none`: 0 advisor calls in 3 runs.
- `lean`: 7 advisor calls, with at least 1 call in each of 3 runs.

The records of this step are in `bench/results/evals/20261123T111239Z-harness.jsonl`.

## Step 3: the headroom

All 12 tasks, 2 runs each, in the setup `lean-no-advisor`. This step shows if a task is too easy. The cost was 15.00 USD.

| Result | Tasks |
| --- | --- |
| 2 of 2 runs passed | `team-01` to `team-04`, `team-06` to `team-12` |
| 1 of 2 runs passed | `team-05` |
| 0 of 2 runs passed | none |

23 of 24 runs passed, at about 0.6 USD for a run. The failed run of `team-05` missed the hidden test, and its second run passed, so the task is not broken. The runner writes a plain run in no record file, so this step has no record in `bench/results/evals/`.

## Decision

The owner's brief for this work says to start the batch only if the pilot shows that the tasks are not at the ceiling or at the floor in every arm. ADR 0029 has no such rule. The pilot found the tasks at the ceiling: 3 of 3 for each setup on `team-01` to `team-03`, and 23 of 24 for `lean-no-advisor` on all 12 tasks. So the agent that ran the pilot stopped before the batch, and it did not change the protocol.

The rules of the ADR alone would go on. No fixture is broken, so the batch keeps 10 tasks: `team-05`, then `team-01` to `team-04` and `team-06` to `team-10`. The projected cost is about 178 USD for 5 waves, and the budget `M` is about 370 USD. The owner can order that batch. The default harness level and the advisor gate of 0.8.1 stay as they are.

## Limits

- The pilot has no interval and no verdict. Step 2 has 1 run for each setup and task, and step 3 has 2 runs for each task.
- The setups `none` and `lean` ran only on `team-01` to `team-03`. The other tasks ran only in `lean-no-advisor`.
- An agent with an unlimited shell can search the disk for the hidden tests. The count of the tool inputs and results found 1 match in 33 runs. It is the word "oracle" in a comment of a script that the agent wrote (`team-04`). No run opened a hidden place.
- The tasks are public. A model may know them.
- A `team-03` check uses the file name `QUESTIONS.md`, and macOS does not tell the case of a name.
- The cost comes from the runner, not from an invoice.
