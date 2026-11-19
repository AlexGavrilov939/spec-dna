---
name: spec-dna-finish
description: In a spec-dna project, when a change is ready, run spec-dna finish to apply its delta.
---

When to use: The change is ready to apply. Its tasks are done, and its `Proof` passes. Its intent, spec and plan, with their approval commits, are merged in an earlier pull request.

Commands:

1. Run `spec-dna finish <slug> --dry-run`. Read each step and each stop.
2. Run `spec-dna finish <slug>`. It applies the delta, deletes the folder of the change and stages the paths. It makes no commit.
3. Commit the result. If `spec-dna finish` prints a line `Hours: <total>`, put it into the commit message. Keep it in the squash message.

Artifact: The result of `spec-dna finish`: the delta applied, and the folder of the change deleted.

Check: `spec-dna finish <slug>` ends with exit code 0, and `spec-dna check --base origin/HEAD` gives no error. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: `finish` names the step that stopped it, and it changes nothing. Fix that step, then run `spec-dna finish <slug>` again. If a status stops it, ask the approver. SD910 reason 7 means that the branch added the artifacts and finished the change. Put `finish` in a new pull request. Do not delete the folder by hand.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v0.7.0 h:2f97a5f7c620e4ae -->
