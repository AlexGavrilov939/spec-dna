---
status: draft
---
# 0.7.0 design

## Files

- `docs/design/0.7.0.md` The design page.
- `docs/adr/` ADRs 0010 to 0017.
- `docs/research/matrix.md` The versions of the 3 tools, and the row of Superpowers.
- `docs/intent/0-7-0-design/` This change.

## Order

- [x] t1: Read the evidence and measure the pull requests. Files: `docs/design/0.7.0.md`. Verify: the lab scripts in `spec-dna-lab/session-24/pr-count/` give the table of section 4.
- [x] t2: Write the design page. Files: `docs/design/0.7.0.md`. Verify: `spec-dna check` reports no error.
- [x] t3: Write ADRs 0010 to 0017 with `architect`. Files: `docs/adr/`. Verify: each file has at most 14,000 characters (the budget of an ADR).
- [x] t4: Check the language of the new text with `docs-editor`. Files: `docs/design/0.7.0.md`, `docs/adr/`. Verify: `pre-commit run --all-files` passes.
- [x] t5: Run an independent critique with `checker`, and apply the fixes. Files: `docs/design/0.7.0.md`, `docs/adr/`. Verify: the report of `checker` has no open problem.
- [ ] t6: Open the draft pull request. Files: `docs/intent/0-7-0-design/`. Verify: the owner reads the pull request.

## Risks

- The estimate of the build comes from one earlier version. Its error can be 50%.
- The cost of the eval run is an estimate from earlier sessions.
- The model of the approval rule is not product code. The build can find a case that the model does not cover.

## Proof

`spec-dna check` reports no error. The lab scripts reproduce the table of section 4 of the design. The owner does 2 steps after the merge. First, the owner sets the statuses `accepted` and `approved`, and commits them. Then the owner runs `spec-dna finish 0-7-0-design` in a later pull request.
