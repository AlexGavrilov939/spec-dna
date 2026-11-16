---
status: draft
---
# Approval in the branch: plan

## Files

- `tests/branch_approval.rs` The integration tests of the branch rule: the 5 lab variants and the other cases of the test plan.
- `tests/common/mod.rs` A helper that commits with a given author e-mail.
- `tests/change_config.rs` The values and the errors of the keys `approval` and `approvers`.
- `tests/lifecycle_base.rs` The test of item 1, and the 3 tests whose tested artifact becomes `approved`.
- `tests/lifecycle_commands.rs` The tests of item 2 and of `post-mortem` in `status`, and the 2 tests of the last line of a change with no plan.
- `tests/init_change.rs` The keys that `init` writes for each profile, and the case with no `user.email`.
- `tests/golden/check-all.json` The field `approvals`.
- `src/config.rs` The keys `approval` and `approvers`, and the errors of D14 and D15.
- `schema/spec-dna.schema.json` The 2 keys.
- `src/git.rs` The walk of the commits of the branch, and the read of `user.email`.
- `src/lifecycle.rs` SD910: the draft rule, the walk of D11, D13 and D16, and the reasons of D17.
- `src/diag.rs` The fix text of SD910.
- `src/main.rs` The field `approvals` of `check --json`.
- `src/model.rs` The flag of an intent with no class.
- `src/change.rs` The class flag of a change.
- `src/stage.rs` `next` and `status`: the class, the approval line, D7, D8 and the skill of a follow-up.
- `src/init.rs` `init` writes the keys of D15.
- `docs/config.md` The 2 keys.
- `docs/init.md` The keys that `init` writes.
- `docs/diagnostics.md` SD910.
- `docs/profiles.md` `strict` reads the approvals of the base only.
- `docs/commands.md` `check --json`, `next` and `status`.
- `docs/guides/change.md` The approval commit, the class first, and `fetch-depth: 0`.
- `docs/glossary.md` The terms "approval commit" and "approver".
- `AGENTS.md` The rule of Q5.
- `CONTRIBUTING.md` The section "Changes": the approval commit and the rule of Q5.
- `.claude/agents/ownership.md` The owner of `tests/branch_approval.rs`.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Write the helper that commits with an author e-mail, and one failing test for each lab variant: `ok`, `late`, `stranger`, `mixed` and `backdoor` (`test-engineer`, `diagnostics-engineer`). Files: `tests/branch_approval.rs`, `tests/common/mod.rs`. Verify: `cargo test --test branch_approval` fails on each variant.
- [ ] t2: Write the failing tests of the stops of D13 and of D16 (`diagnostics-engineer`). The cases are a shallow clone, a merge of the base into the branch, a test merge of a host, a merge queue commit, and a change added and finished in one branch (reason 7). Files: `tests/branch_approval.rs`. Verify: `cargo test --test branch_approval` fails on each new case.
- [ ] t3: Write the failing tests of the other cases of an approval commit (`diagnostics-engineer`). The cases are an approver commit that edits another file, the case of an e-mail and a `noreply` e-mail. More cases are a path added to `Files` after the approval, a status set back to `draft`, and a rebase that reorders the commits. Files: `tests/branch_approval.rs`. Verify: `cargo test --test branch_approval` fails on each new case.
- [ ] t4: Write the failing tests of the output in branch mode (`diagnostics-engineer`, `core-engineer`). The cases are the line of D7 in `next`, the source of D8 (`base` or `branch <short sha>`) in the text of `status` and in `approved_in`, and the items of `approvals` in `check --json`. Add the squash replays of a higher-risk change and of a routine change. Files: `tests/branch_approval.rs`. Verify: `cargo test --test branch_approval` fails on each new case.
- [ ] t5: Write the failing tests of the keys: an empty `approvers` and `strict` with `branch` give exit code 3 (`core-engineer`). Files: `tests/change_config.rs`. Verify: `cargo test --test change_config` fails on each new case.
- [ ] t6: Write the failing tests of items 1 and 2 and of the skill `post-mortem` in `status`, and change the 5 tests that assert the old rule (`diagnostics-engineer`, `core-engineer`). Files: `tests/lifecycle_base.rs`, `tests/lifecycle_commands.rs`. Verify: `cargo test --test lifecycle_base --test lifecycle_commands` fails on these tests only.
- [ ] t7: Write the failing tests of the keys that `init` writes for each profile, also with no `user.email` (`init-engineer`). Files: `tests/init_change.rs`. Verify: `cargo test --test init_change` fails on the new tests only.
- [ ] t8: Add the keys `approval` and `approvers` with the errors of D14 and D15 (`core-engineer`). Files: `src/config.rs`, `schema/spec-dna.schema.json`, `docs/config.md`. Verify: `cargo test --test change_config` passes.
- [ ] t9: Add the walk of `base..HEAD` to git access (`core-engineer`): the commits from the oldest, with the parents, the author e-mail, the changed paths and the shallow state. Files: `src/git.rs`. Verify: `cargo test --bin spec-dna git::` passes.
- [ ] t10: Apply the draft rule of item 1 to SD910 (`diagnostics-engineer`). Files: `src/lifecycle.rs`. Verify: `cargo test --test lifecycle_base` passes.
- [ ] t11: Add the rule of D11, the stops of D13, the case of D16 and the 8 reasons of D17 (`diagnostics-engineer`). Files: `src/lifecycle.rs`, `src/diag.rs`. Verify: the tests of t1 to t3 pass.
- [ ] t12: Add the field `approvals` to `check --json` (`diagnostics-engineer`). Files: `src/main.rs`, `tests/golden/check-all.json`. Verify: `cargo test --test cli` passes.
- [ ] t13: Give `next` and `status` the class of item 2, the approval line only for a file that exists, the line of D7 and the source of D8 (`core-engineer`). Add `[skill: post-mortem]` to each open follow-up. Files: `src/model.rs`, `src/change.rs`, `src/stage.rs`. Verify: `cargo test --test lifecycle_commands --test branch_approval` passes.
- [ ] t14: Make `init` write the keys of D15 (`init-engineer`; `core-engineer` for the read of `user.email` in git access). With no `user.email`, `init` writes `approval = "base"` and prints a note. Files: `src/init.rs`, `src/git.rs`, `docs/init.md`. Verify: `cargo test --test init_change` passes.
- [ ] t15: Update the docs of SD910, the profiles and the commands. Files: `docs/diagnostics.md`, `docs/profiles.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t16: Update the guide, the glossary and the change log. Files: `docs/guides/change.md`, `docs/glossary.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t17: Write the rule of Q5 and the owner of the new test file (`architect`; the owner accepts the conventions). Files: `AGENTS.md`, `CONTRIBUTING.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- Merge order: this change merges first. `task-checks` then rebases on it, because both edit `src/lifecycle.rs`, `src/stage.rs` and `tests/lifecycle_commands.rs`.

- The walk needs the full history. A shallow clone gives the stricter result of 0.6, never a false pass (D13). The CI of this repository and the workflow of `init` use `fetch-depth: 0`.
- An agent that commits with the e-mail of its owner meets the condition of D11. The host stays the control, and the rule of Q5 limits the agent.
- Item 1 removes findings also with `approval = "base"`. A project that used SD910 to stop a draft loses that result. The amendments of 2026-11-16 state it.
- The text of `next` and `status` changes, so the demo and the examples can need new output.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t7 failed before its task. The squash replay with the real binary gives 2 pull requests for a routine change and for a higher-risk change with `gate = "block"`. `spec-dna check --base origin/master` gives no error on this repository.
