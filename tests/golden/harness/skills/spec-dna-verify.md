---
name: spec-dna-verify
description: In a spec-dna project, run the proof and read its output before you say that the work is done.
---

When to use: Before you say that the work is done.

Commands:

1. Run each command of `Proof` in `plan.md`. Read its whole output.
2. Run `spec-dna check`.

Artifact: The output of the commands.

Check: Each command passes, and `spec-dna check` gives no error. `finish` stops on SD915 and SD916.

If the check fails: Find the cause. Fix it, and run the commands again. Do not say that the work is done.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:a2f47ab8f4a030ef -->
