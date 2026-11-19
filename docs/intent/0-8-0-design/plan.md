---
status: draft
---
# 0.8.0 design

## Files

- `docs/design/0.8.0.md` The design page.
- `docs/adr/` ADRs 0024 to 0031.
- `docs/intent/0-8-0-design/` This change.

## Order

- [x] t1: Read the code and the evals of 0.7.1 and trace the 3 carry-over defects. Files: `docs/design/0.8.0.md`. Verify: section 1 and section 8 of the design page name the cause of each defect with a cite.
- [x] t2: Write the design page and ADRs 0024 to 0031 with `architect`. Files: `docs/design/0.8.0.md`, `docs/adr/`. Verify: each ADR has at most 14,000 characters (the budget of an ADR).
- [x] t3: Add the overview and the slice plan. Files: `docs/design/0.8.0.md`. Verify: section 12 of the design page lists each slice with its tests, its estimate and its eval budget.
- [x] t4: Run an independent critique with `checker`, and apply the fixes. Files: `docs/design/0.8.0.md`, `docs/adr/`. Verify: the report of the second pass of `checker` has no open blocker.
- [x] t5: Check the language of the new text with `docs-editor`. Files: `docs/design/0.8.0.md`, `docs/adr/`. Verify: `pre-commit run --all-files` passes.
- [ ] t6: The owner reads the design and answers the questions of section 9. Files: none. Verify: the owner gave the answers.

## Risks

- The estimate of the build comes from 2 earlier versions. The error of the estimate can be 50%.
- The numbers of cost, of power and of speed are estimates. The slices measure them.
- The design has no code, so a case that the design does not cover can appear in the build.

## Proof

`spec-dna check` reports no error. The owner does 2 steps after the review. First, the owner sets the statuses `accepted` and `approved`, and the ADRs to `accepted`. Then the owner runs `spec-dna finish 0-8-0-design` in a later pull request.
