---
status: accepted
---

# 0001. Information model

## Context and Problem Statement

spec-dna must keep specifications small, correct and cheap for agents to load. This applies to any repository, from a greenfield project to a mature system with thousands of documents. Which document model, identifiers, lifecycle and controls give this result with the least extra work? The answer must also not make the CI of a mature repository fail on the day that the repository adopts the tool.

The decision uses this evidence (the details are in [the research matrix](../research/matrix.md); consumer findings stay outside this repository):

- Corpus A (mature product, about 2.3k Markdown docs, about 39 MB, mixed languages, about 14 months of history): the main problems are size and accretion. There are almost no exact duplicates, and most normative docs are 4 link hops or fewer from the entry files. Feature specs and decision records grow far past any budget for reads. Most repeated reads by agents are page reads of the same large file. Some sequential numbers belong to several files. Citations to ids that do not exist accumulate. The project removed a gate on human-typed dates after false alarms. Some mandatory gates depend on agent tooling that is not in the repository.
- Corpus B (public proposal process, about 700 proposals): typed metadata with enums, cross-field rules and supersede links works at this scale. The process validates this metadata only on changed files.
- Corpus C (synthetic, 60 to 20,000 docs): the performance measurements use it (see [ADR 0002](0002-language.md)).
- An independent architecture review of the first draft found risks for adoption, merge safety and false errors. This version has solutions for these risks.
- The review of that version by the owner added requirement aliases, legacy requirement keys and budgets for amendments. It also added content hashes from the first release and a rule against automatic deletion. These additions are part of the decision below.

## Considered Options

1. Adopt the model of an existing framework (Spec Kit, OpenSpec or BMAD).
2. A large typed model with many document types and mandatory metadata (29148 or INCOSE attribute sets).
3. A small typed model: 6 document types, ids derived from paths, metadata defaults from configuration, state derived from git and the link graph, gates relative to the merge base.

## Decision Outcome

Chosen option: 3. Option 1 fails at scale (matrix, section 3). Option 2 adds metadata that nobody maintains and that agents must read.

### Layers

| Layer | Content | Owner | Location |
| --- | --- | --- | --- |
| Standard | Rules, schemas, document types, diagnostic codes | spec-dna, versioned with SemVer | Inside the binary. The project pins a version |
| Project configuration | Roots, per-root defaults, patterns for legacy ids and legacy requirement keys, excluded paths, budgets, language policy, repository aliases | Project | `spec-dna.toml` at the repository root |
| Project knowledge | The documents | Project | Where they are now. `init` proposes `docs/` for a new project |
| Derived data | Index cache, maps, context bundles, diagnostics, statistics | Generated | Local cache, never committed |
| Agent adapters | Entry points that call the CLI, plus a namespace-level index | Generated | `AGENTS.md` block, `CLAUDE.md` import, one skill. No normative rules |

A gate MUST run from the repository with only spec-dna and git. Skills, plugins and models are adapters and MUST NOT be a precondition of a gate. Every gate needs only two trees: the merge base and the head (`--base <rev>`). A depth-1 fetch of the base commit is sufficient. A missing base is a configuration error with a hint for the fix.

### Document types

| Type | Purpose | Authority | Default in agent context |
| --- | --- | --- | --- |
| `spec` | Intended behavior and constraints of one capability, or one cross-cutting concern | Normative | Yes |
| `contract` | An interface: prose, or a pointer to an OpenAPI, AsyncAPI or schema file | Normative | When the task affects it |
| `guide` | Conventions and procedures | Normative | For paths in its `applies-to` |
| `decision` | The reason for a choice (MADR minimal) | Rationale. Its normative effects are in the specs, contracts or guides that it links | Accepted and linked decisions, with the summary first |
| `change` | A proposal, plan or task that is not yet merged into current truth | Proposal | Only on request |
| `note` | Research, observations, reference material | Informative | No |

A cross-cutting concern (security, logging, error handling) is one `spec` or `guide` with `applies-to` (path globs or namespaces). Context assembly adds it for matching paths. Specs do not contain copies of it.

### Metadata

