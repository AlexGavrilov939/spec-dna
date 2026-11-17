---
name: spec-dna-review
description: In a spec-dna project, review the diff in 3 passes before a merge, and check each review comment.
---

When to use: Before a merge, or when a person asks for a review or answers one.

Commands:

1. Find the base with `git merge-base HEAD origin/HEAD`. Read the diff with `git diff <base>`.
2. Run `spec-dna status`. Read `spec.md` and `plan.md` of the change with `spec-dna show <id>`.
3. Make the 3 passes below. Check each review comment against the code before you act on it.

Artifact: The findings, each with its severity, file and line.

Check: `spec-dna check` runs in CI. Do not report its findings.

If the check fails: If you find nothing, say so, and list what you read.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

## Passes

Review the diff in 3 passes. For each finding, give the file and the line.

1. Bugs: a logic error, a missing check, a wrong edge case, and a test that does not prove the change.
2. Security: a leaked secret, an unsafe input, and an access that the change does not need.
3. Compliance: compare the diff with `spec.md` and `plan.md` of the change, and with the documents of the layer `rules`. Report a broken requirement, a delta line that the code does not meet, and a file that the `Files` of the plan do not list.

## Severity

- Important: a finding that breaks behavior, leaks data or breaches a policy.
- Nit: each other finding. Report at most 5 Nits in one review.

## Do not report

- A finding of `spec-dna check`. CI runs it.
- The text of a file that spec-dna generates: each file with a `spec-dna harness` marker, `.claude/skills/spec-dna/SKILL.md`, the spec-dna lines of `CLAUDE.md` and `AGENTS.md`, and the entries of `.claude/settings.json` for `model`, `modelSettings`, `fallbackModel`, `advisorModel`, `permissions.deny`, `env.CLAUDE_CODE_SUBAGENT_MODEL` and each `spec-dna hook` command.

<!-- spec-dna harness v{VERSION} h:81adebdb083bc380 -->
