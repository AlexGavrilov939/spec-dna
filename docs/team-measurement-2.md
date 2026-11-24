# Measurement of the model team: protocol 2, real slices from the history

This page gives the result of protocol 2 in [ADR 0033](adr/0033-protocol-2-of-the-model-team-measurement.md). It is for the owner, who decides the default harness level, the advisor rule of 0.8.1 and the advisor gate. The gate of the ADR first stopped the work after the 3 probes. On 2026-11-23 the owner raised the cap to 700 USD (the ADR is at its hard budget of 14,000 characters, so it keeps its text and this page records the change), and the batch follows. The section [Rules before the batch](#rules-before-the-batch) gives the rules that were set before its first run. The sections after it give the result.

## What ran

- 5 tasks in `bench/team2/tasks/`. Each task is a slice of this repository whose first implementation had defects that a later review found. The agent gets a copy of the start commit with no history, and the approved plan of the slice as its brief.
- The oracle has 2 parts. Part 1 holds the tests that the slice merged. Part 2 holds one test for each later defect, and a run that changes nothing scores 0 in it. The oracle is hidden from the agent.
- The self-test `bench/team2/selftest.py` proved the 5 tasks on a stub agent, with no cost. The start tree fails part 1, the solution with the defects passes part 1 and fails part 2, and the reference passes both.
- Models: Claude Opus 5.5 as the main model and Claude Fable 5.2 as the advisor, Claude Code 2.1.330, spec-dna 0.8.0, effort `xhigh`.
- The order is fixed by the history. The commit of ADR 0033 (`d320f84`) has the time 14:53:59Z. The first probe started at 14:54:13Z. The only change of a rule of the ADR after that is the cap of 2026-11-23 (section [The gate](#the-gate)), which came before the batch. The cap changes no other rule: the projection, the limits, the arms, the tasks, the oracle, the runs and the verdict stay as they are.

## The probes

1 paid run of each arm on `t2-05-approver-hook` (12 tests in part 1, 3 in part 2). Limits: 300 turns, 25 USD and 7,200 seconds for the agent. No run hit a limit. The records are in `bench/results/evals/20261123T145413Z-harness.jsonl`. The result file, with the result of each test, is `bench/team2/results/arms-20261123T145413Z.json`. The scores of both parts come from it.

| Arm | Part 2 (main measure) | Part 1 | Cost (USD) | Minutes | Turns | Advisor calls |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | 1 of 3 | 5 of 12 | 5.90 | 27.9 | 93 | 0 |
| `lean-no-advisor` | 2 of 3 | 5 of 12 | 14.96 | 75.2 | 152 (the runner recorded 2, see below) | 0 |
| `lean` | 2 of 3 | 9 of 12 | 19.85 | 65.0 | 98 | 6 (5 results) |

Each arm has 1 run, so these numbers show the size of a run. They do not show a difference between the arms. The task is not at the ceiling or at the floor: part 2 is neither 1 nor 0 in each arm.

## The gate

The gate of the ADR stopped the work first. The projected cost is `15 x (5.90 + 14.96 + 19.85) x 0.99`, or about 605 USD (the script `gate.py` uses the exact ratio of sizes and prints 604). The room was 380 USD, minus the spend of the probes (40.71 USD), minus 10 USD of reserve, or 329 USD, so the batch did not fit. The other rules are false. The tasks are not at the ceiling or the floor on the probe task, no probe hit a limit, and the `models` of each probe match its arm (only `lean` lists the advisor model).

With the cap of 700 USD the room is 649.29 USD, and the gate is open: 604 USD plus 25 USD for the budget of one run is below it. The limits of the batch are 300 turns and 25 USD for a run, the same as in the probes (`bench/team2/limits.json`). The corrected turns of the probe of `lean-no-advisor` (152) give 2 times 152, rounded to 310, which the ADR caps at 300. The recorded value (2) had given 200.

## Rules before the batch

These rules were set and committed before the first batch run. They change no rule of ADR 0033, except where the text says so.

1. **The turn count of the runner.** The runner of `spec-dna eval run` reads `num_turns` of the last `result` event of `claude -p`. A background task that ends after the agent has stopped wakes the agent, and each wake-up ends with its own `result` event whose `num_turns` counts that segment only. The transcript of the `lean-no-advisor` probe has 9 segments (the prompt and 8 wake-ups) with 101, 8, 4, 18, 8, 5, 4, 2 and 2 turns. The last one has 2 turns, which is the recorded value. The count of the user entries of the main transcript gives 93 and 98 for the 2 probes with no wake-up, as recorded, so it is the count that the batch uses. In 3 runs the transcript count is lower than the record (191 to 189, 123 to 120, 96 to 95), and the corrections file lowers those 3 too. These 3 runs are among the 4 runs that ran while the disk was full (see below), and their transcripts are cut short, so their counts are lower bounds. The cost and the tokens of the runner are totals of the session, and they agree with the `cost-state` entry of each transcript. The stored value of the probe is corrected to 152 in the result file and in its record (`bench/team2/results/probe-corrections.json` keeps the old value and the method). The batch takes the turns of a run from its transcript (`transcript_counts.py`). The runner stays as it is, because the batch must run the binary of 0.8.0 of the probes, and ADR 0033 says that the work changes no code of `src/`. The defect is open for 0.8.1. The runner did not keep the output of `claude`, so the cause is proved by the transcript only. A consequence: `--max-turns` probably bounds each segment and not the whole run.
2. **A second leak column.** The pattern of the ADR stays. A second column, `strict`, reads the target of a call only, and not the text that a call writes: the command of a shell call and the path, the glob or the URL of another call. It matches the oracle, a checkout of the repository (a path under `/var/www/`), the code host, `refs/pull`, a call of `gh`, and `git clone` or `git ls-remote`. The report gives both columns for each run, and 3 verdicts: all runs, without the runs that the pattern of the ADR marks, and without the runs that `strict` marks. The ADR rule stays: if the verdicts have different classes, the owner reads the result as `inconclusive`. The ADR pattern flagged all 3 probes. `strict` flags none of them, and a synthetic test shows that it flags the real cases. The pattern of the ADR can mark most of the runs of the batch. Then the verdict without them has no data, and the report says so.
3. **The probes count as wave 1 of `t2-05-approver-hook`.** The rule of the owner is: the probes count as wave 1 if the fixes change no score. They change none: the scores come from the checks, and the fixes change only the turns and the reports. The limits are the same (300 turns, 25 USD, 7,200 seconds for the agent, 1,800 seconds for a check), and so are the binary, Claude Code 2.1.330, the models and the effort. This sets aside the sentence of the ADR "The probes do not count in the verdict" for this batch. The differences are: the probes ran alone, in the order `none`, `lean-no-advisor`, `lean` of the first start, and with no tap log. The report gives the means of `t2-05` with and without the probe run as a description. The batch has 14 jobs, so it runs 42 new runs.
4. **The jobs.** The runner runs the arms of one command one after the other, and `--arms` needs 2 arms at least, so the arms of one task cannot run at the same time. The batch runs up to 3 jobs at the same time, and a job is the command of the ADR for one task and one wave (`eval run --arms ... <task> --runs 1`), in the order of the waves. Each job has its own result file, and `merge.py` joins them. The arms of a job come in the rotation of the ADR. The concurrency changes the minutes of a run (3 builds at once), and it can raise the risk of the 7,200 seconds limit for the agent.
5. **The cap.** The room of the batch is 649.29 USD. A job gets a cap of 80 USD (3 runs of 25 USD, and 5 USD of slack for a run that passes its budget a little). A job starts when the room, minus the spend of the finished jobs, minus the cap of each job in flight or waiting to run again, is at least 80 USD. It starts with a lower cap (the rest, at least 25 USD) only when no other job runs or waits. A job that the API stopped or a signal stopped goes on with `--resume` after 30 minutes. A job of a wave starts only after the job of the same task in the wave before it is done. A job whose attempt a signal stopped counts its whole cap as spent, because the runner does not write the cost of such an attempt. A stop by the cap ends the batch, and only the complete waves count. After each complete wave, the running totals go to this page, with a commit.

## Result

The batch ran 3 complete waves: 5 tasks, 3 arms, 45 runs, with the probe as the first run of `t2-05-approver-hook`. No attempt was refused (0 excluded attempts) and no job ran again. All runs used Claude Code 2.1.330. The 14 jobs cost 461.40 USD, so the whole protocol cost 502.11 USD of the 700 USD cap (197.89 USD left).

| Arm | Part 2 (main measure) | Part 1 | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run | Marked by the ADR pattern | Marked by `strict` | Hit a limit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.21 | 0.55 | 8.13 | 29.8 | 123 | 0 | 11 | 1 | 0 |
| `lean-no-advisor` | 0.29 | 0.52 | 10.20 | 44.2 | 133 | 0 | 9 | 3 | 0 |
| `lean` | 0.19 | 0.70 | 15.14 | 57.5 | 125 | 3.1 | 14 | 5 | 1 |

The turns come from the transcripts. The minutes are the wall time of a run with up to 3 jobs on the machine, so they are higher than in a quiet run. The 1 run that hit a limit is `lean` on `t2-05`, wave 2, with the 7,200 seconds of the agent. It scores 0 and counts 25 USD.

### Each run

Part 2 is the main measure. The minutes are the wall time of the agent. The turns are the count of the transcript. The 2 last columns count the tool uses that the pattern of the ADR and the column `strict` mark in that run. The wave 1 run of `t2-05` is the probe, and the advisor calls of the probes come from their transcripts.

| Task | Wave | Arm | Part 2 | Part 1 | USD | Minutes | Turns | Advisor calls | ADR pattern | `strict` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `t2-01-tree-binary` | 1 | `none` | 1.00 | 1.00 | 3.07 | 12.9 | 60 | 0 | 0 | 0 |
| `t2-01-tree-binary` | 1 | `lean-no-advisor` | 1.00 | 1.00 | 6.87 | 44.8 | 79 | 0 | 0 | 0 |
| `t2-01-tree-binary` | 1 | `lean` | 0.00 | 1.00 | 7.52 | 33.9 | 68 | 3 | 2 | 0 |
| `t2-01-tree-binary` | 2 | `none` | 0.00 | 0.00 | 2.29 | 7.6 | 42 | 0 | 0 | 0 |
| `t2-01-tree-binary` | 2 | `lean-no-advisor` | 0.00 | 1.00 | 6.92 | 44.2 | 92 | 0 | 0 | 0 |
| `t2-01-tree-binary` | 2 | `lean` | 0.00 | 1.00 | 11.01 | 45.7 | 85 | 4 | 4 | 0 |
| `t2-01-tree-binary` | 3 | `none` | 0.00 | 1.00 | 3.21 | 21.1 | 65 | 0 | 1 | 0 |
| `t2-01-tree-binary` | 3 | `lean-no-advisor` | 0.00 | 0.67 | 5.09 | 45.5 | 67 | 0 | 0 | 0 |
| `t2-01-tree-binary` | 3 | `lean` | 0.00 | 1.00 | 9.73 | 65.7 | 88 | 3 | 5 | 0 |
| `t2-02-change-feedback` | 1 | `none` | 0.00 | 0.75 | 11.18 | 28.4 | 189 | 0 | 7 | 0 |
| `t2-02-change-feedback` | 1 | `lean-no-advisor` | 0.00 | 0.00 | 9.69 | 25.8 | 179 | 0 | 5 | 0 |
| `t2-02-change-feedback` | 1 | `lean` | 0.00 | 0.75 | 17.67 | 53.1 | 177 | 3 | 11 | 0 |
| `t2-02-change-feedback` | 2 | `none` | 0.00 | 0.75 | 10.02 | 32.9 | 161 | 0 | 1 | 0 |
| `t2-02-change-feedback` | 2 | `lean-no-advisor` | 0.00 | 0.75 | 11.92 | 46.9 | 197 | 0 | 0 | 0 |
| `t2-02-change-feedback` | 2 | `lean` | 0.00 | 0.75 | 16.17 | 59.5 | 146 | 3 | 3 | 1 |
| `t2-02-change-feedback` | 3 | `none` | 0.00 | 0.75 | 9.52 | 38.5 | 191 | 0 | 2 | 0 |
| `t2-02-change-feedback` | 3 | `lean-no-advisor` | 0.00 | 0.75 | 11.16 | 33.1 | 183 | 0 | 5 | 0 |
| `t2-02-change-feedback` | 3 | `lean` | 0.00 | 1.00 | 13.87 | 38.8 | 176 | 2 | 2 | 0 |
| `t2-03-task-checks` | 1 | `none` | 0.00 | 0.79 | 11.32 | 28.3 | 153 | 0 | 6 | 0 |
| `t2-03-task-checks` | 1 | `lean-no-advisor` | 0.00 | 0.76 | 11.86 | 49.3 | 126 | 0 | 1 | 0 |
| `t2-03-task-checks` | 1 | `lean` | 0.00 | 0.83 | 10.52 | 27.5 | 120 | 1 | 0 | 0 |
| `t2-03-task-checks` | 2 | `none` | 0.00 | 0.69 | 13.71 | 33.2 | 195 | 0 | 4 | 0 |
| `t2-03-task-checks` | 2 | `lean-no-advisor` | 0.00 | 0.66 | 14.73 | 41.5 | 176 | 0 | 12 | 1 |
| `t2-03-task-checks` | 2 | `lean` | 0.00 | 0.59 | 17.97 | 50.2 | 133 | 4 | 1 | 0 |
| `t2-03-task-checks` | 3 | `none` | 0.00 | 0.52 | 12.08 | 58.5 | 187 | 0 | 3 | 0 |
| `t2-03-task-checks` | 3 | `lean-no-advisor` | 0.00 | 0.62 | 13.15 | 32.4 | 143 | 0 | 2 | 0 |
| `t2-03-task-checks` | 3 | `lean` | 0.00 | 0.97 | 19.17 | 54.1 | 145 | 3 | 1 | 1 |
| `t2-04-locked-links` | 1 | `none` | 0.00 | 0.12 | 11.37 | 44.4 | 85 | 0 | 0 | 0 |
| `t2-04-locked-links` | 1 | `lean-no-advisor` | 0.50 | 0.12 | 5.43 | 16.4 | 95 | 0 | 0 | 0 |
| `t2-04-locked-links` | 1 | `lean` | 0.50 | 0.12 | 10.65 | 34.4 | 102 | 2 | 10 | 1 |
| `t2-04-locked-links` | 2 | `none` | 0.50 | 0.12 | 6.64 | 19.6 | 95 | 0 | 0 | 0 |
| `t2-04-locked-links` | 2 | `lean-no-advisor` | 0.50 | 0.12 | 9.61 | 38.5 | 118 | 0 | 3 | 0 |
| `t2-04-locked-links` | 2 | `lean` | 1.00 | 1.00 | 16.54 | 60.2 | 103 | 4 | 6 | 2 |
| `t2-04-locked-links` | 3 | `none` | 0.00 | 0.12 | 6.39 | 20.6 | 94 | 0 | 1 | 1 |
| `t2-04-locked-links` | 3 | `lean-no-advisor` | 1.00 | 0.12 | 11.33 | 44.9 | 115 | 0 | 8 | 3 |
| `t2-04-locked-links` | 3 | `lean` | 0.00 | 0.12 | 19.81 | 98.6 | 126 | 4 | 2 | 1 |
| `t2-05-approver-hook` | 1 | `none` | 0.33 | 0.42 | 5.90 | 27.9 | 93 | 0 | 1 | 0 |
| `t2-05-approver-hook` | 1 | `lean-no-advisor` | 0.67 | 0.42 | 14.96 | 75.2 | 152 | 0 | 5 | 0 |
| `t2-05-approver-hook` | 1 | `lean` | 0.67 | 0.75 | 19.85 | 65.0 | 98 | 6 | 1 | 0 |
| `t2-05-approver-hook` | 2 | `none` | 0.67 | 0.58 | 7.05 | 28.0 | 118 | 0 | 1 | 0 |
| `t2-05-approver-hook` | 2 | `lean-no-advisor` | 0.00 | 0.25 | 11.64 | 52.5 | 157 | 0 | 4 | 1 |
| `t2-05-approver-hook` | 2 | `lean` | 0.00 | 0.00 | 25.00 | 120.0 | 184 | 3 | 6 | 0 |
| `t2-05-approver-hook` | 3 | `none` | 0.67 | 0.67 | 8.22 | 45.7 | 118 | 0 | 4 | 0 |
| `t2-05-approver-hook` | 3 | `lean-no-advisor` | 0.67 | 0.50 | 8.62 | 72.6 | 111 | 0 | 0 | 0 |
| `t2-05-approver-hook` | 3 | `lean` | 0.67 | 0.67 | 11.65 | 55.6 | 121 | 2 | 2 | 0 |

### The 2 verdicts

The difference is the mean over the 5 tasks of the difference of the task means, with the 90% interval of the bootstrap by task (seed 24, 2,000 resamples). The cost ratio has the same interval.

| Comparison | Part 2 difference (main measure) | Cost ratio | Verdict, all runs | Verdict without the runs marked by the ADR pattern | Verdict without the runs marked by `strict` |
| --- | --- | --- | --- | --- | --- |
| `lean` against `lean-no-advisor` | -0.10 (-0.20 to 0.00) | 1.49 (1.33 to 1.65) | `no-gain-at-cost` | `inconclusive`, no data | `no-gain-at-cost` |
| `lean` against `none` | -0.02 (-0.18 to +0.13) | 1.86 (1.51 to 2.41) | `no-gain-at-cost` | `inconclusive`, no data | `no-gain-at-cost` |

The variant without the runs marked by `strict` has 4 tasks: all 3 `lean` runs of `t2-04-locked-links` are marked, so that task leaves both comparisons. It is the task where both `lean` arms did better than `none`. So the 2 verdicts keep their class only without the task that favors `lean` most.

Two sensitivity checks, as a description. Without the wave 2 run of `t2-05` (the run of `lean` that hit the limit), `lean` against `lean-no-advisor` stays `no-gain-at-cost` (-0.10, -0.20 to 0.00, cost ratio 1.42), and `lean` against `none` becomes `inconclusive` (+0.03, -0.13 to +0.20, cost ratio 1.79). Without the probe run, both stay `no-gain-at-cost` (-0.10 at 1.52 times the cost, and -0.07 at 1.82 times).

For description, `lean-no-advisor` against `none` gives +0.08 (-0.04 to +0.28) at 1.25 times the cost: `inconclusive`. The difference on part 1 is +0.19 for `lean` against `lean-no-advisor` (+0.14 without the spoiled run of the next section), and +0.15 for `lean` against `none`.

The 2 verdicts of each comparison have different classes. The ADR pattern marks 34 of the 45 runs, so only 11 runs are left (`none` 4, `lean-no-advisor` 6, `lean` 1), and no task has runs in both arms of a comparison. By the rule of the ADR, the owner reads the result as `inconclusive`. In the batch, the pattern matched with only 3 of its parts: `git clone`, `git fetch` or `git remote` (59 tool uses in 25 runs), `gh pr` text (8 uses in 2 runs) and the output folder of Claude Code (124 uses in 25 runs). It matched 0 times with the oracle, `/var/www`, the code host, `refs/pull`, the fixtures and the folder `claude/projects`. Neither column matches a path into the config folder of Claude Code with its `projects` folder: 19 tool inputs name it (`lean` 10, `lean-no-advisor` 6, `none` 3), and each names the copy of its own run only. This page also gives the reason to read it otherwise: the pattern of the ADR marks the output folder of Claude Code, a `git clone` of the copy and test text. The column `strict` marks 9 runs. In the 9 marked runs, read by hand, each marked call is a `git clone` of the copy of the run itself or of a scratch repository of the run (a local path), or a test text with a `gh` command inside a heredoc that the pattern could not strip. No call reaches the oracle, a checkout of the repository, the code host or `refs/pull`. The owner decides.

### The disk was full during wave 1

The disk of the machine was full (ENOSPC) for a time during wave 1 (the evening of 2026-11-23, at about the time that another session of the account reported it). The job logs show it: the transcript of 4 runs could not be saved, and these 4 runs have `claude_exit` 1: `t2-02` wave 1 `none` and `lean-no-advisor`, `t2-03` wave 1 `lean`, and `t2-04` wave 1 `lean-no-advisor`. In 1 of them (`t2-02` wave 1 `lean-no-advisor`) the copy of the hidden tests to the run failed for lack of space, so its part 1 is 0 of 4 and most likely does not show the work of the agent. The other 8 runs of `t2-02` score 3 or 4 of 4 on part 1, and part 2 is 0 in every run of `t2-02`. The runner counts such a run as it is (the agent stopped with the subtype `success`), and it is no refusal of the API. The ADR forbids a wave that is added after a look at the result, so no run was repeated. The effect, as a description:

- Part 2 verdicts: with the 4 runs left out, `lean` against `lean-no-advisor` is -0.12 (-0.23 to 0.00) at 1.47 times the cost, and `lean` against `none` is -0.02 (-0.18 to +0.13) at 1.95 times the cost. Both stay `no-gain-at-cost`. With only the 1 spoiled run left out, they are -0.10 and -0.02, also `no-gain-at-cost`.
- Part 1: the mean of `lean-no-advisor` is 0.57 (not 0.52) with the spoiled run left out, and `lean` is higher by 0.14 (0.09 to 0.21) against it. Against `none` it is +0.15 (0.03 to 0.26).
- The turns, the advisor calls and the leak marks of the 3 other runs come from transcripts that are cut short, so they are lower bounds.

### Where the arms differed, by task

Mean part 2 of 3 runs, in the order `none`, `lean-no-advisor`, `lean`. Part 1 and the cost per run follow.

| Task | Part 2 | Part 1 | Cost per run (USD) | Note |
| --- | --- | --- | --- | --- |
| `t2-01-tree-binary` | 0.33, 0.33, 0.00 | 0.67, 0.89, 1.00 | 2.9, 6.3, 9.4 | 1 run of `none` and 1 of `lean-no-advisor` caught the later defect, none of `lean` |
| `t2-02-change-feedback` | 0.00, 0.00, 0.00 | 0.75, 0.50, 0.83 | 10.2, 10.9, 15.9 | At the floor in each arm |
| `t2-03-task-checks` | 0.00, 0.00, 0.00 | 0.67, 0.68, 0.79 | 12.4, 13.2, 15.9 | At the floor in each arm |
| `t2-04-locked-links` | 0.17, 0.67, 0.50 | 0.12, 0.12, 0.42 | 8.1, 8.8, 15.7 | Both `lean` arms did better than `none`, and the advisor added nothing |
| `t2-05-approver-hook` | 0.56, 0.44, 0.44 | 0.56, 0.39, 0.47 | 7.1, 11.7, 18.8 | No order of the arms. With the probe left out, the means are 0.67, 0.33 and 0.33 on 2 runs |

In the sum over the 5 tasks of the difference of `lean` against `lean-no-advisor` (-0.50), `t2-01` gives -0.33, `t2-04` gives -0.17 and `t2-05` gives 0, so `t2-01` drives most of it, and on `t2-04` `lean` beats `none`. On `t2-02` and `t2-03` no run of any arm caught a later defect.

### Use of the result

The table "Use of the result" of the ADR gives, for the verdicts on all runs (`no-gain-at-cost`): the rule of 0.8.1 stays, and the owner decides if `lean` keeps the advisor as a default role, and for the default level the owner decides. With the reading of the ADR (`inconclusive`), nothing changes. The "advisor gate" of 0.8.1 is the rule that the advisor runs only on a change of class `higher-risk`. The data do not argue against it: the advisor costs 1.5 times as much and shows no gain on the main measure, so a wider use of the advisor has no support here. The data say: `lean` raises part 1 by 0.15 to 0.19 and does not raise the main measure, at 1.5 to 1.9 times the cost and about twice the minutes of `none`. The advisor (3.1 calls per run) shows no gain on the main measure. The owner decides each change of a default.

## Running totals

These totals are written after each complete wave, so a stopped session loses nothing. They are not the verdict. All verdicts are `inconclusive` until 3 waves are complete.

### After wave 2 (2026-11-24, 04:37)

10 runs in each arm (5 tasks, 2 waves; the probe is wave 1 of `t2-05`). The jobs of waves 1 and 2 cost 298.39 USD, and the whole protocol has cost 339.10 USD, with the probes (the jobs of wave 3 that are done are not in these totals). 1 run hit a limit: `lean` on `t2-05-approver-hook`, wave 2, ran into the 7,200 seconds of the agent. It scores 0 in both parts and counts 25 USD, as the ADR says.

| Arm | Part 2 (main measure) | Part 1 | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run | Marked by the ADR pattern | Marked by `strict` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.25 | 0.52 | 8.26 | 26.3 | 119 | 0 | 6 | 0 |
| `lean-no-advisor` | 0.27 | 0.51 | 10.36 | 43.5 | 137 | 0 | 6 | 2 |
| `lean` | 0.22 | 0.68 | 15.29 | 55.0 | 122 | 3.3 | 9 | 3 |

With all 30 runs, the 2 verdicts are `inconclusive` (a verdict needs 3 complete waves): `lean` against `lean-no-advisor` gives -0.05 on part 2 (interval -0.20 to +0.10) at 1.48 times the cost, and `lean` against `none` gives -0.03 (interval -0.27 to +0.20) at 1.85 times the cost. Part 1 is higher in `lean` by 0.17 and 0.16. The variant without the runs that the ADR pattern marks has 1 run of `lean`, so it has no verdict. The variant without the runs that `strict` marks is also `inconclusive`.

The 5 runs that `strict` marks, read call by call: each call is a `git clone` of the copy of the run itself or of a scratch repository of the run (a local path), and one is a test text with `gh pr create` inside a heredoc that the pattern could not strip. No call reaches the oracle, a checkout of the repository, the code host or `refs/pull`. The column stays as it is, and this reading is a note.

### After wave 1 (2026-11-24, 00:14)

Wave 1 has the 5 tasks in the 3 arms. The run of `t2-05-approver-hook` is the probe. The 4 new jobs cost 117.16 USD, and the whole protocol has cost 157.87 USD, with the probes.

| Arm | Part 2 (main measure) | Part 1 | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.27 | 0.62 | 8.57 | 28.4 | 116 | 0 |
| `lean-no-advisor` | 0.43 | 0.46 | 9.76 | 42.3 | 126 | 0 |
| `lean` | 0.23 | 0.69 | 13.24 | 42.8 | 113 | 3.0 |

Leaks: the pattern of the ADR marks 10 of the 15 runs (`none` 3, `lean-no-advisor` 3, `lean` 4). The column `strict` marks 1 run, of `lean` on `t2-04-locked-links`. The call is `git clone -q .`, a clone of the copy of the run itself, so it reaches no remote, no oracle and no checkout. The column stays as it is, and this reading is a note.

## Limits

- The probes have 1 run for each arm on 1 task. They give no interval and no verdict.
- The runner recorded 2 turns for `lean-no-advisor` for a run of 152 turns. See the rule 1 above for the cause and the correction.
- The count of leaks marks the 3 probes as leaked, as the ADR defines it: 1 tool use of `none`, 5 of `lean-no-advisor` and 1 of `lean` match the pattern of `transcript_counts.py`. The matches are of 3 kinds: the folder of task output that Claude Code makes for the copy, a `git remote -v` inside the copy, and test text with the string `gh pr`. No tool input names the oracle, a checkout of this repository, the remote host or the pull request refs. The pattern has false positives, and the ADR fixes it, so it was not changed. The column `strict` of the rule 2 flags none of the 3 probes. Only tool inputs were checked by kind, not tool results.
- Before the probes, the person deleted the folders `team2-target-*` and an old scratch copy of the runner from the temporary folder of the system. No tool input of a probe names a folder `team2-` or the copy of another run.
- 5 tasks and 3 waves give wide intervals. Part 2 has 1 to 4 tests for each task, so a task mean moves in steps. 2 of the 5 tasks are at the floor of part 2 in each arm.
- 3 jobs ran at the same time on one machine and one account, so the minutes (and the risk of the 7,200 seconds limit) are higher than in a quiet run. The arms of a job ran one after the other, not at the same time as the owner asked, because the runner cannot run 1 arm alone. The load of the machine was high during the batch (other sessions ran builds).
- The check of the scheduler for other processes of `spec-dna eval run` and `claude -p` used `ps`, and the sandbox of the session showed only part of the processes, so this check was blind. The evidence that no other eval disturbed the batch is indirect: 0 excluded attempts, no refusal of the API and no job that ran again. Other sessions of the same account ran at the same time.
- The probe is 1 of the 45 runs. It ran alone and with no tap log, and its counts come from the transcripts.
- `--max-turns` probably bounds each segment between wake-ups and not the whole run. The turn limit did not decide any run. The runner defect (the turns of the last segment only) stays open for 0.8.1. In the batch, the recorded turns of 27 of the 42 new runs (and of the wave 1 run of the probe of `lean-no-advisor`, which is 28 of 45) differ from the transcript count (the list is in `bench/team2/results/probe-corrections.json`), so the result file of the batch (`arms-batch-20261124T073247Z.json`) and its records hold the transcript count.
- The disk was full for a time in wave 1 (see "The disk was full during wave 1"). No job ran again and no attempt was refused, so the logic of the scheduler for retries and for the cap was not used in this batch.
- The pattern of the ADR marks 34 of 45 runs, and `strict` marks 9. Both columns over-mark: each of the 9 marks of `strict` is a local `git clone` or a test text. A leak that neither column sees (a read by a path built in a script) is possible.
- One repository, one language, one model family, one version of Claude Code.
- The tasks are public and the fixed code is on the public repository. A model may know them.
- The cost comes from the runner, not from an invoice.

## Spend

The probes cost 40.71 USD and the 14 jobs of the batch cost 461.40 USD, so the protocol cost 502.11 USD of the 700 USD cap. The dry run and the self-test cost nothing, because they use the stub agent. The pilot of protocol 1 is on [its page](team-measurement.md).
