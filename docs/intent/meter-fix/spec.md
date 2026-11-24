---
status: draft
---
# The context meter reads the last executor iteration, and the briefs name sections: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the reading of the hook `context-meter`: the context of a line is the sum of the 3 token fields of the last iteration of type `message`, else of `usage`, and a line of the model `<synthetic>` does not count. The output of the hook, its levels, its state files and its messages keep their form. The agent files of this repository get new lines "Read first" and one line "Reads:". The [design](../../design/0.8.1-a-lifecycle-and-harness.md) states the contract in D94.

## Interfaces

- None: no interface document describes the hook.

## Decisions

- [[adr/0004-harness]]

## Concerns

- [x] The change sets "Hook" and "Agent team" are rows of `.claude/agents/ownership.md`, and `CONTRIBUTING.md`, section "Changes", asks for `higher-risk` on a change of a contract. Resolved: the class rule of the design (section "Slices") gives `routine` to a slice that changes no gate, no exit code and no default of a new project. The meter only advises, and the agent files are text that the owner accepts in the pull request. The owner can raise the class before the approval of the spec.
- [x] No real transcript has a line with 2 iterations, so the fix of H1 rests on a synthetic line. Resolved: with one iteration, the last iteration is the top-level usage, so the fix gives the result of 0.8.0 in each case that the lab saw. The test `meter_is_not_cumulative` pins the rule of one request.
- [x] An iteration of another type (the advisor) has its own tokens, and they are not the context of the executor. Resolved: only an iteration of type `message` counts (D94 item 2).
- [x] The line "Reads:" is a rule of text, and no check sees a read by range. Resolved: the design states the limit, and the cost is one line in the body of each agent file, not in a request of the main session.
- [x] `checker.md` has no line "Read first". Resolved: its line "Reads:" goes after its paragraph "Scope".

## Open questions

- [x] Which cause stopped the subagent (Q5 of the design)? Answer: the owner runs the script of D94 on the transcript of the stopped subagent and sends the counts. The fix covers both causes, so this change does not wait for the answer.
