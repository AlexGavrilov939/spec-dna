---
status: accepted
class: higher-risk
---
# Eval records, the release table and harder evals for the full run

## Intent

Build items 5 and 4 of part B of 0.7.0, as the section 15 bullets of [the design](../../design/0.7.0.md) and [ADR 0021](../../adr/0021-eval-records-and-the-cap.md) state them. A reader can then check each verdict of the release in a committed file, and the full run can measure `verify` and the line of `AGENTS.md`.

- Item 5: one record for each run in `bench/results/evals/<stamp>-<target>.jsonl`, with 9 keys: the skill, the eval, the run, the arm, the pass, the cost, the tokens, the models and the Claude Code version. `spec-dna eval records <file>` writes them from a result file of `--ab`, with an allowlist and a self-check. The binary holds the table of the release as a constant, a test compares it with the records, and `eval verdict --release` prints it.
- Item 5: the runner keeps the tokens, the models and the check names of each run. `--resume` takes a higher `--runs`, so a smoke run is run 1 of its batch. `--ab` starts below its estimate and stops at the cap, so a hard total budget can cover the plan.
- Item 4: `verify-6` to `verify-10` replace `verify-1` to `verify-5`, with a `Proof` that the task does not name. The pair `agents-line` runs `tdd-1`, `debug-1`, `execute-1`, `agents-line-1` and `agents-line-2`, and the key `skill` accepts `agents-line`.

## Why now

The release needs verdicts that a reader can check, and the result files of the pilot are outside the repository. Both arms of the pilot passed each run of `verify`, so the old evals cannot show a gain. The runner refuses a cap below the worst case, and the worst case of the 35 evals alone is above the budget that is left.

## Out of scope

- The full eval run itself, and the import of its result files. The main session runs them after this change, and the records go into the release preparation.
- A change of the rules of the verdict (D33), of the bootstrap or of the arms.
- A change of the text of the 11 skills, of the line of `AGENTS.md`, or of the evals of the other skills.
- The `hours-line` fixtures of `finish-1` and `finish-3` (the change `tasks-hours` updates them).
