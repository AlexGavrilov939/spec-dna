---
status: accepted
---

# 0033. Protocol 2 of the model-team measurement: slices from the history

## Context and Problem Statement

Protocol 1 ([ADR 0029](0029-protocol-of-the-model-team-measurement.md)) ended with no result. Its synthetic tasks were at the ceiling (3 of 3 in each arm). A `lean` run cost 4 to 6 times more than a run of the other arms, with no gain. On 2026-11-23 the owner ordered a new protocol on hard tasks from the real history of this repository, with a cap of 380 USD (the rest of 400 USD). The result informs the default harness level, the advisor rule and an advisor gate. The rule "the advisor only on higher-risk changes" enters 0.8.1 now, with no wait for this result ([the note of 0.8.1](../design/0.8.1-note.md)).

This ADR is the pre-registration. It fixes the tasks, the oracle, the limits, the gate, the verdict and the use of the result before any paid run. No rule changes after the first paid run.

## Considered Options

1. More synthetic tasks, harder than those of protocol 1.
2. Slices of this repository whose first implementation had defects that a later review found. The brief is the approved plan of the slice, and the oracle is the merged tests and a test for each later defect.
3. A model grades the work of each arm.

## Decision Outcome

Option 2.

### Arms

The arms of [ADR 0030](0030-harness-arms-time-record-and-verdict-floor.md): `none`, `lean-no-advisor` and `lean`. The runner writes the team into each copy and gives each arm the same `--model` and `--effort`. The files of the team of this repository (`.claude/`, `CLAUDE.md`, `AGENTS.md`, `REVIEW.md`, `.spec-dna/`, `.github/`) are not in a fixture, so no arm has a team that the runner did not write.

### Tasks

Five tasks. Each is a pull request of this repository, or a part of one. The start commit is the parent of the slice on `master`, so it holds the approved plan of the slice. A fix commit comes after the first implementation. It is a commit of the branch of the pull request, or, for `t2-01`, a commit of pull request 60. Each defect in the table has a test that failed before its fix and fails on the start tree. Other fixes are not in part 2: 2 commits have no test (`d0313ed` of `t2-02`, `fdf3d7a` of `t2-05`), and 3 tests pass on the start tree (a symlinked manifest, a plan that the branch deletes, a glob id of an `ADD` item). The reference holds them all.

| Task | Slice | Tests of part 1 and part 2 | Later defects (the tests of part 2) |
| --- | --- | --- | --- |
| `t2-01-tree-binary` | The tree binary of SD503 (0.7.0, slice 1), tasks t1 and t2 | 3 and 1 | A pin change with CRLF line ends. The review of part A found it (pull request 60) |
| `t2-02-change-feedback` | The feedback items of 0.7.0 (slice 6), tasks t1 to t13 | 4 and 1 | A `[budgets.plan]` table with one key |
| `t2-03-task-checks` | The task checks SD923 to SD925 (0.7.0, slice 3a), tasks t1 to t8 | 29 and 4 | `init --reconfigure` drops `max-task-files`, and the `ADD` item of `Interfaces` with a bad id, no type, or a document of another type |
| `t2-04-locked-links` | The locked links and SD612 (0.7.0, slice 3b), tasks t2, t3, t6, t7 and t8 | 8 and 2 | SD612 pairs with the line that SD611 reports, and `check` reads only the changes that a dead locked link names |
| `t2-05-approver-hook` | The hook `protect-approvals` (0.7.0, slice 5), tasks t4, t9 and t10 | 12 and 3 | A folder that the hook cannot resolve, a line continuation between `git` and `commit`, and the read of the repository for the subcommand `commit` only |

The files are in `bench/team2/`. `prepare.py` builds the patches and the oracle from the history, and `recipes.json` names each commit.

### Fixture and brief

- The fixture is `git archive` of the start commit, with the files of the team taken out. It has no history. The runner makes one base commit.
- The brief is the intent, the spec and the plan of the slice, which are in the fixture, and one prompt (the function `prompt_of` of `make_project.py`). The prompt names the change and the tasks of the plan to do. It says that the docs, the changelog and the CI files are out of scope, and that the agent makes no commit.
- The agent never gets the oracle, the later history, or a path of the batch project. The batch runs a binary that is outside the project folder.
- The runner makes each copy in the temporary folder of the system. The batch project and the tools folder are not in it. Before the probes, the person deletes `team2-*` and `spec-dna-eval-*` there: the self-test builds hold the reference binary and the oracle.