Each root in `spec-dna.toml` sets defaults: the type, the initial status, legacy id patterns and legacy key patterns for requirements. Thus a legacy document needs no edit for adoption. The configuration sets excluded paths (vendored or generated documents) generically, for all roots or per root. Excluded paths are not documents, but links to them still resolve as files. Front matter overrides a default only where the root allows mixed types. A front-matter `type` that disagrees with a single-type root is an error.

| Field | Required for | Meaning |
| --- | --- | --- |
| `status` | `spec`, `contract`, `guide`, `decision`, `change` (or the root default) | Lifecycle state |
| `type` | Documents in mixed-type roots | One of the 6 types |
| `supersedes`, `depends-on` | Optional | Ids of other documents |
| `applies-to` | Optional | Path globs or namespaces |
| `verified-by` | Optional | Paths of tests or verification records |
| `aliases` | Optional | Earlier ids and paths of this document |
| `owner` | Optional | A team or person handle |
| `x-*` | Optional | spec-dna ignores it |

spec-dna derives the values that follow, and nobody types them. The id comes from the path, and the title comes from the first H1. Dates and authors come from git. Reverse links, such as superseded-by and used-by, come from the graph. spec-dna also derives if a document has linked evidence, which means that every `verified-by` path exists. spec-dna does not claim that tests pass.

Front matter is a YAML subset that parses the same in YAML 1.1 and 1.2. Its keys are strings, and each key maps to a plain string or to a list of strings. Plain scalars are strings (`status: no` is the string "no"). The subset has no anchors, tags, multi-line scalars or multiple documents.

### Identifiers

The id of a document is its path under its root without the extension: `<namespace>/<slug>`. The namespace is the directory. The slug is the file stem, or the directory name for `index.md` or `README.md`. There is no counter, and the id always agrees with the path. If two branches add the same id, they add the same file, and git reports an add/add conflict before the merge.

- A move of a file changes its id. `spec-dna mv` moves the file and records the old id in `aliases`. An id in the base tree that is not in the head tree, and is not an alias there, is an error.
- Legacy ids (numbered decisions, defect or task numbers) become aliases through per-root patterns on the file name, such as `ADR-{n}`. Thus existing citations continue to resolve. spec-dna reports an alias that names several documents, for example numbered satellites of one decision. Gating is base-relative, so an ambiguity that exists at adoption does not block CI, but a new ambiguity does.
- spec-dna checks uniqueness over ids and aliases together.
- Cross-repository references use `<repo>:<namespace>/<slug>`, and the configuration contains the repository aliases. They resolve only against a pinned index that the referenced repository exports. Without such an index, they are warnings.

### References and requirements

An explicit reference is a front-matter id field, a Markdown link target, or the inline form `[[namespace/slug]]` or `[[namespace/slug#key]]`. Explicit references MUST resolve. Resolution uses the git tree (tracked files only, exact case, Unicode NFC) and not the file system. Thus a result does not depend on the machine.

In prose, a bare token that matches an id pattern or an alias pattern (`ADR-12`, `auth/session`) is only a candidate reference. An unresolved candidate reference is a warning, because the same text can be a route or a ticket number.

A requirement is a list item that starts with a bold key, inside a `spec` or `contract`:

```md
- **token-expiry**: When a session token expires, the gateway SHALL reject the request with status 401.
```

The key is unique in the document, and it is never a counter. Its global form is `<doc-id>#<key>`. The sentence uses BCP 14 keywords in uppercase and an EARS pattern. spec-dna checks conformance to the pattern and reports a deviation as a warning.

- New keys are slugs: lowercase ASCII letters and digits in hyphen-separated groups. An existing key such as `R-12` stays unchanged. Each root declares its legacy key patterns (for example `R-{n}`), and a key that matches one of them is valid. A key that is not a slug and not a legacy key is an error.
- A requirement can move to another document, for example after a split or a merge of a document. `spec-dna mv <doc-id>#<key> <doc-id>` moves the list item. It also records the old global form as an alias of the requirement, inline on one line of the item:

  ```md
  - **token-expiry**: When a session token expires, the gateway SHALL reject the request with status 401. <!-- aliases: auth/session#token-expiry -->
  ```

  The marker is an HTML comment, so rendered views do not show it. It lists one or more earlier global forms, separated by commas. A reference to an alias resolves to the requirement. spec-dna checks uniqueness over global forms and their aliases together. A document alias also applies to the keys of the document: after a document moves, `<old-id>#<key>` continues to resolve.
