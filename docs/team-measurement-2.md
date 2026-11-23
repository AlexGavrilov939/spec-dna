# Measurement of the model team: protocol 2, real slices from the history

This page gives the result of protocol 2 in [ADR 0033](adr/0033-protocol-2-of-the-model-team-measurement.md). It is for the owner, who decides the default harness level, the advisor rule of 0.8.1 and the advisor gate. The gate of the ADR stopped the work after the 3 probes, so the batch did not run. This page gives no verdict.

## What ran

- 5 tasks in `bench/team2/tasks/`. Each task is a slice of this repository whose first implementation had defects that a later review found. The agent gets a copy of the start commit with no history, and the approved plan of the slice as its brief.
- The oracle has 2 parts. Part 1 holds the tests that the slice merged. Part 2 holds one test for each later defect, and a run that changes nothing scores 0 in it. The oracle is hidden from the agent.
- The self-test `bench/team2/selftest.py` proved the 5 tasks on a stub agent, with no cost. The start tree fails part 1, the solution with the defects passes part 1 and fails part 2, and the reference passes both.
- Models: Claude Opus 5.5 as the main model and Claude Fable 5.2 as the advisor, Claude Code 2.1.330, spec-dna 0.8.0, effort `xhigh`.
- The order is fixed by the history. The commit of ADR 0033 (`d320f84`) has the time 14:53:59Z. The first probe started at 14:54:13Z. No rule of the ADR changed after that.

## The probes

1 paid run of each arm on `t2-05-approver-hook` (12 tests in part 1, 3 in part 2). Limits: 300 turns, 25 USD and 7,200 seconds for the agent. No run hit a limit. The records are in `bench/results/evals/20261123T145413Z-harness.jsonl`.

| Arm | Part 1 | Part 2 (main measure) | Cost (USD) | Minutes | Turns | Advisor calls |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | 5 of 12 | 1 of 3 | 5.90 | 27.9 | 93 | 0 |
| `lean-no-advisor` | 5 of 12 | 2 of 3 | 14.96 | 75.2 | 2 (see Limits) | 0 |
| `lean` | 9 of 12 | 2 of 3 | 19.85 | 65.0 | 98 | 6 (5 results) |

Each arm has 1 run, so these numbers show the size of a run. They do not show a difference between the arms. The task is not at the ceiling or at the floor: part 2 is neither 1 nor 0 in each arm.

## The gate

The work stops when one of the rules of the ADR is true. This one is true:

- The projected cost is `15 x (5.90 + 14.96 + 19.85) x 0.99`, or about 605 USD (the script `gate.py` uses the exact ratio of sizes and prints 604). The room is 380 USD, minus the spend of the probes (40.71 USD), minus 10 USD of reserve, or 329 USD. The batch needs about 605 USD plus the budget of one run, so it does not fit.

The other rules are false. The tasks are not at the ceiling or the floor on the probe task, no probe hit a limit, and the `models` of each probe match its arm (only `lean` lists the advisor model). The room pays for about 8 of the 15 task waves that the floor needs. A batch that the cap stops would end below the floor of ADR 0030, so it would give no verdict. No variant of the ADR fits the cap: without the arm `none`, the projection is still about 517 USD.

## Decision

No batch ran. There is no verdict for the 2 comparisons, so they are below the floor, and the tables of the ADR give:

| Comparison | Meaning |
| --- | --- |
| `lean` against `none` | No change to the default harness level |
| `lean` against `lean-no-advisor` | No change to the rule "advisor only on a change of class `higher-risk`" of 0.8.1, and no change to the advisor gate |

The owner decides each change of a default. A new protocol needs a new ADR, and a new cap or fewer tasks. The probes give the size: a full wave of 3 arms costs about 40 USD on this task, and the 5 tasks have a mean size of 99% of it.

## Limits

- The probes have 1 run for each arm on 1 task. They give no interval and no verdict.
- The runner gave `lean-no-advisor` 2 turns for a run of 75 minutes. The transcript of that run has 435 messages of the assistant, and the run used a background monitor, 6 messages to a subagent and 1 subagent. The runner reads `num_turns` of the last `result` event, so a result event after a background wake-up could give this count. The cause is not proved, because the runner does not keep the output of `claude`. The turn limit of the batch (200) comes from the 2 other probes, and it would rest on this count. The code was not changed.
- The count of leaks marks the 3 probes as leaked, as the ADR defines it: 1 tool use of `none`, 5 of `lean-no-advisor` and 1 of `lean` match the pattern of `transcript_counts.py`. The matches are of 3 kinds: the folder of task output that Claude Code makes for the copy, a `git remote -v` inside the copy, and test text with the string `gh pr`. No tool input names the oracle, a checkout of this repository, the remote host or the pull request refs. The pattern has false positives, and the ADR fixes it, so it was not changed. Only tool inputs were checked by kind, not tool results.
- One repository, one language, one model family, one version of Claude Code.
- The tasks are public and the fixed code is on the public repository. A model may know them.
- The cost comes from the runner, not from an invoice.

## Spend

The probes cost 40.71 USD of the 380 USD cap. The dry run and the self-test cost nothing, because they use the stub agent. The pilot of protocol 1 is on [its page](team-measurement.md).
