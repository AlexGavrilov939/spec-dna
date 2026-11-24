---
status: draft
---
# A new project has no advisor, and the advisor is consulted only in a higher-risk change: plan

## Files

- `docs/design/0.8.1-a-lifecycle-and-harness.md` The design of the default (D97) and of the rule of text (D98).
- `docs/adr/0004-harness.md` The amendment of 2026-11-24 (slice A10 of the design).
- `src/harness.rs` The consult sentence of the model-team line, the section Advisor of the team page, the warning for the policy none, and the test of the line.
- `src/init.rs` The preset of the role advisor, the rule of reconfigure, the texts of the questions 12 and 13, the default line, the note of reconfigure, and the warning.
- `tests/init_profiles.rs` The tests of the default of each profile, of reconfigure, of upgrade and of the warning.
- `tests/harness_roles.rs` The test of a project of 0.8.0 with the advisor, and the answer of the advisor in the tests of the agent files.
- `tests/generated_lint.rs` The answer of the advisor in the setup of the lint of the generated files.
- `tests/harness.rs` The tests of a new project that follow the new default, and the answer of the advisor in the tests that read its files.
- `tests/cli.rs` The files of a fresh project with no advisor.
- `tests/eval_run.rs` The guard of the arm lean with the advisor.
- `tests/golden/harness/model-team.md` The new section Advisor.
- `tests/golden/harness/model-team-no-advisor.md` The team page of a new project of standard (new).
- `tests/golden/harness/settings-standard.json` The settings of a new project of standard, with no advisorModel (new).
- `docs/init.md` The questions 12 and 13, their default, and the note of reconfigure.
- `docs/harness.md` The table of the roles and the levels.
- `docs/profiles.md` The row of the role advisor in the table of what init writes.
- `docs/config.md` The default of the role advisor that init writes.
- `docs/evals.md` The row lean of the table of the arms, and the line of the arm lean.
- `CLAUDE.md` The rule of the advisor in this repository.
- `.claude/agents/ownership.md` The 3 texts of the advisor in this repository.
- `.claude/agents/README.md` The 2 texts of the advisor and the row of fallback-advisor.
- `.claude/agents/fallback-advisor.md` The description.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Write the consult sentence of D98 in the model-team line where the role advisor is on, and the section Advisor of D98 in the team page of `standard` and `strict`. The page of `lite` keeps the text of 0.7.1. Change the test of the line and the golden file of the page, and add a test that `init` on a project of 0.8.0 with the advisor writes the new line. Files: `src/harness.rs`, `tests/golden/harness/model-team.md`, `tests/harness_roles.rs`. Verify: `cargo test --bin spec-dna harness::` and `cargo test --test harness_roles --test harness_skills`. Hours: 2.
- [ ] t2: Make `init` print the warning of D97 when the role advisor is on and the policy is none. Files: `src/harness.rs`, `src/init.rs`, `tests/init_profiles.rs`. Verify: `cargo test --test init_profiles` and `cargo test --bin spec-dna harness::`. Hours: 2.
- [ ] t3: Give the answer `harness-models = ["advisor=fable"]` to the 5 tests of the agent files and to the setup of the lint that the probe of this plan names. Each of them reads the file of `fallback-advisor` or the page with the advisor, so it states that it needs the advisor, and it passes before and after the new preset. Files: `tests/harness_roles.rs`, `tests/generated_lint.rs`. Verify: `cargo test --test harness_roles --test generated_lint`. Hours: 1.
- [ ] t4: Write the preset of D97: the role advisor of a new configuration is none, with the agent opus and the effort xhigh, in `lite` and `standard`, and fable in `strict`. An answer of question 13 wins, and `init` with no `--reconfigure` on an existing file applies no preset. Change the texts of the questions 12 and 13 and the default line. In the 7 tests of the harness that the probe names, a test of the output of a new project follows the new default, and a test that reads the files of the advisor gets the answer of t3. The test of a fresh project follows the new default. Files: `src/init.rs`, `tests/harness.rs`, `tests/cli.rs`. Verify: `cargo test --test harness --test cli`, `cargo test --bin spec-dna init::` and `cargo test`. Hours: 4.
- [ ] t5: Add the tests of the default of each new profile, with the golden files of a new project of `standard`: the settings with no advisorModel and the team page with no section Advisor. The golden files of the settings of `lean` and `strict` keep their bytes. Files: `tests/init_profiles.rs`, `tests/golden/harness/settings-standard.json`, `tests/golden/harness/model-team-no-advisor.md`. Verify: `cargo test --test init_profiles`. Hours: 2.
- [ ] t6: Make `--reconfigure` keep the role, and move it with a new answer to question 1 only when its value is the default of the old profile. Add question 13 to the note of reconfigure. Test that `upgrade` keeps the advisor of a project of 0.8.0. Files: `src/init.rs`, `tests/init_profiles.rs`. Verify: `cargo test --test init_profiles`. Hours: 2.
- [ ] t7: Add the guard that the copy of the arm lean has `advisorModel`. Change the row lean of the table of the arms, and say that the arm lean of 0.8.1 has the consult sentence of D98. Files: `tests/eval_run.rs`, `docs/evals.md`. Verify: `cargo test --test eval_run` and `pre-commit run --all-files`. Hours: 1.
- [ ] t8: Document the questions 12 and 13, the default of each profile, the note of reconfigure, the warning, the roles of a new project and the rule of text. Files: `docs/init.md`, `docs/harness.md`, `docs/profiles.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [ ] t9: Document the default of the role advisor that `init` writes, and that a table with no row advisor reads as fable. Files: `docs/config.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t10: Write the rule of D98 in the line of `CLAUDE.md` of this repository, in the 3 texts of the ownership file, and in the 2 texts of the agents README and the row of fallback-advisor in its table of agents. Files: `CLAUDE.md`, `.claude/agents/ownership.md`, `.claude/agents/README.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t11: Write the description of D98 in the agent file fallback-advisor, and write the entry of the change. Files: `.claude/agents/fallback-advisor.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t12: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish advisor-default --dry-run`. Hours: 3.

## Risks

- The texts of the skills and of `acceptor.md` are frozen for the skill evals (`docs/evals.md`, section "The freeze of the skill texts for the reruns"). This change must not change them. It may change the model-team line of `CLAUDE.md`, `model-team.md` of `standard` and `strict`, `.claude/settings.json` and the presence of `fallback-advisor.md`. The tests of the golden files of the skills and of the acceptor, and the hashes of the freeze table, stay green with no change (guard).
- The arm `lean` of the runner gets the new consult sentence. A rerun of a skill after this change measures the skill with another line of `CLAUDE.md` than the reruns of 0.8.0. The freeze table does not list the line, so the rule asks for no rerun, and t7 states the difference.
- The probe of this plan applied the preset after the answers, so it also replaced an answer of `advisor=fable`. A test that a probe cannot see can fail in t4. Then the builder gives it the answer of t3 in a task of its own before t4, and records the task in the plan.
- `src/harness.rs` and `tests/harness_roles.rs` also change in the change `lifecycle-fixes`, and `tests/eval_run.rs`, `src/init.rs` and `docs/evals.md` in the change `eval-runner`. The design orders this change after both, so it rebases on them and runs each check again.
- A project of 0.8.0 with the advisor gets SD502 for its model-team line and `model-team.md` until `init` or `upgrade` writes them again, as for each release.
- A person can set `advisorModel` in the user settings, so a session of a project with no advisor can still have one (Q3 of the design).

## Proof

Each test fails before the fix, unless it says "guard".

- `tests/init_profiles.rs`, `a_new_standard_project_has_no_advisor`: `init --non-interactive` in `standard` writes `advisor = { claude-code = "none", agent = "opus", effort = "xhigh" }`, no `advisorModel` and no `fallback-advisor.md`, and the settings and the team page equal the new golden files. Before: `fable`, `advisorModel` and the file. The same in `lite`.
- `a_new_strict_project_has_the_advisor`: as 0.8.0 (guard).
- `reconfigure_moves_the_advisor_with_the_profile`: `standard` to `strict` moves `none` to `fable`, and a value that a person set stays. The note names the questions 4, 5, 12, 13 and 14.
- `upgrade_keeps_the_advisor_of_a_project_of_0_8_0`: a table with no row `advisor` keeps `advisorModel` after `upgrade` (guard).
- The warning: the advisor on with the policy none gives the warning of D97. The policy three-class gives none (guard).
- `src/harness.rs`, the test of the line: the line with the advisor has "only in a change of class `higher-risk`" and "no gate checks it". Before: the text of 0.8.0. A line with no advisor has no advisor sentence (guard).
- `tests/harness_roles.rs`: the golden file `model-team.md` with the new section Advisor, and a project of 0.8.0 with the advisor that gets the new line from `init`.
- `tests/harness.rs`: `the_preview_has_14_answers_and_questions_13_and_14_write_nothing_without_a_harness` shows `advisor=none` in the default line of question 13, and `init_writes_the_lean_harness_and_check_is_clean` and the 2 tests of a dry run list no file of the advisor. `check_reports_each_kind_of_drift_after_init`, `a_file_of_the_project_makes_its_role_none_and_stays` and `reconfigure_moves_between_the_levels_and_removes_what_it_wrote` keep their result with the answer of t3 (guard).
- `tests/cli.rs`, `fresh_project_init_check_uninstall`: no `fallback-advisor.md` in the output, in the line "to commit" and in the files that `uninstall` deletes.
- The 5 tests of `tests/harness_roles.rs` and the test of `tests/generated_lint.rs` of t3 keep their result (guard).
- `tests/eval_run.rs`: the copy of the arm `lean` has `advisorModel` (guard).
- Guards with no change: `lite_writes_the_team_page_of_0_7_1`, the golden files of the skills, `acceptor.md`, `acceptor-no-advisor.md`, `settings-lean.json`, `settings-strict.json` and `fallback-advisor.md`, and the tests of the arms `lean` and `lean-no-advisor` in `tests/eval_ab.rs`.

The probe, on 2026-11-24 in `spec-dna-lab/session-37`: a copy of the tree of 27eb114 with the preset of D97 after the answers of `init`, and no other change. The unit tests and 56 of the 83 test files ran, each file that runs `init` among them. 14 tests failed, in 4 files: 7 in `tests/harness.rs`, 5 in `tests/harness_roles.rs` (`each_agent_file_ends_with_the_rule_of_the_scratch_folder`, `full_writes_the_implementer_and_the_code_simplifier`, `lean_writes_the_verifier_and_no_checker`, `lite_writes_the_agent_files_of_0_7_1_at_each_level` and `the_agent_files_match_the_golden_files`), 1 in `tests/generated_lint.rs` (`every_generated_markdown_file_passes_the_lint`) and 1 in `tests/cli.rs` (`fresh_project_init_check_uninstall`). `tests/init_profiles.rs`, `tests/eval_ab.rs`, `tests/hook_protect.rs`, `tests/harness_skills.rs` and the tests of `upgrade` passed.

The cost of D97 and D98, from the design: a new project of `lite` or `standard` loses 432 bytes of advisor sentences in its model-team line and 225 bytes of the description of `fallback-advisor` in each request. Where the advisor is on, the consult sentence grows from 227 to 373 bytes. The `CLAUDE.md` of this repository grows by 131 bytes. The build measures the line of a new project of `standard` before and after, and puts the 2 numbers in the pull request.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
