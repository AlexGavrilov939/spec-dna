---
name: spec-dna-verify
description: In a spec-dna project, before you say that the work is done, run the proof and read its output.
---

When to use: Before you say that the work is done, or that a task or a fix works.

Commands:

1. Run each command of `Proof` in `plan.md`. Read the whole output of each command.
2. Run `spec-dna check --base origin/HEAD`. If git has no such ref, pass the default branch of the project as the base.
3. In your answer, name each command that you ran and each command that you did not run.

Artifact: The output of the `Proof` commands and of `spec-dna check`.

Check: Each command ends with exit code 0, and the check gives no error. `finish` stops on SD915 (the tree lacks the result of the delta) and on SD916 (a requirement has no evidence).

If the check fails: Find the cause. Fix it, and run all the commands again. Do not say that the work is done.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:974595530a7cf940 -->
