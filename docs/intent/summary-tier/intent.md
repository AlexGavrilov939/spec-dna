---
status: draft
class: higher-risk
---
# The tier `summary` of `context` serves the summary of the area

## Intent

`context` serves the summary of the area of a file. The tier `summary` fills the title section of each document of the chain first, from the deepest summary up to `product`, and then the other sections in the same order. The tier uses at most a half of the budget, in place of a quarter (design D83). The output keeps its order from the top down.

## Why now

A fixture with a real layer graph gives no summary in `context src/motor.rs` at each budget from 4,000 to 20,000 characters (E15 of the design). At the default budget of 8,000, the output uses 2,277 characters, so the share of the tier is the limit, not the budget. A project that writes summaries thus gets no summary in `context`.

## Out of scope

- The source layers of the roll-up: the key `rolls-up`, SD646 and SD647 (D84).
- A key for the share of the tier. The share stays a constant.
- A retrieval task set with summaries (D56 of 0.8.0).
