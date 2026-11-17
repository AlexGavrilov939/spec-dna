---
name: spec-dna-brainstorm
description: In a spec-dna project, when a request starts a change, write intent.md with its class.
---

When to use: A request starts a change.

Commands:

1. Run `spec-dna new intent <slug>`. Add `--template prd` for a product requirement, or `--template rfc` for a design proposal.
2. Write what the change gives, why now and what it does not do, in the words of the request. Set `class` to `routine` or `higher-risk`. If you are not sure, use `higher-risk`.
3. Run `spec-dna next <slug>`.

Artifact: `intent.md` with its class.

Check: `spec-dna check` gives no SD306 (a placeholder stays).

If the check fails: Replace each placeholder. Ask the person for each fact that the request does not give. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:1cf53f3eb7f1ab9a -->
