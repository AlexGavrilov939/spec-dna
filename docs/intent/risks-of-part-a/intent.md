---
status: draft
class: higher-risk
---
# Risks of part A

## Intent

Close the open items of the review of 0.7.0 part A (item 7 of the owner task):

- SD503: the compare of the pin line reads `\r\n` as `\n`. A `Cargo.toml` that is a symbolic link makes no tree binary, in the working tree and with `--staged`.
- SD910: an approval of the branch clears work only while the artifact keeps its approval status at the head. A plan that a commit sets back to `draft` after the code gives SD910 with a new reason 9.
- SD923: `[` and `]` are plain characters in a path of a task, as for each other reader of the paths. Only `*` and `?` make a glob.
- The walk of the branch reads the paths of all its commits with a fixed number of git processes, not one `git diff-tree` for each commit. `bench/git-access` records the cost before and after.

## Why now

Each item is a known defect of a contract that 0.7.0 ships. A branch that sets its plan back to `draft` passes with `approval = "branch"` and fails with `"base"`. SD923 rejects a common path of web frameworks, such as `app/[id]/page.tsx`. The walk of a branch of 500 commits takes 2.9 to 7.3 seconds of each `check`.

## Out of scope

- A new diagnostic code. SD910 keeps its code, and the message names the new reason.
- A character class in the globs of spec-dna.
- The approver rules of items 1 to 3: the change `approver-safety`.
