---
status: approved
---
# Build of 0.8.0, part A: plan

## Files

- `scripts/verify-release.sh`, `.github/workflows/release.yml` The release check.
- `src/change.rs`, `src/stage.rs`, `src/harness_text.rs` The label `By: person` and its text.
- `src/upgrade.rs` The base of the in-memory check.
- `src/eval_ab.rs`, `src/verdict.rs`, `src/eval_run.rs`, `src/main.rs` The runner: arms, time and turns, the floor of a verdict, the key `change`.
- `src/tree.rs`, `src/report.rs`, `src/evidence.rs`, `src/git.rs` The code tree, the readers of a report, the stamp and the bind.
- `src/check.rs`, `src/diag.rs`, `src/config.rs`, `src/finish.rs`, `src/init.rs` The codes SD613 to SD620, their groups and keys, and step 5 of `finish`.
- `src/parse.rs`, `src/model.rs`, `src/sdlc.rs` The marker `proof-owed`, `PARSER_VERSION` 17, `status` and `stats`.
- `schema/spec-dna.schema.json` The keys of `[evidence]`.
- `tests/*.rs`, `tests/fixtures/**`, `tests/golden/**` The tests, the fixtures of 5 runners and the golden files.
- `.claude/agents/ownership.md`, `.claude/agents/release-engineer.md` The file rows and the release checklist.
- `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/types.md`, `docs/evals.md`, `docs/quickstart.md`, `docs/guides/change.md`, `docs/guides/upgrade-0.5.md`, `docs/guides/upgrade-0.6.md` The reference pages.

## Order

- [x] t1: Build slice S0: the release check, `By: person` and the base of the `upgrade` check (`release-engineer`, `core-engineer`, `init-engineer`). Files: `scripts/verify-release.sh`, `src/stage.rs`, `src/upgrade.rs`. Verify: `cargo test --test release_check --test upgrade_cli --test lifecycle_commands` passes, and each new test failed before the change. Hours: 3.
- [ ] t2: Build slice S1: the harness arms, the time and turns, the verdict floor and the key `change` in the runner (`core-engineer`, `init-engineer`). Files: `src/eval_ab.rs`, `src/verdict.rs`, `tests/eval_ab.rs`. Verify: `cargo test --test eval_ab` and `cargo test --lib verdict` pass with a stub `claude`, and each new test failed before the change. Hours: 6.
- [ ] t3: Build slice S2: the code tree, the 2 readers, the stamp, the bind, `check --reports` and SD613 to SD617 (`diagnostics-engineer`, `core-engineer`, `test-engineer`). Files: `src/tree.rs`, `src/report.rs`, `tests/evidence_reports.rs`. Verify: `cargo test --test evidence_reports` passes, the golden files of 0.7.1 stay, and each new test failed before the change. Hours: 10.
- [ ] t4: Build slice S3: the proof levels, the marker `proof-owed`, SD618 to SD620, `status` and `stats` (`core-engineer`, `diagnostics-engineer`). Files: `src/parse.rs`, `src/evidence.rs`, `tests/evidence_reports.rs`. Verify: `cargo test` passes, and each new test failed before the change. Hours: 4.

## Risks

- The estimate comes from 0.7.1 and 0.7.0. The error can be 50%.
- S2 binds a tag to a case by a rule that can be wrong in 4 cases (design, Q42). The 5 fixtures of runners are the proof.
- The pin stays 0.7.1 until the release, so each check in CI runs the 0.7.1 binary, and the tree binary runs the new code.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass for each slice, and `checker` reports no open blocker. `spec-dna finish build-080-a` runs in the pull request of S3.
