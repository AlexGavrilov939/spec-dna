---
status: draft
class: higher-risk
---
# Each subagent gets a card and ends with a state, an agent stops after 3 failed fixes, and a change lists its lessons

## Intent

`init` writes 3 new hooks for a new configuration of `standard` and `strict` (items 18 to 20 of the [harness additions of 0.8.1](../../design/0.8.1-harness-additions.md)). `subagent-card` gives each subagent one fixed card of 800 bytes or fewer: the scope, no change of the git state, the report form, the lessons and the state line. `return-state` asks a subagent once for the last line `State: <state>` when its report has none. `fix-limit` counts the failed fixes of one shell command for each agent, and tells the agent to stop at the limit. The skill `execute` gets the template of the task brief and the actions of the 4 return states. `finish` prints a note that asks the main session to list the lessons of the change for the person, and no lesson enters the repository before the person accepts it. The keys `[harness] subagent-card` and `[harness] fix-limit` turn the hooks on, and each generated file of `lite` keeps its bytes. The change is for the main session and the subagents of each project with the harness of Claude Code.

## Why now

The owner answered Q1 to Q9 of the design with the recommended option of each (word of 2026-11-25), and asked for the build in 0.8.1. Today a subagent returns a report in free form, and the main session cannot tell a done task from a stop on a question. An agent can also repeat a failed fix with no count. The texts of `execute` must be final before the paid reruns of `execute` and `review` in this part.

## Out of scope

- The cost report and `outline` (items 21 and 22): the change `harness-tools`.
- The team of this repository (`.claude/agents/` and `CLAUDE.md`): a separate change of the owner.
- A hook on the tool `Agent` (Q2 c), a question at the fix limit (Q6 b), a file of lessons (Q7 b and c) and a guard of the lesson targets (Q9 b).
- The frozen texts of `specify`, `finish`, `write-plan`, `review` and `acceptor.md`.
- A preset of the 2 keys in a current configuration: `upgrade` and `init --reconfigure` keep the keys of the file.
