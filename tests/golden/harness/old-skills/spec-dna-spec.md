---
name: spec-dna-spec
description: Writes spec.md of a higher-risk change with the delta, interfaces, decisions and concerns. Use it after the intent is accepted.
---

You write `spec.md` of a higher-risk change. A routine change has no spec.

1. Run `spec-dna next <slug>`. If the intent is not `accepted`, stop and tell the person.
2. Run `spec-dna new spec <slug>`.
3. Under `Delta`, write one list item for each requirement that the change adds, changes or removes. Use the forms below.
4. For `CHANGE` and `REMOVE`, run `spec-dna show <id>#<key>`. The first line shows `h:<hash>`. Copy the 16 digits after `h:` to the item.
5. Under `Interfaces`, write the item `- CHANGE <interface id>: <text>` for each interface that the change touches. Under `Decisions`, write the item `- [[decisions/<slug>]]` for each decision. If the change needs a new decision, run `spec-dna new decision <slug>`.
6. Write each open concern under `Concerns`, and each open question under `Open questions`, as an item `- [ ] <text>`. Ask the person about each of them.
7. Run `spec-dna check`. Fix each error. Run `spec-dna next <slug>`.

The forms of a `Delta` item:

```text
- ADD <id>#<key>: <new sentence>
- CHANGE <id>#<key>@<hash>: <new sentence>
- REMOVE <id>#<key>@<hash>
```

Do not set `status` to `approved`. A person approves the spec in a pull request.

<!-- spec-dna harness v{VERSION} h:d401999e9c077bc4 -->
