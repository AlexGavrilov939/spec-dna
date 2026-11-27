---
status: draft
class: higher-risk
---
# `context` for a source file serves the summaries of its areas

## Intent

`context <file>` serves the summaries of the areas of the documents that cite or apply to the file, and the `system` and `product` levels above them, before the detail sections. For a request with a code path and no document target, the tier `summary` comes after the best detail section and before the rules and the other sections in the packing order. Its floor is the room of the title sections of the chain, so a project with many rules still gets a summary (design D101). The output order and the form of the output do not change. A request with a document target keeps the rules of D83.

## Why now

Agents call `context` mostly for a file that they will edit. On a fixture with the layer graph of a mature project, `context --path <file>` of 0.8.1 serves no section of the tier at the budgets 4,000, 8,000, 12,000 and 20,000, as 0.8.0 (E5 of the design). A project that writes summaries thus gets none for its main use.

## Out of scope

- A request with a document target, also with `--path`: it keeps D56 and D83.
- The fill order of the tier: it stays the order of D83. An order by the rank of the citing documents in one depth stays open.
- A key for the share or the floor. Both stay rules of the tier.
- A retrieval task set with summaries (D56 of 0.8.0).
