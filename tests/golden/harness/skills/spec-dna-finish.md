---
name: spec-dna-finish
description: In a spec-dna project, when a change is ready, run spec-dna finish to apply its delta.
---

When to use: The change is ready to apply. Its tasks are done, and its `Proof` passes. Its intent, spec and plan, with their approval commits, are merged in an earlier pull request.

Commands:

1. If `next` names a due summary, edit its text before `finish` when its meaning changes. Then run `spec-dna refresh <id>` on a higher summary that pins it, with `--holds` if its meaning holds. Run `spec-dna finish <slug> --dry-run` alone: one plain command with no other command after it. If git has no `origin/HEAD`, add `--base <rev>`. Read each step and each stop.
2. Run `spec-dna finish <slug>`. It applies the delta, deletes the folder of the change and stages the paths. It makes no commit. Name each other due summary with `--holds <id>`, one option for each id. If it prints `Deviations: <n>` and one line for each, tell the owner: the merge acknowledges them, or, with `approval = branch`, an approver does it with `spec-dna approve <slug> acceptance`.
3. Commit the result. If `spec-dna finish` prints a line `Hours:`, put it into the commit message. Keep it in the squash message.

Artifact: The result of `spec-dna finish`: the delta applied, and the folder of the change deleted.

Check: `spec-dna finish <slug>` ends with exit code 0, and `spec-dna check --base origin/HEAD` gives no error. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: A command that the tool denies is no stop of `finish`: run the plain command again. A stop is a line `stopped at step N` that `spec-dna finish` prints. The stop changes nothing. Fix that step, then run `spec-dna finish <slug>` again. A stop at step 0 lists each open task of `plan.md` (do the task, or write a reason in `Skip:`) or each reference into the folder. If the stop names a status, ask the approver. If `spec-dna finish` prints no stop, read its message: it can name an edit that is not committed. If `spec-dna check --base origin/HEAD` gives SD910 reason 7, the branch added the artifacts and finished the change: put `finish` in a new pull request. Do not delete the folder by hand. If `spec-dna finish` prints `roll-up: stale <id>`, tell the owner: it needs an edit and `spec-dna refresh <id>` in a new commit, or `spec-dna refresh <id> --holds` if its text holds.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.

<!-- spec-dna harness v{VERSION} h:e044b30d108e6825 -->
