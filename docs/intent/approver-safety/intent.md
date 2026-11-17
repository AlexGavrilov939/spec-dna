---
status: accepted
class: higher-risk
---
# Approver safety

## Intent

Make sure that an approval comes from a person, and that the rule for an agent is the same in each project. This change closes items 1 to 3 of the owner review of 0.7.0 part A:

- `init` never writes the address of an agent or a bot into `[change] approvers`. One list in the binary holds these addresses. Interactive `init` asks for the approvers. `init --approver <e-mail>` gives them in a script. With no person to list, `init` writes `approval = "base"` and a note.
- The new code SD926 warns about an agent or bot address in `approvers`.
- The new command `spec-dna approve <slug> <artifact>` sets the `status` line of one artifact and commits that file alone. It is for a person.
- The new key `[harness] protect-approvals`, which `init` writes for `strict`, adds a hook. The hook denies an agent call of `spec-dna approve`, and an agent commit that sets an approval status.
- The rule "an agent writes an approval commit only when the owner says so explicitly in the chat" is the default of each project. The change guide says how to change it, and it loses a repeated sentence.

## Why now

In a cloud agent session, `git config user.email` is the address of the agent, and `init` makes it the approver. Locally, an agent commits with the e-mail and the key of the owner, so `check` cannot tell its approval from the approval of the owner. The release of 0.7.0 ships the approval in the branch to each project.

## Out of scope

- The length of the generated line of `AGENTS.md` (item 6). This change does not change the bytes of the line.
- A proof of the identity of a person. The code host stays the control (ADR 0012).
- The hook for an agent other than Claude Code.
- The risks of part A (item 7): the change `risks-of-part-a`.
