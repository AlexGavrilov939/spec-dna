---
status: draft
class: higher-risk
---
# Approval in the branch

## Intent

Add the approval in the branch of [ADR 0012](../../adr/0012-approval-in-the-branch.md) (decisions D11 to D17 of [the design](../../design/0.7.0.md)). With `[change] approval = "branch"`, `check` walks the commits of the branch. An approval commit of an approver then counts before the work that it approves. A change then needs 2 pull requests, not 3 or 4.

The change also adds these items:

- The keys `[change] approval` and `[change] approvers` (D15), and the errors of D14 and D15.
- The field `approvals` of `check --json`, and the line of `next` for an approval in branch mode (D7).
- In `status` (D8): the source of each approval, and the skill `post-mortem` for an incident with an open follow-up.
- `init` writes the keys as D15 says.
- Items 1 and 2 of the feedback of a second project (section 15 of the design): a `draft` artifact needs no earlier approval, and `next` and `status` ask for a class.
- The rule of Q5 in `AGENTS.md` and `CONTRIBUTING.md`: an agent writes an approval commit only when the owner says so explicitly in the chat.

## Why now

Each pull request of 0.7.0 after this slice can use the new rule. The change of 0.6.1 took 4 pull requests for one routine change.

## Out of scope

- The pull request that sets `approval = "branch"` and the approvers in `spec-dna.toml` of this repository. The main session makes it as a task of its own, because SD503 rejects a change of the configuration with other files.
- `[change] max-task-files` and the codes SD923, SD924 and SD925 (slice 3a, `task-checks`).
- The profiles at `init` beyond the keys of D15, and `upgrade` (slice 8).
- A new diagnostic code: SD910 reports each case.
