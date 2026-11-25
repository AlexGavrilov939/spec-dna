---
status: approved
---
# `transcript --cost` gives the USD of a change, and `outline` gives the line ranges of the definitions of a code file: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It adds the mode `--cost` of `transcript` with the options `--change` and `--prices`, the module `src/prices.rs` with the price table, the command `outline`, and one sentence on `outline` in the adapter skill and in `Explore` of `standard` and `strict`. Items 21 and 22 of the [design](../../design/0.8.1-harness-additions.md) state the contract, with the JSON of each command. [ADR 0004](../../adr/0004-harness.md) holds the amendment of 2026-11-25 for the price table. The plan gives each agent-facing text in full, with its bytes, and the text output of `--cost`.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0003-distribution]]
- [[adr/0004-harness]]
- [[adr/0032-acceptance-in-one-pass]]

## Concerns

- [x] The facts that the build confirms on a real file (section 2 of the design): the usage of a request with an advisor call has no tokens of the advisor model, the usage splits the cache writes into a 5-minute part and a 1-hour part (E7), and one field of a transcript gives the type of a subagent. Resolved: tasks t1a, t1b and t1c confirm the 3 facts on transcripts of the lab, with a script outside the repository that prints key names and counts only. The fixtures of the repository are synthetic. For each other result, each task names the change, inside the files of the plan.
- [x] `lite` keeps the bytes of each generated file (ADR 0032), but `adapters::skill_file` and `adapters::check` take no profile, and `harness_text::agent_body` freezes only `code-simplifier`. Resolved: the adapter skill gets the profile as an input, and `lite` gets the body of 0.7.1. That body is the current text: the golden file `tests/golden/harness/skills/spec-dna.md` has not changed since 0.7.0. `Explore` of `lite` gets a frozen body, as `code-simplifier` has. The callers in `src/init.rs`, `src/main.rs` and `src/upgrade.rs` pass the profile of the configuration.
- [x] The path rule of `outline`. The design follows the rule of `context` (E14), which refuses an absolute path. The read tool of an agent gives absolute paths. Resolved: an absolute path below the root of the repository counts as its relative path. Each other rule of E14 stays: a path with `..`, a path outside the root and a path that names no file give exit code 5. A relative path starts at the root of the repository. A folder that is in no git repository gives exit code 3, as for each command.
- [x] The design gives no exit code for `--change` with a slug that names no change. Resolved: a slug that is not a valid name of a change gives exit code 2. A slug that names no change folder of the head tree, no archive file under `.spec-dna/accepted/` and no change that `spec-dna log` lists gives exit code 5. `--change` and `--prices` without `--cost` give exit code 2, and so does a `--prices` file that is not valid.
- [x] The design gives the JSON of `--cost` and no text output. Resolved: the plan gives the text output, and the JSON of the design is the contract of `--json`. `prices.source` is `built-in` or `file`. `prices.date` is the date of the table in the binary, or the key `date` of the file (else `null`). The rows of the binary that the file does not replace still apply.
- [x] The tie of a session to a change reads `spec-dna log` (design, item 21, step 6). Resolved: the command reads the finished changes with the function of `log` (`sdlc::ended_changes`) and the head tree, and it runs no other git command. Without a configuration, each change is `null`.
- [x] `outline` joins the list of the spec-dna commands that `transcript` counts. Resolved: a session that ran `outline` before 0.8.1 counted it as `other`, and now it counts as `outline`. The read report and its golden files change only for a transcript that has such a call.
- [x] The price table names the price of each model id of the table of model ids of the release. Resolved: a model with no public price page gets no row, so its USD is `null` and the total has the mark `partial`. The owner accepts the rows in the release pull request (amendment of ADR 0004).
- [x] The texts of `Explore` and of the adapter skill change, and the freeze table does not list them. Resolved: the paid reruns of `execute` and `review` in this part start after the merge of this change, so they measure the new texts.

## Open questions

- [x] Q10 to Q16 of the design. Answer of the owner (word of 2026-11-25, the recommended option of each): Q10 (a), `transcript --cost`. Q11 (a), the table in the binary and `--prices <file>`. Q12 (a), the slugs of the episodes and the sessions of the records, and (c) when a session trailer exists. Q13 (b), the calls and a lower limit. Q14 (a), line rules. Q15 (a), the 5 families. Q16 (a), the adapter skill and `Explore`.
