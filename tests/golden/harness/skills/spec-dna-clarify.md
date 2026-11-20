---
name: spec-dna-clarify
description: In a spec-dna project, when spec.md has an open concern or question, close each one before the spec is approved.
---

When to use: `spec.md` has an open concern or an open question.

Commands:

1. Read `Concerns` and `Open questions` of `spec.md`.
2. Give each item a box: `- [ ]` is open, and `- [x]` is resolved. If no item applies, write one item `- none`.
3. Ask the person about each open item. Write the answer in the item, then change its box to `- [x]`.
4. Run the check below.

Artifact: `Concerns` and `Open questions` of `spec.md`, each item answered.

Check: Before the approval, confirm that no `- [ ]` item stays in `Concerns` and `Open questions`. SD919 (an open concern) and SD922 (an open question) report it only after the spec is `approved`. Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD925 (an item with no box). Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Ask the person about the item that the message names. Do not mark an item that has no answer. Add a box to an item that has none.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.

<!-- spec-dna harness v{VERSION} h:e792bfbeea589ab4 -->
