---
status: draft
---
# Evals of each skill against no skill: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the eval pack, the commands `eval pack` and `eval verdict`, and the option `--ab` of `eval run`. It also adds the key `skill` of an eval file and the code SD507. The rules are in [the design](../../design/0.7.0.md), D32 to D36, and the docs of the build state them.

## Interfaces

None. No interface document describes the CLI or the eval files. The new surface is this.

`spec-dna eval pack` writes `<[evals] dir>/pack/<id>.json` and `<[evals] dir>/pack/fixtures/<id>/` for the 55 evals. The ids are `<skill>-1` to `<skill>-5`. It keeps each file that exists, and the report names it. `spec-dna eval run --ab <target>...` takes one or more targets: a skill name, `all` (the 11 skills) or `agents-line`. It accepts `--runs`, `--max-total-usd`, `--out`, `--dry-run`, `--isolate` and `--json`. `--min-pass-rate` with `--ab` is a usage error. `spec-dna eval verdict` prints one line for each skill and for `agents-line`. A line has the name, the verdict (`on`, `off`, `inconclusive` or `unmeasured`) and the source (`project <file>`, `release` or `-`). It takes `--json`.

The result of `--ab` is `<dir>/skills-<YYYYMMDD>T<HHMMSS>Z.json`, where `<dir>` is `<[evals] dir>/results` or the value of `--out`. Its keys are `spec_dna`, `claude_code`, `models`, `started`, `targets`, `seed`, `resamples`, `max_total_usd`, `estimate_usd`, `spend_usd`, `excluded`, `stopped` and `skills`. It has no key `pass_rate` at the top level. Each item of `skills` has `name`, `evals`, `with`, `without`, `difference`, `cost_ratio`, `verdict`, `rule` and `pairs`. An arm has `runs`, `passes`, `pass_rate`, `cost_usd` and `cost_per_correct_usd`. An interval has `value`, `low` and `high`, and `cost_ratio` is `null` when it is undefined. A pair has `eval`, `run`, `with` and `without`, and each holds `claude_exit`, `subtype`, `cost_usd`, `excluded` and `passed`. The result of `eval run` without `--ab` gets the key `excluded` at the top level and in each eval. `stopped` can also be `"api-error"`.

## Decisions

- [[adr/0016-skill-evals]]
- [[adr/0008-harness-and-evals]]
- [[adr/0011-entry-points]]

## Concerns

- [x] What signal marks a run that the API refused? Resolved, from the lab harness of an earlier session. An attempt is excluded when 3 conditions are true. First, `claude` exits with a code other than 0, or the result object has `"is_error": true`. Second, the `result` text has fewer than 600 bytes, so a long answer that names a status does not count. Third, the text has the status 401, 403, 429 or 500 to 599 as a separate word, or a phrase of this list, in any case: "rate limit", "usage limit", "overloaded", "Failed to authenticate", "Attention Required", "Cloudflare". With no result object, the runner reads the stdout of `claude` as the text. The build confirms the form with one real failed output, and the fake `claude` gives the same form.
- [x] What does an excluded attempt cost? Resolved: the reported cost, or 0 when `claude` reports none. Today an absent cost counts `max-budget-usd`, and that rule stays for a counted run. Thus a refused request does not trip the cap. The spend still includes the cost of each excluded attempt.
- [x] Does a retry break a pair? Resolved: the retry repeats the same eval, arm and run number in a new temporary folder. The result counts only the last attempt, and `excluded` counts the others. The result keeps no output text.
- [x] How long is the back-off? Resolved: the waits are 60, 120, 240, 480, 960 and 1,920 seconds. After the sixth retry, the command stops: `stopped` is `"api-error"`, the exit code is 3, and the file has no verdict for an incomplete target. The variable `SPEC_DNA_EVAL_BACKOFF` sets the first wait in seconds, from 0 to 3,600, so a test does not wait.
- [x] ADR 0016 says that `eval run` without `--ab` keeps the behavior of ADR 0008. Resolved: the rule of the owner applies to each run, because both modes share the function of one run. This is a deviation, and `architect` writes it into section 15 of the design in this change. The owner accepts it.
- [x] `stats --sdlc` reads each `*.json` file in `<[evals] dir>/results`. Resolved: the result of `--ab` has no `pass_rate` at the top level, so `stats --sdlc` does not count it as complete. A test of `tests/stats_evals.rs` proves it.
- [x] The arms differ in generated files, so `spec-dna check` in the copy can give SD501 in one arm only. Resolved: a check of the pack names the codes that it reads. It never uses the exit code of `spec-dna check` alone. A test of `tests/eval_pack.rs` proves it for each eval.
- [x] A fixture pins a version, and a later binary gives exit code 3 in each run. Resolved: in the copy of a fixture, the runner writes the version of the binary into the pin line of `spec-dna.toml`.
- [x] If each run has the same cost, a gain of the pass rate also lowers the cost per correct outcome. Then rule 2 can give `on` where rule 1 does not. Resolved: the simulation reports the share of rule 1, of rule 2 and of `on`. The build records the 3 shares in `docs/evals.md`, and it reports a share of `on` above 0.36 to `architect`.
- [x] D35 reads only the newest project result file, and Q3 reuses the 160 runs of the pilot in the full run. Resolved: D35 stays as it is. One `eval run --ab` with several targets writes one file. The run step makes the table of the release from the committed results of the pilot and the full run.

## Open questions

- [x] Which 5 evals does the pair of the line of `AGENTS.md` use? Answer: the first eval of each skill that a discipline names: `tdd-1`, `debug-1`, `verify-1`, `review-1` and `execute-1`.
- [x] What differs between the 2 arms of `agents-line`? Answer: both arms have no `.claude/skills/` folder, because the line serves an agent with no skill folder. `with` has the line in `AGENTS.md` and in `CLAUDE.md`, as `init` writes it. `without` has no line and denies `spec-dna skill`.
- [x] How does the arm `without` deny the command? Answer: it adds `--disallowedTools` with `Bash(spec-dna skill:*)` after the place of `--allowedTools`. The build confirms the form of the rule in the Claude Code docs, and a test fixes the order of the arguments.
- [x] How can a reader repeat the interval? Answer: the generator is SplitMix64 with the seed 24, and an index is the next value modulo the count. The 90% interval is the 100th and the 1,900th value of the 2,000 sorted values.
- [x] In which order do the arms run? Answer: for each eval and run number, `with` runs first for an odd number and `without` runs first for an even number.
- [x] Which model does the file record? Answer: `models` lists the sorted keys of `modelUsage` of each result object of `claude`. It holds names only.
- [x] What are the exit codes? Answer: `eval run --ab` gives 0 for a complete run and 2 for a usage error. It gives 3 for a stop by the cap or by `api-error`, and 4 for a git or file error. `eval pack` gives 0, 3 with no `[evals]` table, and 4 for a write error. `eval verdict` gives 0, also with no `[evals]` table, and then it prints the table of the release. It gives 3 for a configuration error.
- [x] Where does SD507 read the verdict? Answer: from the same function as `eval verdict`, in the tree that `check` reads. The newest complete `skills-*.json` file of `<[evals] dir>/results` comes first, then the table of the release.
