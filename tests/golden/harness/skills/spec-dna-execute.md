---
name: spec-dna-execute
description: In a spec-dna project, when the plan is approved and a task is open, run the task with a fresh subagent.
---

When to use: The plan is `approved` and a task is open.

Commands:

1. Work in your own git worktree or branch. Run `spec-dna next <slug>`.
2. Take the first open task. Start one fresh subagent for it, and give it the task and `spec-dna context <file>...` for its files. It changes only the files of the task.
3. Review the diff in 2 passes. Pass 1, compliance: compare the diff with the task, `spec.md` and the rules of the project. Pass 2, quality: compare it with the style of the code, the tests and the simplicity.
4. Fix each gap that a pass finds. Run the `Verify:` of the task. Then change its box to `- [x]`.

Artifact: The code, the tests and the checked task.

Check: The `Verify:` of the task passes. `spec-dna next <slug>` names the next open task, and the stage stays `build` until each task is `- [x]` or has a reason in `Skip:`. Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD913 (a file that `Files` does not list). Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`. The hook `protect-tests`, if it is on, asks the person before an edit of a test file.

If the check fails: Fix the cause in the same task. If a file is outside `Files`, stop and ask the person to change the plan.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v{VERSION} h:00f6fd3bc3d1572a -->
