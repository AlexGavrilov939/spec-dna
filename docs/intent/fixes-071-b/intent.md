---
status: draft
class: higher-risk
---
# Fixes of 0.7.1, part B: the findings of a second project and the release

## Intent

Fix the defects that a second real project found on 0.7.0, close the 3 review items of part A, and release 0.7.1. The work has 11 items. The design has the decision of each item ([design 0.7.1, part B](../../design/0.7.1-b.md)).

- A plan can end with a task that runs `finish`. That task closes with `finish`, so `finish` never waits for a task that only `finish` can do.
- A spell checker in pre-commit can rewrite the hex hashes of the markers. The docs give an exclude pattern, and `init` and `upgrade` print it.
- An upgrade is one commit that passes the hooks and CI. SD503 and SD913 do not report a pin that moves to a later version.
- `upgrade` keeps the default skills of 0.7.0, and its report of decisions with no status reads the front matter.
- An approved plan of an earlier version gets one note, not a warning for each task.
- A spec that returns to `draft` has a documented way through the hooks.
- `finish` keeps the actual hours, and `log` shows them.
- A change can edit the shared documents of the list `[change] shared-paths` without SD913.
- `check --base <older tag>` gives no SD910 for work that a squash merge hid.
- The `execute` skill tells where a path outside `Files` goes, and the work folder of an eval run gives no SD501 or SD502.
- The skill `write-plan` has a new run of its eval, and the release 0.7.1 follows with its tag.

## Why now

Part A is merged and the owner reviewed it. The second project found 8 defects, and the 3 review items of part A are open. The tag waits for these fixes.

## Out of scope

- A metric of the actual hours in `stats --sdlc`.
- A key `shared-paths` that `upgrade` adds. A project adds it by hand.
- More runs of the evals of `execute` and `brainstorm`. They stay at the ceiling (issue 69).
- A new run of the evals of `specify`, `execute` and `finish` after the text change. The release table keeps their verdicts, and the notes of the release say so.
