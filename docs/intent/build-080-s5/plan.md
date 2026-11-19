---
status: draft
---
# Build of 0.8.0, slice S5: plan

## Files

- `src/record.rs`, `src/accept.rs`, `src/finish.rs` The invariant and deviation tables, the skeleton and `finish` 5.3.
- `src/lifecycle.rs`, `src/check.rs`, `src/diag.rs`, `src/config.rs` SD936 to SD940 and the key `[change] invariants`.
- `src/change.rs`, `src/templates.rs`, `src/harness_text.rs` The sections of the templates and the text of the skills.
- `schema/spec-dna.schema.json` The key.
- `tests/*.rs`, `tests/golden/**` The tests and the golden files.
- `docs/commands.md`, `docs/config.md`, `docs/diagnostics.md`, `docs/profiles.md`, `docs/types.md`, `docs/guides/change.md` The reference pages.
- `docs/intent/build-080-s5/acceptance.toml` The record of this slice.

## Order

- [ ] t1: Build the invariants: the section, the ids with hashes, the key, SD936 and SD937, and the answers in the record (`core-engineer`, `diagnostics-engineer`). Files: `src/lifecycle.rs`, `src/record.rs`, `src/config.rs`. Verify: `cargo test --test lifecycle_checks --test accept_cmd` passes, and each new test failed before the change. Hours: 6.
- [ ] t2: Build the deviations: the section, `finish` 5.3, SD939 and SD940 (`hygiene-engineer`, `diagnostics-engineer`). Files: `src/finish.rs`, `src/lifecycle.rs`, `src/record.rs`. Verify: `cargo test --test finish --test lifecycle_checks` passes, and each new test failed before the change. Hours: 4.
- [ ] t3: Accept this slice with the mechanism of S4 and commit the record (`acceptor`). Files: `docs/intent/build-080-s5/acceptance.toml`. Verify: the tree binary gives no SD927 to SD931 for the record. Hours: 2.

## Risks

- A rule that the same change adds or removes cannot be an invariant (Q45). The check for it is new code.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the slice is valid. `spec-dna finish build-080-s5` runs in the pull request of the slice.
