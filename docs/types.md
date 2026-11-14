# Document types

spec-dna has 15 document types. [ADR 0001](adr/0001-information-model.md) defines the first 6 types of version 0.1.0. Version 0.2.0 adds more, and version 0.6.0 renames and merges some of them (see [Types of 0.5](#types-of-05)). Each type has a template: `spec-dna new <type> <id>` writes a document from it.

| Type | Purpose | Statuses | Required sections | Budget (soft, hard) | In `context` |
| --- | --- | --- | --- | --- | --- |
| `product` | What the product is, and for whom | none | none | 8,000, 16,000 | for its `applies-to` |
| `system` | The parts of the system and how they work together | none | none | 8,000, 16,000 | for its `applies-to` |
| `terms` | The terms of the project: term, meaning and forbidden synonyms | none | none | 10,000, 20,000 | for its `applies-to` |
| `roadmap` | Work that is planned and not done | none | none | 8,000, 16,000 | listed only |
| `questions` | Open questions | none | none | 8,000, 16,000 | listed only |
| `rule` | Project-wide rules with stable ids and an `applies-to` scope | draft, active, deprecated, superseded, retired (status moves forward only) | `Rules` | 3,000, 6,000 | first, for a matching scope |
| `requirements` | The behavior and constraints of one capability | as `rule` | none | 10,000, 20,000 | yes |
| `interface` | An interface, or a pointer to its schema | as `rule` | none | 10,000, 20,000 | yes |
| `decision` | The reason for a choice (MADR minimal) | proposed, accepted, superseded, rejected | `Decision Outcome` or `Decision` | 7,000, 14,000 | summary of a linked decision |
| `procedure` | Conventions and operational procedures | as `rule` | none | 14,000, 28,000 | for its `applies-to` |
| `incident` | What happened, why, and the knowledge it involves | none | `Summary` and `Root cause` | 6,000, 12,000 | listed only |
| `research` | Research, data and other material that agents seldom read | none | none | none | never, unless a target names it |
| `intent` | The request for one change | draft, accepted, closed | `Intent`, `Problem` or `Summary` | 6,000, 12,000 | listed only |
| `spec` | The delta that one change makes to the requirements | draft, approved | `Delta` | 10,000, 20,000 | listed only |
| `plan` | The files and the order of one change | draft, approved | `Files` | 6,000, 12,000 | listed only |

A root can change the required sections (`required-sections`) and the budgets (`[budgets.<type>]`). [config.md](config.md) gives the keys.

The types `intent`, `spec` and `plan` are the documents of one change. Their files are `intent/<slug>/intent.md`, `intent/<slug>/spec.md` and `intent/<slug>/plan.md`.

## Types of 0.5

Version 0.6.0 renames and merges the types of 0.5. It accepts no old name as input. A type name of 0.5 in `spec-dna.toml` is an error (exit code 3), and in a front matter it gives SD303. The message names the new type. The name `spec` is also a type of 0.6, so a message cannot tell the 0.5 `spec` from the new one. Use `requirements` for the type that was `spec`.

| 0.5 type | 0.6 type | Note |
| --- | --- | --- |
| `spec` | `requirements` | |
| `contract` | `interface` | |
| `invariant` | `rule` | The tier label of `context` is `rule` |
| `guide`, `runbook` | `procedure` | A runbook had the required section `Steps`. Set `required-sections = { procedure = ["Steps"] }` to keep it |
| `note`, `reference` | `research` | The behavior of `reference` stays: no budget, `context` never serves it, and SD202 reports its deletion |
| `change` | `intent` | The statuses are draft, accepted and closed. Use `draft` for `proposed`, and `closed` for `rejected`. A finished change (`done`) is a `research` document |

## Statuses and namespaces

Each type has its own first status and its own namespace in a mixed root.

| Type | First status | Namespace of `spec-dna new` |
| --- | --- | --- |
| `product`, `system`, `terms`, `roadmap`, `questions` | none | none |
| `rule` | draft | `rules/` |
| `requirements` | draft | none |
| `interface` | draft | `interfaces/` |
| `decision` | proposed | `decisions/` |
| `procedure` | draft | `procedures/` |
| `incident` | none | `incidents/` |
| `research` | none | `research/` |
| `intent`, `spec`, `plan` | draft | `intent/<slug>/` |

`draft` is the state of a document that waits for approval. The types `rule`, `requirements`, `interface`, `procedure` and `intent` have no `proposed` status. `proposed` belongs to a decision, which a person accepts or rejects. When a document of another type has the status `proposed`, `check` gives SD304 and names `draft`. A root that uses `proposed` for its requirements can map it: `status-map = { proposed = "draft" }`.

An `intent` ends with the status `closed`, and then it can be deleted (SD202 allows it).

## Links between types

In a new corpus, link the documents in these directions. A link of another direction is not an error, except the case that SD611 reports.

| From | To | Why |
| --- | --- | --- |
| `intent` | the `requirements`, `interface`, `rule` or `decision` that it changes | An intent is a request. It links the lasting documents. |
| `decision` | `requirements`, `interface`, `rule`, `research`, and another `decision` (`supersedes`) | A decision gives the reason for a requirement. |
| `requirements`, `interface`, `rule`, `procedure` | `decision` (the reason), `requirements`, `interface` | A lasting document names the reason for its rules. |
| `incident` | `requirements`, `rule`, `decision`, `procedure` | An incident without such a link gives SD609. |
| `research` | any type | These documents give information only. |

A lasting document does not link an intent. An intent ends with the status `closed`, and then it can be deleted (SD202 allows it). A link to a deleted intent breaks (SD101, SD102). For a decision, this is a problem that the author cannot fix later: the text of an accepted decision is frozen (SD405).

Thus an accepted decision that links an intent in its frozen text or in `depends-on` gets SD611, a warning. To fix it, link the decision from the intent instead. A link in an amendment section, a history section or a live section can change, so SD611 does not report it.

## Rule

A rule states what every change in its scope keeps, such as "the system stores amounts in minor units". Its rules are requirements: list items with a bold slug key. Their ids (`<doc-id>#<key>`) are stable, and tests can cover them.

`applies-to` sets the scope: path globs or namespaces. A rule without `applies-to` applies to the whole repository. `context` serves the requirement sections of each matching rule first, before the entry documents. Keep each rule document short, because its budget is small.

## Procedure

A procedure tells how to do a task: when to use it, the steps, how to verify the result, and how to roll back. It is normative. `context` serves it for a matching `applies-to` and when it cites the code of the request. The template `procedure` has the sections `Purpose` and `Steps`. A root can require `Steps` with `required-sections = { procedure = ["Steps"] }`.

## Incident

An incident records an event: a summary, a timeline, the root cause, links, and follow-up actions. It must link the requirements, rules, decisions or procedures that it involves, with `[[<id>]]` or a Markdown link. An incident without such a link gives SD609 (a warning). An incident describes the past. Thus it gives no drift warning, and `context` only lists it.

## Research

A research document holds research, data, transcripts or other material that agents seldom read. It has no budget and no status. `context` never serves it and never lists it, unless a target names it. `show` reads it. Use it for large material that agents read only on request. `spec-dna history` moves the history sections of a document into a research document.

## Intent, spec and plan

An intent holds the request for one change. A spec holds the delta that the change makes to the requirements, and a plan holds the files and the order of the work. An intent has the statuses draft, accepted and closed. A spec and a plan have the statuses draft and approved. These documents describe work that is not done. Thus they give no drift warning (a plan names files that do not exist yet), and `context` only lists them.

## Templates

`spec-dna new <type> <id>` writes the template of a type. In a mixed root, an id without a namespace gets the namespace of its type. For example, `spec-dna new decision pump-choice` writes `<root>/decisions/pump-choice.md`. A requirements document has no namespace. For an intent, a spec or a plan, the id is the slug of the change: `spec-dna new intent add-sensor` writes `<root>/intent/add-sensor/intent.md`. `spec-dna new` uses this root: `--root <dir>`, or the first root of that single type, or the first mixed root.

A placeholder of a template has the form `{{text}}`, such as `{{key}}` or `{{The cause.}}`. `check` reports each placeholder that remains in a document (SD306, an error). Thus a template cannot pass the gate before an author replaces each placeholder. `${{ ... }}` and `{{ ... }}` with spaces inside the braces (GitHub Actions and Jinja syntax) are not placeholders.

This is the template of a rule:

```md
---
type: rule
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

`types` in `spec-dna.toml` lists the types of a project. A document of another type is an error (SD303), and `spec-dna new` does not accept it. If `types` is absent, spec-dna allows every type. `spec-dna init` writes the list from its answers.
