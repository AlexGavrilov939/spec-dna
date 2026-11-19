---
status: draft
---
# 0.8.0 design

## Delta

None. This change adds, changes or removes no requirement. The documents that it adds are the design page and ADRs 0024 to 0031.

## Interfaces

None. The design proposes changes to the interfaces of `spec-dna`. The keys are `[evidence] reports`, `[evidence] reliable-runs`, `[change] approval = "review"`, `[change] higher-risk-acceptors`, `[owners]` and `[budgets.area]`. The commands are `evidence`, `accept`, `reviews` and `refresh`. The options are `check --reports` and `eval run --arms`. The codes are SD613 to SD620, SD641 to SD645, SD705 to SD707 and SD927 to SD940. Each interface is a change of its own in the build. Each change has its own spec.

## Decisions

- [[adr/0024-test-report-as-evidence]]
- [[adr/0025-independent-acceptance]]
- [[adr/0026-approval-through-the-code-host]]
- [[adr/0027-owners-invariants-and-write-back]]
- [[adr/0028-roll-up-and-summaries]]
- [[adr/0029-protocol-of-the-model-team-measurement]]
- [[adr/0030-harness-arms-time-record-and-verdict-floor]]
- [[adr/0031-the-release-check]]

## Concerns

- [ ] The record of acceptance cannot show that the acceptor is not the builder. The design states the limit and names the person who acknowledges as the trust root. The owner must accept the limit (Q10, Q11, Q12).

## Open questions

- [ ] Answer the questions of section 9 of the design page. The owner answers them before the build of slice S2.
