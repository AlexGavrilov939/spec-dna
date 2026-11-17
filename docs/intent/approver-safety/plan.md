---
status: approved
---
# Approver safety: plan

## Files

- `src/config.rs`
- `src/diag.rs`
- `src/check.rs`
- `src/init.rs`
- `src/main.rs`
- `src/approve.rs`
- `src/git.rs`
- `src/harness.rs`
- `src/hook.rs`
- `schema/spec-dna.schema.json`
- `tests/init_change.rs`
- `tests/branch_approval.rs`
- `tests/approve_cmd.rs`
- `tests/cli.rs`
- `tests/hook_approvals.rs`
- `tests/harness.rs`
- `tests/golden/harness/settings-strict.json`
- `docs/commands.md`
- `docs/config.md`
- `docs/init.md`
- `docs/quickstart.md`
- `docs/harness.md`
- `docs/diagnostics.md`
- `docs/profiles.md`
- `docs/guides/change.md`
- `docs/glossary.md`
- `docs/design/0.7.0.md`
- `docs/adr/0018-approval-by-a-person.md`
- `AGENTS.md`
- `CONTRIBUTING.md`
- `.claude/agents/ownership.md`
- `CHANGELOG.md`

## Order

- [x] t1: Write the failing tests of `init` and the approvers: an agent or bot e-mail, no terminal, `--approver`, the answers key, `strict` and `--reconfigure` (`init-engineer`). Files: `tests/init_change.rs`. Verify: `cargo test --test init_change` fails on the new cases.
- [x] t2: Write the failing tests of SD926 (`diagnostics-engineer`). Files: `tests/branch_approval.rs`. Verify: `cargo test --test branch_approval` fails on the new cases.
- [ ] t3: Write the failing tests of `approve` (`core-engineer`). Files: `tests/approve_cmd.rs`, `tests/cli.rs`. Verify: `cargo test --test approve_cmd` fails on each case.
- [ ] t4: Write the failing tests of the hook and its entry (`init-engineer`). Files: `tests/hook_approvals.rs`, `tests/harness.rs`, `tests/golden/harness/settings-strict.json`. Verify: `cargo test --test hook_approvals --test harness` fails on the new cases.
- [ ] t5: Add the list of agent addresses, the group of SD926 and the range texts (`core-engineer`). Files: `src/config.rs`, `schema/spec-dna.schema.json`, `docs/config.md`. Verify: `cargo test --bin spec-dna config::`.
- [ ] t6: Add SD926 to `check` (`diagnostics-engineer`). Files: `src/diag.rs`, `src/check.rs`. Verify: `cargo test --test branch_approval sd926`.
- [ ] t7: Add `--approver`, the answers key, the prompt and the notes (`init-engineer`; `core-engineer` for the flag). Files: `src/init.rs`, `src/main.rs`. Verify: `cargo test --test init_change`.
- [ ] t8: Add the command `approve` (`core-engineer`). Files: `src/approve.rs`, `src/git.rs`, `src/main.rs`. Verify: `cargo test --test approve_cmd --test cli`.
- [ ] t9: Add the key `protect-approvals` and the hook entry (`init-engineer`). Files: `src/harness.rs`, `src/init.rs`. Verify: `cargo test --test harness`.
- [ ] t10: Add the hook `protect-approvals` (`init-engineer`). Files: `src/hook.rs`. Verify: `cargo test --test hook_approvals`.
- [ ] t11: Update the commands (`core-engineer`; `init-engineer` for the hook). Files: `docs/commands.md`. Verify: `pre-commit run --all-files`.
- [ ] t12: Update the docs of `init`, the harness and the quickstart (`init-engineer`). Files: `docs/init.md`, `docs/harness.md`, `docs/quickstart.md`. Verify: `pre-commit run --all-files`.
- [ ] t13: Update the codes and the profiles (`diagnostics-engineer`). Files: `docs/diagnostics.md`, `docs/profiles.md`. Verify: `pre-commit run --all-files`.
- [ ] t14: Make the guide and the glossary agree with the default rule (`architect`; `docs-editor` for the language). Files: `docs/guides/change.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files`.
- [ ] t15: Update the conventions and the owners of the new files (`architect`; the owner accepts). Files: `AGENTS.md`, `CONTRIBUTING.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files`.
- [ ] t16: Write ADR 0018, which amends ADR 0006, ADR 0012 and ADR 0017 (`architect`; the owner accepts). Files: `docs/adr/0018-approval-by-a-person.md`. Verify: `cargo run --quiet -- check`.
- [ ] t17: Write section 15 and the entry of the change (`architect`, `release-engineer`). Files: `docs/design/0.7.0.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`.

## Risks

- `risks-of-part-a` also edits `src/diag.rs`, `tests/branch_approval.rs`, `docs/diagnostics.md` and the guide. Rebase after each merge.
- A script that passes an agent e-mail to `init --approver` stops with exit code 2.
- The hook can deny a commit while a person keeps a status edit in the working tree.
- A teammate with spec-dna before 0.7.0 gets no hook, because the command ends with `|| true`.

## Hours

- 1: t1 to t4 tests
- 1.5: t5 to t8 code
- 1.5: t9 and t10 hook
- 1: t11 to t17 docs

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t4 failed before its code. In a scratch repository, `spec-dna approve` makes a commit that `check` counts as an approval, and the hook denies the same act of an agent. `spec-dna check --base origin/master` gives no error on this repository.
