---
name: spec-dna-clarify
description: In a spec-dna project, when spec.md has an open concern or question, close each one before the spec is approved.
---

When to use: `spec.md` has an open concern or an open question, and nobody has approved the spec.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Give each item a box: `- [ ]` is open, and `- [x]` is resolved. If no item applies, write one item `- none`.
3. Ask the person about each open item. Write the answer in the item, then change its box to `- [x]`.
4. Run `spec-dna check`.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: `spec-dna check` gives no SD919 (an open concern), no SD922 (an open question) and no SD925 (an item with no box).

If the check fails: Ask the person about the item that the message names. Do not mark an item that has no answer. Add a box to an item that has none.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:2b538f01aaaa0937 -->
