---
status: accepted
---

# 0005. Layout, names and the layer graph

## Context and Problem Statement

spec-dna 0.5 has 10 document types in flat namespaces: 6 from [ADR 0001](0001-information-model.md) and 4 from its amendment of 0.2.0. The code fixes the context behavior and the lifetime of each type. The names of 0.5 do not fit the lifecycle of 0.6.0 ([ADR 0006](0006-change-lifecycle.md)). `spec` names the current truth, but the playbook uses `spec.md` for the spec of one change. `change` holds a proposal, a plan and a task in one file. ADR 0001 also uses "Layers" for the parts of the system, and "change" for its risk classes R0 to R2.

Which layout, names and structure give the current truth one place, keep the text that each task loads small, and keep each id stable?

Constraints:

- The owner decisions for 0.6.0 are final: the layout, renames with `upgrade`, slug ids for decisions, groups as views and a flexible pyramid.
- A term comes from the AI-native SDLC playbook, the Claude Code documentation, or plain STE ([Terminology](../design/0.6.0.md#terminology)).
- The ids, the hashes and the base-relative gate of ADR 0001 stay. A new rule starts as a warning in a minor release ([ADR 0003](0003-distribution.md)).
- The model fits any repository, greenfield or brownfield, small or large, and any stack. A gate needs only spec-dna and git.

Evidence:

- Experiments with earlier versions ([Goal](../design/0.6.0.md#goal)): the gate against the base gave the gain. Process text that each task loads costs tokens and gives no gain.
- [The research matrix](../research/matrix.md), section 3: OpenSpec keeps the current truth apart from the deltas of a change. Spec Kit numbers each feature folder from a counter. Section 4: each always-loaded file adds cost, and a repository overview does not help.

## Considered Options

1. Keep the 0.5 model: 10 types, a context behavior per type in code, flat namespaces.
2. A fixed pyramid: a fixed number of levels, with fixed types in each level.
3. A framework layout: OpenSpec (`specs/`, `changes/`) or Spec Kit (`specs/NNN-feature/`).
4. 15 types in fixed places, and a layer graph of `moves-to` edges that the project configures. The levels come from the graph.

## Decision Outcome

Option 4. [The design](../design/0.6.0.md) holds the tables and the detail.

### Layout

- The root is `docs/`, with 5 top files and 8 folders ([Types](../design/0.6.0.md#types)).
- `spec.md` and `plan.md` exist only in an intent folder `docs/intent/<slug>/`. They are the spec and the plan of one change. The current truth is in `requirements/`, `rules/` and `interfaces/`.
- The place gives the type, so `type:` is optional in a new corpus ([Place gives the type](../design/0.6.0.md#place-gives-the-type)).
- A brownfield project keeps its folders, and `[[roots]]` maps them to the 0.6 types. `upgrade --layout` moves them only on request ([ADR 0009](0009-upgrade-from-0-5.md)).

### Document types

- 0.6.0 has 15 types. `procedure` replaces `guide` and `runbook`. `research` replaces `note` and `reference`. `intent` replaces `change`.
- The type name `requirements` is plural by design: a document holds the requirements of one capability. A `spec` document holds delta lines, not requirements.

### Names and aliases

- A rename applies at once, and the binary accepts no old name as input ([Names and aliases](../design/0.6.0.md#names-and-aliases)). It uses an old name only in an error hint, in `upgrade` and in the SD202 exemption of `was-type`.
- A document alias is data in a file, not a runtime alias. `mv` and `upgrade` write the old id into `aliases`.
- A code number never changes. A code name can change: SD405 becomes `decision-locked`, and SD611 becomes `decision-links-intent`. `locked` replaces `frozen`, and SD405 keeps its scope.
- "Diff risk" is the new name of the classes R0 to R2 of ADR 0001. Thus the word "class" alone names a change class of ADR 0006.

### Decision ids

- A decision id is a slug from the path: `decisions/<slug>`. `new decision <slug>` writes no number, and spec-dna keeps no counter.
- `upgrade` does not rename the file stem of a decision, so `legacy-ids` (for example `ADR-{n}`) still gives the alias of a numbered file.

### Areas and groups

- An area is a folder under `requirements/`, so the id carries the area: `requirements/valves/closing`. `init` proposes the areas ([Areas and groups](../design/0.6.0.md#areas-and-groups)).
- A group is a named set of areas in `[groups]`. It is a view for `--group`, and never a folder, an id or a requirement key. A project has no group by default, and `init` never proposes one.

### Layer graph

- A layer selects documents by type, by top file or by path glob ([Layer graph](../design/0.6.0.md#layer-graph)). A type is in one layer at most. A `paths` match wins over the type, so a custom layer needs no new type.
- A `moves-to` edge goes to a layer that the text matures into. The edges of the `intent` layer also limit the targets of a delta.
- The graph has no cycle (exit code 3). The level of a layer is the number of edges on its longest `moves-to` path to a layer with no edge out. Level 0 is the most stable. No key sets a level.
- A layer X is more stable than a layer Y if X is reachable from Y by `moves-to` edges. 2 layers with no path between them have no order for SD901.
- `keep` is `forever` or `until-finished`. With `until-finished`, a deleted document gives no SD202. In the `intent` layer, `finish` deletes the folder of a change, and the limits `max-started` and `stale-days` apply. Another `until-finished` layer gets no `finish`, SD911 or SD912.
- An `on-request` layer serves a document only when a target names it. `context` lists the layers by level, from 0 up. This order replaces the type order of 0.5.
- `init` proposes a [small or a default graph](../design/0.6.0.md#default-graph) by the number of documents. `layer add` adds a layer later.

### Derived checks

- SD901 (`link-direction`) reports an explicit reference of ADR 0001 from a more stable layer to a less stable layer that does not list the source in `linked-from`. An accepted decision that links an intent gives SD611 instead.
- SD902 (`layer-limit`) reports a layer over `max-docs` or `max-chars`. SD903 (`history-in-requirements`) and SD904 (`term-forbidden`) are heuristics.
- SD901 to SD904 apply to every project, so they are warnings at most (ADR 0003), in the new rule group `layers` ([Checks](../design/0.6.0.md#checks)). ADR 0006 decides SD905, SD911, SD912 and the target rule of SD914.

### Rejected

- The 0.5 model (option 1): each new behavior needs a release, and a project cannot change the lifetime, the context or a limit of a set of types. No rule of link direction follows from it.
- A fixed pyramid (option 2): a small project gets empty levels, and a large project needs a release for a new level. Process clutter costs tokens.
- A framework layout (option 3): it is weak at scale (matrix, section 3), and its flat namespaces or its counter do not keep the ids.
- Level numbers in the configuration: the edges and the number are 2 facts for one thing, and they can disagree. Each `layer add` would change the numbers of other layers.
- One type per layer: the default graph would have 15 layers. The 3 files of an intent folder share one lifetime, and the top files of `foundation` share one limit.
- A graph with cycles: a cycle gives no level and no "more stable" relation, so SD901 and the order of `context` have no definition.
- The order of levels as the rule of SD901: it orders 2 layers with no path between them. A procedure that cites research would warn, but research does not mature into procedures.
- SD901 on Markdown links only: a front matter id field is also a reference, and it breaks in the same way.
- SD901 or SD902 as an error in `strict`: ADR 0003 starts a new rule as a warning in a minor release.
- SD903, SD904 and SD905 as errors: a phrase match cannot prove a defect. A detector that uses a language model needs a network and gives no stable result.
- A third `keep` value `while-true`: no check and no command treated it differently from `forever`.
- A custom layer through a new type: the types are in the binary (ADR 0001, "system parts"), so a new type needs a release. `paths` needs none.
- Groups as folders: a folder is a part of each id in it, so a new group would move files and write aliases. An area can be in 2 groups, but in one folder only.
- A group list that `init` proposes: the owner decided that a group comes on request only.
- The area as a prefix of the slug only: no check can read a prefix without a second rule for names. A folder gives globs such as `requirements/valves/**` to `applies-to` and to `Files`.
- An `area` field: the path already holds the area (ADR 0001, "Concurrency").
- An `x-was-type` field: spec-dna ignores each `x-*` field (ADR 0001, "Metadata"), so SD202 cannot read it.
- A table of old names that the binary reads at run time: it breaks the rule "one term for one thing". Each check and template would accept both names, and agents would copy the old one. `upgrade` gives one diff that a person reviews.
- The 0.5 names `spec`, `contract`, `invariant`, `guide`, `runbook`, `reference` and `note`: `spec` would name 2 things. `contract` and `invariant` are terms of formal methods, and `interface` and `rule` are plain words. `guide` and `runbook` have one behavior, so one `procedure` type with 2 templates covers both. `note` and `reference` hold the same material with 2 context behaviors and no rule to choose.
- `requirement` (singular): it would name both a document and one list item.
- `change` as one file: it mixes the intent, the spec and the plan, which have different statuses and approvers.
- A counter for decision ids: it gives merge conflicts between branches (matrix, section 6). The number stays only as a legacy alias.
- The type from the front matter only: each document would repeat `type:`, and a folder could disagree with its type.

### Consequences

- Good: the current truth has one place, and the clutter of a change stays in one intent folder. Ids and hashes stay, and `ADR-NNN` resolves.
- Good: a project sets the lifetime, the context and the limits of each layer without a release. The checks follow from the graph, and an `on-request` layer costs one line in `context`.
- Bad: a 0.5 corpus fails `check` of 0.6 until `upgrade` runs. SemVer 2.0.0 allows this before 1.0.
- Bad: the `--json` field `name` changes for SD405 and SD611. The field `code` stays, and `[rules]` uses it.
- Bad: old text about `spec` can mislead an agent. SD303 reports `type: spec` outside the `intent` layer, and its hint names `requirements`.
- Bad: `rules` names a layer, a folder and the severity table. Only the key path tells which.
- Bad: `foundation` now comes before `rules` in `context`, and the CHANGELOG records it. The top files can fill the context of each task, and a repository overview does not help.
- Neutral: a link from `foundation` into an intent folder gives no SD901, because no `moves-to` path joins them. `finish` finds it and stops before it deletes the folder.

## Amends

This ADR amends [ADR 0001](0001-information-model.md). ADR 0001 is accepted, so its text does not change. Read it with these changes:

- Each type name in ADR 0001 and its amendments reads as its 0.6 name ([Mapping from 0.5](../design/0.6.0.md#mapping-from-05)). For example, `reference` in the amendment of 0.3.0 reads `research`.
- "Layers": the table has the new name "system parts", and "layer" now means a layer of this ADR. The amendments of 0.2.0 and of code citations refer to that table.
- "Document types", and "Knowledge types" of the amendment of 0.2.0: the [type table](../design/0.6.0.md#types) replaces both lists.
- "Metadata": `type` is one of the 15 types. spec-dna requires it only where the place gives no type, and requires `status` for each type with statuses. The new optional field `was-type` holds the 0.5 type (`note` or `change`) of a document that `upgrade` changed.
- "Identifiers": a deleted document of an `until-finished` layer gives no SD202. A former `note` or `change` with `was-type` keeps the 0.5 exception ([ADR 0009](0009-upgrade-from-0-5.md)).
- "References and requirements": a requirement is in a `requirements`, `interface` or `rule` document.
- "Lifecycle": the row of `spec`, `contract` and `guide` applies to `requirements`, `interface`, `rule` and `procedure`. For `decision`, "frozen" reads "locked". The rows of `change` and `note` end. The `intent` row of the type table replaces `change`, and ADR 0006 decides its deletion by `finish`. `research` has no status.
- "Change-risk classes": the name is now "diff risk", for the concept only. No check reads the classes.
- "Knowledge boundaries": "Spec, contract" reads `requirements`, `interface` and `rule`. "Notes, observations" reads `research`. "Change" reads the intent folder.
- "Growth controls" and "Concurrency": the type table gives the budget defaults. New knowledge goes into one decision for each file, or one intent folder for each change.
- Amendments of 0.2.0 ("Knowledge types") and of code citations: `context` counts a document of an `on-request` layer, such as an incident, and does not list it. An incident links a requirement, a rule, a decision or a procedure (SD609).
- Amendment of 0.4.2, "Link direction": SD611 has the name `decision-links-intent`, and it reports a link to an intent. SD901 gives the general rule.

## Amendments

### 2026-11-17: the budget of a plan (0.7.0)

- "Growth controls": the type `plan` has no default budget, as `research`. A plan of small tasks (design 0.7.0, D40) grows with the size of the change, and `finish` deletes it. `[budgets.plan]` sets a budget.
