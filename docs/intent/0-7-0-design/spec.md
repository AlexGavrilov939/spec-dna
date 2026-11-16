---
status: draft
---
# 0.7.0 design

## Delta

None. This change adds, changes or removes no requirement. The documents that it adds are the design page and ADRs 0010 to 0017.

## Interfaces

None. The design proposes changes to the interfaces of `spec-dna`. These are the keys `[change] approval`, `[change] approvers` and `[change] max-task-files`, the commands `skill`, `import` and `eval pack`, the option `eval run --ab`, and the codes SD507, SD923 and SD924. Each of them is a change of its own in the build, and each has its own spec.

## Decisions

- [[adr/0010-workflow-pack]]
- [[adr/0011-entry-points]]
- [[adr/0012-approval-in-the-branch]]
- [[adr/0013-self-hosting]]
- [[adr/0014-import]]
- [[adr/0015-limits-of-0-6-1]]
- [[adr/0016-skill-evals]]
- [[adr/0017-profiles]]

## Concerns

- [ ] The approval rule of ADR 0012 trusts an author e-mail. The ADR states the limit. The owner must accept it (Q1 of the design).

## Open questions

- [ ] Q1: Accept the amendment of ADR 0006 by ADR 0012?
- [ ] Q2: Add `[change] record = "none"`?
- [ ] Q3: The budget of the eval run: the pilot or the full run?
- [ ] Q4: Keep `execute`, `tdd`, `verify` and `debug` off by default until a verdict?
- [ ] Q5: Which e-mail goes into `approvers` of this repository?
- [ ] Q6: Is `--keep none` the right default of `import`?
- [ ] Q7: One key `profile`, or a separate key for the preset?
- [ ] Q8: Credit Superpowers in the matrix only, or in each skill?
