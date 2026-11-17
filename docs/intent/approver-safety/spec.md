---
status: draft
---
# Approver safety: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the command `approve`, the flag `init --approver`, the answers key `approvers`, the key `[harness] protect-approvals`, the hook `protect-approvals` and the code SD926. Section 15 of [the design](../../design/0.7.0.md) and the docs of the build state the rules.

## Interfaces

- None: no interface document describes `init`, `check`, `next`, the hooks or the configuration.

## Decisions

- [[adr/0006-change-lifecycle]]
- [[adr/0008-harness-and-evals]]
- [[adr/0012-approval-in-the-branch]]
- [[adr/0017-profiles]]

## Concerns

- [x] The hook reads the text of a command, and it is not a shell parser. An agent can pass it with a script, a variable or git plumbing. Resolved: the hook guards against an agent that acts on its own, not against an attack. `docs/harness.md` states each limit, and the code host stays the control (ADR 0012).
- [x] The Claude Code documentation does not say if a command with the `!` prefix runs the `PreToolUse` hooks, and the hook input does not tell a person from an agent. Resolved: the guide tells the person to run `spec-dna approve` in a terminal of their own. The design is correct both ways, and no text claims a behavior of `!`.
- [x] A variable that turns the hook off for a person would also work for an agent. Resolved: the design has no such variable.
- [x] `init --approver <agent address>` gives exit code 2, so a script that copies the e-mail of git can stop. Resolved: the owner asked that `init` never writes such an address. The message names the address and the fix.
- [x] The default rule restricts the routine plan of ADR 0006: the agent approves it on the word of the owner. Resolved: ADR 0018 states it, and the guide says how a project names an exception. ADR 0006 and ADR 0012 are at their hard budget, so a new ADR holds the rules.
- [x] The hook denies an unrelated commit while the working tree holds a status that a person set by hand. Resolved: `docs/harness.md` names the case. The person commits the status, or runs `spec-dna approve`.

## Open questions

- [x] Which code reports an agent address in `approvers`? Answer: SD926 `approver-agent`, in the rule group `changes` (off with `policy = "none"`), a warning in each profile. The 5xx range holds the codes of files, and a code below 600 is an error by the rule of `src/diag.rs`.
- [x] Does the hook let an agent approve a routine plan? Answer: no. With `protect-approvals`, a person approves each artifact.
- [x] Does `spec-dna approve` check the author with `approval = "base"`? Answer: no. The merge is the approval there. With `"branch"`, the author must be in `approvers`, else the command stops with exit code 1.
- [x] Does a profile turn the hook on by itself? Answer: no. `init` writes `protect-approvals = true` for `strict`, and the harness reads the key (D37).
- [x] Do the bytes of the generated line change? Answer: no. The sentence of the rule stays as it is.
- [x] Does `next` name `spec-dna approve`? Answer: no. An agent reads `next`, and it commits with the e-mail of the owner, so a ready command invites it to approve. The line keeps its text.
- [x] Can a project list the identity of an agent to approve a routine plan? Answer: yes, with `[rules] SD926 = "off"`. ADR 0018 amends the advice of D12 and ADR 0012: by default, `approvers` lists persons only.
