---
name: spec-dna-finish
description: In a spec-dna project, when a change is ready, run spec-dna finish to apply its delta.
---

When to use: The change is ready to apply.

Commands:

1. Run `spec-dna finish <slug> --dry-run`.
2. Run `spec-dna finish <slug>`.
3. Commit the result.

Artifact: The result of `spec-dna finish`: the delta applied, and the folder of the change deleted.

Check: `spec-dna finish` ends with exit code 0, and `spec-dna check` gives no error.

If the check fails: `finish` names the step that stopped it. Fix that step, and run `spec-dna finish <slug>` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:d1c765149dba9ffb -->
