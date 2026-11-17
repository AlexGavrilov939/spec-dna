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

Check: `spec-dna check --base origin/HEAD` gives no SD918 and no SD101. SD918 is an incident with no eval. SD101 is an incident id that names no document.

If the check fails: Add or fix the eval file, or fix the id in `from:`. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v{VERSION} h:e6bf3d5b7a548885 -->
