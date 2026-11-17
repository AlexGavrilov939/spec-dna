---
status: approved
---
# Feedback of a second project: spec

## Delta

None. This change adds, changes or removes no requirement. It changes when SD913 runs, adds the partial tag, a JSON field of `show` and a row of `stats --sdlc`, and removes the default budget of `plan`. Section 15 of [the design](../../design/0.7.0.md) and the docs of the build state the rules.

## Interfaces

- None: no interface document describes `check`, `show`, `status` or `stats`.

## Decisions

- [[adr/0001-information-model]]
- [[adr/0006-change-lifecycle]]
- [[adr/0012-approval-in-the-branch]]

## Concerns

- [x] Code that a branch edits before a plan exists gets no SD913 and no SD910. Resolved: SD913 checks the scope of a plan, and a change has no scope before its plan. A spike before the plan is such code. The review is the control.
- [x] A diff has a change with no plan and a change with a plan. Resolved: SD913 then reads the scopes of all changes, so the code of the planned change keeps SD913.
- [x] One branch holds the edits of `clarify` and then the plan. Resolved: SD913 reports those edits at the commit of the plan. The guide says to list them in `Files`, or to merge them in a pull request of their own.
- [x] A binary before 0.7.0 reads a partial tag. Resolved: it reads no id after the word `partial`, so it reads no evidence. The lab run proved it.
- [x] A word after the ids, such as `covers: a#b (partial)`. Resolved: rejected, because 0.6.1 reads it as full evidence. The lab run proved it.
- [x] A plan with no budget has no size limit. Resolved: `[budgets.plan]` sets one. The `intent` layer is `on-request`, and `next` serves one task.
- [x] An edit of the delta in the pull request of `finish`. Resolved: step 3 stays, also after a new approval commit. A squash merge keeps no commit of the branch, so `log` reads the delta of the base tree.

## Open questions

- [x] Does a partial tag clear SD608? Answer: no. Each reader of the evidence uses one rule: a partial tag is not evidence.
- [x] How does `status` show it? Answer: the token `partial <n>` and the field `partial` of `status --json`, because `status` prints only the last item of `next`.
- [x] Where does `stats --sdlc` show it? Answer: in a new row after "Regressions caught in CI", play `Test`, kind `lagging`. The value is the share of the requirements that need evidence and have a partial tag only.
- [x] Does a delta that waits for a spike need new syntax? Answer: no. A lab run with the binary of the tree proved the steps of the guide.
- [x] Does the partial tag change `PARSER_VERSION`? Answer: no. `git grep` reads the tags on each run, and the cache holds no tag.
