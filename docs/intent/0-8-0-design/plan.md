---
status: approved
---
# 0.8.0 design

## Files

- `docs/design/0.8.0.md` The design page.
- `docs/adr/0024-test-report-as-evidence.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0026-approval-through-the-code-host.md`, `docs/adr/0027-owners-invariants-and-write-back.md`, `docs/adr/0028-roll-up-and-summaries.md`, `docs/adr/0029-protocol-of-the-model-team-measurement.md`, `docs/adr/0030-harness-arms-time-record-and-verdict-floor.md`, `docs/adr/0031-the-release-check.md` ADRs 0024 to 0031.
- `docs/design/0.8.1-note.md` The start of the design of 0.8.1: the text that moved out of 0.8.0.
- `docs/README.md` The map of the docs.
- `docs/intent/0-8-0-design/` This change.

## Order

- [x] t1: Read the code and the evals of 0.7.1 and trace the 3 carry-over defects. Files: `docs/design/0.8.0.md`. Verify: section 1 and section 8 of the design page name the cause of each defect with a cite.
- [x] t2: Write the design page and ADRs 0024 to 0031 with `architect`. Files: `docs/design/0.8.0.md`, `docs/adr/0024-test-report-as-evidence.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0026-approval-through-the-code-host.md`, `docs/adr/0027-owners-invariants-and-write-back.md`, `docs/adr/0028-roll-up-and-summaries.md`, `docs/adr/0029-protocol-of-the-model-team-measurement.md`, `docs/adr/0030-harness-arms-time-record-and-verdict-floor.md`, `docs/adr/0031-the-release-check.md`. Verify: each ADR has at most 14,000 characters (the budget of an ADR).
- [x] t3: Add the overview and the slice plan. Files: `docs/design/0.8.0.md`. Verify: section 12 of the design page lists each slice with its tests, its estimate and its eval budget.
- [x] t4: Run an independent critique with `checker`, and apply the fixes. Files: `docs/design/0.8.0.md`, `docs/adr/0024-test-report-as-evidence.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0026-approval-through-the-code-host.md`, `docs/adr/0027-owners-invariants-and-write-back.md`, `docs/adr/0028-roll-up-and-summaries.md`, `docs/adr/0029-protocol-of-the-model-team-measurement.md`, `docs/adr/0030-harness-arms-time-record-and-verdict-floor.md`, `docs/adr/0031-the-release-check.md`. Verify: the report of the second pass of `checker` has no open blocker.
- [x] t5: Check the language of the new text with `docs-editor`. Files: `docs/design/0.8.0.md`, `docs/adr/0024-test-report-as-evidence.md`, `docs/adr/0025-independent-acceptance.md`, `docs/adr/0026-approval-through-the-code-host.md`, `docs/adr/0027-owners-invariants-and-write-back.md`, `docs/adr/0028-roll-up-and-summaries.md`, `docs/adr/0029-protocol-of-the-model-team-measurement.md`, `docs/adr/0030-harness-arms-time-record-and-verdict-floor.md`, `docs/adr/0031-the-release-check.md`. Verify: `pre-commit run --all-files` passes.
- [x] t6: The owner reads the design and answers the questions of section 9. Files: none. Verify: the owner gave the answers.

## Risks

- The estimate of the build comes from 2 earlier versions. The error of the estimate can be 50%.
- The numbers of cost, of power and of speed are estimates. The slices measure them.
- The design has no code, so a case that the design does not cover can appear in the build.

## Proof

`spec-dna check` reports no error. The owner does 2 steps after the review. First, the owner sets the statuses `accepted` and `approved`, and the ADRs to `accepted`. Then the owner runs `spec-dna finish 0-8-0-design` in a later pull request.
