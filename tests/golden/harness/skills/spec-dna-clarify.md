---
name: spec-dna-clarify
description: In a spec-dna project, when spec.md has an open concern or question, close each one before the spec is approved.
---

When to use: `spec.md` has an open concern or an open question.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Give each item a box: `- [ ]` is open, and `- [x]` is resolved. If no item applies, write one item `- none`.
3. Ask the person about each open item. Write the answer in the item, then change its box to `- [x]`.
4. Run `spec-dna check --base origin/HEAD`.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: `spec-dna check --base origin/HEAD` gives no SD919, SD922 and SD925. SD919 is an open concern, SD922 is an open question, and SD925 is an item with no box.

If the check fails: Ask the person about the item that the message names. Do not mark an item that has no answer. Add a box to an item that has none.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v{VERSION} h:6ccb96b5c35a05c9 -->
