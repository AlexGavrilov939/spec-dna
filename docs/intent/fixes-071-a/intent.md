---
status: accepted
class: routine
---
# Fixes of 0.7.1, part A: the open-task gate, the skill evals and the harness rules

## Intent

Fix the defects that the owner and the full run of 0.7.0 found, and stop before the tag, so that the owner can add the findings of the second project. The work has 7 items:

- An open task gates a change. With 0 of 27 tasks done and a `covers:` tag for each delta requirement, `next` gave the stage `deploy` and `finish --dry-run` passed. The stage stays `build` for each plan while a task of `Order` is open, and `finish` stops at step 0 and lists the open tasks.
- The skills `finish` (12 of 15 runs against 15 of 15 without the skill, issue 67) and `verify` (4 of 10 against 7 of 10, issue 68) scored below no skill. `failure-analyst` proved the causes: 3 faults of the `verify` evals and, most probably, a stop sentence of the `finish` skill.
- The evals of `brainstorm` and `execute` were passed by both arms in each run (issue 69). They get a trap that a run without the skill falls into.
- The verdicts of `tdd`, `verify` and `agents-line` rest on 2 runs for each eval. Each target that the work touches is run again with 15 runs for each arm, and the table of the release is rebuilt.
- The 200 pilot records have `"tokens": null` (issue 70).
- SD606 warns on a document that names `docs/intent/` after `finish` deleted the last change folder.
- Feedback of a second project: a red test commit against a hook that runs the tests, a subagent that deleted the shared scratchpad, a browser tool that wrote into the repository, and no place in the hour log for the final review of the owner.

## Why now

The gate lets a change pass with no work done, and 3 skills have a verdict that rests on faulty evals. The owner adds the findings of the second project before the tag, so this part ends with a report and no version change.

## Out of scope

- The version, the tag and the release. The owner decides them after the second project.
- A change of the runner, of the verdict rules and of the cap of an eval run.
- The hook runner of a project, and the tools of a browser. The text only tells where the files go.
