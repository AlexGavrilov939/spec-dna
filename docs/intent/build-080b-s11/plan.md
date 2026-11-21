---
status: draft
---
# Build of 0.8.0 part B, slice S11: the release candidate of 0.8.0: plan

## Files

- `Cargo.toml`, `Cargo.lock`, `spec-dna.toml`, `CHANGELOG.md` The version, the pin and the notes.
- `tests/upgrade_cli.rs`, `tests/fixtures/` The fixture of 0.7.1.
- `docs/README.md`, `docs/glossary.md`, `docs/guides/` The map and the upgrade guide.

## Order

- [ ] t1: Set the version, the pin and the CHANGELOG. Files: `Cargo.toml`, `Cargo.lock`, `spec-dna.toml`, `CHANGELOG.md`. Verify: `cargo test` passes. Hours: 3.
- [ ] t2: Prove `upgrade` on a fixture of 0.7.1 and on this repository. Files: `tests/upgrade_cli.rs`, `tests/fixtures/`. Verify: a second run changes nothing. Hours: 4.
- [ ] t3: Run the paired benchmark and the final `checker` review. Files: `bench/results/`. Verify: the benchmark is within 1.10 of v0.7.1, and `checker` has no open high item. Hours: 3.

## Risks

- A new error without a key that the project set is a defect, except what the notes name.

## Proof

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, and `checker` reports no open blocker. `spec-dna finish build-080b-s11` runs in the pull request of the slice.
