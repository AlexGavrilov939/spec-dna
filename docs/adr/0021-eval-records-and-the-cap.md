---
status: accepted
---

# 0021. Eval records, harder evals and the cap of a run

## Context and Problem Statement

[ADR 0016](0016-skill-evals.md) measures each skill against no skill. The pilot of 0.7.0 part A had 200 runs. The owner review of part A and the plan of the full run found 4 gaps:

- The result files of a run stay outside the repository, so a reader cannot check the table of the release.
- Both arms of the pilot passed each run of `verify-1` to `verify-5`, and of `verify-1` and `review-1` in the pair `agents-line`. Such an eval cannot show a gain.
- `eval run --ab` stops before the first run when its estimate is above `max-total-usd`. The worst case of the 35 evals that did not run is 500 US dollars, against about 403 US dollars of budget. Thus no set of caps can start the plan.
- A side of a result has no tokens, no models and no names of its checks.

Constraints:

- A record has no path, no text, no time and no secret (rule zero).
- The runner of [ADR 0008](0008-harness-and-evals.md) and the bootstrap of ADR 0016 stay.
- ADR 0016 is near its hard budget. Thus this ADR holds the new rules.

## Considered Options

1. Commit the raw result files.
2. Commit one summary record for each run. The binary holds the table of the release as a constant, and a test compares it with the records.
3. Embed the records in the binary, and compute the table at run time.

## Decision Outcome

Option 2.

- Each run of a release has one record: one JSON object on one line, with the 9 keys `skill`, `eval`, `run`, `arm`, `pass`, `cost`, `tokens`, `models` and `claude_code`, in this order. No record has a path, a text or a time. The records of one result file are one file `bench/results/evals/<stamp>-<target>.jsonl`.
- `spec-dna eval records <result file>` writes the records to stdout. It builds each record from the 9 values and copies no object of the input. A value outside its form stops the import, and the message names the key, not the value. It computes each item of the file again from the records, and a difference stops the import.
- The binary holds the table of the release as a constant. A test reads the committed records, computes the table with the same functions, and fails when the 2 differ. `eval verdict --release` prints the table.
- For each name, the newest batch that has records of the name decides. Thus a confirmation must hold alone.
- `eval run --ab` starts when its estimate is above `max-total-usd`, and it stops before a run that could pass the cap. `--resume` goes on, and it takes a higher `--runs`. Thus a hard total budget can cover a plan of many targets. `eval run` without `--ab` keeps the refusal of ADR 0008.
- Each side of a result of `--ab` has `tokens`, `models` and `checks`: the name and the result of each check, with no command and no output. The records do not copy `checks`.
- `verify-6` to `verify-10` replace `verify-1` to `verify-5`. Their tasks do not tell the agent to run the tests.
- The key `skill` of an eval file also accepts `agents-line`. The pair `agents-line` runs `tdd-1`, `debug-1`, `execute-1`, `agents-line-1` and `agents-line-2`.

### Rejected

- Option 1: a raw file has fields that can hold a path or the text of an error.
- Option 3: about 140 KB more in the binary, 2,000 resamples for each name in `eval verdict` and in SD507 on the path of `check`, and a test that compares the code with itself.
- A script in another language for the import: a second implementation of the bootstrap, or no self-check.
- A deny list on the raw file: it misses a new field.
- Pool the pilot of `tdd` and its confirmation: it mixes 2 binaries, and a weak confirmation rides on the strong pilot.

### Consequences

- Good: a reader can check the table of the release against the committed records.
- Good: a project can commit a short public summary of its own runs with `eval records`.
- Bad: a run that the cap stops in the middle has no verdict, and `eval records` refuses its file until `--resume` completes it.
- Neutral: the records add about 140 KB to the repository, and nothing to the binary or the package.

## Amends

ADR 0016 is accepted and locked, so this ADR does not edit it. These rules apply in place of the named text:

- ADR 0016, "The set and the arms": the key `skill` also accepts `agents-line`, and the pack has 57 evals.
- ADR 0016, "Cost": `max-total-usd` does not stop `eval run --ab` before its first run. It stops the run before a run that could pass the cap.
- ADR 0016, "Commands and SD507", and its amendment of 2026-11-16: `eval records` and `eval verdict --release` are new. The table of the release comes from the committed records, not from the committed result files.

## Amendments
