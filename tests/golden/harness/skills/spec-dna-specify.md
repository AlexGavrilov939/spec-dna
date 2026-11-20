---
name: spec-dna-specify
description: In a spec-dna project, when the intent is accepted and its class is higher-risk, write spec.md with the delta.
---

When to use: The intent is `accepted` and its class is `higher-risk`.

Commands:

1. Run `spec-dna new spec <slug>`.
2. Under `Delta`, write one list item for each requirement that the change adds, changes or removes: `- ADD <id>#<key>: <sentence>`, `- CHANGE <id>#<key>@<hash>: <sentence>` or `- REMOVE <id>#<key>@<hash>`. `check` and `finish` ignore a paragraph there.
3. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>` and copy the hash.
4. Under `Interfaces`, write `- none` or one paragraph if no interface changes. Else write one item for each interface: `CHANGE <id>: <text>` for an interface that exists, `ADD <id>: <text>` for a new one.
5. Under `Decisions`, link each decision that the change needs: `[[decisions/<slug>]]`.
6. Write each item of `Concerns` and `Open questions` with a box: `- [ ]` is open. If none applies, write one item `- none`.
7. To change the spec after its approval, set `status: draft` in the commit of the edit. A hook that runs `spec-dna check` rejects that commit (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on.

Artifact: `spec.md` with `Delta`, `Interfaces`, `Decisions`, `Concerns` and `Open questions`.

Check: Stage the new files with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD914 (an invalid delta or interface item). SD920 (a decision that is absent or not accepted) reports only after the spec is `approved`. Before the approval, read the status of each linked decision. Without `origin/HEAD` and `[change] base`, pass the default branch with `--base`.

If the check fails: Fix each item that the message names. For a new hash, run `spec-dna show <id>#<key>` again. Run the check again.

Never set `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.

<!-- spec-dna harness v{VERSION} h:f357438ef8138107 -->
