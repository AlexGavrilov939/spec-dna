---
status: draft
---
# Approval in the branch: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the rule of SD910, the output of `check --json`, `next` and `status`, and the keys of `[change]`. The rules are in [the design](../../design/0.7.0.md), D7, D8, D11 to D17 and section 15, and the docs of the build state them.

## Interfaces

None. No interface document describes the rule of SD910, the keys or the JSON output.

## Decisions

- [[adr/0006-change-lifecycle]]
- [[adr/0011-entry-points]]
- [[adr/0012-approval-in-the-branch]]
- [[adr/0017-profiles]]

## Concerns

- [x] The e-mail of an author is not a proof (D12). Resolved: ADR 0012 states the limit, the code host stays the control, and the rule of Q5 tells an agent when it can write an approval commit.
- [x] Item 1 also changes SD910 with `approval = "base"`. Resolved: the amendments of 2026-11-16 of ADR 0006, ADR 0012 and ADR 0014 state the rule. The 3 tests of `tests/lifecycle_base.rs` that write a `draft` artifact and expect SD910 write it `approved` after the change.
- [x] `next --json` and `status --json` can give `class: null`, so a reader that expects a string breaks. Resolved: `architect` accepts the change of the `--json` shape, and `CHANGELOG.md` names it.
- [x] A rebase can reorder the commits, and the rule reads their order, not their time (ADR 0012, "Consequences"). Resolved: a test of t3 fixes the result of a reordered branch.

## Open questions

- [x] How does `status` show the source of an approval (D8)? Answer: `status --json` gives each change the object `approved_in`, with the keys `intent`, `spec` and `plan`. A value is `"base"`, `"branch <short sha>"` or `null` (no approval counts). The short sha has 12 characters, as in the rest of `status`. With `approval = "branch"`, the text adds `(base)` or `(branch <short sha>)` after each `accepted` or `approved` status. With `approval = "base"`, the text does not change.
- [x] How does `status` name the skill `post-mortem` (D8)? Answer: the text line of an open follow-up ends with `[skill: post-mortem]`, and each item of `follow_ups` in `--json` gets `"skill": "post-mortem"`.
- [x] What does `init` write when git has no `user.email`? Answer: `approval = "base"` and no `approvers`, and it prints a note that names the 2 keys to set. An empty `approvers` with `approval = "branch"` is exit code 3 (D15), so `init` never writes that pair.
- [x] Is `approvals` of `check --json` always in the output? Answer: yes, as `locked_links`. It is an empty list with `approval = "base"`. Each item has `commit`, `path` and `author`.
- [x] Does the `fix` text of SD910 change with the reason (D17)? Answer: the message names the reason and ends with its fix. The `fix` field of the code gives the short fix of each of the 8 reasons.
- [x] What is "no class" for item 2? Answer: a `class` field that is absent, empty or invalid. `check` still reads it as `routine` (SD910, SD917). `next` and `status` give `class: null`, the token `none` in the text of `status`, and the stage `plan` for an accepted intent. The last line asks for the class before the approval, because an approval commit changes the status line only.
