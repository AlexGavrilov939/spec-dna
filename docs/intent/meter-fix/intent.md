---
status: draft
class: routine
---
# The context meter reads the last executor iteration, and the briefs name sections

## Intent

The hook `context-meter` reads the context of a request from the last iteration of type `message` in `usage.iterations`, and it skips a line of the model `<synthetic>` (design of lifecycle and harness, D94). With one iteration, the result is the result of 0.8.0. The agent files of this repository name sections in each line "Read first", and each agent file that builds or checks gets one line on reads by range.

## Why now

The meter stopped a builder subagent at its level of 250,000 tokens, and the subagent returned before the end of its step. The analyst found 2 candidate causes: a request with 2 iterations whose top-level usage is their sum (H1), and briefs that make an agent read whole pages of up to 140,000 bytes and modules of up to 5,559 lines (H2). A `<synthetic>` line with a usage of 0 deletes the state of the meter, so its notice speaks again. Both fixes are cheap, so the change builds both.

## Out of scope

- A change of the message of a subagent, of the levels, or of `DEFAULT_SUBAGENT_METER`.
- A generated file of a project: the hook is in the binary, and no agent file of `init` changes.
- The counts of the stopped transcript (Q5 of the design). They tell which cause acted, and the fixes do not wait for them.
- The briefs of `Explore.md`, `fallback-advisor.md`, `acceptor.md`, and the files `ownership.md` and `README.md`.
