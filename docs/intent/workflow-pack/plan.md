---
status: draft
---
# Workflow pack of 11 skills: plan

## Files

- `tests/harness_skills.rs` The tests of the 11 skills, the budgets, the old names, the level `full`, the note of D5 and SD503.
- `tests/skill_cmd.rs` The tests of `skill list` and `skill show`.
- `tests/upgrade_cli.rs` The test of the rename of the 0.6 names by `upgrade`.
- `tests/init_evals.rs` The new skill names in the cases and the answers.
- `tests/harness_check_step4.rs` The new skill names in the cases.
- `tests/generated_lint.rs` The new skill names in the answers.
- `tests/cli.rs` The new skill names in the output of `init`.
- `tests/config_step4.rs` The new skill names, and the message of an old name.
- `src/config.rs` The new skill names in the unit tests.
- `schema/spec-dna.schema.json` The 11 names of `[harness] skills`.
- `src/harness.rs` The 11 skills with kind, stage and default, the old paths, and the removal of an old skill file.
- `src/harness_text.rs` The descriptions and the bodies of the 11 skills.
- `src/adapters.rs` The discipline block, the words of D5 and the line of `AGENTS.md`.
- `src/init.rs` The skills of the levels `lean` and `full`, and the note of D5.
- `src/main.rs` The command `skill list|show`.
- `src/upgrade_config.rs` The rename of the 0.6 names in `[harness] skills`.
- `src/upgrade.rs` The rename before the first parse of the file in `source_of`.
- `tests/golden/harness/skills/spec-dna-brainstorm.md` The golden file of `brainstorm`.
- `tests/golden/harness/skills/spec-dna-specify.md` The golden file of `specify`.
- `tests/golden/harness/skills/spec-dna-clarify.md` The golden file of `clarify`.
- `tests/golden/harness/skills/spec-dna-write-plan.md` The golden file of `write-plan`.
- `tests/golden/harness/skills/spec-dna-execute.md` The golden file of `execute`.
- `tests/golden/harness/skills/spec-dna-tdd.md` The golden file of `tdd`.
- `tests/golden/harness/skills/spec-dna-review.md` The golden file of `review`, in the 5 parts.
- `tests/golden/harness/skills/spec-dna-verify.md` The golden file of `verify`.
- `tests/golden/harness/skills/spec-dna-finish.md` The golden file of `finish`.
- `tests/golden/harness/skills/spec-dna-debug.md` The golden file of `debug`.
- `tests/golden/harness/skills/spec-dna-post-mortem.md` The golden file of `post-mortem`.
- `tests/golden/harness/skills/spec-dna-intent.md` Deleted: the 0.6 name ends.
- `tests/golden/harness/skills/spec-dna-spec.md` Deleted: the 0.6 name ends.
- `tests/golden/harness/skills/spec-dna-plan.md` Deleted: the 0.6 name ends.
- `tests/golden/harness/skills/spec-dna.md` The adapter skill with the discipline block.
- `tests/golden/harness/model-team.md` The team page lists the new skill names.
- `scripts/demo-replay.sh` The new skill paths of the demo.
- `docs/harness.md` The 11 skills, the disciplines, the budgets and D5.
- `docs/init.md` The skills of each level, the removal of an old skill file and the note of D5.
- `docs/agents.md` The longer line of `AGENTS.md`.
- `docs/commands.md` The section of `skill`, and the summary.
- `docs/config.md` The 11 names of `[harness] skills`.
- `docs/quickstart.md` The line of `AGENTS.md` and the stage skills.
- `docs/diagnostics.md` The list "Files that SD503 counts" and the paths of SD504.
- `docs/glossary.md` The terms workflow pack, stage skill, event skill and discipline.
- `CHANGELOG.md` The entry of the change.
- `.claude/agents/ownership.md` The owner of `tests/skill_cmd.rs`.

## Order

