---
name: spec-dna-write-plan
description: In a spec-dna project, before any code, write plan.md with its files and small tasks.
---

When to use: Before the code of a change.

Commands:

1. Run `spec-dna next <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Run `spec-dna new plan <slug>`.
3. Under `Files`, list each path of the scope. Write each path in a code span.
4. Under `Order`, write one task for each step of 2 to 5 minutes. Give it a stable id, at most `max-task-files` exact paths (default 3) and one `Verify:`. Use this form: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
5. Write `Risks`. Write `Proof`: the commands that show the result, and the `covers:` ids of the tests.
6. A task can end with `Hours: <n>` or `Hours: <a>-<b>`. `spec-dna finish` adds them and prints the line `Hours: <total>`. For a spike, write one task for each branch. After the spike, add `Skip: <reason>` to each task that is not needed, and tick it.

Artifact: `plan.md` with `Files`, `Order`, `Risks` and `Proof`.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). SD923 (a task with no exact `Files:` or no `Verify:`) and SD924 (a task with too many paths) report only after the plan is `approved`. Before the approval, count the paths and the `Verify:` of each task. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Split a task that lists more than `max-task-files` paths. Give each task exact paths and one `Verify:`. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v0.7.0 h:291bb8821e942534 -->
