# Configuration: `spec-dna.toml`

`spec-dna.toml` is in the repository root. `spec-dna init` writes the first version of the file. The JSON Schema is [schema/spec-dna.schema.json](../schema/spec-dna.schema.json). An unknown key is an error (exit code 3), so a typo cannot disable a rule without an error message.

```toml
spec-dna = "0.3.2"              # required: the pinned version
profile = "standard"            # "lite", "standard" (default) or "strict"
types = ["spec", "decision", "invariant", "runbook", "incident", "reference"]  # default: all 10
exclude = ["**/vendor/**"]      # repository globs that are never documents
self-urls = ["https://example.org/acme/specs/blob/*/"]  # URLs of this repository
install-source = "ssh://git@example.org/tools/spec-dna.git"  # where `cargo install` gets spec-dna

[rules]                         # severity overrides (docs/profiles.md)
drift = "error"                 # a rule group, or a code such as SD704

[evidence]
tests = ["tests/**", "**/*_test.go"]  # test files with `covers:` tags

[[roots]]                       # required: one table per document root
path = "docs"
type = "mixed"                  # one of the 10 types, or "mixed" (default)
default-type = "spec"           # mixed roots: type when front matter has none (default "note")
status = "draft"                # status of a default-type document without `status`
exclude = ["drafts/**"]         # globs relative to the root
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]
legacy-keys = ["R-{n}"]
status-map = { released = "active", wip = "draft" }            # legacy status words
required-sections = { decision = ["Decision Outcome", "Decision"], incident = [["Summary"], ["Root cause"]] }
amendment-sections = { decision = ["Amendments", "Nachtrag"] }  # amendment headings
history-sections = { guide = ["Iteration log"], decision = ["Review rounds"] }  # history headings
companion-sections = { spec = ["Acceptance map"] }  # sections with a row per requirement
live-sections = { decision = ["Implementation status"] }  # sections that change in place
enforce-budgets = true          # false: the hard budget gives warnings only
profile = "lite"                # the profile of this root
rules = { lint = "off" }        # severity overrides for this root

[budgets]
growth = 4000                   # characters that a document over its hard budget can grow in one change

[budgets.spec]                  # characters of body text, per type
soft = 10000
hard = 20000

[lint]
max-words = 30                  # longest requirement sentence before a warning

[context]
budget = 8000                   # default character budget of `spec-dna context`

[repos]
platform = "https://example.org/platform.git"   # alias for `platform:<namespace>/<slug>`

[conventions]                   # sections that state the project conventions, per type
contract = ["docs/README.md#contract-changes"]
decision = ["docs/adr/README.md#how-to-amend"]
```

