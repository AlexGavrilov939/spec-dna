---
status: accepted
---

# 0029. The protocol of the model-team measurement

## Context and Problem Statement

`init` writes a model team for Claude Code at the level `lean` ([ADR 0004](0004-harness.md)). The team has a main model, an advisor, a verifier, a search agent and 2 hooks. No measurement shows that the team or the advisor gives better work for its cost. The owner gives 400 USD to compare no harness, `lean` without the advisor, and `lean`, on about 10 tasks with 5 runs for each arm. The result decides whether 0.8.1 gets an advisor gate. This ADR is the pre-registration: it fixes the arms, the tasks, the oracle, the verdict and the use of the result before any paid run.

Evidence:

- The committed records of 0.7.0 and 0.7.1 list the advisor model in 755 of 770 runs. The host team of those runs is not recorded. Thus no past result can separate the advisor from the rest.
- The mean cost of a past run, with the advisor, was 0.7 to 1.9 USD.
- A lab simulation of the bootstrap of [ADR 0016](0016-skill-evals.md) with 10 tasks and 5 runs gave the verdict of a gain in about 48% of trials for a true gain of 0.20. For a true gain of 0.30, the share was 85%. With no gain, a false gain occurred in 3% of trials or less.
- A check that greps the prose of the agent is weak: the evals `review-N` show it.

## Considered Options

1. No measurement: keep the team as a default with no evidence.
2. Read the past skill runs. All of them had the advisor, so they cannot answer.
3. Run the arms with the skill runner and the team of the host project.
4. 3 harness arms that the runner writes into each copy, on synthetic tasks with hidden checks, and a verdict rule fixed before the runs.
5. A model grades the output of each arm.

## Decision Outcome

Option 4.

### Arms

- A: `none`. B: `lean-no-advisor`, which is `lean` with the advisor role set to `none`: no `advisorModel`, no `fallback-advisor.md`, no advisor sentence in the line of `CLAUDE.md`. C: `lean`, the default of `init`.
- Each arm has no skill, no `REVIEW.md` and no hook `protect-tests`. Each arm has the same adapter line. The runner passes the main model and the effort of `lean` to each arm, so arm A has the same main model.
- Two comparisons get a verdict by one rule. C against B tests the advisor and decides the gate. C against A tests `lean` as `init` writes it, against no harness. B against A only splits the effect between the team and the advisor: a description, with no verdict.

### Tasks

12 candidates, 10 in the batch. Each task is a small synthetic project in Python with `unittest`, with no code of a consumer project. Each has a task text, visible tests, a hidden oracle, a reference solution and a wrong solution. The kinds are: a rule in the docs, a wrong first idea (the trace points at the wrong module), a spec conflict (the rule of the project wins over the request), a trap at a boundary, and a regression of a refactor. Rules of the selection:

- The wrong solution passes the visible tests and fails the oracle.
- The task text does not state the trap.
- A second opinion can change the result.
- At least 3 kinds in the batch.

The files are in `bench/team/` of this repository.

### Oracle

- Only deterministic checks: the visible tests stay unchanged, the visible tests pass, and the hidden tests pass. No check reads the prose of the agent.
- The hidden tests are in `<evals dir>/oracle/<id>/` of the project that runs the batch, outside the fixture. The runner never copies them into the copy. After `claude` exits, a function of `check.sh` copies them in and runs them.
- The agent gets no path of the project. The batch runs a binary that is outside the project folder.
- Self-test with no model. The fixture fails the oracle. The reference solution passes each check. The wrong solution passes the visible tests and fails the oracle.

### Metrics

For each arm: the pass rate with its 90% interval, the cost, the cost per correct outcome, the tokens, the turns, the median seconds, and the share of runs whose models list the advisor model.

### Pilot

The pilot runs do not count in the verdict.

1. The self-test of the oracle and the runner tests with a stub `claude`. No cost.
2. The manipulation check: 3 tasks, 1 run, arms A, B and C, about 18 USD. The step passes when each run lists the main model of `lean`, no run of A or B lists the advisor model, and at least 2 of 3 C runs list that model. If the step fails, the work stops until a person fixes the arms. Then this step runs again. Claude Code 2.1.328 lists `--model` and `--effort` in its help.
3. The headroom: 12 candidates, 2 runs in arm B, about 45 USD. The batch keeps 10 tasks in this order: the tasks with 1 of 2 passes, then those with 0 of 2, then those with 2 of 2. Each group keeps the order of the ids. A task with an error of `claude` in both runs is a broken fixture, and the batch drops it. If fewer than 10 candidates remain, the work stops before the main batch. The owner then decides on a fix of the broken fixtures, or on fewer tasks under a floor that the owner states. No task drops in silence below the floor.

