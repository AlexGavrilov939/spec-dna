---
status: approved
---
# A new project has no advisor, and the advisor is consulted only in a higher-risk change: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the default that `init` writes for the role `advisor` of a new configuration, the default line and the texts of the questions 12 and 13, the rule of `--reconfigure` for the role, and the note of `--reconfigure`. It adds one warning of `init` for the advisor with the policy `none`. It changes the consult sentence of the generated model-team line and the section "Advisor" of `model-team.md`. It adds no key, no code and no exit code. The [design](../../design/0.8.1-a-lifecycle-and-harness.md) states the contract in D97 and D98, and [ADR 0004](../../adr/0004-harness.md) holds the amendment of 2026-11-24.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0004-harness]]
- [[adr/0017-profiles]]

## Concerns

- [x] The texts of the skills and of `acceptor.md` of `standard` and `strict` are frozen for the skill evals, and a change of one of their hashes needs a paid rerun (`docs/evals.md`, section "The freeze of the skill texts for the reruns"). Resolved: this change edits none of them. It may change the model-team line of `CLAUDE.md`, `model-team.md` of `standard` and `strict`, `.claude/settings.json` and the presence of `fallback-advisor.md`, and the freeze table lists none of these. The line of `acceptor.md` on the advisor follows the key `advisor` of the entry `acceptor`, not the role, so its bytes do not change. The texts of `lite` stay the texts of 0.7.1.
- [x] The reruns of 0.8.0 ran with `--harness lean`, and the arm `lean` gets the new consult sentence. Resolved: the freeze table does not list the model-team line, so the rule asks for no rerun. The page of the evals says that a run of the arm `lean` in 0.8.1 and a run of 0.8.0 do not have the same line.
- [x] Each test that sets up a new project of `standard` with `init` and reads the advisor fails when the preset changes. Resolved: the plan first gives such a test an explicit answer of the advisor, so each step keeps the tree green. The tests of a new project follow the new default.
- [x] With the policy `none`, no change has a class, so the line asks for no consult. Resolved: `init` prints a warning when the advisor is on with that policy (D97 item 6).
- [x] The slice of the design does not list `tests/cli.rs`, `tests/harness.rs` and `tests/generated_lint.rs`. Resolved: they set up a new project with `init` and read the advisor, so the plan lists them.
- [x] The changes `lifecycle-fixes` and `eval-runner` edit `src/harness.rs`, `tests/harness_roles.rs`, `tests/eval_run.rs` and `docs/evals.md`. Resolved: the design orders this change after both, so it rebases on them.

## Open questions

- [x] With the advisor off, may the settings of the project also turn off an advisor of the user settings with `env.CLAUDE_CODE_DISABLE_ADVISOR_TOOL = "1"` (Q3 of the design)? The design recommends (a): no, the project writes no `advisorModel`, and the person keeps control of the own settings. The plan follows (a). Option (b) adds a key of the environment to the settings and one task. The owner answers before the approval of the spec. Answer of the owner (word of 2026-11-24, the recommendation of the design): (a), no.