## Keys

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `spec-dna` | string | required | The pinned version. If the binary has a different version, `check` does not run and prints the install command. The other commands print a warning. |
| `profile` | string | `"standard"` | The default severity of each diagnostic code: `"lite"`, `"standard"` or `"strict"`. [docs/profiles.md](profiles.md) gives the table. |
| `rules` | table | `{}` | Severity overrides for every root. A key is a rule group (`references`, `ids`, `budgets`, `drift`, `evidence`, `lint`) or a code (`SD704`). A value is `"off"`, `"warning"` or `"error"`. A code rule wins over a group rule. |
| `types` | list of strings | all types | The document types of the project. A document of another type is an error (SD303), and `spec-dna new` refuses it. |
| `evidence.tests` | list of globs | the usual test paths | The files where `check` reads `covers: <doc-id>#<key>` tags. The default covers `**/tests/**`, `**/test/**`, `**/__tests__/**` and test file names such as `*_test.*`, `test_*.*`, `*.test.*`, `*.spec.*`, `*Test.*` and `*_spec.*`. An empty list turns the tags off. |
| `exclude` | list of globs | `[]` | Repository paths that are not documents. A link to such a path still resolves as a file. |
| `self-urls` | list of strings | `[]` | URL prefixes that point to this repository, such as its web view. A link that starts with one of these prefixes resolves as a path from the repository root. Each prefix is an `http` or `https` URL that ends in `/`. A `*` matches one full path segment, such as a branch. |
| `install-source` | string | the public repository | The git source of spec-dna for `cargo install --git`: a URL such as `file:///srv/git/spec-dna` or `ssh://git@host/team/spec-dna.git`, without spaces. The message of a version mismatch gives the install command with this source. Set it when the project installs spec-dna from a local or private clone. |
| `roots` | list of tables | required | The document roots. A path belongs to the deepest root that contains it. |
| `roots.path` | string | required | The directory, relative to the repository root. `.` is the full repository. |
| `roots.type` | string | `mixed` | The type of every document in the root (one of the 10 types of [docs/types.md](types.md)), or `mixed`. |
| `roots.default-type` | string | `note` | For a mixed root: the type of a document that has no `type` field. |
| `roots.status` | string | none | The status of a default-type document that has no `status` field. The value must be a status of that type. |
| `roots.exclude` | list of globs | `[]` | Paths, relative to the root, that are not documents. |
| `roots.legacy-ids` | list | `[]` | Patterns that make existing file names into aliases. A string `ADR-{n}` makes the alias `ADR-{n}` from a slug that starts with it. In a table, `file` sets the slug pattern. |
| `roots.legacy-keys` | list | `[]` | Patterns of requirement keys that spec-dna keeps as they are, such as `R-{n}`. For a legacy key, the colon after the bold key is optional: `- **R-12** sentence`, `- **R-12**: sentence`, `- **R-12:** sentence` and `- **R-12.** sentence` are requirements. A spec or contract gets SD806 if it has tokens that match a pattern but no recognized requirement. It also gets SD806 if a list item or a table row starts with a token that matches a pattern, and no recognized requirement of the document has that key. |
| `roots.status-map` | table | `{}` | Legacy status words and the status that each word maps to, such as `released = "active"`. The file keeps its word. Every rule reads the mapped status. A word applies only to a document whose type does not allow that word. The target must be a status of the root's type (in a mixed root, a status of any type). |
| `roots.required-sections` | table | see [docs/types.md](types.md) | For each type, its required sections, in any language. The value is a list of headings (one section, with alternative headings) or a list of such lists (several sections). A document of that type needs one heading of each list (SD305). The comparison ignores case. A value replaces the default of its type, and an empty list removes the requirement. Defaults: decision `["Decision Outcome", "Decision"]`, invariant `["Rules"]`, runbook `["Steps"]`, incident `[["Summary"], ["Root cause"]]`. For a decision, `context` serves the first required section as the summary. |
| `roots.amendment-sections` | table | `{ decision = ["Amendments"] }` | For each type, the names of its amendment sections, in any language. A heading of level 2 or more starts an amendment section if it is a name, or if it starts with a name and a character that is not a letter or a digit: `## Nachtrag 2026-01-02: TTL` matches `Nachtrag`, and `## Nachtragsplan` does not. The comparison ignores case. The section continues to the next heading of the same or a higher level. A list replaces the default of its type. After a decision is accepted, the text outside its amendment sections is frozen (SD405), and new amendment text that states an outcome gives SD605. For a decision, `context` serves the amendment sections with the outcome. |
| `roots.history-sections` | table | `{}` | For each type, the names of its history sections, such as `Iteration log`, `Review rounds` or `Superseded notes`. They match as amendment sections do. `spec-dna history <id>` moves them, and the amendment sections of a decision, into a `reference` document. In a decision, a history section is not frozen text. |
| `roots.companion-sections` | table | `{}` | For each type, the names of the sections that belong to the requirements, such as an acceptance map or an evidence map. A table row or a list item of such a section belongs to the requirement keys in its first cell (or the lead of the item). When `mv` moves requirements, their rows move with them into the section of the destination that matches the same name, or into a new section. A row of a key that moves and a key that stays refuses the move. |
| `roots.live-sections` | table | `{}` | For each type, the names of its live sections, such as `Implementation status`, in any language. They match as amendment sections do. A live section records a current state, so its text can change in place. In a decision, a live section is not frozen text, new text in it gives no SD605, and `mv` and `history` do not move it. |
| `roots.enforce-budgets` | boolean | `true` | If `false`, a document over its hard budget gives the warning SD704 and never the error SD702. |
| `roots.profile` | string | the project profile | The profile of the documents of this root. |
| `roots.rules` | table | `{}` | Severity overrides for this root, with the keys of `rules`. They win over `[rules]` and over every profile. |
| `roots.lint-severity` | string | none | The key of 0.1 for the rule `lint` of this root (SD801 to SD805). `roots.rules` wins over it. |
| `roots.drift-severity` | string | none | The key of 0.1 for the rule `drift` of this root (SD606, SD607). `roots.rules` wins over it. |
| `budgets.<type>` | table | see below | The `soft` and `hard` character budgets. |
| `budgets.growth` | integer | `4000` | The characters that a document over its hard budget in the base can grow in one change with a warning (SD703) instead of an error (SD702): about one amendment or a few registry rows. |
| `lint.max-words` | integer | `30` | The requirement sentence length before a `sentence-length` warning. |
| `context.budget` | integer | `8000` | The default budget of `spec-dna context`, in characters. Each later request of an agent carries the output again, so the default is small. |
| `repos` | table | `{}` | Aliases of other repositories. A reference into one of them is a warning until pinned indexes exist. |
| `conventions` | table | `{}` | For each type, pointers to the sections of project documents that state the conventions of the project for documents of that type, such as "a contract change needs a dated amendment and a `verified_against` update". A pointer is a document id or a path, with an optional `#<anchor>`; without an anchor, it names the full document. When a target of `context` is a document of that type, `context` serves these sections first. `show --conventions <type>` prints them. A pointer that names no document or section gives SD106. |

Default budgets (soft, hard): `spec`, `contract` and `change` 10,000 and 20,000; `guide` 14,000 and 28,000; `decision` 7,000 and 14,000; `invariant` 3,000 and 6,000; `runbook` 8,000 and 16,000; `incident` 6,000 and 12,000; `note` and `reference` none. A budget counts the Unicode characters of the body, without front matter and code blocks. The text of amendment sections counts.

