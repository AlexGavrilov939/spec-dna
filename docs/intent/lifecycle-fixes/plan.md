---
status: draft
---
# Fixes of the change lifecycle: finish, scope, the acceptor, layers and small items: plan

## Files

- `docs/design/0.8.1-a-lifecycle-and-harness.md` The design of D87, D88 and D90 to D93.
- `src/change.rs` The filter of the open tasks for SD915 and finish, and the text of SD939 for a repeated id.
- `src/lifecycle.rs` SD915 for a change with no delta, and the scope and the text of SD913.
- `src/finish.rs` Step 0 with the shared filter of the open tasks, and the title of a new document.
- `src/check.rs` SD401 with the steps of the rework for a spec or a plan.
- `src/diag.rs` The fix text of SD401 after its mark.
- `src/sdlc.rs` The bind of the reports in stats only on one tree, and the key reports_read.
- `src/tree.rs` The hash of the code tree of HEAD, if no function gives it.
- `src/acceptance.rs` The checks of the sections of a change with no record.
- `src/stage.rs` The item of next before the acceptor, and the note on a test path of the proof.
- `src/accept.rs` The refusal of accept, and the fallback from the first parent.
- `src/harness_text.rs` One sentence in step 5 of the skill execute.
- `src/main.rs` The handler of layer add.
- `src/upgrade.rs` The last line of upgrade.
- `src/harness.rs` The steps of the hook in the acceptor file only when the hook exists.
- `src/templates.rs` The plan template, and the title rule in one function.
- `src/split.rs` The title of a split with the shared function.
- `tests/lifecycle_finished.rs` The tests of SD915.
- `tests/lifecycle_checks.rs` The tests of SD401 and SD913.
- `tests/lifecycle_base.rs` The tests of the scope of SD913 that encode the rule of 0.8.0.
- `tests/harness_skills.rs` The test that pins the text of step 5 of the skill execute.
- `tests/init_graph.rs` The test that expected SD502 after layer add.
- `docs/init.md` The line on init after a layer change.
- `tests/proof_debt.rs` The test of stats on 2 trees.
- `tests/accept_next.rs` The test of next before the acceptor.
- `tests/accept_cmd.rs` The tests of the refusal of accept and of a squash commit.
- `tests/golden/harness/skills/spec-dna-execute.md` The golden file of the skill execute.
- `tests/layer_cmd.rs` The tests of layer add.
- `tests/upgrade_cli.rs` The test of the last line of upgrade.
- `tests/harness_roles.rs` The tests of the acceptor file with and with no hook.
- `tests/golden/harness/acceptor*.md` The golden files of the acceptor file.
- `tests/templates_new.rs` The test of the plan template.
- `tests/finish.rs` The test of the title of a new document.
- `tests/branch_approval.rs` The test of the note of next.
- `docs/diagnostics.md` The rows of SD401, SD502, SD913 and SD915.
- `docs/guides/change.md` The 2 orders of the pull requests of a routine change, and the rework.
- `docs/commands.md` The sections of accept, next, stats and layer add.
- `docs/types.md` The ids of the deviations, and the code spans of a Files item.
- `docs/evals.md` The second exception of the freeze for the acceptor file of standard.
- `CHANGELOG.md` The entry of the change.

## Order

