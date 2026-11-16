---
status: draft
class: higher-risk
---
# Hours of a change and links into a change

## Intent

Build items 5, 6 and 7 of the feedback of a second project (section 15 of [the design](../../design/0.7.0.md)). A team can then record the hours of a change, see a link that will stop `finish` before it runs `finish`, and record a dead link of a locked decision.

- Item 5: `finish` adds the items of the section `Hours` of `plan.md` and prints the line `Hours: <total>`. The person or the agent that commits puts the line in the commit message. `log` and `stats --sdlc` read the last such line of the commit that deletes `intent.md`.
- Item 6: the new warning SD612 (`link-into-change`) reports a reference into the folder of a change that step 0 of `finish` stops on, when SD611 and SD901 do not report it.
- Item 7: an amendment section that names the dead target of a locked link clears its info line. The pointer of a finished change names the commit that `log` gives.

## Why now

A squash merge keeps only the commit message of the last pull request of a change, so the hours need the line now. A link into a change folder stops `finish` late, in the pull request of the code.

## Out of scope

- The parse of the section `Hours` (slice 3a, `task-checks`).
- A trailer that `finish` writes: `finish` makes no commit (ADR 0006).
- A gate on the hours: the line is a record, not a control.
- A change of the stop of `finish` step 0: it keeps SD101 and SD102.
