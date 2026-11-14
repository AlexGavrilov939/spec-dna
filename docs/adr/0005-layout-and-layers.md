---
status: proposed
---

# 0005. Layout, names and the layer graph

## Context and Problem Statement

spec-dna 0.5 has 10 document types in the flat namespaces of a root. [ADR 0001](0001-information-model.md) gives 6 of them, and its amendment of 0.2.0 adds 4. The code fixes the context behavior and the lifetime of each type. Version 0.6.0 adds a lifecycle for a change ([ADR 0006](0006-change-lifecycle.md)), and the names of 0.5 do not fit it:

- `spec` names the current truth of a capability. The playbook uses `spec.md` for the spec of one change.
- `change` holds a proposal, a plan and a task in one file. The playbook gives the intent, the spec and the plan a file and an approval each.
- `note` and `reference` hold the same kind of material with 2 context behaviors. `guide` and `runbook` have one behavior.
- ADR 0001 uses "Layers" for the parts of the system, and the word "change" for its risk classes R0 to R2.

Which layout, names and structure let a person or an agent find the current truth in one place? The answer must also keep the text that each task loads small, and keep each id stable.

Constraints:

- The owner decisions for 0.6.0 are final: the layout, renames with `upgrade`, slug ids for decisions, groups as views and a flexible pyramid.
- A term comes from the AI-native SDLC playbook or the Claude Code documentation. Where they name nothing, the term is a plain STE word ([Terminology](../design/0.6.0.md#terminology)).
- The ids, the hashes and the base-relative gate of ADR 0001 stay. A new rule starts as a warning in a minor release ([ADR 0003](0003-distribution.md)).
- The model fits any repository: greenfield or brownfield, small or large, any stack. A brownfield project keeps its folders.
- A gate needs only spec-dna and git.

Evidence:

- Experiments with earlier versions of spec-dna. The data stays outside this repository.
  - The gate against the base gave the measured gain. Retrieval and the split of big documents did not.
  - Agents search the repository. They seldom follow a tree of links.
  - Large files and process documents in the text that each task loads cost tokens and give no gain.
- [The research matrix](../research/matrix.md), section 3: OpenSpec keeps the current truth apart from the deltas of a change. Spec Kit gives each feature folder a number from a counter.
- The research matrix, section 4: each always-loaded file adds cost, and a repository overview does not help.

## Considered Options

1. Keep the 0.5 model: 10 types, a context behavior per type in code, flat namespaces.
2. A fixed pyramid: a fixed number of levels, and fixed types in each level.
3. The layout of a framework: OpenSpec (`specs/` and `changes/`) or Spec Kit (`specs/NNN-feature/`).
4. 15 types in fixed places, and layers that the project configures as a graph of `moves-to` edges. The levels come from the graph.

| Option | Fits small and large projects | Text that each task loads | Names match the playbook | Ids and hashes stay | Checks from the model |
| --- | --- | --- | --- | --- | --- |
| 1 | Partly: one set of types for every size | Medium: `context` lists each `change` and `note` | No: `spec` is the current truth | Yes | SD611 only |
| 2 | No: a small project gets empty levels | Low | Yes | Yes | Yes |
| 3 | No: weak at scale (matrix, section 3) | Varies | Partly | No: flat namespaces or a counter | Few |
| 4 | Yes: `init` proposes a graph by size | Low: `context` counts an `on-request` layer in one line | Yes | Yes: a move writes aliases | Yes: SD901, SD902, the order of `context` |

## Decision Outcome

Option 4. [The design of 0.6.0](../design/0.6.0.md) holds the tables. This ADR states the decisions and links the tables.

### Layout

- The layout has the root `docs/`, 5 top files and 8 folders. The [type table](../design/0.6.0.md#types) gives the place of each type. The size of a project decides which layers `init` writes.
- `spec.md` and `plan.md` exist only in an intent folder `docs/intent/<slug>/`. They are the spec and the plan of one change. The current truth is in `requirements/`, `rules/` and `interfaces/`.
- The place gives the type ([Place gives the type](../design/0.6.0.md#place-gives-the-type)). A document under a layer folder takes the type of the layer, if the layer has one type. In the `intent` layer, the file name gives the type, and a top file gives the type by its name. Front matter `type` overrides the place only in a root with `type = "mixed"`. Thus `type:` is optional in a new corpus.
- The types `intent`, `spec` and `plan` are valid only in the `intent` layer. In a new corpus, a mixed root has no `default-type`. `check` reports SD303 for a document that is outside every layer place and has no `type`.
- A brownfield project keeps its folders. `[[roots]]` maps them to the 0.6 types. `upgrade --layout` moves them only on request ([ADR 0009](0009-upgrade-from-0-5.md)).

### Document types

- 0.6.0 has 15 types. The [type table](../design/0.6.0.md#types) gives the layer, the statuses, the required sections and the budgets of each type. The [table of 0.5 types](../design/0.6.0.md#mapping-from-05) gives the 0.6 type of each 0.5 type.
- The type name `requirements` is plural by design. A `requirements` document holds the requirements of one capability. A requirement is one list item with a bold key, as in ADR 0001.
- Requirements are in `requirements`, `interface` and `rule` documents. A `spec` document holds delta lines, not requirements.
- `procedure` replaces `guide` and `runbook`. The runbook form is a template ([ADR 0007](0007-templates-and-document-mapping.md)). `research` replaces `note` and `reference`. `intent` replaces `change`.

### Names and aliases

- A rename applies at once. The binary accepts no old name as input: no old type, configuration key, folder name or command. The table ["Names and aliases"](../design/0.6.0.md#names-and-aliases) gives the rule for each kind of name.
- The binary knows the old names for 3 uses only: error hints, `upgrade` and the SD202 exemption of a document with `was-type` ([ADR 0009](0009-upgrade-from-0-5.md)). For example, SD303 names the new type, and exit code 3 names the new key.
- A document alias is not a runtime alias. It is data in a file, and the id model of ADR 0001 reads it. A move without an alias breaks each citation, and SD202 reports it. Thus `mv` and `upgrade` write the old id into `aliases`.
- A code number never changes. A code name can change: SD405 becomes `decision-locked`, and SD611 becomes `decision-links-intent`.
- The word `locked` replaces `frozen`. SD405 keeps its scope from ADR 0001: all text outside the amendment, history and live sections of an accepted decision.
- "Diff risk" is the new name of the classes R0, R1 and R2 of ADR 0001. The word "class" alone names a change class of ADR 0006. The name applies only to that concept of ADR 0001. The source code has no such classes, and SD921 compares the requirement hashes directly ([Checks](../design/0.6.0.md#checks)).

### Decision ids

- A decision id is a slug from the path, as each id of ADR 0001: `decisions/<slug>`. `new decision <slug>` writes no number, and spec-dna keeps no counter.
- A numbered file of a brownfield project keeps its name, because `upgrade` does not rename the file stem of a decision. The root pattern `legacy-ids` (for example `ADR-{n}`) makes the alias from the file name, as in 0.5. When a move changes an id, `upgrade` writes the old id and each old legacy alias into `aliases`. Thus a citation of `ADR-12` still resolves.

### Areas and groups

- An area is a folder under `requirements/`. The id of a document carries its area, because the id comes from the path: `requirements/valves/closing`. The configuration lists the areas in `areas`. `init` proposes them at each size, when it detects 2 or more ([Init questions](../design/0.6.0.md#init-questions)).
- A group is a named set of areas in `[groups]` ([Areas and groups](../design/0.6.0.md#areas-and-groups)). A project has no group by default, and `init` never proposes one. The owner adds a group on request.
- A group is a view: `status`, `context` and `stats` read it with `--group`. It never makes a folder, an id or a requirement key.

### Layer graph

- A layer is a named set of documents. The [layer keys](../design/0.6.0.md#definition) select them by type (`types`), by top file (`files`) or by path glob (`paths`). The other keys are `dir`, `moves-to`, `linked-from`, `keep`, `context` and 4 limits.
- A type is in one layer at most. A `paths` match puts a document in a layer, also when its type is in another layer. Of the layers that match, the first in the file applies. Thus a project adds a custom layer by its paths, with no new type.
- A `moves-to` edge goes from a layer to a layer that its text matures into. The edges of the `intent` layer also limit the targets of a delta ([ADR 0006](0006-change-lifecycle.md)).
- The graph has no cycle. A cycle is a configuration error (exit code 3), and `layer add` refuses it. The level of a layer is the number of edges on its longest `moves-to` path to a layer with no edge out. Level 0 is the most stable. The configuration has no level key.
- A layer X is more stable than a layer Y if X is reachable from Y by `moves-to` edges. 2 layers with no path between them have no order for SD901.
- `keep` has 2 values. With `forever`, a document stays, and SD202 reports its deletion without an alias. With `until-finished`, a deleted document gives no SD202. In the `intent` layer, the limits `max-started` and `stale-days` also apply.
- The `intent` layer uses `until-finished` for `finish`, which deletes the folder of a change ([ADR 0006](0006-change-lifecycle.md)). Another `until-finished` layer, such as a custom layer, gets the SD202 exemption and the limits `max-docs` and `max-chars`. It gets no `finish` command, no SD911 and no SD912, because these codes are for changes.
- `context = "always"` lets a layer fill the plan of `context` without a named target. `context = "on-request"` serves a document only when a target names it, and the plan counts the other documents in one line.
- Context order from the top: `context` lists the layers by level, from 0 up, and layers of one level keep the order of the configuration. This order replaces the type order of 0.5, in which a rule came first.
- `max-docs` and `max-chars` apply to each layer. A project with no `[layers]` table gets the default graph for its types.
- A `moves-to` entry names only a layer that the graph has. `init` writes into `[change] high-risk-layers` only a layer that the graph has.

### Default graph and size

- `init` proposes a graph by the number of documents, also for a new project ([Default graph](../design/0.6.0.md#default-graph)). The size decides only which layers `init` writes. `layer add` adds the other layers later.
- The small graph has 7 layers: `foundation`, `pending`, `rules`, `requirements`, `decisions`, `intent` and `incidents`. The default graph is for 30 documents or more. It adds `interfaces`, `procedures` and `research`.
- In the default graph, the stable layers are `foundation`, `rules`, `requirements`, `interfaces`, `decisions` and `procedures`. `foundation` holds the top files `product.md`, `system.md` and `terms.md`.
- `layer add` adds a layer, and its option `--moved-from` adds an edge into it. `layer list` prints each layer with its level ([Commands](../design/0.6.0.md#commands)).

### Derived checks

- SD901 (`link-direction`) reports an explicit reference of ADR 0001 from a more stable layer to a less stable layer. The reference can be a Markdown link, a `[[id]]` or a front matter id field such as `from`. The less stable layer can allow the source layer in `linked-from`. An accepted decision that links an intent gives SD611, not SD901.
- SD902 (`layer-limit`) reports a layer over `max-docs` or `max-chars`. SD901 and SD902 apply to every project, so they are warnings at most, as ADR 0003 requires. They are in the new rule group `layers`, with SD903 and SD904.
- SD903 (`history-in-requirements`) and SD904 (`term-forbidden`) are heuristics, so they are warnings at most. SD903 reads a configurable phrase list (`[lint] history-terms`). SD904 reads the forbidden synonyms of `terms.md`. SD905 (`fan-out`) belongs to the lifecycle: [ADR 0006](0006-change-lifecycle.md) decides it. The 3 checks are rejected as errors, because a phrase match cannot prove a defect. A detector that uses a language model is also rejected, because it needs a network and gives no stable result.
- The [Checks](../design/0.6.0.md#checks) table gives the severity in each profile. ADR 0006 decides SD911, SD912 and the target rule of SD914.

### Rejected

- A fixed pyramid of N levels: a small project gets levels with no documents, and a large project cannot add a level without a release. The evidence shows that process clutter in the loaded text costs tokens.
- Explicit level numbers in the configuration: the edges and the number are 2 facts for one thing, and they can disagree. Each `layer add` would also change the numbers of other layers.
- One type per layer: the default graph would have 15 layers. The 3 files of an intent folder share one lifetime, and the top files of `foundation` share one limit. A project can still give a type its own layer.
- A graph that allows cycles: a cycle gives no level and no "more stable" relation. Then SD901 and the order of `context` have no definition.
- The order of levels as the rule of SD901: it orders 2 layers with no path between them. A procedure that cites research would warn, but research does not mature into procedures.
- SD901 on Markdown links only: a front matter id field is also a reference of ADR 0001, and it breaks in the same way.
- SD901 or SD902 as an error in `strict`: ADR 0003 starts a new rule as a warning in a minor release.
- A third `keep` value `while-true`: no check and no command treated it differently from `forever`.
- A custom layer through a new document type: the types are in the binary (ADR 0001, "system parts"), so a new type needs a release. `paths` needs none.
- A context behavior per type with no layer (the 0.5 model): each new behavior needs a release. A project cannot change the lifetime, the context or a limit of a set of types. No rule of link direction follows from it.
- Groups as folders: a folder is a part of each id in it. A new group would move files and write aliases. An area can be in 2 groups, but in one folder only.
- A group list that `init` proposes for any project: the owner decided that a project adds a group on request only.
- The area as a prefix of the slug only, with no folder: no check can read a prefix without a second rule for names. A folder gives path globs such as `requirements/valves/**` to `applies-to` and to the `Files` of a plan.
- An `area` field in the front matter: the path already holds the area. The field would be a second place for one fact (ADR 0001, "Concurrency").
- An `x-was-type` field for the old type: spec-dna ignores each `x-*` field (ADR 0001, "Metadata"), so SD202 cannot read it.
- A table of old names that the binary reads at run time: 2 names for one thing break the rule "one term for one thing". Each check and template would accept both, and agents would copy the old name. `upgrade` gives one diff that a person reviews.
- Keep the names `spec`, `contract`, `invariant`, `guide` and `runbook`:
  - `spec` would name both the current truth and the spec of one change.
  - `interface` and `rule` are plain words for the content. `contract` and `invariant` are terms of formal methods.
  - `guide` and `runbook` have one behavior. One `procedure` type with 2 templates is sufficient. The playbook also uses "runbook" for a pre-approved action.
- Keep `reference` and `note`: both hold informative material, and an author must choose between 2 context behaviors with no rule. One `research` type with `context = "on-request"` covers both.
- `requirement` (singular) as the type name: it would name both a document and one list item with a key.
- Keep `change` as one file: one file mixes the intent, the spec and the plan, which have different statuses and approvers.
- A counter for new decision ids: a global counter gives merge conflicts between branches (matrix, section 6). The number stays only as a legacy alias.
- The type from the front matter only: each new document would repeat `type:`, and a folder could disagree with its type.

### Consequences

- Good: the current truth has one place, and the clutter of a change stays in one intent folder. The ids and the hashes stay, and `ADR-NNN` still resolves.
- Good: a project sets the lifetime, the context and the limits of each layer without a release. The checks follow from the graph. An `on-request` layer costs one line in the plan of `context`.
- Bad: a 0.5 corpus fails `check` of 0.6 until `upgrade` runs, because no old name stays. SemVer 2.0.0 allows this change before 1.0.
- Bad: the `--json` field `name` changes for SD405 and SD611. The field `code` stays, and the keys of `[rules]` use the code.
- Bad: `spec` now names a different document, and old text can mislead an agent. SD303 reports `type: spec` outside the `intent` layer, and its hint names `requirements`.
- Bad: the word `rules` names a layer, a folder and the severity table. Only the key path tells which: `[rules]` or `[layers.rules]`.
- Bad: `foundation` now comes before `rules` in `context`, and the CHANGELOG records this change. The top files can fill the context of each task, and the matrix shows that a repository overview does not help.
- Neutral: SD901 compares only layers on one `moves-to` path, so a link from `foundation` into an intent folder gives no SD901. `finish` finds such a link and stops before it deletes the folder.
- Neutral: SD901 and the default graph replace the table "Links between types" of `docs/types.md`. The implementation updates that page and section 5 of the matrix.

## Amends

This ADR amends [ADR 0001](0001-information-model.md). ADR 0001 is accepted, so its text does not change. Read it with these changes:

- Each type name in ADR 0001 and its amendments reads as its 0.6 name in the [table of 0.5 types](../design/0.6.0.md#mapping-from-05). For example, `reference` in the amendment of 0.3.0 reads `research`.
- "Layers": the table has the new name "system parts". The word "layer" now means a layer of this ADR. The amendments of 0.2.0 and of code citations refer to that table.
- "Document types", and "Knowledge types" of the amendment of 0.2.0: the [type table](../design/0.6.0.md#types) replaces both lists.
- "Metadata": `type` is one of the 15 types. spec-dna requires it only where the place gives no type. spec-dna requires `status` for each type with statuses in the type table. The new optional field `was-type` holds the 0.5 type (`note` or `change`) of a document that `upgrade` changed.
- "Identifiers": a deleted document of an `until-finished` layer gives no SD202. A former `note` or `change` that `upgrade` marks with `was-type` keeps the 0.5 exception ([ADR 0009](0009-upgrade-from-0-5.md)).
- "References and requirements": a requirement is in a `requirements`, `interface` or `rule` document.
- "Lifecycle": the row of `spec`, `contract` and `guide` applies to `requirements`, `interface`, `rule` and `procedure`. In the row of `decision`, "frozen" reads "locked".
- "Lifecycle": the rows of `change` and `note` end. The `intent` row of the type table replaces the row of `change`, and ADR 0006 decides the deletion by `finish`. `research` has no status.
- "Change-risk classes": the classes have the name "diff risk". The name is for the concept only, and no check reads the classes.
- "Knowledge boundaries": "Spec, contract" reads `requirements`, `interface` and `rule`. "Notes, observations" reads `research`. "Change" reads the intent folder.
- "Growth controls" and "Concurrency": the budget defaults are in the type table. New knowledge goes into one decision for each file, or one intent folder for each change.
- Amendments of 0.2.0 ("Knowledge types") and of code citations: `context` counts a document of an `on-request` layer, such as an `incident` or a `research` document. It does not list it. An incident links a requirement, a rule, a decision or a procedure (SD609).
- Amendment of 0.4.2, "Link direction": SD611 has the name `decision-links-intent`, and it reports a link to an intent. SD901 gives the general rule of link direction.

## Amendments
