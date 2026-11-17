# Document types

This page gives the 15 document types of spec-dna. It lists the purpose, the statuses, the required sections, the budgets and the use in `context` of each type. Read it when you write a document or choose a type.

[ADR 0001](adr/0001-information-model.md) defines the first 6 types of version 0.1.0. Version 0.2.0 adds more, and version 0.6.0 renames and merges some of them (see [Types of 0.5](#types-of-05)). Each type has a default template: `spec-dna new <type> [<id>]` writes a document from it. [Templates](#templates) lists them all.

| Type | Purpose | Statuses | Required sections | Budget (soft, hard) | In `context` |
| --- | --- | --- | --- | --- | --- |
| `product` | What the product is, and for whom | none | none | 8,000, 16,000 | first, for its `applies-to` |
| `system` | The parts of the system and how they work together | none | none | 8,000, 16,000 | first, for its `applies-to` |
| `terms` | The terms of the project: term, meaning and forbidden synonyms | none | none | 10,000, 20,000 | first, for its `applies-to` |
| `roadmap` | Work that the project plans and does not do yet | none | none | 8,000, 16,000 | counted only (`on-request` layer) |
| `questions` | Open questions | none | none | 8,000, 16,000 | counted only (`on-request` layer) |
| `rule` | Project-wide rules with stable ids and an `applies-to` scope | draft, active, deprecated, superseded, retired (status moves forward only) | `Rules` | 3,000, 6,000 | first, for a scope that matches |
| `requirements` | The behavior and constraints of one capability | as `rule` | none | 10,000, 20,000 | yes |
| `interface` | An interface, or a pointer to its schema | as `rule` | none | 10,000, 20,000 | yes |
| `decision` | The reason for a choice (MADR minimal) | proposed, accepted, superseded, rejected | `Decision Outcome` or `Decision` | 7,000, 14,000 | summary of a linked decision |
| `procedure` | Conventions and operational procedures | as `rule` | none | 14,000, 28,000 | for its `applies-to` |
| `incident` | What happened, why, and the knowledge it involves | none | `Summary` and `Root cause` | 6,000, 12,000 | counted only (`on-request` layer) |
| `research` | Research, data and other material that agents seldom read | none | none | none | counted only, unless a target names it |
| `intent` | The request for one change | draft, accepted, closed | `Intent`, `Problem` or `Summary` | 6,000, 12,000 | counted only (`on-request` layer) |
| `spec` | The delta that one change makes to the requirements | draft, approved | `Delta` | 10,000, 20,000 | counted only (`on-request` layer) |
| `plan` | The files and the order of one change | draft, approved | `Files` | none | counted only (`on-request` layer) |

A root can change the required sections (`required-sections`) and the budgets (`[budgets.<type>]`). [config.md](config.md) gives the keys.

The types `intent`, `spec` and `plan` are the documents of one change. Their files are `intent/<slug>/intent.md`, `intent/<slug>/spec.md` and `intent/<slug>/plan.md`.

## Types of 0.5

Version 0.6.0 renames and merges the types of 0.5. It accepts no old name as input. A type name of 0.5 in `spec-dna.toml` is an error (exit code 3), and in a front matter it gives SD303. The message names the new type.

The name `spec` is also a type of 0.6, so a message cannot tell the 0.5 `spec` from the new one. Use `requirements` for the type that was `spec`. A `spec` document outside the `intent` layer gives SD303, and its message names `requirements` ([Place gives the type](#place-gives-the-type)).

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

`spec-dna mv` of a whole document into a mixed root removes the `type` line that the new place makes redundant, and it names a change of the type ([mv](commands.md#mv)).

If no rule gives a type, the document gives SD303. A mixed root has no `default-type` unless the file sets it. Thus `type:` is optional in a new corpus, wherever the place gives the type.

The layer of a document is the first layer in the file whose `paths` match it. If no layer matches, it is the layer that has the type of the document. A `paths` match does not give a type.

The types `intent`, `spec` and `plan` are valid only as `<dir>/<slug>/<type>.md`. `<dir>` is the folder of the layer that has the type (`intent` by default). A document of one of these types at another place gives SD303. For a `spec`, the message says that the 0.5 type `spec` is the 0.6 type `requirements`. The document can need `type: requirements`.

These 3 types are also not valid as `type` or `default-type` of a root. That is an error (exit code 3).

A document of a type that no layer has gives SD303. The message says to run `spec-dna layer add`. A project with no `[layers]` table has the [default graph](config.md#default-graph), so each type has a layer.

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

A change ends with `finish`, which deletes its intent folder: an accepted change after its work, and a closed change at once. A folder that a pull request deletes with an unapplied delta gives SD915.

## Links between types

The layer graph gives the direction of each link. A document must not link a document of a less stable layer, unless that layer lists it in `linked-from` (SD901). [config.md](config.md#default-graph) gives the default graph. An incident links the requirement, the rule, the decision or the procedure that it involves (SD609).

A permanent document does not link an intent. `finish` deletes the intent folder, so a link to it breaks (SD101, SD102). For a decision, this is a problem that the author cannot fix later: the text of an accepted decision is locked (SD405).

Thus an accepted decision that links an intent in its locked text or in `depends-on` gets SD611, a warning. To fix it, link the decision from the intent instead. A link in an amendment section, a history section or a live section can change, so SD611 does not report it. In such a section, name the intent as plain text, not as a link. A link there gives SD901, and `finish` stops on it.

## Rule

A rule states what every change in its scope keeps, such as "the system stores amounts in minor units". Its rules are requirements: list items with a bold slug key. Their ids (`<doc-id>#<key>`) are stable, and tests can cover them.

`applies-to` sets the scope: path globs or namespaces. A rule without `applies-to` applies to the whole repository. `context` first serves the requirement sections of each rule whose scope matches, before the entry documents. Keep each rule document short, because its budget is small.

## Procedure

A procedure tells how to do a task: when to use it, the steps, how to verify the result, and how to undo it. It is normative. `context` serves it when its `applies-to` matches, and when it cites the code of the request. The template `procedure` has the sections `Purpose` and `Steps`. A root can require `Steps` with `required-sections = { procedure = ["Steps"] }`.

## Incident

An incident records an event: a summary, a timeline, the root cause, links, and follow-up actions. It must link the requirements, rules, decisions or procedures that it involves, with `[[<id>]]` or a Markdown link. An incident without such a link gives SD609, a warning. An incident describes the past. Thus it gives no drift warning, and `context` only counts it, unless a target names it.

## Research

A research document holds research, data, transcripts or other material that agents seldom read. It has no budget and no status. `context` never serves it, unless a target names it. It counts the research documents that a request reaches, in one line. `show` reads it.

Use it for large material that agents read only on request. `spec-dna history` moves the history sections of a document into a research document.

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
| `Interfaces` | spec | `CHANGE <interface id>: <text>` for an interface that exists, `ADD <interface id>: <text>` for a new interface, or one item `none` ([the `none` rule](#the-none-rule)) |
| `Decisions` | spec | Links `[[decisions/<slug>]]` |
| `Concerns` | spec | A task list: `- [ ]` is open, `- [x]` is resolved. An item with no box gives SD925 |
| `Open questions` | spec | A task list, as `Concerns`: `- [ ]` is an open question, `- [x]` is resolved and ends with its resolution. A spec with an open question cannot be `approved` (SD922). An item with no box gives SD925 |
| `Files` | plan | An item is a path or a glob from the repository root. Its first code span is the path. An item with no code span is a path if its text is a plain path ([Files and paths](#files-and-paths)). A final `/` matches the folder and all files in it. Another item has no effect. The section can also have a paragraph ([A paragraph under `Files`](#a-paragraph-under-files)) |
| `Order` | plan | The tasks of the plan, in order. A list item with a task box and a task id is a task ([Tasks of a plan](#tasks-of-a-plan)) |
| `Follow-ups` | incident | A task list: `- [ ]` is open, `- [x]` is done |

Each item has its text, its line, and its byte range in the file. The range starts at the list marker and ends after the last line of the item, with its nested list. Thus a command can edit an item by its byte range. It keeps the line ends (LF or CRLF), a byte order mark and the final newline. Only the top-level items of a section count. A nested list belongs to its item.

A delta line is one list item with one paragraph and no nested list. The key is a slug. The hash is 16 hexadecimal digits, as `show` prints them. A line can wrap. Each key has one line in the delta. An item in another form is an invalid line: the parser keeps it with its reason, and a check reports it later (SD914).

A requirement also has the byte range of its sentence. The sentence is the text after the bold key and its colon. It has no markers `aliases` and `verified-by` at its end, and no space at its start or at its end. The hash of a requirement is the hash of this text.

### Tasks of a plan

The `Order` section of a plan holds a task list. A task is one top-level list item with a task box and a task id:

```md
- [ ] t1: Add the timer to the valve loop. Files: `src/valve.py`, `tests/`. Verify: `cargo test valve`.
- [x] t2: Write the sensor driver. Files: `src/sensor.py`. Verify: run the driver on the bench. Hours: 1-2.
- [x] t3: Add a second timer. Files: `src/valve.py`. Verify: `cargo test valve`. Hours: 2. Skip: the first timer is enough.
```

- The box `[ ]` is an open task. The box `[x]` (or `[X]`) is a done task. A task with a reason in `Skip:` is a skipped task, whatever its box.
- The id is a letter or a digit, then letters, digits, `_` or `-`. The colon follows the id, and then a space or the end of the line. The id is stable: do not renumber the tasks of a started change. The id is case-sensitive.
- The text is what follows the colon, up to the first label. spec-dna joins the lines of the item into one line. It removes one final `.`.
- `Files:`, `Verify:`, `Hours:` and `Skip:` are labels. They are optional, they are case-sensitive, and each follows a space or the start of the line. A code span (`` ` ``) cannot hold a label. A label ends at the next label, so the order of the labels does not matter. The first label of each kind counts.
- The files of a task are the entries after `Files:`. spec-dna removes one final `.` or `;` of the text, and the parentheses that wrap it. An entry is a part of the text up to a comma that is not in a code span and not in a link (`[text](destination)`). An entry gives its code spans, or, if it has no code span, its text if the text is a plain path. They are paths or globs from the repository root.
- The `Verify:` text is a check: a sentence or a command in a code span. An empty text counts as no text.
- The value of `Hours:` is the estimate or the real time of the task, as `<n>` or as a range `<a>-<b>` with `a` not above `b`. A number has up to 4 digits, up to 2 decimals after a `.`, and an optional `h`. Spaces or tabs can be around the `-`. `finish` adds the hours of the tasks, and prints the line `Hours: <total>` ([commands.md](commands.md#finish)). When you know the real time, replace a range with a number. Another value is invalid: `finish` gives a note and no total. A plan has no other source of hours: the section `Hours` is plain text, and nothing reads it.
- The text of `Skip:` is the reason why the task is not needed, for example after a spike. A task with a reason is skipped: it is closed, and `next`, `status` and the counts treat it as closed. `finish` leaves its hours out. Write `- [x]` for a skipped task. A `Skip:` with no reason does not skip the task, and SD923 reports it.
- The text of `Files:` is `none` when its first word is `none`, followed by `:`, `.` or the end of the text ([the `none` rule](#the-none-rule)). Such a task lists no path.
- An `approved` plan gives SD923 for a task with no `Files:` or no `Verify:`, with a glob or a folder in `Files:`, or with a `Skip:` that has no reason. A skipped task still needs `Files:` and `Verify:`. It gives SD924 for a task with more than `[change] max-task-files` paths (default 3). A `draft` plan gives neither ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)).
- A list item of `Order` with no task box, or with no id of this form, is not a task. It is not an error. A nested list belongs to its task, and its items are not tasks.
- If an earlier task of the plan has the same id, the later task is a duplicate. The first task with an id counts. A duplicate is never the next task, and it gives no diagnostic.

The next task is the first task that is not closed (not done and not skipped) and is not a duplicate, in the order of the file. The commands `status` and `next` read the boxes and name it ([commands.md](commands.md#next)).

### Files and paths

A `Files` item of a plan, and an entry of the label `Files:` of a task, gives paths and globs. The code span is the first choice. Write each path in a code span, as `` `src/valve.py` ``.

An item that has no code span, or an entry that has none, is a plain path if it follows these rules:

- The text is one token: it has no white space. A link `[text](destination)` gives its text. A destination is not read.
- Before this check, spec-dna removes the white space at the ends of the text. It also removes one final `.`, `,`, `:` or `;` (a mark of the sentence), and the parentheses that wrap the text.
- The token has a `/`, a `.` or a `*`, and a letter, a digit or a `*`. Thus `src/dose.py`, `tests/*.py`, `README.md` and `docs/` are plain paths. `Makefile`, `LICENSE` and `none` are words, and not paths: write a file with no `/` and no `.` in a code span.
- The token has no backtick, no `<`, no `>`, no `"`, no `|`, no `{{` and no `://`. A URL, a tag and a template placeholder are not paths.
- `src/a.py and src/b.py`, `no code here` and `src/a.py: the main file` are prose, and they are not paths. An item or an entry that is not a path has no effect. A code span that holds a template placeholder (`` `{{path}}` ``) is not a path either.

The rules are about the form only. A token such as `n/a` or `**src/a.py**` passes them, and it is then a pattern that matches no file. Write a path in a code span if it has a form that these rules do not take.

A plain path has the same meaning as the same text in a code span. A final `/` matches the folder and all files in it.

The patterns of a plan are the paths of its section `Files`, then the `Files:` paths of its tasks of `Order`, with no repeat. Each reader of the scope uses this list: the checks SD905, SD910, SD911, SD913, SD915, SD916, SD917 and SD921, `finish`, `stats --sdlc`, `status` and `next`. A plan can list its paths only in its tasks.

A plan can state that the work edits no file. The `Files` section then has one item only, a list item, and the item starts with the word `none` in any case, such as `- none: research only`. The word ends at the first character that is not a letter or a digit, so `nonexistent` is not the word. Such a plan lists no file when no task of `Order` has a path in `Files:`. In that case, the item is not a wrong item: `next` and `status` do not name it, and the messages of SD913 and SD915 do not name it. For a routine change, `next` and `status` then show `[x] the plan lists no files: the change edits no file` in place of the step of the files ([commands.md](commands.md#next)). A higher-risk change keeps the step of the files, because its spec has a delta.

### A paragraph under `Files`

The parser keeps the first paragraph of the section `Files` that is not in a list. A paragraph whose first word is `none` means the same as the list item `- none`. The word is `none` in any case. It must be followed by `:`, `.` or the end of the paragraph. So `None.` and `None: this change is research.` state "no files". `None of the old files stay.` is another paragraph.

Another paragraph gives no path. When the section has a paragraph and no list item, `next`, `status` and the messages of SD913 and SD915 say: "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`". When the section has a list item, the list gives the paths, and the paragraph is only text.

Assume that the items of `Files` are not paths, and the plan lists no path in `Files` or in its tasks. Then `next` and `status` say so and show the first such item. The messages of SD913 and SD915 do the same ([diagnostics](diagnostics.md)). If the plan lists a path, they do not name an item, because the item is not the reason for the diagnostic.

### The `none` rule

One item can state "nothing" in the sections `Interfaces`, `Concerns` and `Open questions`. The section then has one item only, and the first word of the item is `none`, in any case. The word must be followed by `:`, `.` or the end of the text. The task box does not matter: `- none`, `- None.` and `- [ ] None.` state "nothing". `- None of the risks is open` does not, and a `none` item with other items is an item like the others.

A section that states "nothing" gives no SD914 (`Interfaces`), no SD919 (`Concerns`), no SD922 (`Open questions`) and no SD925. A list item under `Files` keeps the rule of 0.6.1: it states "no files" when it is the only item and its first word is `none`. The paragraph under `Files` uses the rule of this section.

## Templates

`spec-dna new <type> [<id>] [--template <name>]` writes a template at the place of its type. spec-dna embeds the templates below. A project can add its own in `[templates] dir` ([config.md](config.md#the-table-templates)). A file there with the name of an embedded template replaces it.

| Template | Type | Written by | Sections |
| --- | --- | --- | --- |
| `prd` | `intent` | `new intent <slug> --template prd` | Problem, Users, Goals, Non-goals, Success measures, Open questions |
| `rfc` | `intent` | `new intent <slug> --template rfc` | Summary, Motivation, Proposal, Alternatives, Unresolved |
| `intent` (default) | `intent` | `new intent <slug>` | Intent, Why now, Out of scope. The 3 intent templates have `class: routine` |
| `spec` (default) | `spec` | `new spec <slug>` | Delta, Interfaces, Decisions, Concerns (the item `- [ ] {{An open concern.}}`), Open questions |
| `plan` (default) | `plan` | `new plan <slug>` | Files (the item `` `{{path}}` ``), Order (2 example tasks, each with `` Files: `{{path}}` ``), Risks, Proof |
| `madr` (default) | `decision` | `new decision <slug>` | Context and Problem Statement, Considered Options, Decision Outcome, Consequences, Status, Amendments |
| `post-mortem` (default) | `incident` | `new incident <slug>` | Summary, Timeline, Root cause, Links, Follow-ups (a task list), Eval |
| `runbook` | `procedure` | `new procedure <slug> --template runbook` | When to use, Steps, Verify, Rollback |
| `procedure` (default) | `procedure` | `new procedure <slug>` | Purpose, Steps |
| `product`, `system`, `terms`, `roadmap`, `questions` (default) | by name | `new <type>` | Writes `<root>/<type>.md`. `terms` has the table term, meaning and forbidden synonyms. The header cells `Term` and `Forbidden synonyms` (any order, any case) mark the table that SD904 reads. A cell of synonyms splits at `,` and `;`. The column `Meaning` is not read |
| `requirements`, `rule`, `interface`, `research` (default) | by name | `new <type> <id>` | One form for each type |

The `prd` and `rfc` templates satisfy the required section of an `intent`, because each has a `Problem` or a `Summary` heading. Each embedded template is under 700 bytes, and a test checks it.

The `Status` section of a decision is a live section: its text can change after the decision has the status `accepted` (`live-sections`, [config.md](config.md#sections-of-a-decision)). It comes before `Amendments`, because an amendment goes at the end of the file and must not be inside the live section. The `Amendments` section of the template is empty.

A project template is a Markdown file `<dir>/<name>.md`. Its front matter names its `type`. `new` stops with exit code 2 for an unknown name and for a template of another type. `new` copies the template and writes the H1 from the id. For `intent`, `spec` and `plan`, the H1 is the placeholder `{{The title of the change.}}`, because a slug is a short folder name, and no rule makes a title from it. SD306 reports it until the author writes the title. It sets the first status of the type.

`new` also removes `was-type`, and it removes `type` where the place gives the type. `check` reads no file of the folder as a document.

`spec-dna new <type> [<id>]` writes the template of a type at the place of its layer. [commands.md](commands.md#new) gives each place and the exit codes.

`spec-dna new` uses this root: `--root <dir>`, or the first root of that single type, or the first mixed root. It writes the `type:` line only where the place gives no type.

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

## Standard documents

A team can already know a document form, such as an ADR, a PRD or a runbook. The table gives the 0.6 form of each standard document. The first column is the name that a team uses.

| Standard document | 0.6 form | Note |
| --- | --- | --- |
| ADR (Nygard, MADR, Y-statement) | `decision` | `required-sections` maps the section names. A Y-statement has no heading for the outcome, so give it one |
| RFC, design proposal | `intent` with `--template rfc`, and `spec` for the design | The delta of the `spec` holds the requirements. A `decision` holds the reasons. `finish` deletes the RFC text |
| PRD | `intent` with `--template prd` | `finish` deletes the intent. Before `finish`, the author moves each goal that stays into `product` or `requirements` |
| Technical specification, design document | `spec` (one change) and `requirements` (the current truth) | |
| Software requirements specification (ISO/IEC/IEEE 29148) | `requirements` | One document for each capability, one folder for each area |
| User story | `intent` | Each acceptance criterion becomes an `ADD` line |
| Epic | An entry of `roadmap` that names its intents | One intent is one change, and `max-started` limits them |
| API reference, OpenAPI, AsyncAPI | `interface` that names the schema file | |
| Architecture overview, C4 levels 1 and 2 | `system` | Levels 3 and 4 stay in the code or in `research` |
| Product vision, strategy | `product` | |
| Glossary, controlled vocabulary | `terms` | |
| Roadmap | `roadmap` | |
| Open questions, risk register | `questions` | |
| Engineering standard, coding convention | `rule` (a testable rule) or `procedure` (a how-to) | |
| Non-functional requirement | `rule` if it applies to the whole project, `requirements` if it is local | |
| SLO, SLA | `rule`, or `requirements` if it is local | |
| Runbook, playbook | `procedure` with `--template runbook` | |
| Post-mortem, incident report | `incident` | Each incident adds an eval |
| Threat model | `research`, and each mitigation as a `rule` | |
| Test plan | The `Proof` section of a `plan`, and `covers:` tags | |
| Project-wide test strategy | `procedure` or `rule` | |
| BDD feature file | A test file with `covers:` tags | |
| Meeting notes, research notes | `research` | |
| How-to or tutorial (Diataxis) | `procedure` | An explanation page goes to `system` or `research` |
| Data model, database schema | `interface` that names the schema file | |
| arc42 | Split into `system`, `decision`, `terms` and `rule` | |
| Constitution of a spec framework | `rule` | |
| Change folder of a spec framework | The intent folder. A task list goes to the `Order` section of a `plan` | |
| Release notes, changelog | Outside spec-dna. `CHANGELOG.md` stays | |
| README, onboarding | Outside spec-dna, or `procedure` | |
| `AGENTS.md`, `CLAUDE.md`, skills | Outside spec-dna. `init` writes its adapter lines and skills | |
| `SECURITY.md` | Outside spec-dna | |

## Project types

`types` in `spec-dna.toml` lists the types of a project. A document of another type is an error (SD303), and `spec-dna new` does not accept it. If `types` is absent, the layers of the project give the types. `spec-dna init` writes no `types` list: question 2 writes the layers.