- spec-dna allows the removal of a requirement. If something still cites a removed requirement, that citation is an unresolved explicit reference.

### Lifecycle

| Type | States | Checked between base and head |
| --- | --- | --- |
| `spec`, `contract`, `guide` | `draft` → `active` → `deprecated` → `superseded` or `retired` (`deprecated` is optional) | `superseded` needs a successor in `active` whose `supersedes` names it. The model does not allow supersede cycles. A new `depends-on` to a `deprecated` document is a warning, and to a `superseded` or `retired` document it is an error |
| `decision` | `proposed` → `accepted` → `superseded`; or `rejected` | `superseded` needs an `accepted` successor. After a decision is `accepted`, the text above an `## Amendments` section is frozen. New amendments go at the end, and they count toward the decision budget. An amendment that changes the outcome needs a new decision that supersedes the old one. New amendment text that states a requirement, uses a BCP 14 keyword or uses a status word is a warning |
| `change` | `proposed` → `accepted` → `done`; or `rejected` | Only the state order. spec-dna never deletes a document. A migration can propose to delete a `done` or `rejected` change when nothing cites it. It can do this only after the decisions of the change exist as `decision` documents. Each deletion batch needs explicit approval from the owner. The git history is the archive |
| `note` | none | none |

The model does not have "Implementing" and "verified" as typed states. Typed progress states become out of date. Superseded, retired and deprecated documents stay in place while anything cites them, so links continue to work. Context assembly includes them only on request.

### Change-risk classes

spec-dna parses the requirement map (key to whitespace-normalized sentence) of each changed document in the base and head trees and compares them:

| Class | Detected when | Effect |
| --- | --- | --- |
| R0 | No requirement, contract structure or decision status changed | Structural checks |
| R1 | Requirements added, changed or removed in a document that nothing depends on | R0, plus requirement lint (warnings) |
| R2 | Requirements change in a document with dependents, a contract's structure changes, or a decision changes status | R1, plus an impact report: every transitive dependent through `depends-on` and explicit references, grouped by `owner` |

The impact report does not block. The break that must block is a removed `<doc-id>#<key>` that something still cites. That break is already an unresolved explicit reference. Stored suspect links are an option for later, if an R2 check that blocks becomes necessary. A stored suspect link keeps the hash of the target at review time, as in Doorstop.

Hashes exist from the first release, so suspect links will need no format change later. The index stores a normalized hash for every requirement and every section. `context` and `show` print the hash next to the id. Normalization applies Unicode NFC and LF line ends. It changes each run of whitespace to one space, and it removes the whitespace at the start and at the end of the text.

The hash of a requirement covers its sentence without the key and without the alias marker. Thus a move or a key rename does not change the hash. The hash is the first 16 hexadecimal digits of SHA-256 over the normalized UTF-8 text. A reference MAY end with `@<hash>` (`[[auth/session#token-expiry@3f2a9c1b7e0d4a11]]`). spec-dna parses that suffix and ignores it until a later version adds suspect links.

### Knowledge boundaries

| Source | Says | Conflict signal |
| --- | --- | --- |
| Spec, contract | The intended behavior of the system | n/a |
| Decision | Why | A spec that contradicts a decision needs a review by people |
| Code | What the system does | A cited path is not in the git tree (error). A cited symbol does not occur as a literal token in the cited file (warning). The code changed after the last requirement change of the spec (warning) |
| Tests, verification records | Evidence | A `verified-by` path is not in the git tree (error) |
| Notes, observations | Runtime reality | None. Informative only |
| Change | Proposed truth | spec-dna never serves it as current truth |

spec-dna never rewrites a requirement to match the code. A `disputed` marker on the requirement records a known conflict and links the evidence. Agents receive the marker with the requirement.

### Growth controls

CI fails only on error diagnostics that are new relative to the merge base. The fingerprint of a diagnostic is (code, document id, target). `--all` reports every diagnostic. A full check on the main branch reports drift that two merges can cause when each passes CI alone. The CI template recommends a merge queue or up-to-date branches. There is no committed baseline file.

