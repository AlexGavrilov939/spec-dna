---
name: spec-dna-write-plan
description: In a spec-dna project, write plan.md with its files and small tasks before any code.
---

When to use: Before the code of a change.

Commands:

1. Run `spec-dna new plan <slug>`. Run `spec-dna context <file>...` for the files that you plan to change.
2. Under `Files`, list each path of the scope. Under `Order`, write one small task for each step: `- [ ] <id>: <text>. Files: <paths>. Verify: <check>.`
3. Write `Risks` and `Proof`.

Artifact: `plan.md` with `Files` and small tasks.

Check: `spec-dna check` gives no SD913, SD923 and SD924.

If the check fails: Split a task that lists too many paths. Give each task exact paths in `Files:` and one `Verify:`. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:e67254dc5d582c7d -->
