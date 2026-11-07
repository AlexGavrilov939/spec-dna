# Configuration: `spec-dna.toml`

`spec-dna.toml` is in the repository root. `spec-dna init` writes the first version of the file. The JSON Schema is [schema/spec-dna.schema.json](../schema/spec-dna.schema.json). An unknown key is an error (exit code 3), so a typo cannot disable a rule without an error message.

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
| `spec-dna` | string | required | The pinned version. If the binary has a different version, `check` does not run and prints the install command. The other commands print a warning. |
| `exclude` | list of globs | `[]` | Repository paths that are not documents. A link to such a path still resolves as a file. |
| `self-urls` | list of strings | `[]` | URL prefixes that point to this repository, such as its web view. A link that starts with one of these prefixes resolves as a path from the repository root. Each prefix is an `http` or `https` URL that ends in `/`. A `*` matches one full path segment, such as a branch. |
| `roots` | list of tables | required | The document roots. A path belongs to the deepest root that contains it. |
| `roots.path` | string | required | The directory, relative to the repository root. `.` is the full repository. |
| `roots.type` | string | `mixed` | The type of every document in the root, or `mixed`. |
| `roots.default-type` | string | `note` | For a mixed root: the type of a document that has no `type` field. |
| `roots.status` | string | none | The status of a default-type document that has no `status` field. The value must be a status of that type. |
| `roots.exclude` | list of globs | `[]` | Paths, relative to the root, that are not documents. |
| `roots.legacy-ids` | list | `[]` | Patterns that make existing file names into aliases. A string `ADR-{n}` makes the alias `ADR-{n}` from a slug that starts with it. In a table, `file` sets the slug pattern. |
| `roots.legacy-keys` | list | `[]` | Patterns of requirement keys that spec-dna keeps as they are, such as `R-{n}`. |
| `roots.status-map` | table | `{}` | Legacy status words and the status that each word maps to, such as `released = "active"`. The file keeps its word. Every rule reads the mapped status. A word applies only to a document whose type does not allow that word. The target must be a status of the root's type (in a mixed root, a status of any type). |
| `roots.decision-sections` | list of strings | `["Decision Outcome", "Decision"]` | Headings that hold the outcome of a decision. The comparison ignores case. The list replaces the default. `check` requires one of these headings (SD305). `context` serves that section as the summary of the decision. |
| `budgets.<type>` | table | see below | The `soft` and `hard` character budgets. |
| `lint.max-words` | integer | `30` | The requirement sentence length before a `sentence-length` warning. |
| `context.budget` | integer | `12000` | The default budget of `spec-dna context`, in characters. |
| `repos` | table | `{}` | Aliases of other repositories. A reference into one of them is a warning until pinned indexes exist. |

Default budgets (soft, hard): `spec`, `contract` and `change` 10,000 and 20,000; `guide` 14,000 and 28,000; `decision` 7,000 and 14,000; `note` none. A budget counts the Unicode characters of the body, without front matter and code blocks. Text under `## Amendments` counts.

A document over the soft budget gives a warning. A document over the hard budget gives an error. This error fails `check` only when it is new against the base. A document that was already over the hard budget does not cause a failure, and its growth gives a warning ([ADR 0001](adr/0001-information-model.md), "Amendments").

## Patterns and globs

- A pattern (`legacy-ids`, `legacy-keys`) is literal text with exactly one `{n}`, which matches one or more digits, and any number of `*`. The alias does not keep the zeros at the start of the number: `ADR-012-cache` gives `ADR-12`, and a reference to `ADR-012` or `ADR-12` resolves to it.
- A glob (`exclude`, `applies-to`) matches a full `/`-separated path. `**` matches across directories; `*` and `?` match only inside one segment. In `applies-to`, a value without `*` or `?` is a namespace or directory, and it matches everything under it.

## Links by URL

A link with a scheme (`https:`) or a `//` prefix is external, unless it starts with one of the `self-urls`. In that case, the rest of the URL is a path from the repository root: `https://example.org/acme/specs/blob/main/docs/a.md#b` is `docs/a.md#b`. A link to a directory, by URL or by relative path, refers to the `README.md` or `index.md` document in that directory, if there is one. If a self URL names no path in the tree, spec-dna does not report it, because the URL can point to another branch or commit.

## Documents

A document is a `.md` or `.markdown` file under a root and outside every exclude. It can be a tracked or an untracked file, but not an ignored file. Its id is its path under the root, without the extension. A `README.md` or `index.md` file takes the id of its directory. If that file is directly in a root, it takes the name of the root directory. [ADR 0001](adr/0001-information-model.md), section "Metadata", describes the front matter.
