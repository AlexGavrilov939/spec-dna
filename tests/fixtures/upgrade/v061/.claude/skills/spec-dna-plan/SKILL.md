---
name: spec-dna-plan
description: Writes plan.md of a change with the files, the ordered tasks, the risks and the proof. Use it after the intent, or after the spec.
---

You write `plan.md` of a change. Use plan mode if the session has it. Do not change code until a person approves the plan.

1. Run `spec-dna next <slug>`. Read the intent, and the spec if the change has one.
2. Run `spec-dna context <file>...` for the files that you expect to change. Read a listed section with `spec-dna show <id>#<key>` only when you need it.
3. Run `spec-dna new plan <slug>`.
4. Under `Files`, list each path or glob that the change touches. This list is the scope of the change.
5. Under `Order`, write one task for each step, in order. Give each task an id that is not in the file yet.
6. Under `Risks`, write what can go wrong and what limits it. Under `Proof`, write the tests with their `covers:` ids, and the commands that show the result.
7. Run `spec-dna check`. Fix each error. Run `spec-dna next <slug>`.

A task has this form:

```text
- [ ] <id>: <text>. Files: `path`, `glob`. Verify: <text or `command`>.
```

Do not set `status` to `approved`. A person approves the plan in a pull request.

<!-- spec-dna harness v0.6.1 h:82742c60a9ce7390 -->