The fallback uses the projected cost of the 3 arms. `M` is 400 USD minus the spend of the pilot minus 10 USD. One wave costs `W` = 10 x (`a` + `b` + `c`). `b` is the mean cost of a B run of step 3. `a` and `c` are `b` times the ratios of A and C to B in step 2.

With 5 x `W` <= `M`, the batch has 5 runs. If not, the work stops, and the owner decides: 4 runs (the floor needs 3), or more money. The pilot costs about 63 USD (18 and 45), with a cap of 65 USD, so `M` is 327 USD at the estimate.

### Main batch

- 10 tasks, 5 runs, 3 arms: 150 runs. The runner runs in waves. Wave `r` runs each task once in each arm. The order of the arms rotates with the task and the wave.
- `max-budget-usd` is 4.0 for each run. `--max-total-usd` is 400 USD minus the spend of the pilot, minus 10 USD of reserve.
- A refused run follows the back-off of the runner and does not count.
- The floor: 10 tasks and 3 complete waves. A partial wave does not count. Below the floor, there is no verdict.

### Verdict

Each comparison of a treatment X with a control Y uses the bootstrap of ADR 0016 by task (seed 24, 2,000 resamples, 90% interval). It uses complete waves only:

- `better`: rule 1 or rule 2 of ADR 0016.
- `worse`: the high end of the interval of the difference is below 0.
- `no-gain-at-cost`: not `better` and not `worse`. The high end of the difference is below 0.20. The low end of the ratio of the costs per correct outcome is above 1.00.
- `inconclusive`: each other result. An undefined ratio of the costs gives `inconclusive` in place of `no-gain-at-cost`.

`eval verdict --compare` prints the verdict of ADR 0016 and both intervals. A person reads the result of this protocol from these numbers. The binary holds no rule of this protocol.

### Use of the result

| C against B | 0.8.1 |
| --- | --- |
| `better` | Gate in: 0.8.1 designs an advisor gate |
| `no-gain-at-cost` | Gate out: no gate. The owner decides if `lean` keeps the advisor |
| `worse` | Gate out. The owner decides if `lean` drops the advisor |
| `inconclusive` | Stay: no gate, and the advisor stays a default role |
| Below the floor | No result: stay. The owner decides if more money goes to the waves that did not run |

- C against A: `better` goes into `docs/evidence.md` as evidence for the default `lean`. `worse` or `no-gain-at-cost` goes to the owner. `inconclusive` changes nothing.
- The main batch decides alone. No wave is added after a look at the result, because more runs after a look make a false `better` more likely. A later measurement is a new batch with its own protocol.
- The records of the batch go to `bench/results/evals/`. They do not enter the table of the release of the skills.

### Rejected

- Option 1: a default with no evidence has a cost in each session that uses the advisor.
- Option 2: the past runs had the advisor in 98% of the runs and no record of the team.
- Option 3: the host files make the arms depend on the machine that runs them.
- Option 5: the grade changes from run to run, and the grader shares the family of the agent ([ADR 0008](0008-harness-and-evals.md)).
- A top-up of runs after the result.
- Keyword checks of the report of the agent.

### Consequences

- Good: the rules and the use of the result are fixed before the money is spent.
- Good: the runner work also makes each later batch record its team.
- Bad: 150 runs show only a large effect. The most likely result for a small effect is `inconclusive`, and the table then says "stay".
- Bad: the tasks are public. A later model can know them.
- Neutral: the pilot costs about 63 USD that the verdict does not use.

### Limits

- One model family, one Claude Code version, and sessions of at most 40 turns. The value of the advisor in a long session is not measured.
- 10 small synthetic tasks in one language. No result is a proof for another project.
- The share of runs with the advisor model shows a consult, not its use.

## Amendments

### 2026-11-23: the pilot ends this protocol with no result

The pilot found the tasks at the ceiling. In the 3 setups, 3 of 3 runs passed on `team-01` to `team-03`, and 23 of 24 runs passed in `lean-no-advisor` on the 12 tasks. A `lean` run cost 4 to 6 times more than a run of the other setups, with no gain. The main batch did not run. No verdict exists.

The owner decided on 2026-11-23 that this protocol ends with the result "no result". The row "Below the floor" of the table "Use of the result" applies: the advisor stays a default role, `init` keeps the level `lean`, and 0.8.1 gets no gate from this protocol. The records and the page of the pilot stay as they are: `bench/results/evals/20261123T111239Z-harness.jsonl` and `docs/team-measurement.md`. A new protocol on tasks from the history of this repository replaces it.
