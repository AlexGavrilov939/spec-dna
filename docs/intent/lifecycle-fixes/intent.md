---
status: draft
class: higher-risk
---
# Fixes of the change lifecycle: finish, scope, the acceptor, layers and small items

## Intent

Fix 6 defects of the change lifecycle that the use of 0.8.0 found (design of lifecycle and harness, slices A1, A2, A3, A5 and A6):

- SD915 passes the pull request of `finish` of a change with no delta when each task of its plan is closed (D87). SD401 on a spec or a plan of a change gives the steps of the rework (D91).
- SD913 runs only for a change whose folder the diff touches or that the branch added, and `stats` binds the test reports only when the working tree has the code tree of `HEAD` (D92).
- `next` and `accept` stop before the acceptor while an item of `Deviations` or `Invariants` is not valid, and SD939 names the fix of a repeated id (D88). The fallback of `accept` diffs from the first parent of the commit that added the plan (Q8 of the design).
- `layer add` updates the adapter skill with `spec-dna.toml` (D90).
- 5 small items: the last line of `upgrade`, the steps of the hook in the acceptor file, packages in `Files`, the title rule, and a note for a test path of `Proof` that `Files` does not list (D93).

## Why now

The change `team-measurement-2` could not finish in a pull request of its own: SD915 rejected it, so it closed in 2 more pull requests and `log` lists no finished change. A small fix that edits a file of an open change gets SD913. The first commit of an acceptor failed on a plan with 2 deviations of one task, and a repeated id is the likely cause. `layer add summaries` leaves SD502 until `init` runs. The text of SD401 contradicts the skills and the guide. `finish` writes the title `# Mcp`.

## Out of scope

- The context meter and the briefs of the agents (D94), the eval runner (D95, D100), the measurement of the acceptor as a subagent (D96), and the default and the rule of the advisor (D97, D98).
- The texts of the skills `specify`, `write-plan` and `tdd`: they are frozen for the verdicts. The skill `execute` gets one sentence, because the freeze table does not have it.
- The page `docs/design/0.8.1-harness-additions.md`: it is a design only.
- An exception of SD503 for a new layer: the path stays 2 pull requests.
