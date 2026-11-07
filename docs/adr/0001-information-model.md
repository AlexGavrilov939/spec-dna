---
status: accepted
---

# 0001. Information model

## Context and Problem Statement

spec-dna must keep specifications small, correct and cheap for agents to load in any repository, from a greenfield project to a mature system with thousands of documents. Which document model, identifiers, lifecycle and controls give that with the least ceremony, and without turning a mature repository's CI red on the day it adopts the tool?

Evidence used (details in [the research matrix](../research/matrix.md); consumer findings stay outside this repository):

- Corpus A (mature product, about 2.3k Markdown docs, about 39 MB, mixed languages, about 14 months of history): size and accretion dominate. Exact duplicates are near zero and most normative docs are within 4 link hops of the entry files. Feature specs and decision records grow far past any reading budget, and most repeated reads by agents page through the same large file. Sequential numbers are shared by several files. Citations to ids that do not exist accumulate. A gate on human-typed dates was removed after false alarms. Some mandatory gates depend on agent tooling that the repository does not contain.
- Corpus B (public proposal process, about 700 proposals): typed metadata with enums, cross-field rules and supersede links, validated on changed files only, works at this scale.
- Corpus C (synthetic, 60 to 20,000 docs): used for performance, see [ADR 0002](0002-language.md).
- An independent architecture review of the first draft found adoption, merge-safety and false-error risks. This version addresses them.
- The owner's review of that version added requirement aliases, legacy requirement keys, budgets for amendments, content hashes from the first release, and a rule against automatic deletion. They are part of the decision below.

## Considered Options

1. Adopt the model of an existing framework (Spec Kit, OpenSpec or BMAD).
2. A large typed model with many document types and mandatory metadata (29148 or INCOSE attribute sets).
3. A small typed model: 6 document types, ids derived from paths, metadata defaulted by configuration, state derived from git and the link graph, gates evaluated against the merge base.

## Decision Outcome

Chosen option: 3. Option 1 fails at scale (matrix, section 3). Option 2 adds metadata that nobody maintains and that agents must read.

### Layers

| Layer | Content | Owner | Location |
| --- | --- | --- | --- |
| Standard | Rules, schemas, document types, diagnostic codes | spec-dna, versioned with SemVer | Inside the binary. The project pins a version |
| Project configuration | Roots, per-root defaults, legacy id and requirement key patterns, excluded paths, budgets, language policy, repository aliases | Project | `spec-dna.toml` at the repository root |
| Project knowledge | The documents | Project | Where they already are. `init` proposes `docs/` for a new project |
| Derived data | Index cache, maps, context bundles, diagnostics, statistics | Generated | Local cache, never committed |
| Agent adapters | Entry points that call the CLI, plus a namespace-level index | Generated | `AGENTS.md` block, `CLAUDE.md` import, one skill. No normative rules |

A gate MUST run from the repository with spec-dna and git alone. Skills, plugins and models are adapters and MUST NOT be a precondition of a gate. Every gate needs only two trees: the merge base and the head (`--base <rev>`). A depth-1 fetch of the base commit is enough. A missing base is a configuration error with a fix hint.

### Document types

| Type | Purpose | Authority | Default in agent context |
| --- | --- | --- | --- |
| `spec` | Intended behavior and constraints of one capability, or one cross-cutting concern | Normative | Yes |
| `contract` | An interface: prose, or a pointer to an OpenAPI, AsyncAPI or schema file | Normative | When the task touches it |
| `guide` | Conventions and procedures | Normative | For paths in its `applies-to` |
| `decision` | Why a choice was made (MADR minimal) | Rationale. Its normative effects live in the specs, contracts or guides it links | Accepted and linked ones, summary first |
| `change` | A proposal, plan or task not yet merged into current truth | Proposal | Only when requested |
| `note` | Research, observations, reference material | Informative | No |

A cross-cutting concern (security, logging, error handling) is one `spec` or `guide` with `applies-to` (path globs or namespaces). Context assembly adds it for matching paths. It is never copied into each spec.

### Metadata

Each root in `spec-dna.toml` sets defaults: the type, the initial status, legacy id patterns and legacy requirement key patterns. A legacy document therefore needs no edit on adoption. Excluded paths (vendored or generated documents) are configured generically, globally or per root; they are not documents, although links to them still resolve as files. Front matter overrides a default only where the root allows mixed types; a front-matter `type` that disagrees with a single-type root is an error.

| Field | Required for | Meaning |
| --- | --- | --- |
| `status` | `spec`, `contract`, `guide`, `decision`, `change` (or the root default) | Lifecycle state |
| `type` | Documents in mixed-type roots | One of the 6 types |
| `supersedes`, `depends-on` | Optional | Ids of other documents |
| `applies-to` | Optional | Path globs or namespaces |
| `verified-by` | Optional | Paths of tests or verification records |
| `aliases` | Optional | Earlier ids and paths of this document |
| `owner` | Optional | A team or person handle |
| `x-*` | Optional | Ignored by spec-dna |