A document over the soft budget gives a warning (SD701), and SD703 if it also grew against the base. The hard budget compares the document with the base ([ADR 0001](adr/0001-information-model.md), "Amendments"):

- A new document over the hard budget is an error (SD702), unless a split moved it out of a document that was over the hard budget in the base (its section aliases name that document): then it gives SD704.
- A document that was under the hard budget in the base and is over it now is an error (SD702).
- A document that was over the hard budget in the base and grew by more than `budgets.growth` characters (default 4,000) is an error (SD702). Growth up to that allowance gives the warning SD703. The comparison uses the body characters of the two versions.
- A document that was over the hard budget in the base and did not grow gives a warning (SD704).

A stub that `mv --stub` leaves and an index that `spec-dna index` generates have no budget. In a root with `enforce-budgets = false`, all of these give SD704. `check --all` uses the same base, so a document that did not change does not give SD702. The profile sets the severity of each budget code ([docs/profiles.md](profiles.md)).

## Sections of a decision

An accepted decision has 3 kinds of section:

| Kind | Key | After acceptance | `context` | `mv`, `history` |
| --- | --- | --- | --- | --- |
| Frozen text | (all other sections) | Cannot change (SD405). Supersede the decision to change its outcome. | The title and the outcome are the summary. | Stays in place. |
| Amendment section | `amendment-sections` (default `Amendments`) | Can change and grow. New text that states a requirement, a BCP 14 keyword or a status word gives SD605. | Served with the outcome. | Moves into a `reference` document. |
| Live section | `live-sections` (no default) | Can change in place, with no SD605. Use it for a current state, such as an implementation status. | Served as a section of the document. | Stays in place. |

A history section (`history-sections`) is not frozen text either, and `history` moves it. A heading that changes its name changes the kind of its section: a renamed live heading is frozen text again.

## Evidence

A requirement has evidence if a test or a verification record links to it. `check` resolves 3 links:

- `verified-by` in the front matter: paths that cover every requirement of the document;
- an inline marker at the end of a requirement: `- **renew**: The library SHALL ... <!-- verified-by: tests/renew.py -->`;
- a `covers: <doc-id>#<key>` tag in a test file (`evidence.tests`). The tag is plain text, so it works in a comment of any language: `// covers: loans#renew`, `# covers: loans#renew, loans#fines`. The ids follow the tag, separated by commas or spaces. The first word that is not an id ends the tag.

A `verified-by` path that is not in the git tree gives SD103. A tag that names no requirement gives SD105. An active requirement (of a `spec`, `contract` or `invariant` with the status `active`) without evidence gives SD608. The profile sets the severities. `show` lists the evidence of each requirement. A moved requirement keeps its tags, because its old id is an alias. spec-dna does not run the tests.

## Patterns and globs

- A pattern (`legacy-ids`, `legacy-keys`) is literal text with exactly one `{n}`, which matches one or more digits, and any number of `*`. The alias does not keep the zeros at the start of the number: `ADR-012-cache` gives `ADR-12`, and a reference to `ADR-012` or `ADR-12` resolves to it.
- A glob (`exclude`, `applies-to`) matches a full `/`-separated path. `**` matches across directories; `*` and `?` match only inside one segment. In `applies-to`, a value without `*` or `?` is a namespace or directory, and it matches everything under it.

## Links by URL

A link with a scheme (`https:`) or a `//` prefix is external, unless it starts with one of the `self-urls`. In that case, the rest of the URL is a path from the repository root: `https://example.org/acme/specs/blob/main/docs/a.md#b` is `docs/a.md#b`. A link to a directory, by URL or by relative path, refers to the `README.md` or `index.md` document in that directory, if there is one. If a self URL names no path in the tree, spec-dna does not report it, because the URL can point to another branch or commit.

## Documents

A document is a `.md` or `.markdown` file under a root and outside every exclude. It can be a tracked or an untracked file, but not an ignored file. `check --base <rev>` reads tracked files only, as CI does: an untracked document is not part of that gate until `git add`. Its id is its path under the root, without the extension. A `README.md` or `index.md` file takes the id of its directory. If that file is directly in a root, it takes the name of the root directory.

In a root of type `reference`, an id starts with a namespace: the shortest suffix of the root path (whole segments) that no other reference root has, such as `plans/rollout` for `tasks/plans/rollout.md`. Thus the ids of two reference roots never collide. The id without the namespace (the id of 0.2) stays a fallback name: it resolves if no other id has it and if it names one document.

A section that `mv` moved keeps its old id as an alias: the destination lists `<old-id>#<anchor>` in its `aliases`. A stub (front matter `moved-to: <id>`) has no id of its own. A generated index has the front matter field `generated: spec-dna index h:<hash>`. [ADR 0001](adr/0001-information-model.md), section "Metadata", describes the front matter.
