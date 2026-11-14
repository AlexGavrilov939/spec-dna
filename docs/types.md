# Document types

spec-dna has 10 document types. [ADR 0001](adr/0001-information-model.md) defines the first 6. Version 0.2.0 adds `invariant`, `runbook`, `incident` and `reference`. Each type has a template: `spec-dna new <type> <id>` writes a document from it.

| Type | Purpose | Statuses | Required sections | Budget (soft, hard) | In `context` |
| --- | --- | --- | --- | --- | --- |
| `spec` | The behavior and constraints of one capability | draft, active, deprecated, superseded, retired | none | 10,000, 20,000 | yes |
| `contract` | An interface, or a pointer to its schema | as spec | none | 10,000, 20,000 | yes |
| `guide` | Conventions and procedures | as spec | none | 14,000, 28,000 | for its `applies-to` |
| `decision` | The reason for a choice (MADR minimal) | proposed, accepted, superseded, rejected | `Decision Outcome` or `Decision` | 7,000, 14,000 | summary of a linked decision |
| `change` | A proposal, plan or task | proposed, accepted, done, rejected | none | 10,000, 20,000 | listed only |
| `note` | Observations and informative material | none | none | none | listed only |
| `invariant` | Project-wide rules with stable ids and an `applies-to` scope | as spec (status moves forward only) | `Rules` | 3,000, 6,000 | first, for a matching scope |
| `runbook` | An operational procedure | as spec | `Steps` | 8,000, 16,000 | for its `applies-to` |
| `incident` | What happened, why, and the knowledge it involves | none | `Summary` and `Root cause` | 6,000, 12,000 | listed only |
| `reference` | Research and other material that agents seldom read | none | none | none | never, unless a target names it |

A root can change the required sections (`required-sections`) and the budgets (`[budgets.<type>]`). [config.md](config.md) gives the keys.

## Invariant

An invariant states rules that every change in its scope keeps, such as "the system stores amounts in minor units". Its rules are requirements: list items with a bold slug key. Their ids (`<doc-id>#<key>`) are stable, and tests can cover them.

`applies-to` sets the scope: path globs or namespaces. An invariant without `applies-to` applies to the whole repository. `context` serves the requirement sections of each matching invariant first, before the entry documents. Keep each invariant short, because its budget is small.

## Runbook

A runbook is a procedure for operations. It states when to use the procedure, the steps, how to verify the result, and how to roll back. It is normative, like a guide. `context` serves it for a matching `applies-to` and when it cites the code of the request.

## Incident

An incident records an event: a summary, a timeline, the root cause, links, and follow-up actions. It must link the requirements, decisions or runbooks that it involves, with `[[<id>]]` or a Markdown link. An incident without such a link gives SD609 (a warning). An incident describes the past. Thus it gives no drift warning, and `context` only lists it.

## Reference

A reference document holds research, data, transcripts or other material that agents seldom read. It has no budget and no status. `context` never serves it and never lists it, unless a target names it. `show` reads it. Use it for large material that agents read only on request.

## Templates

`spec-dna new <type> <id>` writes the template of a type. In a mixed root, an id without a namespace gets the namespace of its type. For example, `spec-dna new decision pump-choice` writes `<root>/decisions/pump-choice.md`. A spec has no namespace. `spec-dna new` uses this root: `--root <dir>`, or the first root of that single type, or the first mixed root.

A placeholder of a template has the form `{{text}}`, such as `{{key}}` or `{{The cause.}}`. `check` reports each placeholder that remains in a document (SD306, an error). Thus a template cannot pass the gate before an author replaces each placeholder. `${{ ... }}` and `{{ ... }}` with spaces inside the braces (GitHub Actions and Jinja syntax) are not placeholders.

This is the template of an invariant:

```md
---
type: invariant
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
