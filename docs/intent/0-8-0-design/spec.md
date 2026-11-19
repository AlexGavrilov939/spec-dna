---
status: approved
---
# 0.8.0 design

## Delta

None. This change adds, changes or removes no requirement. The documents that it adds are the design page and ADRs 0024 to 0031.

## Interfaces

None. This change adds no code. The design proposes these interfaces of `spec-dna`. Sections 9 and 11 of the design page give each one. Each interface is a change of its own in the build, with its own spec.

Keys: `[evidence] reports`, `[evidence] tree-exclude`, `[evidence] reliable-runs`, `[evidence] reliable-for`, the value `review` of `[change] approval`, the form `@<login>` of `[change] approvers`, `[change] higher-risk-acceptors`, `[owners]`, `[budgets.area]` (`soft`, `hard`), `[layers.summaries]`, the key `change` of an eval file, and the rule group `rollup` of `[rules]`.

Commands: `evidence stamp`, `accept <slug>`, `reviews`, `refresh`, `new summary <area>`, and the artifact `acceptance` of `approve <slug>`.

Options: `check --reports`, `accept --role` and `--session`, `reviews --pr`, `finish --holds`, `refresh --holds`, `--all` and `--base`, `eval run --arms` and `--harness`, and `eval verdict --compare`.

Codes: SD613 to SD620, SD641 to SD645, SD705 to SD707 and SD927 to SD940.

Output: the note of `next` for the due roll-up documents, the fields `deviations` and `rollup` of `finish --json`, `proof` of `status --json` and `stats --json`, `person` of `next --json`, the value `accept` of the skill of `next --json`, the tier `summary` of `context`, and 3 keys of an eval record.

Files and tools: a harness role and a skill, the hook `protect-approvals`, the workflow `spec-dna-reviews.yml`, and the script `scripts/verify-release.sh`.

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

None. The limit of the record of acceptance (it cannot show that the acceptor is not the builder) is in the section "Does not prove" of decision D3 of the design page.

## Open questions

None. The questions for the owner are in section 9 of the design page. Task t6 of the plan tracks the answers.
