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
6. In the incident, change the open follow-up to `- [x]`, and name the new intent in it.

Artifact: A new `intent.md` with `from:`, and an eval file for the incident.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD101 (a `from:` that names no document), no SD306 and no SD506 (an invalid eval file). SD918 (an incident with no eval) reports only when the diff adds the incident, so check by hand that the eval file exists. Run `spec-dna status`: it lists no open follow-up for the incident. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Add or fix the eval file, or fix the id in `from:`. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v0.7.1 h:7831d39f3f129c7f -->
