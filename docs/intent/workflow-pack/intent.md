---
status: accepted
class: higher-risk
---
# Workflow pack of 11 skills

## Intent

Ship the workflow pack of [ADR 0010](../../adr/0010-workflow-pack.md), as slice 4 of [the design](../../design/0.7.0.md) says (D2 to D6, D39, D41 and section 14). An agent then gets one short skill for each stage of a change, and each skill ends at a check of spec-dna.

The change adds these items:

- D2: the 11 skills `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify`, `finish`, `debug` and `post-mortem`. Each skill has the 5 parts of D2. 7 skills are on by default, and 4 are off.
- D3: the 5 disciplines. The adapter skill `spec-dna` holds them in one block, and the line that `init` writes into `AGENTS.md` names them.
- D4, D10: the read-only command `spec-dna skill list|show`. The 0.6 names `intent`, `spec` and `plan` end, and `upgrade` rewrites them.
- D5: the description of each skill starts with "In a spec-dna project". `init` prints a note when the project enables a plugin with the word `superpowers`.
- D6: a test fixes each byte limit.
- D39: the level `full` also proposes the 4 skills that are off.
- D41: the list "Files that SD503 counts" names the 11 skill folders.
- Q5: the skills and the line of `AGENTS.md` say that an agent commits an approval only when the approver says so explicitly in the session. A skill never sets `accepted` or `approved`.
- Section 15: the floor of the skill `clarify` adds SD925.
- Section 15: the skill `finish` tells the agent to put the line `Hours: <total>` into the commit message.
- Section 15: the skill `brainstorm` writes the class of the change before the approval of the intent.

## Why now

`next` names 9 skills since 0.6.0, but spec-dna ships only 4 of them. The evals of slice 5 (`skill-evals`) measure the text of these skills, so the text comes first. The floors of the skills (SD923, SD924 and SD925) come from slice 3a (`task-checks`).

## Out of scope

- The evals of the pack, `eval run --ab`, `eval verdict` and SD507 (slice 5, `skill-evals`).
- The skills that `init` turns on for each profile (slice 8). This change keeps one default: the 7 skills that D2 sets on.
- The guide `docs/guides/upgrade-0.6.md` (slice 8).
- A change of the 9 names of `next` or of the step of each name (D7).
- A credit line in a skill file (Q8).