### Oracle

The oracle is hidden and has 2 parts. Each part is a list of integration tests of `tests/`. A test of `src/` is not in the oracle.

- Part 1, the merged tests: a test that the first implementation added or changed, that fails on the start tree, and that passes on the solution with the defects and on the reference.
- Part 2, the later-defect tests: a test that a fix commit added or changed, that fails on the start tree and on the solution with the defects, and that passes on the reference. A run that changes nothing thus scores 0. Part 2 is the main measure: does a team catch what the first author missed.
- A test that fails on the reference is dropped. A guard that passes on the start tree is dropped from part 1. `selftest.py --write` makes the lists and the list of the dropped tests with the reason, in `tasks/<id>/parts.json`. The lists are fixed before the first paid run.
- After the agent stops, a check copies the oracle (`tasks/<id>/oracle/`) over the copy. Each test is one check. It runs by its exact name, and the output must show `1 passed`. A file of `tests/` that does not compile gives a fail for each of its tests.
- Proof with no model: the start tree fails part 1, the solution with the defects passes part 1 and fails part 2, and the reference passes both parts, for each task. `selftest.py` runs the real runner with a stub `claude`.

### Scores and measures

- A run has 2 scores: the share of part 1 that passes, and the share of part 2 that passes. A run whose `subtype` is not `success` (a limit of turns, budget or time, or an error) scores 0 in both.
- Each run has its cost, its minutes and its turns (from the result file) and its advisor calls (a count from the transcript, with no text). A run with no cost counts its `max-budget-usd`, as the spend of the runner does.
- The main measure is the score of part 2.

### Runs

3 runs for each task and arm: 45 runs, in 3 waves of the runner (ADR 0030). The order of the arms rotates. One batch runs at a time. Before each start, the person checks that no other `spec-dna eval run` or `claude -p` of this repository runs.

- A refused attempt (an account limit, 401, 403, 429 or 5xx) follows the back-off of ADR 0016 and does not count. Its cost counts in the spend.
- After a stop by `api-error` or an interrupt, the person goes on with `--resume` and the same `--max-total-usd`. An interrupted attempt does not count.
- The probes and the batch use one version of Claude Code, with the automatic update off. Before each start, the person checks `claude --version`.

### Dry run, probes and limits

1. The dry run is free. The self-test proves each task on the stub agent, and `eval run --arms` with the stub shows the copy of each arm.
2. The probes are 1 paid run of each arm on one task, `t2-02-change-feedback`. Their limits are 300 turns and 25 USD (the option `--limits` of `make_project.py`), 3,600 seconds for the agent (`SPEC_DNA_EVAL_TIMEOUT`) and 1,800 seconds for a check (`SPEC_DNA_EVAL_CHECK_TIMEOUT`). A probe that hits a limit stops the work. A probe with no counted run (a refusal, an interrupt or an error of the runner) runs again after the person removes the cause. Its cost counts in the spend of the probes.
3. The limits of the batch come from the probes by this formula: `max-turns` is 2 times the largest number of turns of a probe, rounded up to a multiple of 10, and at least 80 and at most 300. `max-budget-usd` is 2 times the largest cost of a probe, rounded up to a whole number, and at least 10 and at most 25. The time limits stay as in the probes. `limits.json` holds the result, and `make_project.py` makes a new project for the batch.
4. The probes do not count in the verdict.

### The gate before the batch

The work stops after the probes, with a report and no batch, when one of these is true:

- On the probe task, the main measure is 1 in each arm (the ceiling), or 0 in each arm (the floor), or the score of part 1 is 0 in each arm.
- A probe hit a limit.
- A probe of `none` or `lean-no-advisor` lists the advisor model of `lean` in its `models`, or a probe does not list the main model of `lean`.
- The projected cost plus `max-budget-usd` is above the room (the runner starts no run that could pass the cap). The projection is `15 x (a + b + c) x s`. `a`, `b` and `c` are the costs of the probes of the 3 arms. `s` is 1.32: the mean of the changed lines of the 5 files `reference.patch` (499, 921, 2,097, 1,349, 1,233) over that of the probe task. The room is 380 USD, minus the spend of the probes, minus 10 USD of reserve.

