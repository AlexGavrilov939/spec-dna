---
name: spec-dna-post-mortem
description: In a spec-dna project, when an incident has an open follow-up, write a new intent and an eval for it.
---

When to use: An `incident` has an open follow-up.

Commands:

1. Run `spec-dna status`. It lists each open follow-up of an incident.
2. Read the incident with `spec-dna show <id>`.
3. Run `spec-dna new intent <slug>`. Add the line `from: <incident id>` to its front matter, and set `class`.
4. Write the intent: what the change gives, why now and what is out of scope.
5. Add an eval for the incident: the file `<id>.json` directly in the evals folder, `[evals] dir`.

Artifact: A new `intent.md` with `from:`, and an eval file for the incident.

Check: `spec-dna check` gives no SD918 (no eval for the incident) and no SD101 (an incident id that names no document).

If the check fails: Add or fix the eval file, or fix the id in `from:`. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:d9c7861c110573a8 -->
