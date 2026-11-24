---
status: draft
---
# The roll-up reads interfaces, rules, decisions and research, and an old plan gets one note: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It adds the configuration key `layers.<name>.rolls-up`, the codes SD646 and SD647, and a second definition of a document hash for a pin: the hash of the text in force. It changes the frontier, the area and the messages of the roll-up when the key names more source layers, and the texts of SD643 and SD644. It adds one note to the output of `next`, one entry to `notes` of `next --json`, a new text of the note of `finish`, and an opt-in of the acceptance record. The [design](../../design/0.8.1-a-evidence-and-context.md) states the contract in D84 and D86, and [ADR 0028](../../adr/0028-roll-up-and-summaries.md) holds the amendment of 2026-11-24.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0028-roll-up-and-summaries]]
- [[adr/0025-independent-acceptance]]

## Concerns

- [x] The hash of the text in force is a second hash of a whole document. Resolved: the amendment of ADR 0028 states it, and it combines the frozen hashes of the chunks that the parser gives and the cache holds, as `frozen_hash` does. A pin to a summary, a `system` or a `product` document keeps `frozen_hash`.
- [x] SD646 is new for a current corpus: a summary of 0.8.0 with an empty frontier that another roll-up document pins. Resolved: it is a warning in `standard`. In `strict` it is an error, but the base has the same finding, so it is not new and no gate stops.
- [x] The slice A4 of the design does not list `src/split.rs` and `src/main.rs`. Resolved: `mv` takes its summaries from the function of the move in `src/split.rs`, and `new summary` reads its area in `src/main.rs`. The plan lists both, and a task edits them only if the roll-up alone does not give the result.
- [x] D86 does not say whether the note prints when the folder already has `acceptance.toml`. Resolved: the note prints only while the exemption holds. With a record in the folder, the change needs its record as a change of 0.8.0 does, so `next` shows the record item and `finish` checks the record at step 5.2.
- [x] ADR 0025 gives the need rule of a record, and D86 adds an opt-in that it does not name. Resolved: the rule of ADR 0025 stays true for each plan with no record in its folder, and the opt-in only adds a check that an agent asked for by the record. The design gives no amendment, and ADR 0025 is at its hard budget of 14,000 characters.
- [x] The changes `summary-tier` and `rollup-sources` both edit `src/context.rs` and `tests/context_layers.rs`, and the changes `lifecycle-fixes` and `rollup-sources` both edit `src/stage.rs`, `src/acceptance.rs`, `src/finish.rs` and `tests/accept_next.rs`. Resolved: the design orders this change after `summary-tier`, so it rebases on it and on each change of wave 1 that merges first.

## Open questions

- [x] Does the pin of a decision use a new hash of the text in force (Q2 of the design)? Answer of the owner: (a), in the amendment of ADR 0028 of 2026-11-24. A new amendment of an accepted decision makes the summary stale.
- [x] Does a record in the folder of a change of an old plan end its exemption (Q3 of the design)? The design recommends (a): yes, and `finish` checks the record in full. Option (b) gives no opt-in, so the tasks t10 and t11 of the plan change. Answer of the owner (word of 2026-11-24, the recommendation of the design): (a), yes.
