---
status: proposed
---

# 0018. Approval by a person

## Context and Problem Statement

[ADR 0012](0012-approval-in-the-branch.md) counts an approval commit when the e-mail of its author is in `[change] approvers`. The owner review of 0.7.0 part A found 3 gaps:

- In a cloud agent session, `git config user.email` is the address of the agent, and `init` copied it into `approvers`.
- Locally, an agent commits with the e-mail and the key of the owner. `check` cannot tell its approval commit from a commit of the owner.
- The generated line of `AGENTS.md` says that an agent writes an approval commit only when the owner says so. The change guide called this a rule of this repository only, and [ADR 0006](0006-change-lifecycle.md) lets the agent approve a `routine` plan.

Constraints:

- spec-dna uses no network and no API of a code host (ADR 0006).
- The Claude Code documentation does not say if a command with the `!` prefix runs the `PreToolUse` hooks. The input of a hook has no field that tells a person from an agent.
- An agent can set each variable and each flag that a person can set.
- ADR 0006 and ADR 0012 are at their hard budget, and ADR 0017 is over its soft budget. Thus this ADR holds the new rules.

## Considered Options

1. Documentation only: the person checks `approvers`.
2. A list of agent addresses in the binary, a command for the approval of a person, and a hook that denies the same act to an agent.
3. A variable that marks a person, such as `SPEC_DNA_PERSON=1`.
4. A git `pre-commit` hook that rejects an approval commit.

## Decision Outcome

Option 2.

- One list in the binary holds the author addresses of agents and bots: `noreply@anthropic.com`, and each address that ends with `[bot]@users.noreply.github.com`. A GitHub login of a person has no `[`, so the hidden address of a person does not match.
- `init` never writes an address of the list into `approvers`. Interactive `init` asks for the approvers. The e-mail of git is the default only when it is not on the list. `init --approver <e-mail>` and the answers key `approvers` give the list in a script. An address of the list is a usage error. With no person to list, `init` writes `approval = "base"` and a note.
- SD926 `approver-agent` warns about each item of `approvers` that is on the list. It is in the rule group `changes`, and it is a warning in each profile.
- `spec-dna approve <slug> <artifact>` sets the `status` line of one artifact and commits that file alone. It is for a person. With `approval = "branch"`, it stops when the author is not an approver.
- The key `[harness] protect-approvals` (a boolean) adds the hook `spec-dna hook protect-approvals` on `PreToolUse` of `Bash`. The hook denies a call that runs `spec-dna approve`, and a `git commit` that sets the status of an artifact to `accepted` or `approved`. `init` writes `true` for `strict`, so the profile has no hidden effect ([ADR 0017](0017-profiles.md)).
- The sentence "An agent writes an approval commit only when the owner says so explicitly in the chat." is the default rule of each project. The line of `AGENTS.md` and each skill hold it. A project names an exception in its own text after the generated line.
- The person runs `spec-dna approve` in a terminal of their own. With the hook, a command with the `!` prefix can be denied.

### Rejected

- Option 1: a cloud session writes the agent address, and no person sees it.
- Option 3: an agent can set the variable.
- Option 4: the git hook runs for the person and for the agent, so it cannot tell them apart.
- A pattern on `@users.noreply.github.com`: it matches each person that hides the e-mail.
- The answer `ask` of the hook: a click on a prompt becomes a habit. A run with `claude -p` denies it in any case.
- A deny of `Edit` and `Write` on the `status` line: more hook calls, and the check of the commit sees the same result.

### Consequences

- Good: no path of `init` writes an agent address, and `check` names an old one.
- Good: a person has one command that makes an exact approval commit.
- Good: in `strict`, an agent of the Claude Code adapter cannot commit an approval on its own initiative.
- Bad: the hook reads the text of the command. A script, a variable, git plumbing or an alias passes it. It guards against a mistake. It is not a security control, and the code host stays the control.
- Bad: a teammate with no spec-dna, or with an older one, gets no hook, because the command ends with `|| true`.
- Bad: the hook denies a commit while the working tree holds a status that a person set by hand.
- Neutral: the bytes of the generated line do not change.

## Amends

ADR 0006, ADR 0012 and ADR 0017 are accepted and locked, so this ADR does not edit them. These rules apply in place of the named text:

- ADR 0006, "Who approves": the agent approves a `routine` plan on the word of the owner, by default. With `protect-approvals`, a person approves each artifact with `spec-dna approve`.
- ADR 0012, "The identity of the approver" and "Consequences", and D12 of the design: by default, `approvers` lists persons only. The advice to list the e-mail of the agent for a `routine` change ends. `init` never lists an agent address, and SD926 warns about one. A project that wants an agent identity to approve a `routine` plan sets `[rules] SD926 = "off"`. The e-mail stays no proof.
- ADR 0017, the table of the profiles: `strict` also writes `protect-approvals = true`, and SD926 is a warning in each profile.

## Amendments
