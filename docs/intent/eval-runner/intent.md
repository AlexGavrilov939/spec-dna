---
status: draft
class: higher-risk
---
# The eval runner counts each segment, runs one arm alone, and keeps a floor of free space

## Intent

`spec-dna eval run` reads the whole event stream of `claude`, and `turns` is the sum of `num_turns` over the `result` events of a run (design of lifecycle and harness, D95). Each result gets the new key `segments`. `--arms` takes 1 to 3 arms, and `eval verdict --compare` joins the result files of one arm each. Before each run, the runner checks the free space of the temporary folder and of the folder of the result against the new key `[evals] min-free-mib`, and it stops with `disk-space` and exit code 3 below the floor (D100).

## Why now

In protocol 2 of the model-team measurement, the runner recorded 2 turns for a run of 152 turns, and the recorded turns of 28 of 45 runs differ from the count of the transcript (issue 109). The arms of one task could not run at the same time, as the owner asked. In wave 1, the disk was full for a time: 4 runs lost their transcripts, and 1 run lost the copy of its hidden tests.

## Out of scope

- A real run of `claude`. Each test uses a stub `claude` and a stub `df`, and the change spends nothing. The repro run of Q6 of the design is a separate decision of the owner.
- A new key of the records of `eval records`: the records keep their 12 keys.
- The measurement of the acceptor as a subagent (D96), and the default and the rule of the advisor (D97, D98).
- A check of the free space during a run.
