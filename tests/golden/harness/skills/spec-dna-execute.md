---
name: spec-dna-execute
description: In a spec-dna project, when the plan is approved and a task is open, run the task with a fresh subagent.
---

When to use: The plan is `approved` and a task is open.

Commands:

1. Run `spec-dna next <slug>`. Take the first open task.
2. Start one fresh subagent for the task. It changes only the files of the task.
3. Review the diff in 2 passes: compliance with the task, `spec.md` and the rules, then quality.

Artifact: The code, the tests and the checked task.

Check: The `Verify:` of the task passes. `spec-dna check` gives no SD913.

If the check fails: Fix the cause in the same task. If a file is outside `Files`, ask the person to change the plan.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:b00dfd3870c59a2f -->
