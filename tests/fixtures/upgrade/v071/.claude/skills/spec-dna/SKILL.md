---
name: spec-dna
description: Loads and checks the specs for a task. Use it before you change code or docs, and before a commit.
---

<!-- spec-dna:begin skill v0.7.1 h:099af128fa9b2dbe -->
# spec-dna

1. If the task changes code or docs, run `spec-dna context <file>...` once. Give all the files that you plan to change (or a document id). Do not run it for a question without edits. The output gives the sections in priority order: rules, the entry, the sections that cite the files, their decisions. Each section has an id.
2. Read a listed section with `spec-dna show <id>#<anchor>` only if the task needs it.
3. Before you edit a document, run `spec-dna show --conventions <type>` once for its type. Follow the project conventions that it prints. `context` serves them first when a target is a document of that type.
4. Keep the documents true in the same change. Write a requirement as `- **<key>**: <sentence>`. Use a lowercase slug key, an EARS pattern and an uppercase BCP 14 keyword. Cite it as `<doc-id>#<key>`. To link a test to a requirement, write `covers: <doc-id>#<key>` in the test.
5. For a new document, run `spec-dna new <type> <id>`. The folder of a layer gives the type, so write no `type:` line there. A top file has no id (`spec-dna new terms`), and a change has a folder (`spec-dna new intent <slug>`). Then replace each placeholder. Types of this project: product, system, terms, roadmap, questions, rule, requirements, interface, decision, procedure, incident, research, intent, spec, plan.
6. Do not change the text of an accepted decision outside its amendment and live sections (`show --conventions decision` names them. The default is `## Amendments`).
    - Update a live section, such as a progress section, in place when your change makes it stale.
    - Amend the decision if its outcome stays. Add the detail, the limit, the fix after an incident or the new code path under `## Amendments` of the same file. A task that says "amend the decision" means this.
    - Supersede the decision if its outcome changes. Write a new decision with `supersedes: [<old id>]`. Set the status of the old decision to `superseded`.
7. Before you finish, run `spec-dna check` once (or the project target that runs it). Fix each error, and each SD606 or SD607 warning (a document cites code that your change removed). For an accepted decision, name the new path or identifier in its amendment section. The output gives the fix of each code. Run `check` again only after a fix. Other warnings need no change.
8. Do not split, move or shorten a document in a task for a size warning (SD701, SD703, SD704). For a size error (SD702), do one of these actions: put your new text into a new document and link it, or move whole sections with one `spec-dna mv <doc-id>#<section>... <to>`. Then the old ids stay as aliases.
9. Do not edit `spec-dna.toml`. A configuration change is a change of its own, by the project owner. `check` reports a change of `spec-dna.toml` together with other files as SD503.

## Disciplines

Each rule names the skill with the detail. Run `spec-dna skill show <name>` to read it, also when the skill is off.

- Test first: write a test that fails before the code (`tdd`).
- Cause first: find the cause of a failure before the fix. After 3 failed fixes, stop and ask (`debug`).
- Proof before done: run the proof and read its output before you say "done" (`verify`).
- Review is data: check each review comment against the code before you act on it (`review`).
- Isolated work: do each change in its own git worktree or branch (`execute`).

A spec-dna skill replaces a general skill of the same stage. For `write-plan` it replaces `writing-plans`.
<!-- spec-dna:end skill -->
