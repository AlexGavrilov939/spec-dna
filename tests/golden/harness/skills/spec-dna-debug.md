---
name: spec-dna-debug
description: In a spec-dna project, when a test or command fails and the cause is not known, find the cause before you fix it.
---

When to use: A test or a command fails, and you do not know the cause.

Commands:

1. Make the failure repeat with one command.
2. Find the cause. Do not change the code yet.
3. Write a regression test for the cause, then make one fix. After 3 failed fixes, stop and ask the person.

Artifact: A reproduction, a regression test and one fix.

Check: The regression test and the earlier tests pass.

If the check fails: Go back to the cause. Do not try a new fix without a new cause.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:2b64b5efa965acb7 -->
