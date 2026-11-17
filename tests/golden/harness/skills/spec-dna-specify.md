---
name: spec-dna-specify
description: In a spec-dna project, when the intent is accepted and its class is higher-risk, write spec.md with the delta.
---

When to use: The intent is `accepted` and its class is `higher-risk`.

Commands:

1. Run `spec-dna new spec <slug>`.
2. Under `Delta`, write one item for each requirement that the change adds, changes or removes. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>` and copy the hash.
3. Write the interfaces under `Interfaces` and the decisions under `Decisions`.

Artifact: `spec.md` with `Delta`, `Interfaces` and `Decisions`.

Check: `spec-dna check` gives no SD914 (a delta line) and no SD920 (a decision).

If the check fails: Fix each item that the message names. Run `spec-dna check` again.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:d59601ae3178228d -->
