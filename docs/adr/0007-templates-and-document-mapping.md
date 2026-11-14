---
status: accepted
---

# 0007. Templates and the mapping of standard documents

## Context and Problem Statement

Teams write documents in forms that they know by name, such as an ADR, an RFC, a PRD, a runbook, a post-mortem or a threat model. This ADR calls such a form a standard document. 0.6.0 has 15 document types ([ADR 0005](0005-layout-and-layers.md)). Which standard documents get a type, which get a template, and which stay outside spec-dna? How does a project use its own form of a document?

Constraints:

- spec-dna must work for any repository, any stack and any document language. The language of normative text is an `init` choice.
- A type is a contract: statuses, a layer, a place, budgets, required sections and checks. Each new type needs a release and a step in `upgrade`, and agents must learn it.
- A gate needs only spec-dna and git ([ADR 0001](0001-information-model.md)). `new` must need no network and no second runtime.
- spec-dna ships only text that it can bundle: its own words, and MADR under its MIT licence ([matrix](../research/matrix.md), section 2).
- The placeholder `{{text}}` and SD306 (an error for a placeholder that stays in a document) are contracts of 0.5.

Evidence (the details stay outside this repository):

- The gate against the base gave the measured gain. Retrieval and the split of big documents did not.
- Agents search the repository. They seldom follow a tree of links.
- Large files and process documents in the text that each task loads cost tokens and give no gain.
- In 0.5, `guide` and `runbook` had the same statuses, the same authority and the same rule in `context`. Their sections, their budgets and the required section `Steps` of a runbook were different.
- A real corpus edits a status section of an accepted decision in place. When spec-dna locked that text, agents left the status stale (ADR 0001, amendment of 2026-11-13 for 0.3.2).

## Considered Options

1. One document type for each standard document: an RFC type, a PRD type or a runbook type, for example.
2. The layout and the templates of another framework, with no change: Spec Kit, OpenSpec, BMAD or arc42.
3. Few types, each with a lifetime of its own. Standard documents are templates of these types, in a template language with logic, such as Jinja.
4. As option 3, but each template is plain Markdown with `{{text}}` placeholders. spec-dna ships templates, and a project adds its own in a folder outside every root.

## Decision Outcome

Chosen option: 4.

### Mapping principle

- A standard document maps by its lifetime and its authority, not by its name. Agents search, so the words and the place of a document help them more than a type name. These questions give the type, in this order:
  1. Does another file or tool keep it, such as `CHANGELOG.md` or an OpenAPI file? Then it stays outside spec-dna.
  2. Does it describe one change? Then it goes into the intent folder: `intent`, `spec` or `plan`.
  3. Does it give the reason for a choice? Then it is a `decision`.
  4. Does it state the current truth? Then it is `product`, `system`, `terms`, `requirements`, `interface`, `rule` or `procedure`.
  5. Does it state later work or an open question? Then it is `roadmap` or `questions`.
  6. Does it record a failure and its root cause? Then it is an `incident`.
  7. Does it only inform? Then it is `research`.