### Batch

`spec-dna eval run --arms none,lean-no-advisor,lean --runs 3` with `--max-total-usd` equal to the room. A stop by the cap keeps the complete waves. The floor of the verdict is 5 tasks and 3 complete waves, as in ADR 0030. Below it, there is no verdict. No wave is added after a look at the result.

### Verdict

`bench/team2/analyze.py` computes the verdict. The binary does not, because its `--compare` reads pass or fail and this protocol reads shares.

- A task has the mean score of its 3 runs in each arm. The difference of X and Y is the mean over the tasks of the differences of those means.
- The interval is the bootstrap by task (seed 24, 2,000 resamples, 90% interval). It resamples tasks, not runs. The cost ratio is the cost of X per run divided by the cost of Y per run, with the same bootstrap.
- `better`: the low end of the interval of the difference is above 0.
- `worse`: the high end of the interval is below 0.
- `no-gain-at-cost`: not `better`, not `worse`, the high end of the difference is below 0.20, and the low end of the cost ratio is above 1.00.
- `inconclusive`: each other result, and each result below the floor.

Two comparisons have a verdict: `lean` against `lean-no-advisor` (the advisor), and `lean` against `none` (the default level). `lean-no-advisor` against `none` is a description with no verdict. The same table gives the score of part 1, the cost, the minutes and the advisor calls of each arm.

### Use of the result

| `lean` against `lean-no-advisor` | Advisor rule and advisor gate |
| --- | --- |
| `better` | The advisor has value for these changes. The owner decides if the rule "only on higher-risk changes" of 0.8.1 is too narrow |
| `no-gain-at-cost` or `worse` | The rule of 0.8.1 stays. The owner decides if `lean` keeps the advisor as a default role |
| `inconclusive` or below the floor | No change |

| `lean` against `none` | Default harness level |
| --- | --- |
| `better` | Evidence for the default `lean`, in `docs/evidence.md` |
| `no-gain-at-cost` or `worse` | The owner decides on the default level |
| `inconclusive` or below the floor | No change |

The result decides nothing by itself. The owner decides each change of a default. The work changes no code of `src/` and no version, and it makes no tag. The records go to `bench/results/evals/`, and they do not enter the table of the release of the skills.

### Leaks

The agent can read the disk, and the public repository holds the fixed code. A run is leaked when a tool call of its transcript matches the pattern of `transcript_counts.py`: the oracle, this repository and its checkouts, `github.com`, a `gh` or `git` read of the remote, or the tools of the batch. The report gives the count of leaked runs for each arm, and 2 verdicts: with all runs, and without the leaked runs.

- The verdict without the leaked runs takes the floor from all runs. A task with no run left in an arm leaves that comparison.
- The tap log holds the batch only (a new tools folder after the probes). `analyze.py` matches its lines to the runs in order and skips the excluded attempts. If the counts differ for a task and arm, a leaked line marks each of its runs.
- If the 2 verdicts have different classes, the report says so, and the owner reads the result as `inconclusive`.

### Rejected

- Option 1: protocol 1 had the ceiling. A harder synthetic task has no proof that the defect is a real one.
- Option 3: the grade changes from run to run, and the grader shares the family of the agent.
- A top-up of runs after a look at the result.
- A unit test of `src/` in the oracle: it needs the internal names of the first author.
- A text of the agent as a check.

### Consequences

- Good: the defects are real, and each defect of part 2 has a test that failed before its fix.
- Good: the cost, the limits and the stop rules are fixed before the money is spent.
- Bad: part 2 of a task has 1 to 4 tests, so a score has a large step, and the intervals are wide. The likely result for a small effect is `inconclusive`.
- Bad: the tasks are public, and the fixed code is in this repository.
- Neutral: the probes cost money and do not count in the verdict.

### Limits

- One repository, one language, one model family and one version of Claude Code.
- The tests of a part check the behavior of the real binary. Some of them read the text of a message of the first implementation. An agent that wrote another message fails them, in each arm.
- `t2-04` has the reference of the whole pull request, and its tests are limited to the files of the part. `t2-05` takes only the commits of the part.
- The batch binary is on `PATH`, and the adapter line names it. It holds the fixed code, and the leak pattern does not count a call of it.
- The cost comes from the runner, not from an invoice.