Derived, never typed: the id (path), the title (first H1), dates and authors (git), reverse links such as superseded-by and used-by (graph), and whether evidence is linked (every `verified-by` path exists). spec-dna does not claim that tests pass.

Front matter is a YAML subset that parses the same in YAML 1.1 and 1.2: string keys mapping to plain strings or lists of strings. Plain scalars are strings (`status: no` is the string "no"). No anchors, tags, multi-line scalars or multiple documents.

### Identifiers

The id of a document is its path under its root without the extension: `<namespace>/<slug>`, where the namespace is the directory and the slug is the file stem (the directory name for `index.md` or `README.md`). There is no counter, and the id never disagrees with the path. Two branches that add the same id add the same file, which git reports as an add/add conflict before the merge.

- Moving a file changes its id. `spec-dna mv` moves it and records the old id in `aliases`. An id present in the base tree, absent in the head tree and not kept as an alias is an error.
- Legacy ids (numbered decisions, defect or task numbers) become aliases through per-root patterns such as `ADR-{n}` on the file name, so existing citations keep resolving. An alias that names several documents (for example numbered satellites of one decision) is reported. Because gating is base-relative, an ambiguity that exists at adoption does not block CI; a new one does.
- Uniqueness is checked over ids and aliases together.
- Cross-repository references use `<repo>:<namespace>/<slug>`, with repository aliases in the configuration. They resolve only against a pinned index exported by that repository; without one they are warnings.

### References and requirements

An explicit reference is a front-matter id field, a Markdown link target, or the inline form `[[namespace/slug]]` or `[[namespace/slug#key]]`. Explicit references MUST resolve. Resolution uses the git tree (tracked files only, exact case, Unicode NFC), not the file system, so a result does not depend on the machine.

A bare token that matches an id or alias pattern in prose (`ADR-12`, `auth/session`) is only a candidate reference: an unresolved one is a warning, because the same text can be a route or a ticket number.

A requirement is a list item that starts with a bold key, inside a `spec` or `contract`:

```md
- **token-expiry**: When a session token expires, the gateway SHALL reject the request with status 401.
```

The key is unique within the document (never a counter); its global form is `<doc-id>#<key>`. The sentence uses BCP 14 keywords in uppercase and an EARS pattern. Pattern conformance is checked as a warning.

- New keys are slugs: lowercase ASCII letters and digits in hyphen-separated groups. An existing key such as `R-12` is kept as written: each root declares its legacy key patterns (for example `R-{n}`), and a key that matches one is valid. A key that is neither a slug nor a legacy key is an error.
- A requirement can move to another document, for example when a document is split or merged. `spec-dna mv <doc-id>#<key> <doc-id>` moves the list item and records the old global form as an alias of the requirement, inline on one line of the item:

  ```md
  - **token-expiry**: When a session token expires, the gateway SHALL reject the request with status 401. <!-- aliases: auth/session#token-expiry -->
  ```

  The marker is an HTML comment, so rendered views do not show it. It lists one or more earlier global forms, separated by commas. A reference to an alias resolves to the requirement, and uniqueness is checked over global forms and their aliases together. A document alias also carries its keys: after a document moves, `<old-id>#<key>` keeps resolving.
- Removing a requirement is allowed. A removed requirement that something still cites is an unresolved explicit reference.

### Lifecycle

| Type | States | Checked between base and head |
| --- | --- | --- |
| `spec`, `contract`, `guide` | `draft` → `active` → `deprecated` → `superseded` or `retired` (`deprecated` is optional) | `superseded` needs a successor in `active` whose `supersedes` names it. No supersede cycles. A new `depends-on` to a `deprecated` document is a warning; to `superseded` or `retired`, an error |
| `decision` | `proposed` → `accepted` → `superseded`; or `rejected` | `superseded` needs an `accepted` successor. Text above an `## Amendments` section is frozen once `accepted`; amendments are appended and count toward the decision budget. An amendment that changes the outcome needs a new superseding decision: new amendment text that states a requirement, uses a BCP 14 keyword or uses a status word is a warning |
| `change` | `proposed` → `accepted` → `done`; or `rejected` | Only the state order. spec-dna never deletes a document. A migration may propose deleting a `done` or `rejected` change once nothing cites it and only after its decisions exist as `decision` documents. Each deletion batch needs explicit owner approval; git history is the archive |
| `note` | none | none |

"Implementing" and "verified" are not typed states; typed progress states go stale. Superseded, retired and deprecated documents stay in place while anything cites them, so links keep working. Context assembly skips them unless asked.

### Change-risk classes

spec-dna parses the requirement map (key to whitespace-normalized sentence) of each changed document in the base and head trees and compares them:

| Class | Detected when | Effect |
| --- | --- | --- |
| R0 | No requirement, contract structure or decision status changed | Structural checks |
| R1 | Requirements added, changed or removed in a document that nothing depends on | R0, plus requirement lint (warnings) |
| R2 | Requirements change in a document with dependents, a contract's structure changes, or a decision changes status | R1, plus an impact report: every transitive dependent through `depends-on` and explicit references, grouped by `owner` |