| Control | Kind | Severity |
| --- | --- | --- |
| Unresolved explicit reference; id removed without alias; duplicate id or alias | Proven | Error |
| Front matter fails the type schema; required section missing | Proven | Error |
| Invalid state transition; supersede cycle; `depends-on` to a superseded or retired id | Proven | Error |
| Generated adapter differs from what spec-dna generates now (hash of LF and NFC normalized text) | Proven | Error |
| Bare-token reference unresolved; cited symbol not found | Heuristic | Warning |
| Document over its soft or hard size budget; an over-budget document grew against the base | Proven measurement, policy threshold | Warning until session 2 data sets the default for the hard budget |
| Orphan document; section fan-out over a limit | Proven measurement, uncertain harm | Warning |
| Near-duplicate paragraphs or documents; requirement lint; contradiction candidates; code changed after spec | Heuristic | Warning |

No check reads a human-typed date.

Budgets count the Unicode characters of body text without front matter and code blocks, so they work for every script. Text under `## Amendments` counts. The defaults for `spec` and `change` are 10,000 soft and 20,000 hard (about 1,500 and 3,000 English words). For `decision`, they are 7,000 and 14,000. For `guide`, they are 14,000 and 28,000.

Context assembly serves sections by heading anchor, so a long file costs less than its size. But it is still necessary to read and maintain a long file.

### Diagrams

Diagram source (Mermaid, PlantUML, Structurizr DSL) in fenced blocks inside the document is canonical. Rendered images come from that source, and a project commits them only if a viewer needs them. Context assembly includes diagram source only on request. A C4 level is optional `x-` metadata.

### Concurrency

- Ids come from paths: a duplicate is a git conflict, not a race for a counter.
- There are no committed registries or per-document indexes, and spec-dna generates tables of contents. The committed agent index lists only namespaces, so it changes only when a project adds a namespace.
- Each fact is in one place. spec-dna derives reverse links, so an agent never edits two files to keep a pair consistent.
- New knowledge goes into new files (one decision or change per file).
- Gates are base-relative. The CLI writes its cache atomically and treats it as disposable. The production cache keys tracked files by git blob id instead of size and modification time.

### Consequences

- Good: adoption needs zero edits, and ids cannot drift from paths. Legacy ids continue to resolve. The gates are reproducible from the repository and two git trees.
- Good: the context unit is a section with an id and a hash, so agents can cite what they loaded and check it again.
- Bad: a move needs `spec-dna mv` or a hand-written alias, for documents and for requirements; slugs need naming discipline.
- Bad: base-relative gating hides legacy debt from CI. `--all` and the main-branch report keep it visible.
- Neutral: the ADR file names of this repository contain a number, because the session brief prescribed it. The model allows numbers in slugs, but it does not require them.

## Amendments

### 2026-11-07: rulings after checkpoint 2

The owner gave these rulings after checkpoint 2. They set the default that the severity table left open, and they add 4 configuration keys and 1 option.

- Hard budget. A document over its hard budget is an error (SD702). The check fails only on a new error, so a document that was over the hard budget in the base does not fail it. When that document grows, the check gives a warning (SD703). The soft budget stays a warning.
- Legacy status words. A root can map a legacy status word to a status of the model, for example `released = "active"` (`status-map`). The file keeps its word, and all rules use the mapped status. The map applies only to a word that the type of the document does not allow.
- Decision sections. A root can name the headings that contain the outcome of a decision (`decision-sections`). The default is `Decision Outcome` and `Decision`. `check` (SD305) and `context` use the same list, so a repository can write these headings in a different language.
- Links by URL. `self-urls` lists the URL prefixes of the repository. In a prefix, `*` is one path segment, for example a branch. A link that starts with such a prefix is a path from the root of the repository. A link to a directory refers to the `README.md` or `index.md` document of that directory. The check does not report a self URL without a path in the tree, because the URL can refer to another branch or commit.
- Query. `context --query <text>` puts the sections that match the words of the text first. The ranking uses BM25 and is deterministic. Without `--query`, the order does not change.

### 2026-11-07: untracked documents in the working tree

