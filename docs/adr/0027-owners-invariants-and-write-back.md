---
status: proposed
---

# 0027. Owners by path, invariants and the write-back of deviations

## Context and Problem Statement

The gate keeps the requirements true, but 3 things above them have no check:

- A sensitive path, such as the code of the login, needs a review of a kind, and nothing asks for it.
- A rule of the layer `rules` must stay true, and no change says which rule it can break.
- The work differs from the plan or the spec, and the record of the difference dies with the plan at `finish`.

The owner decided on 2026-11-18: `[owners]` maps a path glob to the needed reviewers, a person or an agent role, with no R, A, C or I letters and no list of personas. A rule can carry a stable id, a spec declares the invariants that bear on it, `check` resolves the ids, and the acceptance record answers each one. During the build, the agent records each deviation in one section of the change. `finish` prints the deviations and needs the acknowledgement of the owner. How do the 3 rules fit the record of [ADR 0025](0025-independent-acceptance.md) and the 3 approval modes?

Constraints:

- One record per change. The owners, the answers and the deviations are parts of it, not new stores.
- Reuse the ids, the aliases and the pinned hashes of [ADR 0001](0001-information-model.md).
- `check` has no network. The persons of `review` are checked by `spec-dna reviews` ([ADR 0026](0026-approval-through-the-code-host.md)).
- An absent table or section keeps the behavior of 0.7.1.

