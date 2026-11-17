---
name: spec-dna-specify
description: In a spec-dna project, when the intent is accepted and its class is higher-risk, write spec.md with the delta.
---

When to use: The intent is `accepted` and its class is `higher-risk`.

Commands:

1. Run `spec-dna new spec <slug>`.
2. Under `Delta`, write one line for each requirement that the change adds, changes or removes: `ADD <id>#<key>: <sentence>`, `CHANGE <id>#<key>@<hash>: <sentence>` or `REMOVE <id>#<key>@<hash>`.
3. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>` and copy the hash.
4. Under `Interfaces`, write `- none` or one paragraph if no interface changes. Else write one item for each interface: `CHANGE <id>: <text>` for an interface that exists, `ADD <id>: <text>` for a new one.
5. Under `Decisions`, link each decision that the change needs: `[[decisions/<slug>]]`.
6. Write each item of `Concerns` and `Open questions` with a box: `- [ ]` is open. If none applies, write one item `- none`.

Artifact: `spec.md` with `Delta`, `Interfaces`, `Decisions`, `Concerns` and `Open questions`.

Check: `spec-dna check --base origin/HEAD` gives no SD914 and no SD920. SD914 is an invalid delta or interface item. SD920 is a decision that is absent or not accepted.

If the check fails: Fix each item that the message names. For a new hash, run `spec-dna show <id>#<key>` again. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line.

<!-- spec-dna harness v{VERSION} h:63ad1cd6a6f5e50d -->
