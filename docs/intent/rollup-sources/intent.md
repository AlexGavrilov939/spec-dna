---
status: draft
class: higher-risk
---
# The roll-up reads interfaces, rules, decisions and research, and an old plan gets one note

## Intent

The layer of the type `summary` gets the key `rolls-up`, the list of its source layers (design of evidence and context, D84). A summary can then pin each requirement of an active interface or rule document, an `accepted` decision as a whole, and a research document as a whole. The pin of a whole document matches the hash of its text in force, so a new amendment of a decision makes the summary stale. The new code SD646 reports a roll-up document that rests on nothing, and SD647 a changed research note. With no key, the roll-up gives the result of 0.8.0.

`next` and `finish` print one note for a plan that a commit added when `spec-dna.toml` pinned a version before 0.8.0, and a record in the folder of the change ends its exemption (D86).

## Why now

Issue 111: a project that keeps its decisions in `decisions/storage/` and its studies in `research/storage/` gets SD644 for `summaries/storage.md` and SD643 for each pin, so the roll-up gives it no benefit. A `higher-risk` change of 0.7.1 that is still open after the upgrade gets the review of 0.7.1 from `next` and no word on the record, so the agent does not know that the change skips the acceptance, or how to ask for one.

## Out of scope

- The fill order and the share of the tier `summary` (the change `summary-tier`).
- A roll-up of procedures, intents, incidents and the pending layer.
- A key that `init` writes: a project opts in by hand.
- A change of `src/hash.rs`, `src/parse.rs`, the cache or `PARSER_VERSION`: the new hash combines the chunk hashes that the cache holds.
- A due roll-up document from a decision or a research note at `finish` and `next`: the delta names requirements only.
- A retrieval task set with summaries (D56 of 0.8.0).
