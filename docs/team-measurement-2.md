# Measurement of the model team: protocol 2, real slices from the history

This page gives the result of protocol 2 in [ADR 0033](adr/0033-protocol-2-of-the-model-team-measurement.md). It is for the owner, who decides the default harness level, the advisor rule of 0.8.1 and the advisor gate. The gate of the ADR first stopped the work after the 3 probes. On 2026-11-23 the owner raised the cap to 700 USD ([the amendment of the ADR](adr/0033-protocol-2-of-the-model-team-measurement.md#2026-11-23-the-cap-rises-to-700-usd)), and the batch follows. The section [Rules before the batch](#rules-before-the-batch) gives the rules that were set before its first run. The sections after it give the result.

## What ran

- 5 tasks in `bench/team2/tasks/`. Each task is a slice of this repository whose first implementation had defects that a later review found. The agent gets a copy of the start commit with no history, and the approved plan of the slice as its brief.
- The oracle has 2 parts. Part 1 holds the tests that the slice merged. Part 2 holds one test for each later defect, and a run that changes nothing scores 0 in it. The oracle is hidden from the agent.
- The self-test `bench/team2/selftest.py` proved the 5 tasks on a stub agent, with no cost. The start tree fails part 1, the solution with the defects passes part 1 and fails part 2, and the reference passes both.
- Models: Claude Opus 5.5 as the main model and Claude Fable 5.2 as the advisor, Claude Code 2.1.330, spec-dna 0.8.0, effort `xhigh`.
- The order is fixed by the history. The commit of ADR 0033 (`d320f84`) has the time 14:53:59Z. The first probe started at 14:54:13Z. The only change of a rule of the ADR after that is the amendment of the cap of 2026-11-23, which came before the batch.

## The probes

1 paid run of each arm on `t2-05-approver-hook` (12 tests in part 1, 3 in part 2). Limits: 300 turns, 25 USD and 7,200 seconds for the agent. No run hit a limit. The records are in `bench/results/evals/20261123T145413Z-harness.jsonl`. The result file, with the result of each test, is `bench/team2/results/arms-20261123T145413Z.json`. The scores of both parts come from it.

| Arm | Part 1 | Part 2 (main measure) | Cost (USD) | Minutes | Turns | Advisor calls |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | 5 of 12 | 1 of 3 | 5.90 | 27.9 | 93 | 0 |
| `lean-no-advisor` | 5 of 12 | 2 of 3 | 14.96 | 75.2 | 152 (the runner recorded 2, see below) | 0 |
| `lean` | 9 of 12 | 2 of 3 | 19.85 | 65.0 | 98 | 6 (5 results) |

Each arm has 1 run, so these numbers show the size of a run. They do not show a difference between the arms. The task is not at the ceiling or at the floor: part 2 is neither 1 nor 0 in each arm.

## The gate

The gate of the ADR stopped the work first. The projected cost is `15 x (5.90 + 14.96 + 19.85) x 0.99`, or about 605 USD (the script `gate.py` uses the exact ratio of sizes and prints 604). The room was 380 USD, minus the spend of the probes (40.71 USD), minus 10 USD of reserve, or 329 USD, so the batch did not fit. The other rules are false. The tasks are not at the ceiling or the floor on the probe task, no probe hit a limit, and the `models` of each probe match its arm (only `lean` lists the advisor model).

With the cap of 700 USD the room is 649.29 USD, and the gate is open: 604 USD plus 25 USD for the budget of one run is below it. The limits of the batch are 300 turns and 25 USD for a run, the same as in the probes (`bench/team2/limits.json`). The corrected turns of the probe of `lean-no-advisor` (152) give 2 times 152, rounded to 310, which the ADR caps at 300. The recorded value (2) had given 200.

## Rules before the batch

These rules were set and committed before the first batch run. They change no rule of ADR 0033, except where the text says so.

1. **The turn count of the runner.** The runner of `spec-dna eval run` reads `num_turns` of the last `result` event of `claude -p`. A background task that ends after the agent has stopped wakes the agent, and each wake-up ends with its own `result` event whose `num_turns` counts that segment only. The transcript of the `lean-no-advisor` probe has 9 segments (the prompt and 8 wake-ups) with 101, 8, 4, 18, 8, 5, 4, 2 and 2 turns. The last one has 2 turns, which is the recorded value. The count of the user entries of the main transcript gives 93 and 98 for the 2 probes with no wake-up, as recorded, so it is the right count. The cost and the tokens of the runner are totals of the session, and they agree with the `cost-state` entry of each transcript. The stored value of the probe is corrected to 152 in the result file and in its record (`bench/team2/results/probe-corrections.json` keeps the old value and the method). The batch takes the turns of a run from its transcript (`transcript_counts.py`). The runner stays as it is, because the batch must run the binary of 0.8.0 of the probes, and ADR 0033 says that the work changes no code of `src/`. The defect is open for 0.8.1. The runner did not keep the output of `claude`, so the cause is proved by the transcript only. A consequence: `--max-turns` probably bounds each segment and not the whole run.
2. **A second leak column.** The pattern of the ADR stays. A second column, `strict`, reads the target of a call only, and not the text that a call writes: the command of a shell call and the path, the glob or the URL of another call. It matches the oracle, a checkout of the repository (a path under `/var/www/`), the code host, `refs/pull`, a call of `gh`, and `git clone` or `git ls-remote`. The report gives both columns for each run, and 3 verdicts: all runs, without the runs that the pattern of the ADR marks, and without the runs that `strict` marks. The ADR rule stays: if the verdicts have different classes, the owner reads the result as `inconclusive`. The ADR pattern flagged all 3 probes. `strict` flags none of them, and a synthetic test shows that it flags the real cases. The pattern of the ADR can mark most of the runs of the batch. Then the verdict without them has no data, and the report says so.
3. **The probes count as wave 1 of `t2-05-approver-hook`.** The rule of the owner is: the probes count as wave 1 if the fixes change no score. They change none: the scores come from the checks, and the fixes change only the turns and the reports. The limits are the same (300 turns, 25 USD, 7,200 seconds for the agent, 1,800 seconds for a check), and so are the binary, Claude Code 2.1.330, the models and the effort. This sets aside the sentence of the ADR "The probes do not count in the verdict" for this batch. The differences are: the probes ran alone, in the order `none`, `lean-no-advisor`, `lean` of the first start, and with no tap log. The report gives the means of `t2-05` with and without the probe run as a description. The batch has 14 jobs, so it runs 42 new runs.
4. **The jobs.** The runner runs the arms of one command one after the other, and `--arms` needs 2 arms at least, so the arms of one task cannot run at the same time. The batch runs up to 3 jobs at the same time, and a job is the command of the ADR for one task and one wave (`eval run --arms ... <task> --runs 1`), in the order of the waves. Each job has its own result file, and `merge.py` joins them. The arms of a job come in the rotation of the ADR. The concurrency changes the minutes of a run (3 builds at once), and it can raise the risk of the 7,200 seconds limit for the agent.
5. **The cap.** The room of the batch is 649.29 USD. A job gets a cap of 80 USD (3 runs of 25 USD, and 5 USD of slack for a run that passes its budget a little). A job starts when the room, minus the spend of the finished jobs, minus the cap of each job in flight or waiting to run again, is at least 80 USD. It starts with a lower cap (the rest, at least 25 USD) only when no other job runs or waits. A job that the API stopped or a signal stopped goes on with `--resume` after 30 minutes. A job of a wave starts only after the job of the same task in the wave before it is done. A job whose attempt a signal stopped counts its whole cap as spent, because the runner does not write the cost of such an attempt. A stop by the cap ends the batch, and only the complete waves count. After each complete wave, the running totals go to this page, with a commit.

## Result

The batch ran 3 complete waves: 5 tasks, 3 arms, 45 runs, with the probe as the first run of `t2-05-approver-hook`. No attempt was refused (0 excluded attempts) and no job ran again. All runs used Claude Code 2.1.330. The 14 jobs cost 461.40 USD, so the whole protocol cost 502.11 USD of the 700 USD cap (197.89 USD left).

| Arm | Part 1 | Part 2 (main measure) | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run | Marked by the ADR pattern | Marked by `strict` | Hit a limit |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.55 | 0.21 | 8.13 | 29.8 | 123 | 0 | 11 | 1 | 0 |
| `lean-no-advisor` | 0.52 | 0.29 | 10.20 | 44.2 | 133 | 0 | 9 | 3 | 0 |
| `lean` | 0.70 | 0.19 | 15.15 | 57.5 | 125 | 3.1 | 14 | 5 | 1 |

The turns come from the transcripts. The minutes are the wall time of a run with up to 3 jobs on the machine, so they are higher than in a quiet run. The 1 run that hit a limit is `lean` on `t2-05`, wave 2, with the 7,200 seconds of the agent. It scores 0 and counts 25 USD.

### The 2 verdicts

The difference is the mean over the 5 tasks of the difference of the task means, with the 90% interval of the bootstrap by task (seed 24, 2,000 resamples). The cost ratio has the same interval.

| Comparison | Part 2 difference (main measure) | Cost ratio | Verdict, all runs | Verdict without the runs marked by the ADR pattern | Verdict without the runs marked by `strict` |
| --- | --- | --- | --- | --- | --- |
| `lean` against `lean-no-advisor` | -0.10 (-0.20 to 0.00) | 1.49 (1.33 to 1.65) | `no-gain-at-cost` | `inconclusive`, no data | `no-gain-at-cost` |
| `lean` against `none` | -0.02 (-0.18 to +0.13) | 1.86 (1.51 to 2.41) | `no-gain-at-cost` | `inconclusive`, no data | `no-gain-at-cost` |

For description, `lean-no-advisor` against `none` gives +0.08 (-0.04 to +0.28) at 1.25 times the cost: `inconclusive`. The difference on part 1 is +0.19 for `lean` against `lean-no-advisor`, and +0.15 for `lean` against `none`.

The 2 verdicts of each comparison have different classes. The ADR pattern marks 34 of the 45 runs, so only 11 runs are left (`none` 4, `lean-no-advisor` 6, `lean` 1), and no task has runs in both arms of a comparison. By the rule of the ADR, the owner reads the result as `inconclusive`. This page also gives the reason to read it otherwise: the pattern of the ADR marks the output folder of Claude Code, `git remote -v` in the copy and test text. The column `strict` marks 9 runs. In the 9 marked runs, read by hand, each marked call is a `git clone` of the copy of the run itself or of a scratch repository of the run (a local path), or a test text with `gh pr create` inside a heredoc that the pattern could not strip. No call reaches the oracle, a checkout of the repository, the code host or `refs/pull`. The owner decides.

### Where the arms differed, by task

Mean part 2 of 3 runs, in the order `none`, `lean-no-advisor`, `lean`. Part 1 and the cost per run follow.

| Task | Part 2 | Part 1 | Cost per run (USD) | Note |
| --- | --- | --- | --- | --- |
| `t2-01-tree-binary` | 0.33, 0.33, 0.00 | 0.67, 0.89, 1.00 | 2.9, 6.3, 9.4 | 1 run of `none` and 1 of `lean-no-advisor` caught the later defect, none of `lean` |
| `t2-02-change-feedback` | 0.00, 0.00, 0.00 | 0.75, 0.50, 0.83 | 10.2, 10.9, 15.9 | At the floor in each arm |
| `t2-03-task-checks` | 0.00, 0.00, 0.00 | 0.67, 0.68, 0.79 | 12.4, 13.2, 15.9 | At the floor in each arm |
| `t2-04-locked-links` | 0.17, 0.67, 0.50 | 0.12, 0.12, 0.42 | 8.1, 8.8, 15.7 | Both `lean` arms did better than `none`, and the advisor added nothing |
| `t2-05-approver-hook` | 0.56, 0.44, 0.44 | 0.56, 0.39, 0.47 | 7.1, 11.7, 18.8 | No order of the arms. With the probe left out, the means are 0.67, 0.33 and 0.33 on 2 runs |

So the main measure rests on 2 tasks (`t2-04` and `t2-05`) and a few runs of `t2-01`. On `t2-02` and `t2-03` no run of any arm caught a later defect.

### Use of the result

The table "Use of the result" of the ADR gives, for the verdicts on all runs (`no-gain-at-cost`): the rule of 0.8.1 stays, and the owner decides if `lean` keeps the advisor as a default role, and for the default level the owner decides. With the reading of the ADR (`inconclusive`), nothing changes. The data say: `lean` raises part 1 by 0.15 to 0.19 and does not raise the main measure, at 1.5 to 1.9 times the cost and about twice the minutes of `none`. The advisor (3.1 calls per run) shows no gain on the main measure. The owner decides each change of a default.

## Running totals

These totals are written after each complete wave, so a stopped session loses nothing. They are not the verdict. All verdicts are `inconclusive` until 3 waves are complete.

### After wave 2 (2026-11-24, 04:37)

10 runs in each arm (5 tasks, 2 waves; the probe is wave 1 of `t2-05`). The jobs of waves 1 and 2 cost 298.40 USD, and the whole protocol has cost 339.11 USD, with the probes (the jobs of wave 3 that are done are not in these totals). 1 run hit a limit: `lean` on `t2-05-approver-hook`, wave 2, ran into the 7,200 seconds of the agent. It scores 0 in both parts and counts 25 USD, as the ADR says.

| Arm | Part 1 | Part 2 (main measure) | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run | Marked by the ADR pattern | Marked by `strict` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.52 | 0.25 | 8.26 | 26.3 | 119 | 0 | 6 | 0 |
| `lean-no-advisor` | 0.51 | 0.27 | 10.36 | 43.5 | 137 | 0 | 6 | 2 |
| `lean` | 0.68 | 0.22 | 15.29 | 55.0 | 122 | 3.3 | 9 | 3 |

With all 30 runs, the 2 verdicts are `inconclusive` (a verdict needs 3 complete waves): `lean` against `lean-no-advisor` gives -0.05 on part 2 (interval -0.20 to +0.10) at 1.48 times the cost, and `lean` against `none` gives -0.03 (interval -0.27 to +0.20) at 1.85 times the cost. Part 1 is higher in `lean` by 0.17 and 0.16. The variant without the runs that the ADR pattern marks has 1 run of `lean`, so it has no verdict. The variant without the runs that `strict` marks is also `inconclusive`.

The 5 runs that `strict` marks, read call by call: each call is a `git clone` of the copy of the run itself or of a scratch repository of the run (a local path), and one is a test text with `gh pr create` inside a heredoc that the pattern could not strip. No call reaches the oracle, a checkout of the repository, the code host or `refs/pull`. The column stays as it is, and this reading is a note.

### After wave 1 (2026-11-24, 00:14)

Wave 1 has the 5 tasks in the 3 arms. The run of `t2-05-approver-hook` is the probe. The 4 new jobs cost 117.16 USD, and the whole protocol has cost 157.87 USD, with the probes.

| Arm | Part 1 | Part 2 (main measure) | Cost per run (USD) | Minutes per run | Turns per run | Advisor calls per run |
| --- | --- | --- | --- | --- | --- | --- |
| `none` | 0.62 | 0.27 | 8.57 | 28.4 | 116 | 0 |
| `lean-no-advisor` | 0.46 | 0.43 | 9.76 | 42.3 | 126 | 0 |
| `lean` | 0.69 | 0.23 | 13.25 | 42.8 | 113 | 3.0 |

Leaks: the pattern of the ADR marks 10 of the 15 runs (`none` 3, `lean-no-advisor` 3, `lean` 4). The column `strict` marks 1 run, of `lean` on `t2-04-locked-links`. The call is `git clone -q .`, a clone of the copy of the run itself, so it reaches no remote, no oracle and no checkout. The column stays as it is, and this reading is a note.

## Limits

- The probes have 1 run for each arm on 1 task. They give no interval and no verdict.
- The runner recorded 2 turns for `lean-no-advisor` for a run of 152 turns. See the rule 1 above for the cause and the correction.
- The count of leaks marks the 3 probes as leaked, as the ADR defines it: 1 tool use of `none`, 5 of `lean-no-advisor` and 1 of `lean` match the pattern of `transcript_counts.py`. The matches are of 3 kinds: the folder of task output that Claude Code makes for the copy, a `git remote -v` inside the copy, and test text with the string `gh pr`. No tool input names the oracle, a checkout of this repository, the remote host or the pull request refs. The pattern has false positives, and the ADR fixes it, so it was not changed. The column `strict` of the rule 2 flags none of the 3 probes. Only tool inputs were checked by kind, not tool results.
- Before the probes, the person deleted the folders `team2-target-*` and an old scratch copy of the runner from the temporary folder of the system. No tool input of a probe names a folder `team2-` or the copy of another run.
- 5 tasks and 3 waves give wide intervals. Part 2 has 1 to 4 tests for each task, so a task mean moves in steps. 2 of the 5 tasks are at the floor of part 2 in each arm.
- 3 jobs ran at the same time on one machine and one account, so the minutes (and the risk of the 7,200 seconds limit) are higher than in a quiet run. The arms of a job ran one after the other, not at the same time as the owner asked, because the runner cannot run 1 arm alone. The load of the machine was high during the batch (other sessions ran builds).
- The probe is 1 of the 45 runs. It ran alone and with no tap log, and its counts come from the transcripts.
- `--max-turns` probably bounds each segment between wake-ups and not the whole run. The turn limit did not decide any run. The runner defect (the turns of the last segment only) stays open for 0.8.1.
- No job ran again and no attempt was refused, so the logic of the scheduler for retries and for the cap was not used in this batch.
- The pattern of the ADR marks 34 of 45 runs, and `strict` marks 9. Both columns over-mark: each of the 9 marks of `strict` is a local `git clone` or a test text. A leak that neither column sees (a read by a path built in a script) is possible.
- One repository, one language, one model family, one version of Claude Code.
- The tasks are public and the fixed code is on the public repository. A model may know them.
- The cost comes from the runner, not from an invoice.

## Spend

The probes cost 40.71 USD and the 14 jobs of the batch cost 461.40 USD, so the protocol cost 502.11 USD of the 700 USD cap. The dry run and the self-test cost nothing, because they use the stub agent. The pilot of protocol 1 is on [its page](team-measurement.md).