- [ ] t1: Write the tests of the 11 skills that fail before the code (`init-engineer`). They cover the names, the descriptions, the 5 parts, the byte limits of D6 and the golden files. They also cover the Q5 words, an old name, the level `full` and the note of D5. A pin change with the new and the old skill paths gives no SD503. Files: `tests/harness_skills.rs`. Verify: `cargo test --test harness_skills` fails on each new case.
- [ ] t2: Write the tests that fail before the code for `skill list` and `skill show`, with `--json`, exit code 2 for an unknown name, the body that `init` writes, and each name that `next` prints (`init-engineer`). Files: `tests/skill_cmd.rs`. Verify: `cargo test --test skill_cmd` fails on each case.
- [ ] t3: Write the test that fails before the code for the rename by `upgrade`, and of a second run that changes nothing (`init-engineer`). Files: `tests/upgrade_cli.rs`. Verify: the new case of `cargo test --test upgrade_cli` fails.
- [ ] t4: Change the old skill names to the new names in the current cases (`init-engineer`; `diagnostics-engineer` for the harness checks). Files: `tests/init_evals.rs`, `tests/harness_check_step4.rs`, `tests/generated_lint.rs`. Verify: the changed cases fail before t6.
- [ ] t5: Change the output of `init` in the cases of `init` to the new skill names (`init-engineer`). Files: `tests/cli.rs`. Verify: the changed cases fail before t6.
- [ ] t6: Replace `Skill` with the 11 skills, their kind, stage and default, and their 11 descriptions. Keep the 3 old paths for SD503. Give exit code 3 for an old name (`init-engineer`). Files: `src/harness.rs`, `src/harness_text.rs`. Verify: the cases of the descriptions and the listings in `cargo test --test harness_skills` pass.
- [ ] t7: Change the skill names in the schema and in the tests of the configuration (`core-engineer`). Files: `schema/spec-dna.schema.json`, `src/config.rs`, `tests/config_step4.rs`. Verify: `cargo test --lib --test config_step4` passes.
- [ ] t8: Write the bodies of `brainstorm` (the class before the approval) and `specify` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-brainstorm.md`, `tests/golden/harness/skills/spec-dna-specify.md`. Verify: the golden cases of the 2 skills pass.
- [ ] t9: Write the bodies of `clarify` (floor SD919, SD922 and SD925) and `write-plan` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-clarify.md`, `tests/golden/harness/skills/spec-dna-write-plan.md`. Verify: the golden cases of the 2 skills pass.
- [ ] t10: Write the bodies of `execute` and `tdd` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-execute.md`, `tests/golden/harness/skills/spec-dna-tdd.md`. Verify: the golden cases of the 2 skills pass.
- [ ] t11: Write the bodies of `review` and `verify` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-review.md`, `tests/golden/harness/skills/spec-dna-verify.md`. Verify: the golden cases of the 2 skills pass.
- [ ] t12: Write the bodies of `finish` (the `Hours:` line) and `debug` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-finish.md`, `tests/golden/harness/skills/spec-dna-debug.md`. Verify: the golden cases of the 2 skills pass.
- [ ] t13: Write the body of `post-mortem` (`init-engineer`). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-post-mortem.md`. Verify: the golden case of `post-mortem` passes.
- [ ] t14: Delete the 3 golden files of the 0.6 names (`init-engineer`). Files: `tests/golden/harness/skills/spec-dna-intent.md`, `tests/golden/harness/skills/spec-dna-spec.md`, `tests/golden/harness/skills/spec-dna-plan.md`. Verify: `cargo test --test harness_skills` passes.
- [ ] t15: Add the discipline block and the words of D5 to the adapter skill, and make the line of `AGENTS.md` longer (`init-engineer`). Files: `src/adapters.rs`, `tests/golden/harness/skills/spec-dna.md`. Verify: the cases of the block and the line in `cargo test --test harness_skills` pass.
- [ ] t16: Write the skills of `lean` and `full`, remove an unedited old skill file, and print the note of D5 (`init-engineer`). Files: `src/init.rs`, `src/harness.rs`, `tests/golden/harness/model-team.md`. Verify: `cargo test --test harness_skills --test init_evals --test harness_check_step4 --test generated_lint` passes.
- [ ] t17: Add the command `skill list|show` (`core-engineer` for the arguments and the help; `init-engineer` for the handler). Files: `src/main.rs`. Verify: `cargo test --test skill_cmd` passes.
- [ ] t18: Rename the 0.6 names in `upgrade` before the parse of the file (`core-engineer` for the rename; `init-engineer` for the call in `source_of`). Files: `src/upgrade_config.rs`, `src/upgrade.rs`. Verify: `cargo test --test upgrade_cli` passes.
- [ ] t19: Update the docs of the harness, `init` and the agents (`init-engineer`). Files: `docs/harness.md`, `docs/init.md`, `docs/agents.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t20: Update the docs of the commands, the configuration and the quickstart (`core-engineer` for the summary; `init-engineer` for the sections). Files: `docs/commands.md`, `docs/config.md`, `docs/quickstart.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t21: Update the list of SD503, the glossary and the demo (`diagnostics-engineer`; `architect` for the glossary; `init-engineer` for the demo). Files: `docs/diagnostics.md`, `docs/glossary.md`, `scripts/demo-replay.sh`. Verify: `pre-commit run --all-files` passes.
- [ ] t22: Add the entry of the change and the owner of the new test file (`init-engineer`; `architect` for the ownership; the owner accepts). Files: `CHANGELOG.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- The listing of the 7 skills that are on has 1,250 bytes or fewer. A long description breaks the limit, so each description is one short sentence.
- The line of `AGENTS.md` has 400 bytes or fewer with its marker, and a draft has 421 bytes. If the text cannot fit, the build stops and asks `architect`.
- A 0.6 project with an old name gets exit code 3 at each command until it runs `upgrade`. The message names the new names and `spec-dna upgrade`.
- A hand-edited old skill file stays, and `check` reports SD501. The person removes it.
- The skill `finish` names the line `Hours:` of slice 3b, which can merge later. The text is conditional, so it stays true in both orders.
- The skills name SD923, SD924 and SD925 as floors. Slice 3a adds them, so this change starts after slice 3a.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 to t5 failed before its task. `spec-dna skill show write-plan` prints the body of `.claude/skills/spec-dna-write-plan/SKILL.md` that `init` writes. `spec-dna check --base origin/master` with the tree binary gives no error on this repository.
