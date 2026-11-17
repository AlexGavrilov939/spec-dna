---
name: spec-dna-write-plan
description: In a spec-dna project, before any code, write plan.md with its files and small tasks.
---

When to use: Before the code of a change. The intent is `accepted`, and a higher-risk change has an approved spec.

Commands:

1. Run `spec-dna next <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Run `spec-dna new plan <slug>`.
3. Under `Files`, list each path of the scope. Write each path in a code span.
4. Under `Order`, write one task for each step of 2 to 5 minutes. Give it a stable id, at most 3 exact paths and one `Verify:`. Use this form: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
5. Write `Risks`. Write `Proof`: the commands that show the result, and the `covers:` ids of the tests.

Artifact: `plan.md` with `Files`, `Order`, `Risks` and `Proof`.

Check: `spec-dna check` gives no SD913 (a file that `Files` does not list), no SD923 (a task with no exact `Files:` or no `Verify:`) and no SD924 (a task with too many paths).

If the check fails: Split a task that lists more than 3 paths. Give each task exact paths and one `Verify:`. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:c54d65cd2b11e95f -->