The implementation differs from "tracked files only" in one case. When the head is the working tree, spec-dna also reads untracked files that git does not ignore. Thus `check` sees a new document before `git add`. A base is always a commit, and CI uses a clean checkout, so the result of a gate in CI does not change. Checkpoint 2 reported this difference as deviation 4.

### 2026-11-08: rulings and review fixes before session 3

The owner gave these rulings before session 3, and a review of session 2 found 4 differences between this ADR and the tool. These changes apply.

- Hard budget. This rule replaces the first item of the amendment of 2026-11-07. The check compares a document with the base. A new document over the hard budget is an error (SD702). A document that goes over the hard budget is an error (SD702). A document that was over the hard budget in the base and grew is an error (SD702). Growth is net: the head has more body characters than the base. A document that was over the hard budget in the base and did not grow gives a warning (SD704). A root can set `enforce-budgets = false`. Then every hard overage gives SD704. `check --all` uses the same base as `check`.
- Required sections. This rule replaces `decision-sections`. A root can name one required section for each type, as a list of accepted headings in any language (`required-sections`). The default requires the outcome of a decision (`Decision Outcome` or `Decision`). The other types have no required section by default.
- Legacy requirement syntax. For a key that matches `legacy-keys`, the colon after the bold key is optional: `- **R-12** sentence` and `- **R-12.** sentence` are requirements. A slug key still needs the colon, because a bold word at the start of a list item is usual in prose. A spec or contract that has tokens that match a legacy key pattern, but no recognized requirement, gives a warning (SD806). `check --json` and `show` report the recognized requirements of each document.
- Inactive documents in context. Context assembly lists a superseded, retired or rejected document with its status and its successors. It does not serve the text, unless a target names the document or `--include-inactive` is set. A deprecated document stays in context with its status, because it is still in effect. This rule makes the last paragraph of "Lifecycle" more exact.
- Section order. In a document with requirements, context assembly serves the sections that contain requirements first, and then the title section. A document without requirements keeps the heading order.
- Tracked files. This rule makes the amendment "untracked documents in the working tree" more exact. Without `--base`, `check` reads tracked files and the untracked files that git does not ignore. Thus `check` sees a new document before `git add`. With `--base`, `check` reads tracked files only. A base is always a commit, and CI uses a clean checkout. Thus base-relative gating and CI use tracked files only.
- Query. The BM25 ranking of `context --query` counts each heading word 3 times. It uses no embeddings and no network. This item records the existing behavior.

### 2026-11-08: rulings after checkpoint 3

The owner gave these rulings after checkpoint 3. These changes apply.

- Amendment sections. This rule replaces "the text above an `## Amendments` section" in "Lifecycle". A root can name the amendment sections of each type, in any language (`amendment-sections`). The default for a decision is `Amendments`. A heading of level 2 or more that is a name, or that starts with a name and a character that is not a letter or a digit, starts an amendment section. The section continues to the next heading of the same or a higher level. Thus a repository can write one section for all amendments, or one section for each amendment, at any position. After a decision is accepted, all text outside its amendment sections is frozen (SD405). Context assembly serves the amendment sections of a decision with its outcome.
- Partial recognition. A spec or contract also gives SD806 if a list item or a table row starts with a token that matches `legacy-keys`, and no recognized requirement of the document has that key. Thus a second syntax of requirements in one document gives a warning.
- Lint severity. A root can set the severity of the requirement lint (SD801 to SD805): `off`, `warning` (default) or `error` (`lint-severity`). A repository can thus adopt spec-dna in legacy roots without a rewrite of normative text.

### 2026-11-08: code citations

The owner gave these rulings after checkpoint 4. A smoke run on corpus A showed that `context <code path>` served no section, because the documents of corpus A have no `applies-to`. These changes apply.

