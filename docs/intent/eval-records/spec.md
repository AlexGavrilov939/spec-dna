---
status: draft
---
# Eval records, the release table and harder evals for the full run: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the command `eval records`, the option `eval verdict --release`, 3 keys of each side of a result of `--ab`, the records of the pilot and the table of the release. It changes the cap and `--resume` of `--ab`, the values of the key `skill` and the evals of the pack. The section 15 bullets of [the design](../../design/0.7.0.md) state the rules, and the docs of the build give them.

## Interfaces

None. No interface document describes the CLI, the eval files or the result files. The surface changes in this way.

A record is one line of compact JSON with the keys `skill`, `eval`, `run`, `arm`, `pass`, `cost`, `tokens`, `models` and `claude_code`, in this order. `tokens` is `null` or `{"in","out","cache_read","cache_write"}`. A pilot record has `"tokens": null` and the models of its file. `spec-dna eval records <result file>` prints the records of each item that has a verdict, in the order of the file. It gives exit code 2 for a usage error, and 3 for a file that is not a result of `--ab`, an item with no verdict, a value outside its form or a failed self-check. A message names the key, never the value.

`verdict::RELEASE` holds one row for each name: the newest batch of the records of that name gives the analysis. `eval verdict` keeps its output. `eval verdict --release [--json]` prints the table: the runs, the passes, the costs, the cost per correct outcome, the 2 intervals, the verdict, the rule, the batch, the models and the Claude Code version.

Each side of a result of `--ab` gets `tokens` (the sums over `modelUsage` of `inputTokens`, `outputTokens`, `cacheReadInputTokens` and `cacheCreationInputTokens`, or `null`), `models` and `checks` (each name with `passed`, no command). `--resume <file> --runs <n>` with `n` above the runs of the file adds runs. `--ab` with an estimate above `max-total-usd` prints a note and starts, and the check before each run stops it at the cap with exit code 3. `eval run` without `--ab` keeps the refusal.

The key `skill` of an eval file accepts the 11 skill keys and `agents-line`. The pack has 57 evals.

## Decisions

- [[adr/0016-skill-evals]]
- [[adr/0008-harness-and-evals]]

## Concerns

- [x] The worst case of the 35 evals ($500) is above the budget that is left ($403.43), so the refusal before the first run blocks the plan. Resolved: `--ab` starts below its estimate. Since `--resume` exists, a run that the cap stops loses no paid run.
- [x] A record could carry a path or a secret of a result file. Resolved: the importer builds each record from 9 values and never copies an object. A test feeds a file with a path, a key and a secret in each other field.
- [x] The bootstrap draws by index, so the order of the records changes the interval. Resolved: the records keep the order of the file, and the analysis groups them by eval in the order of first appearance.
- [x] `tdd-1` is in the batch of `tdd` and in the batch of `agents-line`. Resolved: `skill` is the name of the item, so the 2 groups never merge.
- [x] The records of the pilot have no tokens and one list of models for each file. Resolved: they keep `"tokens": null` and the list of their file, and the docs say so. The runner keeps both for each run before the full run.
- [x] Embedded records add about 140 KB to the binary and a bootstrap on the path of `check` for SD507. Resolved: the binary holds only the table, and a test compares it with the records on the disk.

## Open questions

- [x] Does a confirmation of `tdd` pool with the pilot? Answer: no. The newest batch of a name decides, so the confirmation must give `on` alone.
- [x] Does a smoke run count? Answer: yes. The smoke uses `--runs 1`, and `--resume --runs 4` makes it run 1 of the batch. A smoke that finds a defect is discarded, and its spend still counts.
- [x] Is `execute-1` dropped from the pair `agents-line`? Answer: no. It is the one eval where the line seems to hurt (0 of 4 against 4 of 4), so it stays, and `failure-analyst` reads its checks before the run.
