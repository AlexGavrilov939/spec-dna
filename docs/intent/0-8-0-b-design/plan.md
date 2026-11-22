---
status: approved
---
# 0.8.0 part B design

## Files

- `docs/design/0.8.0.md` The design page, with section 15.
- `docs/design/0.8.1-note.md` The text that moved to 0.8.1.
- `docs/adr/0032-acceptance-in-one-pass.md` The new ADR.
- `docs/adr/0017-profiles.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0027-owners-invariants-and-write-back.md` The dated pointers to ADR 0032.
- `docs/intent/0-8-0-b-design/` This change.

## Order

- [x] t1: Write section 15 of the design page and ADR 0032 with `architect`. Files: `docs/design/0.8.0.md`, `docs/adr/0032-acceptance-in-one-pass.md`, `docs/adr/0017-profiles.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0027-owners-invariants-and-write-back.md`, `docs/design/0.8.1-note.md`. Verify: `spec-dna check --base master` gives no error.
- [x] t2: Check the design with `checker`, and apply the fixes. Files: `docs/design/0.8.0.md`, `docs/adr/0032-acceptance-in-one-pass.md`. Verify: the report of `checker` has no open blocker.
- [x] t3: Check the language with `docs-editor`. Files: `docs/design/0.8.0.md`, `docs/adr/0032-acceptance-in-one-pass.md`. Verify: `pre-commit run --all-files` passes.

## Risks

- The estimate of the build is 7.75 sessions for part B, with an error of 50%.
- The design has no code, so a case that the design does not cover can appear in the build.

## Proof

`spec-dna check --base master` reports no error. The owner sets the status of ADR 0032, and the owner runs `spec-dna finish 0-8-0-b-design` in a later pull request.
