# Configuration: `spec-dna.toml`

`spec-dna.toml` sits at the repository root. `spec-dna init` writes a starting version. The JSON Schema is [schema/spec-dna.schema.json](../schema/spec-dna.schema.json). Unknown keys are errors (exit code 3), so a typo cannot silently turn a rule off.

```toml
spec-dna = "0.1.0"              # required: the pinned version
exclude = ["**/vendor/**"]      # repository globs that are never documents
self-urls = ["https://example.org/acme/specs/blob/*/"]  # URLs of this repository

[[roots]]                       # required: one table per document root
path = "docs"
type = "mixed"                  # one of the 6 types, or "mixed" (default)
default-type = "spec"           # mixed roots: type when front matter has none (default "note")
status = "draft"                # status of a default-type document without `status`
exclude = ["drafts/**"]         # globs relative to the root
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]
legacy-keys = ["R-{n}"]
status-map = { released = "active", wip = "draft" }            # legacy status words
decision-sections = ["Decision Outcome", "Decision"]           # outcome headings

[budgets.spec]                  # characters of body text, per type
soft = 10000
hard = 20000

[lint]
max-words = 30                  # longest requirement sentence before a warning

[context]
budget = 12000                  # default character budget of `spec-dna context`

[repos]
platform = "https://example.org/platform.git"   # alias for `platform:<namespace>/<slug>`
```

## Keys

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `spec-dna` | string | required | Pinned version. `check` refuses to run a different binary and prints the install command. Other commands warn |
| `exclude` | list of globs | `[]` | Repository paths that are not documents. Links to them still resolve as files |
| `self-urls` | list of strings | `[]` | URL prefixes that address this repository, such as its web view. A link under one resolves as a path from the repository root. Each is an `http` or `https` URL that ends in `/`; `*` stands for one whole path segment, such as a branch |
| `roots` | list of tables | required | Document roots. A path belongs to the deepest root that contains it |
| `roots.path` | string | required | Directory relative to the repository root, `.` for all of it |
| `roots.type` | string | `mixed` | The type of every document in the root, or `mixed` |
| `roots.default-type` | string | `note` | Mixed roots: the type of a document without a `type` field |
| `roots.status` | string | none | The status of a default-type document without a `status` field. Must be a status of that type |
| `roots.exclude` | list of globs | `[]` | Paths relative to the root that are not documents |
| `roots.legacy-ids` | list | `[]` | Patterns that make existing file names aliases. A string `ADR-{n}` means alias `ADR-{n}` from a slug that starts with it. A table sets the slug pattern with `file` |
| `roots.legacy-keys` | list | `[]` | Patterns of requirement keys that stay as written, such as `R-{n}` |
| `roots.status-map` | table | `{}` | Legacy status words and the status each stands for, such as `released = "active"`. The file keeps its word; every rule reads the mapped status. A word applies only to a document whose type does not allow it; the target must be a status of the root's type (of any type in a mixed root) |
| `roots.decision-sections` | list of strings | `["Decision Outcome", "Decision"]` | Headings that hold a decision's outcome, compared without case. The list replaces the default. `check` requires one of them (SD305); `context` serves it as the decision's summary |
| `budgets.<type>` | table | see below | `soft` and `hard` character budgets |
| `lint.max-words` | integer | `30` | Requirement sentence length before a `sentence-length` warning |
| `context.budget` | integer | `12000` | Default budget of `spec-dna context`, in characters |
| `repos` | table | `{}` | Aliases of other repositories. References into them stay warnings until pinned indexes exist |

Default budgets (soft, hard): `spec`, `contract` and `change` 10,000 and 20,000; `guide` 14,000 and 28,000; `decision` 7,000 and 14,000; `note` none. Budgets count Unicode characters of the body without front matter and code blocks; text under `## Amendments` counts. Over the soft budget is a warning. Over the hard budget is an error, which fails `check` only when it is new against the base: a document that was already over it does not, and its growth is a warning ([ADR 0001](adr/0001-information-model.md), "Amendments").

## Patterns and globs

- A pattern (`legacy-ids`, `legacy-keys`) is literal text with exactly one `{n}`, which matches one or more digits, and any number of `*`. Leading zeros are dropped from the alias: `ADR-012-cache` gives `ADR-12`, and a reference to `ADR-012` or `ADR-12` resolves to it.
- A glob (`exclude`, `applies-to`) matches a whole `/`-separated path: `**` spans directories, `*` and `?` stay inside one segment. In `applies-to`, a value without `*` or `?` is a namespace or directory and matches everything under it.

## Links by URL

A link with a scheme (`https:`) or a `//` prefix is external unless it starts with one of `self-urls`. Then the rest of the URL is a path from the repository root: `https://example.org/acme/specs/blob/main/docs/a.md#b` is `docs/a.md#b`. A link to a directory, by URL or by relative path, refers to the directory's `README.md` or `index.md` document if it has one. A self URL that names no path in the tree is not reported, because it may point at another branch or commit.

## Documents

A document is a tracked or untracked, not ignored, `.md` or `.markdown` file under a root and outside every exclude. Its id is its path under the root without the extension; a `README.md` or `index.md` takes the id of its directory, and one directly in a root takes the name of the root directory. Front matter is described in [ADR 0001](adr/0001-information-model.md), section "Metadata".
