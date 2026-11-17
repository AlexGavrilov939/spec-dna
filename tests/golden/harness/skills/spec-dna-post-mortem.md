---
name: spec-dna-post-mortem
description: In a spec-dna project, turn an incident with an open follow-up into a new intent and an eval.
---

When to use: An `incident` has an open follow-up.

Commands:

1. Run `spec-dna status`. Find the incident.
2. Run `spec-dna new intent <slug>`. Write `from: <incident id>` in its front matter.
3. Write an eval file for the incident in the evals folder.

Artifact: A new `intent.md` with `from:`, and an eval file.

Check: `spec-dna check` gives no SD918.

If the check fails: Add or fix the eval file. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:6795045e9acb071b -->