- Code citations. A document cites code in 3 ways: a link to a file that is not a document, a path in an inline code span or in prose, or a distinctive identifier in an inline code span. A path can have a location suffix (`:12`, `:12-20`, `#L12`). A path resolves from the repository root, from the directory of the document, or as a file name or path tail that only one file has. An identifier is distinctive if it has an inner `_`, a lowercase letter before an uppercase letter, or the form of a call with 4 characters or more. spec-dna derives the citations, and nobody types them.
- Code-to-document entry. `context <code path>` serves the sections that cite the file by its path or by an identifier that occurs in the file first. Then it serves the requirement sections and the summary of the best citing documents. A citation of a directory of the path is weaker: it comes after `applies-to`. Caps on the number of citing documents keep a common identifier from filling the budget. A citing change or note is only listed. This entry needs no `applies-to`.
- Drift. `check` compares the base tree with the head tree. If the change removes or renames a cited path, `check` gives SD606. If the change removes a cited identifier, `check` gives SD607. Each warning names the citing document. An identifier is removed only if no file outside of Markdown has it. Only current documents give these warnings: a superseded, retired or rejected document, a `done` change and a note can cite code that is gone. A link to a missing path stays an error (SD102).
- Adapters. This rule replaces "`AGENTS.md` block, `CLAUDE.md` import" in "Layers". `init` writes one line into each root instruction file, and one skill. The line never imports another file. `init` reports the bytes that the adapters add to the prompt prefix of each agent request. The target is 300 bytes or less.

### 2026-11-08: batch split

The owner gave these rulings after checkpoint 5. These changes apply.

- Batch split. `spec-dna mv <doc-id>#<key or anchor>... <doc-id>` moves several requirements, or whole sections with their subsections, in one command. The destination is an existing spec or contract, or a new spec that `mv` creates with front matter and a title. Each moved requirement gets an alias marker with its old global id. Links to a moved key or section follow it. In an accepted decision, `mv` changes only link destinations, because they are not frozen text.
- Preservation check. Before `mv` writes a file, it compares the result with the current tree. The requirement hashes must be the same multiset, and every requirement id must resolve to a requirement with the same hash. The frozen text of every accepted decision must not change, and no error diagnostic can be new. If a check fails, `mv` changes no file.
- Missing colon. In a spec or contract, a list item that starts with a bold slug key and has a BCP 14 keyword, but no colon after the key, gives SD807 (warning). The item is not a requirement.
- Drift severity. SD606 and SD607 stay warnings by default. A root can set `drift-severity` to `off`, `warning` or `error`.

### 2026-11-08: rulings after checkpoint 7

The owner gave these rulings after checkpoint 7. A playground product of 12 agent tasks found 5 defects in 0.1.1. These changes apply.

- Aliases of a batch split. This rule replaces "each moved requirement gets an alias marker with its old global id" in the amendment "batch split". A moved requirement gets an alias marker only if its old id is in the base tree (`HEAD`). A key that is new in the change has no old id that a reference can use.
- Preservation hash. The preservation check of `mv` compares the requirement text without link destinations, because the move rewrites links. A refused move names each requirement whose text would change, and the reason.
- Drift of an accepted decision. The frozen text of an accepted decision cannot change (SD405). When it cites a removed path or identifier, an amendment section of the decision that cites new code clears SD606 and SD607 for that decision. New code is a path that the head tree has (the rename target, if git found one), or an identifier that the code has, that the base text of the decision does not cite.
- Text output of `check`. The errors come first, then the drift warnings, then the other warnings. The prose lint warnings (SD801 to SD805) are one count for each code, and `--lint` lists them. The JSON output lists all diagnostics.

### 2026-11-08: profiles, knowledge types and evidence (0.2.0)

The owner gave these rulings for 0.2.0. Their goal: a growing corpus stays orderly, so agents stay fast and cheap. These changes apply.

- Profiles. A profile (`lite`, `standard` or `strict`) sets the default severity of each diagnostic code. This rule replaces the fixed severities of "Growth controls" and of the amendments `lint-severity` and `drift-severity`. `lite` makes references, ids and budgets warnings. `standard` makes them errors for a new problem, a document that grows past its hard budget an error, and drift and missing evidence warnings. `strict` makes drift, missing evidence and every budget overage errors, and the requirement lint warnings. A root or a rule (by rule group or by code) can override the profile. Without `profile`, a project uses `standard`. The table is in docs/profiles.md.
- Knowledge types. The model has 4 more types. This rule extends the table of "Document types" and the table of "Lifecycle".
  - `invariant`: project-wide rules with stable ids (requirements) and an `applies-to` scope. Context assembly serves a matching invariant first. An invariant without `applies-to` applies to the whole repository. Its states are those of a spec.
  - `runbook`: an operational procedure. It is normative for its `applies-to`, as a guide. Its states are those of a spec.
  - `incident`: an event, its root cause, and links to the requirements, decisions and runbooks that it involves. An incident without such a link is a warning. It has no state, and it describes the past: it gives no drift warning, and context assembly only lists it.
  - `reference`: research and other cold material. It has no state and no budget. Context assembly never serves or lists it, unless a target names it.

  Each type has a template, required sections and budgets. `required-sections` can name several sections for a type (a list of heading lists). The 6 types of the first decision do not change.
