---
name: spec-dna-debug
description: In a spec-dna project, when a test or command fails and the cause is not known, find the cause before you fix it.
---

When to use: A test or a command fails, and you do not know the cause.

Commands:

1. Make the failure repeat with one command. Keep the command and its output.
2. Find the cause. Read the code and the output. Do not change the behavior yet.
3. Write a regression test that fails for this cause. Run it.
4. Make one fix. Run the regression test and the earlier tests.

Artifact: A reproduction, a regression test and one fix.

Check: The regression test passes, and the earlier tests pass.

If the check fails: Go back to step 2. Make no new fix without a new cause. After 3 failed fixes, stop and ask the person.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.

<!-- spec-dna harness v{VERSION} h:bf7e54fc92ace4f3 -->
