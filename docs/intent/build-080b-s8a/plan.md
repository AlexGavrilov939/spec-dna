---
status: approved
---
# Build of 0.8.0 part B, slice S8a: the roll-up, type, pins and refresh: plan

## Files

- `src/rollup.rs` (new), `src/model.rs`, `src/layer.rs`, `src/config.rs`, `src/templates.rs`, `src/check.rs`, `src/diag.rs`, `src/main.rs`, `src/init.rs` The type, the layer, the pins, the codes and the commands.
- `tests/rollup_checks.rs`, `tests/rollup_refresh.rs` (new), `tests/templates_new.rs`, `tests/layer_cmd.rs`, `tests/init_graph.rs` The tests.
- `docs/*.md`, `docs/design/0.8.0.md`, `schema/spec-dna.schema.json`, `tests/cli.rs`, `tests/generated_lint.rs` The reference pages, the design and the schema.

## Order

- [x] t1: Build the type `summary`, the layer `summaries`, `new summary` and the pins with the frontier and the match (D50, D51). Files: `src/rollup.rs`, `src/model.rs`, `src/layer.rs`, `src/templates.rs`, `src/config.rs`. Verify: `cargo test` passes, and each new test failed before the change. Hours: 10.
- [x] t2: Build SD641 to SD645 in the group `rollup` and their profile rows (D52). Files: `src/check.rs`, `src/diag.rs`, `src/config.rs`. Verify: `cargo test --test rollup_checks` passes. Hours: 6.
- [x] t3: Build `refresh` with its guard (D53). Files: `src/rollup.rs`, `src/main.rs`. Verify: `cargo test --test rollup_refresh` passes. Hours: 6.
- [x] t4: Update the reference pages. Files: `docs/types.md`, `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files` passes. Hours: 2.

## Risks

- A project with no `[layers]` table must keep its types (P5).
- The dogfood of this repository needs a layer `requirements` that the repository does not have yet.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s8a` runs in the pull request of the slice.
