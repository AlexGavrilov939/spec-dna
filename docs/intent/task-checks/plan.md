---
status: approved
---
# Task checks and the parse of a change: plan

## Files

- `tests/task_checks.rs` The tests of SD923, SD924 and `max-task-files`.
- `tests/lifecycle_checks.rs` The tests of SD925 and of the item forms of `Interfaces`.
- `tests/change_model.rs` The tests of the paragraph under `Files` and of the items of `Hours`.
- `tests/lifecycle_commands.rs` The line of `next` and `status` for a paragraph under `Files`.
- `tests/templates_new.rs` The H1 placeholder and the box of the concern in the template.
- `src/parse.rs` The paragraph under `Files`, `ItemSection::Hours` and `PARSER_VERSION` 14.
- `src/index.rs` The cache of the new field, and `FORMAT` 2.
- `src/change.rs` The `none` rule and the paragraph under `Files`.
- `src/stage.rs` The line of `next` and `status` for a paragraph under `Files`.
- `src/lifecycle.rs` SD923, SD924, SD925, the forms of `Interfaces`, and the messages of SD913 and SD915.
- `src/diag.rs` The codes SD923, SD924 and SD925.
- `src/config.rs` The key `max-task-files`, the group `changes` and the profile table.
- `schema/spec-dna.schema.json` The key `max-task-files`, and the range "SD905 and SD910 to SD925" in the descriptions of `[change]`.
- `src/finish.rs` An `ADD` item of `Interfaces` when `finish` runs.
- `src/sdlc.rs` The copy of `interface_id` goes, and `sdlc` uses the function of `lifecycle`.
- `src/templates.rs` The box of the concern in the spec template.
- `src/main.rs` The H1 placeholder of `new` for a document of a change.
- `docs/diagnostics.md` SD914, SD923, SD924, SD925, and the paragraph in SD913 and SD915.
- `docs/profiles.md` The row of SD923, SD924 and SD925.
- `docs/config.md` The key `max-task-files`, and the codes of the group `changes`.
- `docs/types.md` The paragraph under `Files`, the forms of `Interfaces`, the `none` rule and the templates.
- `docs/commands.md` The H1 of `new`, and the line of `next` and `status`.
- `.claude/agents/ownership.md` The owner of `tests/task_checks.rs`.
- `CHANGELOG.md` The entry of the change.

## Order

- [ ] t1: Write the failing tests of SD923, SD924, `max-task-files` and SD925 (`diagnostics-engineer`). Add the forms of `Interfaces`, also an `ADD` item when the diff deletes the folder: SD914 with no document, and nothing with the document. Files: `tests/task_checks.rs`, `tests/lifecycle_checks.rs`. Verify: `cargo test --test task_checks --test lifecycle_checks` fails on each new case.
- [ ] t2: Write the failing tests of the parser, of the lines of `next` for a paragraph (the line of D28 first), and of the H1 placeholder (`core-engineer`). Files: `tests/change_model.rs`, `tests/lifecycle_commands.rs`, `tests/templates_new.rs`. Verify: `cargo test --test change_model --test lifecycle_commands --test templates_new` fails on the new tests only.
- [ ] t3: Keep the first paragraph of `Files` and the items of `Hours` in the parser, and set `PARSER_VERSION` 14 and `FORMAT` 2 (`core-engineer`). Files: `src/parse.rs`, `src/index.rs`. Verify: `cargo test --bin spec-dna` passes.
- [ ] t4: Add the `none` rule and the paragraph under `Files` to the change model, `next`, `status`, SD913 and SD915. Files: `src/change.rs`, `src/stage.rs`, `src/lifecycle.rs`. Verify: `cargo test --test change_model --test lifecycle_commands` passes.
- [ ] t5: Add the codes SD923, SD924 and SD925 and their checks, with a limit of 3 paths (`diagnostics-engineer`). Files: `src/diag.rs`, `src/lifecycle.rs`. Verify: the cases of `standard` in `cargo test --test task_checks` pass.
- [ ] t6: Add the key `max-task-files` (an integer from 1 to 20, default 3), and the 3 codes to the group `changes` and the profile table (`core-engineer`). Files: `src/config.rs`, `schema/spec-dna.schema.json`. Verify: `cargo test --test task_checks` passes.
- [ ] t7: Accept `none` and `ADD <interface id>: <text>` in `Interfaces` (`diagnostics-engineer`; `hygiene-engineer` for `finish`, `core-engineer` for `sdlc`). Files: `src/lifecycle.rs`, `src/finish.rs`, `src/sdlc.rs`. Verify: `cargo test --test lifecycle_checks` passes.
- [ ] t8: Write the box of the concern and the H1 placeholder (`core-engineer`). Files: `src/templates.rs`, `src/main.rs`. Verify: `cargo test --test templates_new` passes.
- [ ] t9: Update the docs of the codes, the profiles and the key. Files: `docs/diagnostics.md`, `docs/profiles.md`, `docs/config.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t10: Update the docs of the types and the commands, and the change log. Files: `docs/types.md`, `docs/commands.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t11: Name the owner of the new test file (`architect`; the owner accepts). Files: `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- Merge order: this change rebases on `branch-approval` and merges after it. `hours-and-links` starts after this change merges, because `finish` reads `ItemSection::Hours`.
- `PARSER_VERSION` 14 and `FORMAT` 2 make each project rebuild its cache once. An old binary that reads the new cache drops it, so a mix of versions only loses speed.
- An approved plan with tasks that have no `Verify:` gets new SD923 warnings in `standard`. A warning never fails `check`.
- The H1 placeholder shows in `status` as the title until the author writes one.
- `ADD` of `Interfaces` names a document that only `finish` checks. A spec that never writes it stops `finish` with SD914.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass. Each test of t1 and t2 failed before its task. A plan with a `none` paragraph gives no line about `Files` in `next`. `spec-dna check --base origin/master` gives no error on this repository.
