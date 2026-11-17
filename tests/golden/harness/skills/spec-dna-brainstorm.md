---
name: spec-dna-brainstorm
description: In a spec-dna project, when a request starts a change, write intent.md with its class.
---

When to use: A request starts a change.

Commands:

1. Run `spec-dna new intent <slug>`. Add `--template prd` for a product requirement, or `--template rfc` for a design proposal.
2. Replace each placeholder, also in the title. Write the intent, why now and what is out of scope, in the words of the request.
3. Set `class` in the front matter to `routine` or `higher-risk`. If you are not sure, use `higher-risk`. Set it before the approval commit, because that commit changes the `status` line only.
4. Run `spec-dna next <slug>`. It names the next step.

Artifact: `intent.md` with the intent, why now, out of scope and its class.

Check: `spec-dna check` gives no SD306 (a placeholder stays).

If the check fails: Replace each placeholder that the message names. Ask the person for each fact that the request does not give. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:2d51709930081a21 -->