Evidence ([design 0.8.0](../design/0.8.0.md#1-evidence-and-sources)):

- A requirement id `<doc-id>#<key>` is stable: SD202 and the aliases keep it after a move.
- [ADR 0006](0006-change-lifecycle.md), "Delta": a reference with `@<hash>` still ignores its hash, and only a delta line checks it.
- `applies-to` of a rule already selects it for `context` (F16).
- When the diff deletes the folder, each check reads the artifacts from the base tree (F8). Deviations written in the pull request of `finish` are not there.

## Considered Options

For the owners:

1. A table `[owners]`: a glob to a list of roles and persons.
2. Read the code owners file of the host.
3. The field `owner` of the front matter of a document.

For the invariants:

- (a) The requirement id of a rule, with its hash, in a section `Invariants` of the spec.
- (b) A new id form for an invariant.
- (c) Each rule whose `applies-to` matches a touched path, with no declaration.

For the deviations:

- (i) A section `Deviations` of the plan, copied into the record.
- (ii) A file `deviations.md` in the folder of the change.
- (iii) The commit messages of the work.

## Decision Outcome

Option 1, option (a) and option (i). The design page gives the detail in [D5](../design/0.8.0.md#d5-owners-by-path), [D6](../design/0.8.0.md#d6-invariants-with-ids) and [D7](../design/0.8.0.md#d7-write-back-of-deviations).

### Owners by path

- `[owners]` maps a glob from the root to a list of entries. A role is a name of lowercase letters, digits and `-`, at most 40 characters. A person is `@<login>` or an e-mail, in the form of the mode: an e-mail with `branch`, a login or a hidden address that gives one with `review`, either with `base`. A bad glob or entry, or a person in a form that the mode can never match, is exit code 3.
- Each glob that matches a touched path applies. The result is the union, so no order and no last match exist.
- The touched paths of a change are the paths of its diff against the lifecycle base. When the diff holds more than one change, a path counts for a change whose `Files` covers it. The folder of the change and the archive do not count.
- Each role of a matched glob needs an acceptor with that role in the record (SD933). The acceptor runs the skill `accept` with `--role <name>`.
- The persons of one glob form a group, and one person of the group approves. With `branch`, that person runs `spec-dna approve <slug> acceptance` (SD934). With `review`, that login approves the head commit. With `base`, spec-dna checks nothing, and `check` prints a note.
- `approvers` approve the artifacts and the deviations. A person of `[owners]` approves paths. One person in both lists satisfies both with one act.

### Invariants

- The id of an invariant is the requirement id of a rule. No new id: a second id would give one sentence 2 names, and the requirement id is already stable.
- A spec declares its invariants in the optional section `Invariants`, one item `<doc-id>#<key>@<hash>` each, or `none`. `spec-dna show` prints the hash.
- `check` resolves each item. An item that does not have the form, names no requirement, or names a requirement outside a `rule` document gives SD936. A hash that differs from the rule now gives SD937, an error for a spec that the diff touches, as SD914. For an item whose rule the delta of the same spec changes, SD937 compares the hash of the item with the hash of that delta line, not with the rule now: the delta line pins the old text, and after `finish` the rule has the new text by design. A rule that the same delta adds or removes is no invariant of the change, so SD936 reports the item before the approval. When the default branch changes a pinned rule, the author pins it again in a pull request of its own, as for SD914.
- The record answers each item with the same pin: `held`, `not-affected` with a reason, or `violated` with a reason. An absent answer gives SD938. `violated` makes the result `rejected`.
- A change with no spec declares no invariant. The skill `accept` still reads the rules that `context` serves for the files of the change.

### Deviations

- The plan has the optional section `Deviations`. The builder adds an item `- <id> (<kind>): <text>` in the commit of the work. The kind is `plan` (files, tasks, order, approach) or `spec` (the behavior differs from a requirement or a delta line).
- A `spec` item names a delta line of the spec (`Delta: <verb> <doc-id>#<key>`), or a change folder of the tree (`Follow-up: <slug>`). If it names neither, the item gives SD940. An item of a bad form gives SD939.
- The acceptor copies each item into the record with the verdict `accepted` or `rejected`. An item with no copy, or with another text, gives SD938.
- `finish` prints each deviation, and `finish --json` gets the list `deviations`. A record with a deviation needs an acknowledgement: the merge with `base`, an approval commit of an approver with `branch`, an approving review with `review`.
- After `finish`, the deviations live in the archived record. In the pull request of `finish`, the checks read the plan of the base tree, so the record is their only carrier.

### Upgrade

`Invariants` is read only in a spec, and `Deviations` only in a plan, that a commit added at a pin of 0.8.0 or later. A section of the same name in an older artifact keeps its meaning. No commit found gives the rule of 0.8.0.

### Rejected

- Option 2: the code owners file has persons and teams of the host only, not agent roles, and its last-match rule depends on order.
- Option 3: a code path has no front matter.
- Option (b): a new id needs a registry or a new syntax, and it can drift from the text of the rule.
- Option (c) alone: a rule with no `applies-to` applies to the whole project, so each change must answer each such rule. The owner can add that option for a change with no spec (design, Q18).
- Option (ii): one more file type in the folder, and the record still needs a copy.
- Option (iii): a squash merge keeps only the title, and `finish` cannot read messages that do not exist yet.
- R, A, C and I letters and a roster of personas: the owner rejected them.

### Consequences

- Good: a sensitive path asks for the review that it needs, in the record or on the host.
- Good: a rule that a change can break is named, pinned, and answered.
- Good: each deviation survives `finish`, with the verdict of the acceptor and an acknowledgement.
- Bad: each matched role costs one more acceptance run.
- Bad: a change of a rule makes each open spec that pins it give SD937.
- Bad: a spec can omit a rule that applies. The acceptor can miss it too.
- Bad: with `base`, the persons of `[owners]` have no check.
- Neutral: with no table and no section, nothing changes.

## Amends

ADR 0001 and ADR 0006 are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0001, "Change-risk classes", the sentence "spec-dna parses that suffix and ignores it until a later version adds suspect links": an item of the section `Invariants` of a spec compares its hash.
- ADR 0006, "Delta", the sentence "A `[[id#key@hash]]` reference still ignores its hash (ADR 0001). Only a delta line checks it.": an item of the section `Invariants` checks it too. A delta line, `rests-on` of [ADR 0028](0028-roll-up-and-summaries.md) and an `Invariants` item compare the hash. Other references still ignore it.

## Amendments
