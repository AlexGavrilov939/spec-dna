---
status: approved
---
# Profiles at init and the upgrade from 0.6: plan

## Files

- `docs/adr/0023-default-skills-from-the-release-table.md` The rules of this change.
- `src/verdict.rs`, `src/harness.rs`, `src/harness_text.rs` The rule, the constant and the 7 skills of D2.
- `src/init.rs` The preset of each profile.
- `src/upgrade_config.rs`, `src/upgrade.rs` The rename by the rule.
- `src/check.rs` SD507 by the rule.
- `tests/init_profiles.rs`, `tests/upgrade_v061.rs`, `tests/evals_checks.rs`, `tests/cli.rs`, `tests/upgrade_cli.rs`, `tests/init_evals.rs`, `tests/harness_skills.rs`, `tests/skill_cmd.rs`, `tests/init_change.rs` The tests.
- `tests/fixtures/upgrade/make.sh`, `tests/fixtures/upgrade/recorded/v061.txt`, `tests/fixtures/upgrade/v061/**` The fixture of 0.6.1.
- `.markdownlint-cli2.yaml` Only if a hook changes a file of the fixture.
- `scripts/demo-replay.sh` The demo.
- `docs/init.md`, `docs/profiles.md`, `docs/diagnostics.md`, `docs/harness.md`, `docs/commands.md`, `docs/config.md`, `docs/quickstart.md` The docs.
- `.claude/agents/ownership.md`, `CHANGELOG.md` The owners and the entry.

## Order

- [x] t1: Write the failing truth table of the rule, for each value of the constant (`core-engineer`). Files: `src/verdict.rs`. Verify: `cargo test --bin spec-dna verdict` fails on it. Hours: 0.25.
- [x] t2: Write the failing tests of the skills of each profile and level with a fake table (`init-engineer`). Files: `src/init.rs`. Verify: `cargo test --bin spec-dna init::tests` fails on them. Hours: 0.5.
- [ ] t3: Write the failing tests of the rename with a fake table (`core-engineer`). Files: `src/upgrade_config.rs`. Verify: `cargo test --bin spec-dna upgrade_config` fails on them. Hours: 0.25.
- [x] t4: Write the tests of `init` for each profile (`init-engineer`). Files: `tests/init_profiles.rs`. Verify: `cargo test --test init_profiles` fails on `lite`, the note and `--reconfigure`. Hours: 0.75.
- [ ] t5: Write the failing case of SD507 for a skill of D2 with the verdict `off` (`diagnostics-engineer`). Files: `tests/evals_checks.rs`. Verify: `cargo test --test evals_checks sd507` fails on it only. Hours: 0.5.
- [ ] t6: Record the fixture of 0.6.1 with the real binary 0.6.1 (`test-engineer`). Files: `tests/fixtures/upgrade/make.sh`, `tests/fixtures/upgrade/recorded/v061.txt`, `tests/fixtures/upgrade/v061/spec-dna.toml`. Verify: a second run of `tests/fixtures/upgrade/make.sh` leaves `git status --short tests/fixtures/upgrade` empty. Hours: 1.
- [ ] t7: Write the cases of the upgrade of the fixture (`test-engineer`). Files: `tests/upgrade_v061.rs`. Verify: `cargo test --test upgrade_v061`. Hours: 1.
- [x] t8: Add the rule and the constant `DEFAULT_ON_NEEDS_VERDICT_ON = true` (`core-engineer`). Files: `src/verdict.rs`. Verify: `cargo test --bin spec-dna verdict`. Hours: 0.25.
- [x] t9: Split the 7 skills of D2 from the default (`init-engineer`). Files: `src/harness.rs`, `src/harness_text.rs`. Verify: `cargo test --bin spec-dna harness`. Hours: 0.5.
- [x] t10: Add the preset of each profile to `init` (`init-engineer`). Files: `src/init.rs`. Verify: `cargo test --test init_profiles --test init_change`. Hours: 1.5.
- [ ] t11: Rename by the rule, with a line in `edits` for each left-out name (`core-engineer`; `init-engineer`). Files: `src/upgrade_config.rs`, `src/upgrade.rs`. Verify: `cargo test --test upgrade_v061`. Hours: 0.75.
- [ ] t12: Give SD507 the rule (`diagnostics-engineer`). Files: `src/check.rs`. Verify: `cargo test --test evals_checks`. Hours: 0.25.
- [ ] t13: Make the tests of the CLI, the upgrade and the evals of `init` read the default skills (`test-engineer`). Files: `tests/cli.rs`, `tests/upgrade_cli.rs`, `tests/init_evals.rs`. Verify: `cargo test --test cli --test upgrade_cli --test init_evals`. Hours: 0.5.
- [x] t14: Make the tests of the skills and of `init` read the default skills (`init-engineer`). Files: `tests/harness_skills.rs`, `tests/skill_cmd.rs`, `tests/init_change.rs`. Verify: `cargo test --test harness_skills --test skill_cmd --test init_change`. Hours: 0.5.
- [ ] t15: Make the demo read the default skills (`init-engineer`). Files: `scripts/demo-replay.sh`. Verify: `scripts/demo-replay.sh`. Hours: 0.25.
- [ ] t16: Document the preset, the profiles and SD507 (`init-engineer`; `diagnostics-engineer`). Files: `docs/init.md`, `docs/profiles.md`, `docs/diagnostics.md`. Verify: `pre-commit run --all-files`. Hours: 0.75.
- [ ] t17: Replace "the 7 skills that are on by default" with the rule (`init-engineer`; `core-engineer`). Files: `docs/harness.md`, `docs/commands.md`, `docs/config.md`. Verify: `pre-commit run --all-files`. Hours: 0.5.
- [ ] t18: Update the quickstart and write the entry (`init-engineer`; `release-engineer`). Files: `docs/quickstart.md`, `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 0.25.
- [ ] t19: Add the owners and the change set of ADR 0023 (`architect`; the owner accepts). Files: `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files`. Hours: 0.25.

## Risks

- `approver-safety`, `eval-records` and `limits-061` merge first: they edit `src/init.rs` and `src/verdict.rs`.
- With `true` and the table of the pilot, `init` writes only `tdd` until `release-070`.
- A test of another slice can name the default skills. The proof runs `cargo test` with each value.
- `make.sh` needs the binary 0.6.1 (`SPEC_DNA_061`) or a build of `v0.6.1`. It never installs on `PATH`.
- Until 0.7.0, the case of the pin 0.6.0 moves the pin. `release-070` runs the cases again.

## Proof

`cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test` pass, also with the constant `false` (a local edit). Each test of t1 to t5 failed first. `scripts/demo-replay.sh` passes. `spec-dna upgrade` of the fixture prints `proof: ok` with its ids. `spec-dna check --base origin/master` gives no error.
