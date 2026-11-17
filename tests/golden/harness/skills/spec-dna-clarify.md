---
name: spec-dna-clarify
description: In a spec-dna project, when spec.md has an open concern or question, close each one before the spec is approved.
---

When to use: `spec.md` has an open concern or an open question.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Ask the person about each open item. Write the answer in the item, and mark its box `[x]`.
3. Give each item a task box, `- [ ]` or `- [x]`.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: `spec-dna check` gives no SD919, SD922 and SD925.

If the check fails: Ask the person about the item that the message names. Do not mark an item that is not answered.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:542f0523a687163a -->
