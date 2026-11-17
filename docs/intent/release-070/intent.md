---
status: accepted
class: higher-risk
---
# Release 0.7.0

## Intent

Prepare the release of spec-dna 0.7.0, as slice 9 of [the design](../../design/0.7.0.md) says, after each slice of part A and part B merged and the full eval run ended. The change gives these items:

- The records of the full run in `bench/results/evals/`, the table of the release, and the constant of [ADR 0023](../../adr/0023-default-skills-from-the-release-table.md) as the owner sets it.
- The version 0.7.0 in `Cargo.toml` and `Cargo.lock`, and the pin of this repository, in one commit ([ADR 0013](../../adr/0013-self-hosting.md)).
- The section 0.7.0 of `CHANGELOG.md`: the upgrade from 0.6, the verdict of each skill, the benchmark against 0.6.1 and the known limits.
- The guide `docs/guides/upgrade-0.6.md`, the map of the docs, the version in the docs, the text of the README and the research matrix.
- The proof of `upgrade` on a project of 0.6.1 and on a clone of this repository, and the benchmark of the release candidate in Actions.

## Why now

The owner set the release conditions of 0.7.0 on 2026-11-17. This change meets the last of them: the version, the release notes, the pin, the proof of `upgrade` and the benchmark. The signed tag and the release workflow follow the merge (`CONTRIBUTING.md`, section "Releases").

## Out of scope

- The tag, the release workflow and the check of an asset against `SHA256SUMS`. These steps come after the merge, when the folder of this change no longer exists.
- A new feature, or a fix of a slice. A defect that a proof finds stops the release, and a change of its own fixes it.
- The adapters of spec-dna in this repository. The repository keeps its own agent files (design 0.6.0, D291), so the pin moves by hand.
- A pull request of the pin after the release (ADR 0013).