- Project types. `types` lists the document types of a project. A document of another type is an error. Thus a growing corpus keeps its set of types.
- Evidence. A requirement has evidence if `verified-by` (in the front matter, or in an inline marker of the requirement) or a `covers: <doc-id>#<key>` tag in a test file links it to a test. The tag is plain text, so it works in a comment of any language. A missing `verified-by` path and a tag that names no requirement are errors in `standard` and `strict`. An active requirement without evidence follows the profile. This rule makes "spec-dna derives if a document has linked evidence" more exact: the derivation is per requirement. spec-dna still does not claim that tests pass.
- Adoption. `init` detects the document roots, types, legacy ids and keys, statuses and exclusions of a repository, and asks for each detected root. It writes only after a confirmation, and it never edits a document. This rule makes the "Project knowledge" row of "Layers" more exact: `init` proposes `docs/` for a new project, and the detected roots for an existing one.

### 2026-11-11: hygiene functions (0.3.0)

A real corpus of about 2,300 documents showed that most large documents are guides, decisions and registries, and that their size comes from history: amendments, iteration logs and review rounds. The batch split of 0.1.1 moved sections only out of specs and contracts. These rulings extend it. The decision outcome stays.

- Split for every type. `mv` moves sections out of a document of any type, into a new or an existing document of any type, also `reference`. Requirements move only into a spec, a contract or an invariant. Each moved section keeps its old id as an alias: the destination lists `<old-id>#<anchor>` in its `aliases`. A reference `<old-id>#<anchor>` resolves to the old document while it has that anchor, else to the alias. The preservation check also requires that every moved section of the base resolves.
- Decisions. The frozen text of a decision stays in place. Only its amendment sections and its history sections (`history-sections`) move, and only into a `reference` document. A history section of a decision is not frozen text. Each moved section leaves its heading with a one-line pointer (0.3.1; in 0.3.0, only a move into a reference document did). `mv` reports the moved amendments that state a requirement, a BCP 14 keyword or a status word as candidates for a superseding decision. It never writes a decision.
- Companion sections. A root can name the sections that belong to the requirements of a type (`companion-sections`), such as an acceptance map. Their rows move with the moved requirements. If a row names a key that moves and a key that stays, the move is refused with that reason.
- History extraction. `spec-dna history <id>` moves the history sections of a document (and the amendment sections of a decision) into a reference document, with a pointer for each section and a backlink.
- Generated index. `spec-dna index <dir>` writes an index of the documents of a directory, marked by the front matter field `generated`. It can replace a hand-kept registry file. `check` reports a stale or hand-edited index (SD610). A generated index has no budget and no required sections.
- Stubs. `mv --stub` leaves a stub at the old path (front matter `moved-to`) for tools that read a file by its path. A stub has no id and no budget.
- Configuration changes. A change that edits `spec-dna.toml` together with documents or code is an error (SD503). A configuration change is a change of its own, by the owner. The skill tells agents not to edit the configuration.
- Reference namespaces. The id of a document in a root of type `reference` starts with a namespace from the root path, so two reference roots never give the same id (defect D11-1). The id without the namespace stays a fallback name when it is unique.
- Growth allowance. A document that was over its hard budget in the base can grow by at most `[budgets] growth` characters (default 4,000) in one change with the warning SD703. More growth is still an error (SD702). A new document that a split moved out of a document over its hard budget gives SD704, because it holds no new text. Reason: on the real corpus, every code task that kept the documents true (an amendment, a registry row) grew an oversized document by 0.1 to 3.6 thousand characters, and the error made agents split or shorten documents in the task. That added cost without a gain in correctness. Splits are a hygiene step of the owner (`history`, `mv`), not a part of a code task.