- A standard document with parts of different lifetimes splits. Each part goes to the type of its lifetime. For example, a threat model is `research`, and each mitigation is a `rule`.
- A standard document that differs from a type only by its sections is a template of that type. Its sections never make a new type. Its headings map by `required-sections`, as the headings of a Nygard ADR do.
- This ADR adds no type. A later type needs an ADR that names its lifecycle, its layer, or a check that no root key can give.
- The [mapping table](../design/0.6.0.md#mapping-of-standard-documents) of the design page holds the rows.

### Runbook, PRD and RFC

- A runbook is the template `runbook` of the type `procedure`. It differs from a guide by its sections and by SD305 for `Steps`, and a root gives that check with `required-sections`.
- A PRD and an RFC are the templates `prd` and `rfc` of the type `intent`. Each states the request of one change before its design. Each has the statuses of `intent` and the gate of the Plan stage. Its file is `intent.md`, which gives the type and the stage, so the stage derivation needs no new rule.
- `finish` applies only the delta of `spec.md`, and then it deletes the intent folder. Thus the author moves the permanent parts of a PRD or an RFC before `finish`:
  - A requirement moves by a delta line of `spec.md`. A `routine` change has no `spec.md`, so its author edits the requirement in the pull request of the change.
  - A goal of a PRD that stays true moves by hand into `product.md` or into `requirements`.
  - The reasons and the rejected alternatives of an RFC move into a `decision`. In a `higher-risk` change, the `Decisions` section of `spec.md` links it.

### Shipped templates

- spec-dna embeds the templates of the [Templates table](../design/0.6.0.md#templates). Each type that `new` writes has one default template.
- Each shipped template is in the words of spec-dna. `madr` adapts the minimal template of MADR 4.0.0 under its MIT licence. No template copies share-alike or paid text.
- A shipped template stays small, because each new document starts from it. A test parses each shipped template as its type, and checks its required sections and its size.

### Project templates

- `[templates] dir` (default `.spec-dna/templates`) holds the project templates. The folder is outside every root, or an `exclude` glob removes it, as for a convention file (ADR 0001, amendment of 2026-11-13 for 0.3.3).
- A project template is a file `<name>.md` whose front matter names its `type`. A project file with the name of a shipped template replaces it.
- The [Templates](../design/0.6.0.md#templates) section gives the exit codes and the edits that `new` makes to the copy.
- `new` removes `was-type` from the copy, because SD202 allows the deletion of a document with that field.
- spec-dna does not check a project template. The document that `new` writes gets the usual checks, such as SD305 and SD306.

### Placeholders

- A placeholder keeps the form `{{text}}`. SD306 keeps its sense and its severity, so a template cannot pass the gate before the author replaces each placeholder.
- spec-dna has no template language: no variable, condition, loop or include. The title is the one value that `new` writes into a template.
- `{{ x }}` and `${{ x }}` stay literal text, as in 0.5. Thus a template can show Jinja or GitHub Actions code.

### Documents that `finish` writes

- For an `ADD` line to a `requirements` document that does not exist, `finish` writes a minimal document with no placeholder, not a template copy. [Finish](../design/0.6.0.md#finish), step 4, gives its content.
- `new` starts a document that a person writes. Thus it has placeholders and the first status of the type, and SD306 stops the gate until the author replaces them.
- `finish` writes only text that an approved spec holds. Thus it reads no template, and a project template cannot add a placeholder to the result.

### Decision template

- `madr` has a `Status` section. The default of `live-sections` for `decision` is `Status`, so this section can change after the outcome is locked.
- `Status` comes before `Amendments`. Thus an amendment at the end of the file stays outside the live section, and SD405 and SD605 apply to it.
- The `Status` section records the state of the work that the decision starts, and spec-dna does not read its words. The front matter `status` records the lifecycle of the decision. spec-dna reads only this field, or the default of the root.
- A Nygard record keeps its lifecycle word in a `Status` section. In a new root, that section is live, so an author can write "Superseded by" in place without SD405.

### Post-mortem template and evals

- `post-mortem` has an `Eval` section with a placeholder, so SD306 stops the gate until the author replaces the placeholder. The author links the eval file in `[evals] dir` (SD102 checks the path), or writes why the incident has no eval.
- SD918 ([ADR 0006](0006-change-lifecycle.md)) warns about a change that adds an incident and no eval ([Incidents](../design/0.6.0.md#incidents)).
- These rules apply the owner decision that every incident adds an eval.
- `Follow-ups` is a task list. An open item puts the incident in the Maintain stage, and a lesson moves up by an intent with `from:` ([Stage derivation](../design/0.6.0.md#stage-derivation)).

### Rejected

- One type for each standard document (an RFC type, a PRD type, a runbook type). An RFC, a PRD and an intent have one lifecycle, and a runbook and a guide have another. New types add statuses, places, budgets, configuration keys and `upgrade` steps. They add no check that `required-sections` cannot give. Teams use many names for one form, so the list of types has no end.
- The layout of another framework, with no change. It needs the tool of the framework. Spec Kit numbers its change folders, and ADR 0001 rejects counters. OpenSpec keeps each finished change in an archive folder in the tree, so process text stays in the search path. BMAD stores process output, not a current truth. arc42 has many sections and a share-alike licence.
- A template language with logic (such as Jinja). It adds a dependency and a syntax that authors and agents must learn. `{{ x }}` is a Jinja expression and not a placeholder, so one pair of braces would have 2 senses. Logic also hides the form of the document.
- Templates only inside the binary. A project cannot change its sections, its heading aliases or the language of its documents. Each change of a form would need a release of spec-dna.
- Templates inside a document root. There a template is a document: it gets an id, SD306 errors, a budget and link checks, and `context` can serve it as truth. An `exclude` glob is a second step that a project can forget.
- Templates that spec-dna generates from the configuration (`required-sections`, `[conventions]`). A list of headings gives no placeholder text that tells the author what to write. 2 sources would state the form of one document, and a review would not show the form.
- A separate `adr` folder next to `decisions/`. A layer has one `dir`, and the place rule gives one folder to each layer. A second folder splits the ids of one type, and an agent must search 2 places. A project that wants the name sets `[layers.decisions] dir = "adr"`, and `ADR-NNN` ids resolve as aliases.
- `guide` as a type of its own. A guide and a runbook had the same statuses, authority and rule in `context`. 2 types made each author choose. The one different check, SD305 for `Steps`, is a root key. One type, `procedure`, holds both, and the `runbook` template keeps the runbook form.
- A front matter field that records the template, such as `template: prd`. No check reads it, and nobody keeps it true after the sections change.
- Templates from a URL or a registry. They need a network, and the result of `new` would depend on a service.

### Consequences

- Good: 15 types hold the standard documents of the mapping table. A new standard document needs a row and a template, not a release.
- Good: no new dependency and no new syntax. SD306 keeps its sense.
- Bad: `finish` deletes the text of a PRD or an RFC. A permanent part that the author does not move stays only in git, and `spec-dna log` lists the change.
- Bad: the word "status" names the front matter field and the live section. The `madr` template says which is which.
- Bad: a project template without a required section gives SD305 on each new document.
- Bad: a root that requires more sections of `requirements` gets SD305 on a document that `finish` writes. The author adds the sections before the commit.
- Bad: an upgraded project, or a project that answers `none` at question 14, gets no SD918. There, only the `Eval` section of the template asks for an eval.
- Neutral: the shipped templates are in English. A root with another `language` or other heading aliases needs its own templates, and a project can write them.
- Neutral: a former runbook gets the larger budget of `procedure`. It keeps the required section `Steps` only in a root that held only runbooks, because `upgrade` writes `required-sections` there.

## Amends

This ADR amends these parts of ADR 0001. It does not edit them, because ADR 0001 is accepted.

- "Document types": ADR 0005 replaces the list. This ADR adds the mapping principle: a standard document is a template of a type, or it stays outside spec-dna.
- Amendment of 2026-11-08 for 0.2.0, item "Knowledge types": `runbook` is not a type. It is the template `runbook` of `procedure`. "Each type has a template" now means a default template, and a project can add templates.
- Amendment of 2026-11-13 for 0.3.2, item "Live sections": the default of `live-sections` for `decision` is `Status`. Before, there was no default. An upgraded root keeps the 0.5 behavior, because `upgrade` writes `live-sections = {}`.
- "Layers" (ADR 0005 renames the table "system parts"), row "Project configuration": it also covers the project templates in `[templates] dir`, outside every document root.

## Amendments
