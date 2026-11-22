---
name: spec-dna-write-plan
description: In a spec-dna project, before any code, write plan.md with its files and small tasks.
---

When to use: Before the code of a change.

Commands:

1. Run `spec-dna next <slug>`. Run `spec-dna context <file>...` for the files to change.
2. Run `spec-dna new plan <slug>`.
3. Under `Files`, list each path of the scope in a code span.
4. Under `Order`, write one task for each step of 2 to 5 minutes. Give it a stable id, at most `max-task-files` exact paths (default 3) and one `Verify:`. Use this form: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
5. Write `Risks`. Write `Proof` with the commands that show the result and the `covers:` ids.
6. A task can end with `Hours: <n>` or `Hours: <a>-<b>`: the estimate. When the task is done, add `Actual: <n>`: the real hours. `spec-dna finish` adds them and prints the line `Hours:`. For a spike, write one task for each branch. After the spike, add `Skip: <reason>` to each task that is not needed, and tick it.
7. Write no task for `spec-dna finish`: it runs after the last task. If the owner reviews the code before `finish`, end the plan with `- [ ] tN: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: <n>.` Only the owner ends this task: an agent never ticks it and never writes `Skip:` for it. After the review, a person ticks it with `Actual: <n>`. Then `spec-dna finish <slug>` runs, and its result goes in a commit of the same pull request.
8. To change the plan after its approval, set `status: draft` in the commit of the edit. A hook that runs `spec-dna check` rejects that commit (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on.

Artifact: `plan.md` with `Files`, `Order`, `Risks` and `Proof`.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). SD923 (a task with no exact `Files:` or no `Verify:`) and SD924 (too many paths) report only after the plan is `approved`. Before the approval, count the paths. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Split a task that lists more than `max-task-files` paths. Give each task exact paths and one `Verify:`. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v0.7.1 h:02b775af9e46b045 -->
