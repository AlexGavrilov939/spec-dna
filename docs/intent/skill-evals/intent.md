---
status: accepted
class: higher-risk
---
# Evals of each skill against no skill

## Intent

Ship the measurement of [ADR 0016](../../adr/0016-skill-evals.md), as slice 5 of [the design](../../design/0.7.0.md) says (D32 to D36 and section 14). A project can then show, with a committed file, whether a skill gives a gain on its own setup.

The change adds these items:

- D32: the eval pack, 5 evals for each of the 11 skills, with small fixtures and no consumer data. `eval pack` writes it to `<[evals] dir>/pack/`.
- D32: `eval run --ab`, with the arms `with` and `without`, the pairs, and the paired bootstrap of evals and runs (2,000 resamples, seed 24).
- D33: the rules of the verdict, and the simulation of D33 on this bootstrap before the pack ships.
- D34: the extra pair of arms for the line of `AGENTS.md`, and the cost cap before and during a run.
- D35: `eval verdict`, the table of the release in the binary, and SD507 `skill-unproven`.
- The rule of the owner: the runner retries a run that gets 401, 403, 429, a 5xx status or a Cloudflare page, with back-off. No result counts that run.

## Why now

Q4 keeps `execute`, `tdd`, `verify` and `debug` off until a verdict. The owner answered Q3: the pilot runs after this slice, and the full run runs before the tag. Both runs need this code. The skills of slice 4 (`workflow-pack`) give the text that the arms compare.

## Out of scope

- The pilot run, the full run and their cost. The main session starts them with the budget of the owner, and it stops at 600 US dollars.
- The values of the table of the release. The run step fills them from a committed result before the tag.
- An arm with another plugin installed (D36).
- A model grader. Each check is a command of the pack.
