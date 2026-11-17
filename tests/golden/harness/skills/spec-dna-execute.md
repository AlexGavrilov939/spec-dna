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

Check: The `Verify:` of the task passes, and `spec-dna check` gives no SD913 (a file that `Files` does not list). The hook `protect-tests`, if it is on, stops an edit of a protected test.

If the check fails: Fix the cause in the same task. If a file is outside `Files`, stop and ask the person to change the plan.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:4fe5a2041245624a -->