The impact report does not block. The break worth blocking, a removed `<doc-id>#<key>` that something still cites, is already an unresolved explicit reference. Stored suspect links (the target's hash at review time, as in Doorstop) are a later option if a blocking R2 check proves necessary.

Hashes exist from the first release, so suspect links need no format change later. The index stores a normalized hash for every requirement and every section, and `context` and `show` print it next to the id. Normalization: Unicode NFC, LF line ends, runs of whitespace collapsed to one space, leading and trailing whitespace removed. A requirement's hash covers its sentence without the key and without the alias marker, so a move or a key rename keeps it. The hash is the first 16 hexadecimal digits of SHA-256 over the normalized UTF-8 text. A reference MAY end with `@<hash>` (`[[auth/session#token-expiry@3f2a9c1b7e0d4a11]]`). spec-dna parses that suffix and ignores it until suspect links are added.

### Knowledge boundaries

| Source | Says | Conflict signal |
| --- | --- | --- |
| Spec, contract | What the system is intended to do | n/a |
| Decision | Why | A spec that contradicts a decision is for people to review |
| Code | What the system does | A cited path is not in the git tree (error). A cited symbol does not appear as a literal token in the cited file (warning). Code changed after the spec's last requirement change (warning) |
| Tests, verification records | Evidence | A `verified-by` path is not in the git tree (error) |
| Notes, observations | Runtime reality | None. Informative only |
| Change | Proposed truth | Never served as current truth |

spec-dna never rewrites a requirement to match code. A known conflict is recorded on the requirement as a `disputed` marker that links the evidence, and agents receive the marker with the requirement.

### Growth controls

CI fails only on error diagnostics that are new relative to the merge base, fingerprinted by (code, document id, target). `--all` reports every diagnostic. A full check on the main branch reports drift that two separately green merges can cause; the CI template recommends a merge queue or up-to-date branches. There is no committed baseline file.

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

Budgets count Unicode characters of body text, excluding front matter and code blocks, so they work for every script. Text under `## Amendments` counts. Defaults: `spec` and `change` 10,000 soft and 20,000 hard (about 1,500 and 3,000 English words); `decision` 7,000 and 14,000; `guide` 14,000 and 28,000. Context assembly serves sections by heading anchor, so a long file costs less than its size, but it still has to be read and maintained.

### Diagrams

Diagram source (Mermaid, PlantUML, Structurizr DSL) in fenced blocks inside the document is canonical. Rendered images are derived and are not committed unless a viewer needs them. Context assembly includes diagram source only on request. A C4 level is optional `x-` metadata.

### Concurrency

- Ids come from paths: a duplicate is a git conflict, not a race for a counter.
- No committed registries or per-document indexes; tables of contents are generated. The committed agent index lists namespaces only, so it changes only when a namespace is added.
- One fact, one place. Reverse links are derived, so an agent never edits two files to keep a pair consistent.
- New knowledge goes into new files (one decision or change per file).
- Gates are base-relative, and the CLI writes its cache atomically and treats it as disposable. The production cache keys tracked files by git blob id instead of size and modification time.

### Consequences

- Good: zero edits to adopt; ids cannot drift from paths; legacy ids keep resolving; gates are reproducible from the repository and two git trees.
- Good: the context unit is a section with an id and a hash, so agents can cite and re-check what they loaded.
- Bad: a move needs `spec-dna mv` or a hand-written alias, for documents and for requirements; slugs need naming discipline.
- Bad: base-relative gating hides legacy debt from CI. `--all` and the main-branch report keep it visible.
- Neutral: this repository's own ADR file names carry a number, as the session brief prescribed. The model allows numbers in slugs; it does not require them.

## Amendments

### 2026-11-07: rulings after checkpoint 2

The owner gave these rulings after checkpoint 2. They set the default that the severity table left open, and they add 4 configuration keys and 1 option.

- Hard budget. A document over its hard budget is an error (SD702). The check fails only on a new error, so a document that was over the hard budget in the base does not fail it. When that document grows, the check gives a warning (SD703). The soft budget stays a warning.
- Legacy status words. A root can map a legacy status word to a status of the model, for example `implemented = "active"` (`status-map`). The file keeps its word, and all rules use the mapped status. The map applies only to a word that the type of the document does not allow.
- Decision sections. A root can name the headings that contain the outcome of a decision (`decision-sections`). The default is `Decision Outcome` and `Decision`. `check` (SD305) and `context` use the same list, so a repository can write these headings in a different language.
- Links by URL. `self-urls` lists the URL prefixes of the repository. In a prefix, `*` is one path segment, for example a branch. A link that starts with such a prefix is a path from the root of the repository. A link to a directory refers to the `README.md` or `index.md` document of that directory. The check does not report a self URL that has no path in the tree, because the URL can refer to a different branch or commit.
- Query. `context --query <text>` puts the sections that match the words of the text first. The ranking uses BM25 and is deterministic. Without `--query`, the order does not change.
