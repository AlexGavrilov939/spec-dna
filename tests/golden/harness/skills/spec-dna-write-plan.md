---
name: spec-dna-write-plan
description: In a spec-dna project, before any code, write plan.md with its files and small tasks.
---

When to use: Before the code of a change.

Commands:

1. Run `spec-dna next <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Run `spec-dna new plan <slug>`.
3. Under `Files`, list each path of the scope. Write each path in a code span.
4. Under `Order`, write one task for each step of 2 to 5 minutes. Give it a stable id, at most 3 exact paths and one `Verify:`. Use this form: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
5. Write `Risks`. Write `Proof`: the commands that show the result, and the `covers:` ids of the tests.

Artifact: `plan.md` with `Files`, `Order`, `Risks` and `Proof`.

Check: `spec-dna check --base origin/HEAD` gives no SD913, SD923 and SD924. SD913 is a file that `Files` does not list. SD923 is a task with no exact `Files:` or no `Verify:`. SD924 is a task with too many paths.

If the check fails: Split a task that lists more than 3 paths. Give each task exact paths and one `Verify:`. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v{VERSION} h:adc3ce5fffd0490e -->