- [x] t1: Make SD915 read the tasks of the newest plan for a change with no delta, with one shared filter of the open tasks and the message that names the open tasks (D87). Files: `src/change.rs`, `src/lifecycle.rs`, `tests/lifecycle_finished.rs`. Verify: `cargo test --test lifecycle_finished`. Hours: 4.
- [x] t2: Give SD401 the message and the fix of the rework for a spec or a plan that moves from approved to draft (D91). Files: `src/check.rs`, `src/diag.rs`, `tests/lifecycle_checks.rs`. Verify: `cargo test --test lifecycle_checks`. Hours: 2.
- [ ] t3: Run SD913 only for a change of the diff whose folder the diff touches or that the branch added, with the new message (D92). Files: `src/lifecycle.rs`, `tests/lifecycle_checks.rs`, `tests/lifecycle_base.rs`. Verify: `cargo test --test lifecycle_checks --test lifecycle_base`. Hours: 2.
- [x] t4: Make `stats` bind the reports only when the working tree has the code tree of HEAD, and add the key reports_read (D92). Files: `src/sdlc.rs`, `src/tree.rs`, `tests/proof_debt.rs`. Verify: `cargo test --test proof_debt`. Hours: 3.
- [x] t5: Make the item review of `next` run the checks of the sections before it names the acceptor (D88). Files: `src/acceptance.rs`, `src/stage.rs`, `tests/accept_next.rs`. Verify: `cargo test --test accept_next`. Hours: 3.
- [x] t6: Make `accept` refuse an item that is not valid before any write, give SD939 the fix of a repeated id, and diff the fallback of `accept` from the first parent of the commit that added the plan (D88, Q8). Files: `src/accept.rs`, `src/change.rs`, `tests/accept_cmd.rs`. Verify: `cargo test --test accept_cmd` and `cargo test --bin spec-dna change::`. Hours: 4.
- [x] t7: Add the sentence on the ids of the deviations to step 5 of the skill execute and to the types page (D88). Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-execute.md`, `docs/types.md`. Verify: `cargo test --test harness_skills` and `cargo test --bin spec-dna harness_text::`. Hours: 1.
- [x] t8: Make `layer add` write the adapter skill with `spec-dna.toml` when the skill has no hand edit, with the new next lines and the key skill_updated (D90). Files: `src/main.rs`, `tests/layer_cmd.rs`, `docs/commands.md`. Verify: `cargo test --test layer_cmd`. Hours: 3.
- [x] t9: Make the last line of `upgrade` name the count of the items of its report (D93 item 1). Files: `src/upgrade.rs`, `tests/upgrade_cli.rs`. Verify: `cargo test --test upgrade_cli`. Hours: 1.
- [x] t10: Write the steps of the hook protect-approvals in the acceptor file only when the hook exists (D93 item 2). The file of strict keeps its bytes. The cases of the key advisor against the default of the profile compare with the 2 golden files and with the difference of the steps of the hook. Files: `src/harness.rs`, `tests/harness_roles.rs`, `tests/golden/harness/acceptor-no-advisor.md`. Verify: `cargo test --test harness_roles` and `cargo test --test harness_skills`. Hours: 2.
- [x] t11: Add the sentence on code spans to the plan template and to the types page, and give the title rule with acronyms to the function of the templates, with its unit tests (D93 items 3 and 4). Files: `src/templates.rs`, `tests/templates_new.rs`, `docs/types.md`. Verify: `cargo test --test templates_new` and `cargo test --bin spec-dna templates::`. Hours: 2.
- [x] t12: Make `finish` and the split use the title function of the templates, and make step 0 of `finish` use the shared filter of the open tasks (D93 item 4, D87). Files: `src/finish.rs`, `src/split.rs`, `tests/finish.rs`. Verify: `cargo test --test finish`. Hours: 2.
- [x] t13: Make `next` note a test path of the section Proof that no pattern of the plan matches (D93 item 5). Files: `src/stage.rs`, `tests/branch_approval.rs`, `docs/commands.md`. Verify: `cargo test --test branch_approval`. Hours: 2.
- [x] t14: Document SD915, SD401 and SD502, the 2 orders of the pull requests and the rework in the guide, and the sections of accept, next and stats. Files: `docs/diagnostics.md`, `docs/guides/change.md`, `docs/commands.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102 and no SD602. Hours: 2.
- [x] t15: Add the second exception of the freeze of the acceptor file, and write the entry of the change. Files: `docs/evals.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t16: Run the independent review and the acceptance pass, and finish the change. Files: none: finish deletes the folder of the change. Verify: `spec-dna finish lifecycle-fixes --dry-run`. Hours: 3.

## Risks

- D87 relaxes an error code. A plan with each box ticked and no code passes SD915. The rule of 0.6 had the same limit, and the acceptance record of a `higher-risk` change reads the diff.
- The new item of `next` and the refusal of `accept` read the sections of the plan and the spec. A false finding there blocks the acceptor. The checks are the checks of `first_stop`, which run today when a record exists.
- The acceptor file of `standard` changes its hash. Each project of `standard` gets SD502 until `upgrade` or `init` writes the file again, as for each release.
- Other tests can compare the acceptor file of `standard` with its golden file. They follow the golden file. A test that needs a new golden file of the acceptor is in the pattern of the section Files.
- 3 changes of wave 1 edit `src/lifecycle.rs`, `src/finish.rs`, `src/check.rs`, `src/diag.rs`, `src/upgrade.rs`, `tests/upgrade_cli.rs`, `tests/accept_cmd.rs` and some pages. The change that merges second rebases and runs each check again.

## Proof

Each test fails before the fix, unless it says "guard".

- `tests/lifecycle_finished.rs`: `a_routine_change_finishes_in_a_pull_request_of_its_own` and `a_plan_that_states_no_files_finishes` give no SD915. `an_open_task_with_no_edit_gives_sd915_with_the_ids` names the id. `an_open_task_with_an_edit_of_files_passes_as_in_0_8_0` (guard). `a_person_task_ticked_in_the_branch_counts` gives no SD915. `a_routine_change_needs_a_changed_files_path_or_a_changed_hash` passes with no edit (guard).
- `tests/lifecycle_checks.rs`: `the_step_back_of_a_base_approved_plan_gives_the_rework_steps`. A decision that moves from accepted to proposed keeps the text of 0.8.0 (guard). `a_small_fix_that_edits_files_of_an_open_change_gives_no_sd913`. `a_touched_change_still_gives_sd913` (guard).
- `tests/proof_debt.rs`: `stats_reads_no_report_while_the_code_tree_differs` gives the state tagged and reports_read false. A clean tree binds as before (guard).
- `tests/accept_next.rs`: `next_stops_before_the_acceptor_on_a_repeated_deviation_id`.
- `tests/accept_cmd.rs`: `accept_refuses_a_plan_with_a_repeated_deviation_id` gives exit code 1 and no file. A squash commit that holds the plan and the code gives the code paths in the record.
- A unit test in `src/change.rs` for the text of SD939 for a repeated id.
- `tests/harness_skills.rs` with the golden file of `execute`, and the limit of 2,500 bytes of a body (guard).
- `tests/layer_cmd.rs`: `layer_add_updates_the_adapter_skill` gives no SD502 after `layer add summaries`. `a_hand_edited_skill_stays_and_the_init_line_comes_first`.
- `tests/upgrade_cli.rs`: `the_last_line_names_the_items_of_the_report`.
- `tests/harness_roles.rs`: `the_acceptor_names_no_hook_that_the_project_lacks`. The golden file `acceptor.md` keeps the hash `f541cad357de8926` (guard).
- `tests/templates_new.rs`: the plan template has the sentence on code spans. The unit tests of the title function give `MCP`, `HTTP cache`, `SQL limits`, `Cache policy` and `Api keys`.
- `tests/finish.rs`: `finish_titles_the_new_document_mcp_as_MCP`.
- `tests/branch_approval.rs`: `next_notes_a_proof_test_outside_files`. A `covers:` id in the section Proof gives no note (guard).

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result accepted.
