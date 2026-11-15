# Document types

spec-dna has 15 document types. [ADR 0001](adr/0001-information-model.md) defines the first 6 types of version 0.1.0. Version 0.2.0 adds more, and version 0.6.0 renames and merges some of them (see [Types of 0.5](#types-of-05)). Each type has a default template: `spec-dna new <type> [<id>]` writes a document from it. [Templates](#templates) lists them all.

| Type | Purpose | Statuses | Required sections | Budget (soft, hard) | In `context` |
| --- | --- | --- | --- | --- | --- |
| `product` | What the product is, and for whom | none | none | 8,000, 16,000 | first, for its `applies-to` |
| `system` | The parts of the system and how they work together | none | none | 8,000, 16,000 | first, for its `applies-to` |
| `terms` | The terms of the project: term, meaning and forbidden synonyms | none | none | 10,000, 20,000 | first, for its `applies-to` |
| `roadmap` | Work that the project plans and does not do yet | none | none | 8,000, 16,000 | counted only (`on-request` layer) |
| `questions` | Open questions | none | none | 8,000, 16,000 | counted only (`on-request` layer) |
| `rule` | Project-wide rules with stable ids and an `applies-to` scope | draft, active, deprecated, superseded, retired (status moves forward only) | `Rules` | 3,000, 6,000 | first, for a matching scope |
| `requirements` | The behavior and constraints of one capability | as `rule` | none | 10,000, 20,000 | yes |
| `interface` | An interface, or a pointer to its schema | as `rule` | none | 10,000, 20,000 | yes |
| `decision` | The reason for a choice (MADR minimal) | proposed, accepted, superseded, rejected | `Decision Outcome` or `Decision` | 7,000, 14,000 | summary of a linked decision |
| `procedure` | Conventions and operational procedures | as `rule` | none | 14,000, 28,000 | for its `applies-to` |
| `incident` | What happened, why, and the knowledge it involves | none | `Summary` and `Root cause` | 6,000, 12,000 | counted only (`on-request` layer) |
| `research` | Research, data and other material that agents seldom read | none | none | none | counted only, unless a target names it |
| `intent` | The request for one change | draft, accepted, closed | `Intent`, `Problem` or `Summary` | 6,000, 12,000 | counted only (`on-request` layer) |
| `spec` | The delta that one change makes to the requirements | draft, approved | `Delta` | 10,000, 20,000 | counted only (`on-request` layer) |
| `plan` | The files and the order of one change | draft, approved | `Files` | 6,000, 12,000 | counted only (`on-request` layer) |

A root can change the required sections (`required-sections`) and the budgets (`[budgets.<type>]`). [config.md](config.md) gives the keys.

The types `intent`, `spec` and `plan` are the documents of one change. Their files are `intent/<slug>/intent.md`, `intent/<slug>/spec.md` and `intent/<slug>/plan.md`.

## Types of 0.5

Version 0.6.0 renames and merges the types of 0.5. It accepts no old name as input. A type name of 0.5 in `spec-dna.toml` is an error (exit code 3), and in a front matter it gives SD303. The message names the new type. The name `spec` is also a type of 0.6, so a message cannot tell the 0.5 `spec` from the new one. Use `requirements` for the type that was `spec`. A `spec` document outside the `intent` layer gives SD303, and its message names `requirements` ([Place gives the type](#place-gives-the-type)).

| 0.5 type | 0.6 type | Note |
| --- | --- | --- |
| `spec` | `requirements` | |
| `contract` | `interface` | |
| `invariant` | `rule` | The tier label of `context` is `rule` |
| `guide`, `runbook` | `procedure` | A runbook had the required section `Steps`. Set `required-sections = { procedure = ["Steps"] }` to keep it |
| `note`, `reference` | `research` | The behavior of `reference` stays: it has no budget, and `context` never serves it. SD202 reports its deletion, unless the document has `was-type: note` |
| `change` | `intent` | The statuses are draft, accepted and closed. Use `draft` for `proposed`, and `closed` for `rejected`. A finished change (`done`) is a `research` document |

## Place gives the type

In a mixed root, a document gets its type from this list. The first rule that gives a type wins:

1. The `type` field of its front matter. Only a mixed root lets the front matter set the type. In a root of one type, the root gives the type, and a different `type` field gives SD303.
2. A top file of a layer, named by its type: `terms.md` has the type `terms`. The layer lists it in `files`.
3. The folder of a layer that has one type: each document under `rules/` has the type `rule`. A subfolder, such as an area of `requirements/`, does not change the type.
4. The file name, in the folder of a layer that has more types: `intent/<slug>/intent.md`, `spec.md` and `plan.md` have the types `intent`, `spec` and `plan`.
5. `default-type` of the root.

If no rule gives a type, the document gives SD303. A mixed root has no `default-type` unless the file sets it. Thus `type:` is optional in a new corpus, wherever the place gives the type.

The layer of a document is the first layer in the file whose `paths` match it. If no layer matches, it is the layer that has the type of the document. A `paths` match does not give a type.

The types `intent`, `spec` and `plan` are valid only as `<dir>/<slug>/<type>.md`, where `<dir>` is the folder of the layer that has the type (`intent` by default). A document of one of these types at another place gives SD303. For a `spec`, the message says that the 0.5 type `spec` is the 0.6 type `requirements`. The document probably needs `type: requirements`. These 3 types are also not valid as `type` or `default-type` of a root: that is an error (exit code 3).

A document of a type that no layer has gives SD303. The message says to run `spec-dna layer add`. A project with no `[layers]` table has the [default graph](design/0.6.0.md#default-graph), so each type has a layer.

The default graph gives these places. The folder is the name of the layer, unless the layer sets `dir`.

| Type | Place | First status |
| --- | --- | --- |
| `product`, `system`, `terms`, `roadmap`, `questions` | The top file `<type>.md` | none |
| `rule` | `rules/` | draft |
| `requirements` | `requirements/<area>/` | draft |
| `interface` | `interfaces/` | draft |
| `decision` | `decisions/` | proposed |
| `procedure` | `procedures/` | draft |
| `incident` | `incidents/` | none |
| `research` | `research/` | none |
| `intent`, `spec`, `plan` | `intent/<slug>/<type>.md` | draft |

## Statuses

`draft` is the state of a document that waits for approval. The types `rule`, `requirements`, `interface`, `procedure` and `intent` have no `proposed` status. `proposed` belongs to a decision, which a person accepts or rejects. When a document of another type has the status `proposed`, `check` gives SD304 and names `draft`. A root that uses `proposed` for its requirements can map it: `status-map = { proposed = "draft" }`.

An `intent` ends with the status `closed`, and then you can delete it (SD202 allows it).

## Links between types

In a new corpus, link the documents in these directions. A link of another direction is not an error, except the case that SD611 reports.

| From | To | Why |
| --- | --- | --- |
| `intent` | the `requirements`, `interface`, `rule` or `decision` that it changes | An intent is a request. It links the permanent documents. |
| `decision` | `requirements`, `interface`, `rule`, `research`, and another `decision` (`supersedes`) | A decision gives the reason for a requirement. |
| `requirements`, `interface`, `rule`, `procedure` | `decision` (the reason), `requirements`, `interface` | A permanent document names the reason for its rules. |
| `incident` | `requirements`, `rule`, `decision`, `procedure` | An incident without such a link gives SD609. |
| `research` | any type | These documents give information only. |

A permanent document does not link an intent. An intent ends with the status `closed`, and then you can delete it (SD202 allows it). A link to a deleted intent breaks (SD101, SD102). For a decision, this is a problem that the author cannot fix later: the text of an accepted decision is frozen (SD405).

Thus an accepted decision that links an intent in its locked text or in `depends-on` gets SD611, a warning. To fix it, link the decision from the intent instead. A link in an amendment section, a history section or a live section can change, so SD611 does not report it. In such a section, name the intent as plain text, not as a link. SD901 and `finish` would stop on a link.

## Rule

A rule states what every change in its scope keeps, such as "the system stores amounts in minor units". Its rules are requirements: list items with a bold slug key. Their ids (`<doc-id>#<key>`) are stable, and tests can cover them.

`applies-to` sets the scope: path globs or namespaces. A rule without `applies-to` applies to the whole repository. `context` serves the requirement sections of each rule with a matching scope first, before the entry documents. Keep each rule document short, because its budget is small.

## Procedure

A procedure tells how to do a task: when to use it, the steps, how to verify the result, and how to undo it. It is normative. `context` serves it when its `applies-to` matches, and when it cites the code of the request. The template `procedure` has the sections `Purpose` and `Steps`. A root can require `Steps` with `required-sections = { procedure = ["Steps"] }`.

## Incident

An incident records an event: a summary, a timeline, the root cause, links, and follow-up actions. It must link the requirements, rules, decisions or procedures that it involves, with `[[<id>]]` or a Markdown link. An incident without such a link gives SD609, a warning. An incident describes the past. Thus it gives no drift warning, and `context` only counts it, unless a target names it.

## Research

A research document holds research, data, transcripts or other material that agents seldom read. It has no budget and no status. `context` never serves it, unless a target names it. It counts the research documents that a request reaches, in one line. `show` reads it. Use it for large material that agents read only on request. `spec-dna history` moves the history sections of a document into a research document.

## Intent, spec and plan

An intent holds the request for one change. A spec holds the delta that the change makes to the requirements, and a plan holds the files and the order of the work. An intent has the statuses draft, accepted and closed. A spec and a plan have the statuses draft and approved. These documents describe work that is not done. Thus they give no drift warning (a plan names files that do not exist yet), and `context` only counts them, unless a target names them.

### Fields of an intent

An intent has these front matter fields, in addition to `status`. spec-dna reads them only in an `intent`. In another type, they have no meaning and give no diagnostic.

| Field | Value | Meaning |
| --- | --- | --- |
| `class` | `routine` or `higher-risk` | The class of the change ([Change classes](design/0.6.0.md#change-classes)). No `class`, or an empty value, is `routine`. Another value, such as `small-fix`, gives SD302 and counts as `routine`. A small fix has no intent |
| `areas` | a list of area names | The areas that the change touches. The groups of `[groups]` use them |
| `from` | one id | The id of the incident that the change answers. It is a reference: SD101 reports an id that names no document, and SD901 checks its direction |

The templates `intent`, `prd` and `rfc` have `class: routine`, so `spec-dna new intent <slug>` writes it. A project template of an intent keeps its own front matter.

### Sections that carry list items

The parser keeps the list items of 8 sections for each document. The heading text must be the name of the section. The case and the heading level do not matter. A section ends at the next heading of any level.

| Section | Document | Items |
| --- | --- | --- |
| `Delta` | spec | One delta line each: `ADD <id>#<key>: <text>`, `CHANGE <id>#<key>@<hash>: <text>` or `REMOVE <id>#<key>@<hash>` |
| `Interfaces` | spec | `CHANGE <interface id>: <text>` |
| `Decisions` | spec | Links `[[decisions/<slug>]]` |
| `Concerns` | spec | A task list: `- [ ]` is open, `- [x]` is resolved |
| `Open questions` | spec | A task list, as `Concerns`: `- [ ]` is an open question, `- [x]` is resolved and ends with its resolution. A spec with an open question cannot be `approved` (SD922) |
| `Files` | plan | The first code span of an item is a path or a glob from the repository root. A final `/` matches the folder and all files in it. An item with no code span has no effect |
| `Order` | plan | The tasks of the plan, in order. A list item with a task box and a task id is a task ([Tasks of a plan](#tasks-of-a-plan)) |
| `Follow-ups` | incident | A task list: `- [ ]` is open, `- [x]` is done |

Each item has its text, its line, and its byte range in the file. The range starts at the list marker and ends after the last line of the item, with its nested list. Thus a command can edit an item by its byte range. It keeps the line ends (LF or CRLF), a byte order mark and the final newline. Only the top-level items of a section count. A nested list belongs to its item.

A delta line is one list item with one paragraph and no nested list. The key is a slug. The hash is 16 hexadecimal digits, as `show` prints them. A line can wrap. Each key has one line in the delta. An item in another form is an invalid line: the parser keeps it with its reason, and a check reports it later (SD914).

A requirement also has the byte range of its sentence. The sentence is the text after the bold key and its colon. It has no markers `aliases` and `verified-by` at its end, and no space at its start or at its end. The hash of a requirement is the hash of this text.

### Tasks of a plan

The `Order` section of a plan holds a task list. A task is one top-level list item with a task box and a task id:

```md
- [ ] t1: Add the timer to the valve loop. Files: `src/valve.py`, `tests/`. Verify: `cargo test valve`.
- [x] t2: Write the sensor driver. Files: `src/sensor.py`. Verify: run the driver on the bench.
```

- The box `[ ]` is an open task. The box `[x]` (or `[X]`) is a done task.
- The id is a letter or a digit, then letters, digits, `_` or `-`. The colon follows the id, and then a space or the end of the line. The id is stable: do not renumber the tasks of a started change. The id is case-sensitive.
- The text is what follows the colon, up to the first label. spec-dna joins the lines of the item into one line. It removes one final `.`.
- `Files:` and `Verify:` are labels. They are optional, they are case-sensitive, and each follows a space or the start of the line. A code span (`` ` ``) cannot hold a label. A label ends at the next label, so the order of the 2 labels does not matter. The first label of each kind counts.
- The files of a task are the code spans after `Files:`. They are paths or globs from the repository root.
- The `Verify:` text is a check: a sentence or a command in a code span. An empty text counts as no text.
- A list item of `Order` with no task box, or with no id of this form, is not a task. It is not an error. A nested list belongs to its task, and its items are not tasks.
- If an earlier task of the plan has the same id, the later task is a duplicate. The first task with an id counts. A duplicate is never the next task, and it gives no diagnostic.

The next task is the first task that is open and is not a duplicate, in the order of the file. The commands `status` and `next` read the boxes and name it ([commands.md](commands.md#next)).

## Templates

`spec-dna new <type> [<id>] [--template <name>]` writes a template at the place of its type. spec-dna embeds the templates below. A project can add its own in `[templates] dir` ([config.md](config.md#the-table-templates)). A file there with the name of an embedded template replaces it.

| Template | Type | Written by | Sections |
| --- | --- | --- | --- |
| `prd` | `intent` | `new intent <slug> --template prd` | Problem, Users, Goals, Non-goals, Success measures, Open questions |
| `rfc` | `intent` | `new intent <slug> --template rfc` | Summary, Motivation, Proposal, Alternatives, Unresolved |
| `intent` (default) | `intent` | `new intent <slug>` | Intent, Why now, Out of scope. The 3 intent templates have `class: routine` |
| `spec` (default) | `spec` | `new spec <slug>` | Delta, Interfaces, Decisions, Concerns, Open questions |
| `plan` (default) | `plan` | `new plan <slug>` | Files, Order (2 example tasks), Risks, Proof |
| `madr` (default) | `decision` | `new decision <slug>` | Context and Problem Statement, Considered Options, Decision Outcome, Consequences, Status, Amendments |
| `post-mortem` (default) | `incident` | `new incident <slug>` | Summary, Timeline, Root cause, Links, Follow-ups (a task list), Eval |
| `runbook` | `procedure` | `new procedure <slug> --template runbook` | When to use, Steps, Verify, Rollback |
| `procedure` (default) | `procedure` | `new procedure <slug>` | Purpose, Steps |
| `product`, `system`, `terms`, `roadmap`, `questions` (default) | by name | `new <type>` | Writes `<root>/<type>.md`. `terms` has the table term, meaning and forbidden synonyms. The header cells `Term` and `Forbidden synonyms` (any order, any case) mark the table that SD904 reads. A cell of synonyms splits at `,` and `;`. The column `Meaning` is not read |
| `requirements`, `rule`, `interface`, `research` (default) | by name | `new <type> <id>` | One form for each type |

The `prd` and `rfc` templates satisfy the required section of an `intent`, because each has a `Problem` or a `Summary` heading. Each embedded template is under 700 bytes, and a test checks it.

The `Status` section of a decision is a live section: its text can change after the decision has the status `accepted` (`live-sections`, [config.md](config.md#sections-of-a-decision)). It comes before `Amendments`, because an amendment goes at the end of the file and must not be inside the live section. The `Amendments` section of the template is empty.

A project template is a Markdown file `<dir>/<name>.md`. Its front matter names its `type`. `new` stops with exit code 2 for an unknown name and for a template of another type. `new` copies the template and writes the H1 from the id. It sets the first status of the type. It removes `was-type`, and it removes `type` where the place gives the type. `check` reads no file of the folder as a document.

`spec-dna new <type> [<id>]` writes the template of a type at its place:

- A top file type (`product`, `system`, `terms`, `roadmap`, `questions`) takes no id. `spec-dna new terms` writes `<root>/terms.md`.
- `spec-dna new intent add-sensor` writes `<root>/intent/add-sensor/intent.md`. `spec-dna new spec add-sensor` and `spec-dna new plan add-sensor` write the 2 other files into the same folder. Without that folder, they stop with exit code 5.
- Another type writes `<root>/<dir>/<id>.md` in a mixed root, where `<dir>` is the folder of the layer of the type. For example, `spec-dna new decision pump-choice` writes `<root>/decisions/pump-choice.md`, and `spec-dna new requirements valves/closing` writes `<root>/requirements/valves/closing.md`. The id is relative to that folder.
- In a root of one type, the file is `<root>/<id>.md`.

`spec-dna new` uses this root: `--root <dir>`, or the first root of that single type, or the first mixed root. It writes no `type:` line where the place gives the type, and it writes the line where the place gives none. In a template that has the field `was-type`, it removes the field from the copy.

A placeholder of a template has the form `{{text}}`, such as `{{key}}` or `{{The cause.}}`. `check` reports each placeholder that remains in a document (SD306, an error). Thus a template cannot pass the gate before an author replaces each placeholder. `${{ ... }}` and `{{ ... }}` with spaces inside the braces (GitHub Actions and Jinja syntax) are not placeholders.

This is the template of a rule. It has no `type:` line, because the folder `rules/` gives the type. Where the place gives no type, `new` writes `type: rule`.

```md
---
status: draft
applies-to: ["**"]
---
# Water

## Rules

- **{{key}}**: The {{system}} SHALL {{rule that every change in the scope keeps}}.

## Rationale

{{Why the rule exists. Link the decision.}}
```

## Project types

`types` in `spec-dna.toml` lists the types of a project. A document of another type is an error (SD303), and `spec-dna new` does not accept it. If `types` is absent, the layers of the project give the types. `spec-dna init` writes no `types` list: question 2 writes the layers.
