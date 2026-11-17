# Configuration: `spec-dna.toml`

`spec-dna.toml` is in the repository root. This page gives each key, its default and its meaning, for the authors of the file. `spec-dna init` writes the first version of the file, and [schema/spec-dna.schema.json](../schema/spec-dna.schema.json) is its JSON Schema.

An unknown key is an error (exit code 3), so a typo cannot disable a rule without an error message. A key or a type name of 0.5 is also an error, and its message names the new name.

```toml
spec-dna = "0.6.1"              # required: the pinned version
profile = "standard"            # "lite", "standard" (default) or "strict"
types = ["requirements", "decision", "rule", "procedure", "incident", "research"]  # default: all 15
exclude = ["**/vendor/**"]      # repository globs that are never documents
self-urls = ["https://example.org/acme/specs/blob/*/"]  # URLs of this repository
install-source = "ssh://git@example.org/tools/spec-dna.git"  # where `cargo install` gets spec-dna
areas = ["valves", "sensors", "reports"]  # the folders of requirements/
language = "en"                 # the language of the normative text: a BCP 47 tag (default "en")

[layers.intent]                 # one table for each layer (see "The table `layers`")
types = ["intent", "spec", "plan"]
moves-to = ["requirements", "decisions"]
keep = "until-finished"
context = "on-request"
max-started = 3

[layers.requirements]
types = ["requirements"]

[layers.decisions]
types = ["decision"]

[groups]                        # named sets of areas
operations = ["valves", "sensors"]

[spec-dna-sha256]               # optional: the release assets of the pin, and their SHA-256 sums
"spec-dna-0.6.1-x86_64-unknown-linux-gnu" = "<64 lowercase hexadecimal characters>"

[rules]                         # severity overrides (profiles.md)
drift = "error"                 # a rule group, or a code such as SD704

[change]                        # optional: without it the policy is "none"
policy = "three-class"          # required in the table: "three-class", "two-class" or "none"
gate = "block"                  # "block", "warn" or "off"
high-risk-layers = ["interfaces", "rules"]  # a delta in these layers makes a change higher-risk (default)
high-risk-paths = []            # globs from the repository root (default: none)
max-fanout = 8                  # documents that one change can touch before SD905 (default)
approval = "base"               # "base" or "branch": where an approval can be (default: "base")
approvers = []                  # the author e-mails of the approval commits, with approval = "branch"

[templates]
dir = ".spec-dna/templates"     # project templates (default); not a root

[evidence]
tests = ["tests/**", "**/*_test.go"]  # test files with `covers:` tags

[evals]                         # optional: the agent evals (see "The table `evals`")
dir = "evals"                   # the folder of the evals (default "evals")
min-pass-rate = 0.8             # the share of evals that must pass, from 0 to 1 (default 0.8)
runs = 3                        # the runs of an eval that sets no `runs`, from 1 to 100 (default 3)
max-total-usd = 20              # the maximum spend of `eval run`, above 0 (default 20)

[[roots]]                       # required: one table per document root
path = "docs"
type = "mixed"                  # one type of a root (not intent, spec or plan), or "mixed" (default)
default-type = "requirements"   # mixed root: the type of a document with no `type` and no type from its place (no default)
status = "draft"                # the status of a default-type document with no `status`
exclude = ["drafts/**"]         # globs relative to the root
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]
legacy-keys = ["R-{n}"]
status-map = { released = "active", wip = "draft" }            # legacy status words
required-sections = { decision = ["Decision Outcome", "Decision"], incident = [["Summary"], ["Root cause"]] }
amendment-sections = { decision = ["Amendments", "Nachtrag"] }  # amendment headings
history-sections = { procedure = ["Iteration log"], decision = ["Discussion"] }  # history headings
companion-sections = { requirements = ["Coverage table"] }  # sections with a row per requirement
live-sections = { decision = ["Status", "Progress"] }  # sections that change in place; the default is Status
enforce-budgets = true          # false: the hard budget gives warnings only
profile = "lite"                # the profile of this root
rules = { lint = "off" }        # severity overrides for this root

[budgets]
growth = 4000                   # characters that a document over its hard budget can grow in one change

[budgets.requirements]          # characters of body text, per type
soft = 10000
hard = 20000

[lint]
max-words = 30                  # the word limit of a requirement sentence, before a warning
history-terms = ["formerly", "vormals"]  # the phrases of SD903; they replace the default phrases

[context]
budget = 8000                   # default character budget of `spec-dna context`

[repos]
platform = "https://example.org/platform.git"   # alias for `platform:<namespace>/<slug>`

[conventions]                   # sections that state the project conventions, per type
interface = ["docs/README.md#interface-changes"]
decision = ["docs/adr/README.md#how-to-amend"]

[harness]                       # the agent harness (docs/harness.md)
level = "lean"                  # required in the table: "none", "lean" or "full"
deny = ["NotebookEdit"]         # tools that no generated agent gets (default [])
context-meter = [300000, 600000]  # context sizes of the main session for the hook; [] = off
subagent-meter = 250000         # context size of a subagent for the hook; 0 = off
fanout-cap = 10                 # agents that a workflow can start before the hook asks; 0 = off
skills = ["brainstorm", "specify", "clarify", "write-plan", "review", "finish", "post-mortem"]  # the skills to write (default [])
review-md = true                # write REVIEW.md (default false)
protect-tests = "fix-branches"  # "fix-branches", "always" or "off" (default "off")

[harness.roles]                 # a role that is not here takes its default
orchestrator = { claude-code = "opus", effort = "xhigh", fallback = ["opus", "sonnet"] }
advisor = { claude-code = "fable", agent = "opus", effort = "xhigh" }
verifier = { claude-code = "opus", effort = "high" }
researcher = { claude-code = "haiku", effort = "high" }
helper = { claude-code = "haiku" }

[harness.efforts]               # the effort for each model, after a /model selection
sonnet = "high"
```

## Keys

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `spec-dna` | string | required | The pinned version. If the binary has a different version, `check`, `finish`, `init` and `eval run` do not run. They print the install command and exit with code 3. The other commands print a warning. To move the pin, run `spec-dna upgrade`. In the checkout of spec-dna itself (a tree binary), SD503 does not count a change of the pin line. See [the tree binary](diagnostics.md#files-that-sd503-counts). |
| `spec-dna-sha256` | table | none | The SHA-256 sums of the release assets of the pin. Each key is an asset name: `spec-dna-<version>-<target>`, with the suffix `.exe` for a Windows target. Each value is its sum. See [The table `spec-dna-sha256`](#the-table-spec-dna-sha256). |
| `profile` | string | `"standard"` | The default severity of each diagnostic code: `"lite"`, `"standard"` or `"strict"`. [docs/profiles.md](profiles.md) gives the table. |
| `rules` | table | `{}` | Severity overrides for every root. A key is a rule group (`references`, `ids`, `budgets`, `drift`, `evidence`, `lint`, `layers`, `changes`) or a code (`SD704`). A value is `"off"`, `"warning"` or `"error"`. A code rule has priority over a group rule. The group `layers` holds SD901 to SD904. The group `changes` holds SD905 and SD910 to SD925. The word `rules` in `[layers.rules]` is a layer name, not the `[rules]` table: the key path shows which one. |
| `language` | string | `"en"` | The language of the normative text: a BCP 47 language tag, such as `en`, `en-GB` or `de`. It is a top-level key only. The form is a primary subtag of 2 or 3 letters, then subtags of 1 to 8 letters or digits, separated by `-`. spec-dna does not check a tag against the language registry. A tag in another form, such as `english`, `en_GB` or `x-private`, is an error (exit code 3). The primary subtag `en` (in any case) means English. Another primary subtag disables the checks that read English word lists (SD801 to SD805 and SD904). |
| `change` | table | none | The change process. See [The table `change`](#the-table-change). |
| `change.policy` | string | required in the table | `"three-class"`, `"two-class"` or `"none"`. Without the `[change]` table the policy is `"none"`. With `"none"`, the codes SD905 and SD910 to SD925 are off, and no rule can enable them. |
| `change.gate` | string | none | `"block"`, `"warn"` or `"off"`. It sets the severity of SD910: error, warning or off. Without the key, SD910 takes the severity of the profile. A rule of `[rules]` for SD910 has priority. |
| `change.high-risk-layers` | array of strings | `["interfaces", "rules"]`, without the names that the graph does not have | The layers whose documents make a change higher-risk. Each name must be a layer of the graph, or the exit code is 3. |
| `change.high-risk-paths` | array of strings | `[]` | Globs from the repository root. A path that matches one of them makes a change higher-risk. A glob is not empty, is not absolute, and has no `.` or `..` segment. |
| `change.max-fanout` | integer | `8` | The most requirement documents that one change can touch before SD905. |
| `change.max-task-files` | integer | `3` | The most paths that one task of an approved plan lists in its label `Files:` before SD924. An integer from 1 to 20, or the exit code is 3. |
| `change.base` | string | none | The name of a local branch, such as `"main"`. It is the base of the change checks when `--base` and `origin/HEAD` give none, as in a repository with no remote. The name is not empty. It has no space and no `..`, and none of `~ ^ : ? * [ \`. It does not start with `-` or `/`, and it does not end with `/` or `.lock`. A bad name is exit code 3. |
| `change.approval` | string | `"base"` | `"base"` or `"branch"`. With `"branch"`, an approval commit of an approver in the branch also counts, if it comes before the work that it approves. It needs a full git history and a not empty `change.approvers`. With `profile = "strict"` it is an error (exit code 3): `strict` reads the approvals of the base only. |
| `change.approvers` | array of strings | none | The author e-mails of the approval commits that count with `approval = "branch"`. The compare ignores case. An item is an e-mail: it is not empty, has no `*` and has an `@`. With `approval = "branch"` the list is not empty, or the exit code is 3. |
| `types` | list of strings | all types | The document types of the project. A document of another type is an error (SD303), and `spec-dna new` does not accept it. |
| `areas` | list of strings | `[]` | The areas of the project: the names of the folders of `requirements/`. Each name is one folder name, and a name is not listed twice. Two names that differ only in case (`valves` and `Valves`) are an error (exit code 3), because a file system that ignores case gives one folder for both. The compare changes only the letters A to Z. `new` and `mv` stop with exit code 2 for a path whose area differs from a name of `areas` only in case. |
| `groups` | table | `{}` | Named sets of areas. A key is a group name. A value is a list of areas. Each area must be in `areas`, or the file is an error (exit code 3). An area can be in more than one group. |
| `layers` | table | the default graph | The layer graph: one table for each layer. A project with no `[layers]` table gets the [default graph](#default-graph). See [The table `layers`](#the-table-layers). |
| `layers.<name>.types` | list of strings | `[]` | The document types of the layer. A type is in one layer at most. |
| `layers.<name>.dir` | string | the name of the layer | The folder of the layer, relative to the root. No 2 layers have the same folder. |
| `layers.<name>.files` | list of strings | `[]` | The top files of the layer, relative to the root, such as `["terms.md"]`. An entry has no `..`, no `/` at the start, no `\` and no `:`. A layer with `files` has no `dir`, and no 2 layers have the same file. |
| `layers.<name>.paths` | list of globs | `[]` | Globs, relative to the root. A document that matches belongs to this layer, also when its type is in another layer. The first layer in the file that matches wins. |
| `layers.<name>.moves-to` | list of strings | `[]` | The layers that the text of this layer matures into. Each name is a layer of `[layers]`. The edges have no cycle. |
| `layers.<name>.linked-from` | list of strings | `[]` | The layers that can link to this layer, although they are more stable than it. Each name is a layer of `[layers]`. |
| `layers.<name>.keep` | string | `"forever"` | `"forever"` or `"until-finished"`. In an `until-finished` layer, a deleted document gives no SD202. |
| `layers.<name>.context` | string | `"always"` | `"always"` or `"on-request"`. |
| `layers.<name>.max-docs` | integer | none | The most documents in the layer. |
| `layers.<name>.max-chars` | integer | none | The most body characters in the layer. |
| `layers.<name>.max-started` | integer | none | The most started changes. |
| `layers.<name>.stale-days` | integer | none | The days without a commit before a change is stale. |
| `evidence.tests` | list of globs | the default test paths | The files where `check` reads `covers: <doc-id>#<key>` tags. The default covers `**/tests/**`, `**/test/**`, `**/__tests__/**` and test file names such as `*_test.*`, `test_*.*`, `*.test.*`, `*.spec.*`, `*Test.*` and `*_spec.*`. An empty list disables the tags. |
| `exclude` | list of globs | `[]` | Repository paths that are not documents. A link to such a path still resolves as a file. |
| `self-urls` | list of strings | `[]` | URL prefixes that point to this repository, such as its web view. A link that starts with one of these prefixes resolves as a path from the repository root. Each prefix is an `http` or `https` URL that ends in `/`. A `*` matches one full path segment, such as a branch. |
| `install-source` | string | the public repository | The git source of spec-dna for `cargo install --git`. The value is a URL without spaces, such as `file:///srv/git/spec-dna` or `ssh://git@host/team/spec-dna.git`. The message of a version mismatch gives the install command with this source. Set it when the project installs spec-dna from a local or private clone. |
| `roots` | list of tables | required | The document roots. A path belongs to the deepest root that contains it. |
| `roots.path` | string | required | The directory, relative to the repository root. `.` means the whole repository. |
| `roots.type` | string | `mixed` | The type of every document in the root, or `mixed`. The value is a type of [docs/types.md](types.md), but not `intent`, `spec` or `plan`. These 3 types are valid only in the `intent` layer, so a root of one of them is an error (exit code 3). In a mixed root, the front matter and the place of a document give its type ([Place gives the type](types.md#place-gives-the-type)). |
| `roots.default-type` | string | none | For a mixed root: the type of a document that has no `type` field and a place that gives no type. The value is a type of a root, but not `intent`, `spec` or `plan`. Without it, such a document gives SD303. |
| `roots.status` | string | none | The status of a default-type document that has no `status` field. The value must be a status of that type, so the root needs a `default-type` or one type. |
| `roots.exclude` | list of globs | `[]` | Paths, relative to the root, that are not documents. |
| `roots.legacy-ids` | list | `[]` | Patterns that make aliases from the names of files. A string `ADR-{n}` makes the alias `ADR-{n}` from a slug that starts with it. In a table, `file` sets the slug pattern. |
| `roots.legacy-keys` | list | `[]` | Patterns of requirement keys that spec-dna keeps as they are, such as `R-{n}`. For a legacy key, the colon after the bold key is optional. See [Legacy keys](#legacy-keys). |
| `roots.status-map` | table | `{}` | Legacy status words and the status that each word maps to, such as `released = "active"`. The file keeps its word. Every rule reads the mapped status. A word applies only to a document whose type does not allow that word. The target must be a status of the type of the root (in a mixed root, a status of any type). |
| `roots.required-sections` | table | see [docs/types.md](types.md) | For each type, its required sections, in any language. See [Required sections](#required-sections). |
| `roots.amendment-sections` | table | `{ decision = ["Amendments"] }` | For each type, the names of its amendment sections, in any language. See [Amendment sections](#amendment-sections). |
| `roots.history-sections` | table | `{}` | For each type, the names of its history sections, such as `Iteration log`, `Discussion` or `Superseded notes`. They match as amendment sections do. `spec-dna history <id>` moves them, and the amendment sections of a decision, into a `research` document. In a decision, a history section is not locked text. |
| `roots.companion-sections` | table | `{}` | For each type, the names of the sections that belong to the requirements, such as a coverage table or an evidence table. A table row of such a section belongs to the requirement keys in its first cell. A list item belongs to the keys in its lead. When `mv` moves requirements, their rows move with them into the section of the destination that matches the same name, or into a new section. If a row has a key that moves and a key that stays, `mv` stops and changes nothing. |
| `roots.live-sections` | table | `{ decision = ["Status"] }` | For each type, the names of its live sections, such as `Status` or `Progress`, in any language. A list for a type replaces the default of that type, so `{ decision = [] }` disables it. They match as amendment sections do. A live section records a current state, so its text can change in place. In a decision, a live section is not locked text. New text in it gives no SD605. `mv` and `history` do not move it. |
| `roots.enforce-budgets` | boolean | `true` | If `false`, a document over its hard budget gives the warning SD704 and never the error SD702. |
| `roots.profile` | string | the project profile | The profile of the documents of this root. |
| `roots.rules` | table | `{}` | Severity overrides for this root, with the keys of `rules`. They have priority over `[rules]` and over every profile. |
| `roots.lint-severity` | string | none | The key of 0.1 for the rule `lint` of this root (SD801 to SD805). `roots.rules` has priority over it. |
| `roots.drift-severity` | string | none | The key of 0.1 for the rule `drift` of this root (SD606, SD607). `roots.rules` has priority over it. |
| `budgets.<type>` | table | see below | The `soft` and `hard` character budgets. |
| `budgets.growth` | integer | `4000` | The number of characters that a document over its hard budget in the base can grow in one change. Growth up to this number gives a warning (SD703), not an error (SD702). The default of 4,000 characters is about the size of one amendment. |
| `lint.history-terms` | list of strings | `"previously"`, `"formerly"`, `"no longer"`, `"used to"`, `"was changed"`, `"before this change"`, `"after this change"` | The phrases that SD903 reports in a requirement. A list replaces the default phrases. An empty list means no phrase: SD903 then reports only dates, `round <n>` and `iteration <n>`. A phrase is not blank, and no phrase is listed twice. |
| `lint.max-words` | integer | `30` | The word limit of a requirement sentence. A longer sentence gives a `sentence-length` warning. |
| `context.budget` | integer | `8000` | The default budget of `spec-dna context`, in characters. Each later request of an agent carries the output again, so the default is small. |
| `repos` | table | `{}` | Aliases of other repositories. A reference into one of them is a warning until pinned indexes exist. |
| `templates.dir` | string | `.spec-dna/templates` | The folder of the project templates, relative to the repository root. A file `<name>.md` in it is a template for `spec-dna new --template <name>`. See [The table `templates`](#the-table-templates). The folder is never part of a root. |
| `evals` | table | none | The agent evals. No table means no evals. See [The table `evals`](#the-table-evals). |
| `evals.dir` | string | `evals` | The folder of the evals, relative to the repository root. The rules are those of `templates.dir`. The folder cannot be the folder of `templates.dir`, hold it or be inside it. The folder is never part of a root. |
| `evals.min-pass-rate` | number | `0.8` | The share of evals that must pass, from 0 to 1. The integers 0 and 1 are valid. |
| `evals.runs` | integer | `3` | The runs of an eval that sets no `runs`, from 1 to 100. |
| `evals.max-total-usd` | number | `20` | The maximum spend of `eval run`, in USD. A number above 0. |
| `conventions` | table | `{}` | For each type, pointers to the sections of project documents that state the conventions for documents of that type. See [Conventions](#conventions). |
| `harness` | table | none | The agent harness. No table is the same as level `none`. See [The table `harness`](#the-table-harness). |
| `harness.level` | string | required in the table | `none` installs nothing. `lean` installs the orchestrator, the advisor, the verifier, the researcher and the helper. `full` also installs the implementer and the code-simplifier. With `none`, spec-dna reads the other keys and ignores them. |
| `harness.deny` | list of strings | `[]` | Claude Code adapter. Tool names. spec-dna removes each one from the `tools` list of every generated agent, and adds it to `permissions.deny` of `.claude/settings.json`. A rule for the whole tool, `<Tool>(*)` or `<Tool>(**)`, counts as the tool name. A list that removes each tool of a generated agent is an error that names the agent. |
| `harness.context-meter` | list of integers | `[300000, 600000]` | The context sizes, in tokens, at which the hook `context-meter` tells the main session its size. The values are greater than 0 and in ascending order. `[]` disables the main-session meter. |
| `harness.subagent-meter` | integer | `250000` | The context size of a subagent, in tokens, for the same hook. `0` disables it. |
| `harness.fanout-cap` | integer | `10` | The number of agents that a workflow can start before the hook `fanout-cap` asks the person. `0` disables the hook. |
| `harness.skills` | list of strings | `[]` | The skills of the workflow pack that `init` writes. Each value is one of the 11 names `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify`, `finish`, `debug` and `post-mortem`, and each is listed once. The order of the list does not matter: spec-dna uses the order of this sentence. Each skill is the file `.claude/skills/spec-dna-<skill>/SKILL.md`. The names `intent`, `spec` and `plan` of 0.6 give exit code 3, and `spec-dna upgrade` renames them. With `none`, spec-dna reads the key and ignores it. |
| `harness.review-md` | boolean | `false` | `true`: `init` writes `REVIEW.md`, the review policy. With `none`, spec-dna reads the key and ignores it. |
| `harness.protect-tests` | string | `"off"` | When the hook `protect-tests` asks the person before an edit of a test file that git tracks: `"fix-branches"` (on a branch whose name starts with `fix/`), `"always"` or `"off"`. The test files are the globs of `[evidence] tests`. With `none`, spec-dna reads the key and ignores it. |
| `harness.roles.<role>` | table | the default of the role | The model and the effort of a role. See [The table `harness`](#the-table-harness). |
| `harness.efforts` | table | `{ sonnet = "high" }` | Claude Code adapter. The effort for each model alias or model id. Claude Code uses it when the person selects another model with `/model`. A value is `low`, `medium`, `high`, `xhigh` or `inherit`. A table in the file replaces the default. The effort of the orchestrator applies to its own model, so an entry for the main model has no effect, and `init` prints a warning. |

## Details of keys

### Legacy keys

For a legacy key, the colon after the bold key is optional. These 4 forms are requirements:

- `- **R-12** sentence`
- `- **R-12**: sentence`
- `- **R-12:** sentence`
- `- **R-12.** sentence`

A `requirements`, `interface` or `rule` document gets SD806 in either of these cases:

- It has tokens that match a pattern, but no recognized requirement.
- A list item or a table row starts with a token that matches a pattern, and no recognized requirement of the document has that key.

### Required sections

The value of a type is a list of headings, or a list of such lists:

- A list of headings is one section with alternative headings.
- A list of lists is several sections.

A document of that type needs one heading of each list (SD305). The comparison ignores case. A value replaces the default of its type. An empty list removes the requirement. For a decision, `context` serves the first required section as the summary.

The defaults are:

- decision: `["Decision Outcome", "Decision"]`
- rule: `["Rules"]`
- incident: `[["Summary"], ["Root cause"]]`
- intent: `["Intent", "Problem", "Summary"]`
- spec: `["Delta"]`
- plan: `["Files"]`

### Amendment sections

The value of a type is the list of the names of its amendment sections, in any language. A heading of level 2 or more starts an amendment section in one of these cases:

- The heading is a name.
- The heading starts with a name and a character that is not a letter or a digit.

For example, `## Nachtrag 2026-01-02: TTL` matches `Nachtrag`, and `## Nachtragsplan` does not. The comparison ignores case. The section continues to the next heading of the same or a higher level. A list replaces the default of its type.

After a decision is accepted, the text outside its amendment sections is locked (SD405). New amendment text that states an outcome gives SD605. For a decision, `context` serves the amendment sections with the outcome.

### Conventions

The value of a type is a list of pointers. Each pointer names a section of a project document that states the conventions for documents of that type. An example of a convention is "an interface change needs a dated amendment and a new `version` value".

- A pointer is a document id or a path, with an optional `#<anchor>`. Without an anchor, it names the whole document.
- A path can also name a Markdown file that is not a document. Examples are a file outside every root, and a file that an `exclude` glob removes, such as a `README.md`. No check, budget or id applies to such a file. spec-dna reads it only for the conventions.
- A pointer gives SD106 if it names no document, no file of the git tree or no section. A file that git ignores is not in the tree. With `--base`, a file that git does not track is not in the tree. spec-dna does not read a symlink whose target is outside the repository.

When a target of `context` is a document of that type, `context` serves these sections first. The sections of files that are not documents come before the sections of documents (`"type": "file"` in `--json`). `show --conventions <type>` prints the sections. `show <path>#<anchor>` prints one section of such a file.

## The table `layers`

A layer is a named set of documents. The table `[layers.<name>]` gives its keys. The name has only letters, digits, `-` and `_`. The file order of the tables is the order of the layers.

Write each layer as a table (`[layers.intent]`), as dotted keys (`layers.intent.keep = "forever"`) or as an inline table. The order is the same in each form. `spec-dna layer add` adds a layer to the file, and `spec-dna layer list` shows the layers ([commands.md](commands.md#layer)).

The graph has these rules. A break of a rule is a configuration error (exit code 3), and the message names the key:

- The `moves-to` edges have no cycle.
- A type is in one layer at most.
- Each name in `moves-to` and in `linked-from` is a layer of `[layers]`.
- A layer has `files` or `dir`, not both.
- An entry of `files` is a relative path inside the root, and no 2 layers have the same file.
- No 2 layers have the same folder. A layer with no `dir` has the folder of its name.
- A layer has only the keys of the table above.

The layers also give the place of a document: its type, if the front matter has no `type`. [Place gives the type](types.md#place-gives-the-type) has the rule. A mixed root with `[layers]` needs a layer for each type that its documents have. A document of a type that no layer has gives SD303, and the message says to run `spec-dna layer add`.

The level of a layer is the number of edges on its longest `moves-to` path to a layer with no edge out. A layer with no edge in or out has level 0. Level 0 is the most stable. The file has no level key. A layer X is more stable than a layer Y if X is reachable from Y by `moves-to` edges.

A document belongs to the first layer in the file whose `paths` match it. If no `paths` match, it belongs to the layer that has its type. `spec-dna layer list` shows which rule put each document in its layer: `paths`, `files` (a top file of the layer of its type) or `type`.

A project with no `[layers]` table gets the [default graph](#default-graph). If the project sets `types`, the graph keeps only the layers that hold one of these types. It drops the edges to the other layers. A project with a `[layers]` table gets only the layers that it writes.

### Keep and context

`keep` sets what happens to a document of the layer:

- `keep = "forever"`: a document stays. SD202 reports a deleted document that keeps no alias. A status can end a document (`superseded`, `retired`). An author can edit it, unless its type locks text, as a decision does.
- `keep = "until-finished"`: a deleted document gives no SD202. In the `intent` layer, `finish` deletes the folder of a change, and the limits `max-started` and `stale-days` apply. Another layer with this value gets the SD202 exemption and the limits `max-docs` and `max-chars`. It has no `finish` command, no SD911 and no SD912.

`context` sets how `spec-dna context` serves the documents of the layer:

- `context = "always"`: the layer can fill the plan of `context` without a named target. The order of the layers by level replaces the type order of 0.5.
- `context = "on-request"`: `context` serves the documents of the layer only when a target names them, or when `show` reads them. The plan counts them in one line and lists no file.

### Default graph

`init` writes the default graph for a project of 30 documents or more. A project with no `[layers]` table also gets it. The column "Level" is a result of the graph. It is not a key.

| Layer | Types | `moves-to` | `linked-from` | `keep` | `context` | Limits | Level |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `foundation` | `product`, `system`, `terms` (`files`) | none | none | `forever` | `always` | `max-chars = 30000` | 0 |
| `rules` | `rule` | none | none | `forever` | `always` | `max-docs = 30` | 0 |
| `requirements` | `requirements` | none | none | `forever` | `always` | none | 0 |
| `interfaces` | `interface` | none | none | `forever` | `always` | none | 0 |
| `decisions` | `decision` | none | none | `forever` | `always` | none | 0 |
| `procedures` | `procedure` | none | none | `forever` | `always` | none | 0 |
| `research` | `research` | `decisions`, `requirements` | `decisions`, `requirements` | `forever` | `on-request` | none | 1 |
| `intent` | `intent`, `spec`, `plan` | `requirements`, `interfaces`, `rules`, `decisions`, `procedures` | none | `until-finished` | `on-request` | `max-docs = 30`, `max-started = 3`, `stale-days = 14` | 1 |
| `incidents` | `incident` | `intent` | `rules`, `procedures`, `decisions`, `requirements`, `interfaces`, `intent` | `forever` | `on-request` | none | 2 |
| `pending` | `roadmap`, `questions` (`files`) | `intent`, `decisions` | `intent`, `requirements`, `decisions` | `forever` | `on-request` | `max-docs = 2` | 2 |

A layer at level 0 needs no `linked-from`, because no layer is more stable than it. `research` and `incidents` keep `linked-from` entries, because a decision, a rule or a procedure names its source. `pending` lists `requirements` and `decisions`, because a requirement can name an open question and a decision can name the roadmap (SD901).

For a project of fewer than 30 documents, also a new project, `init` writes the small graph. It has 7 of the 10 layers: `interfaces`, `procedures` and `research` are not in it. `layer add` adds a missing layer later.

| Layer | `moves-to` | Level |
| --- | --- | --- |
| `foundation` | none | 0 |
| `rules` | none | 0 |
| `requirements` | none | 0 |
| `decisions` | none | 0 |
| `intent` | `rules`, `requirements`, `decisions` | 1 |
| `pending` | `intent` | 2 |
| `incidents` | `intent` | 2 |

Each layer of the small graph keeps its `keep`, `context` and limit keys from the default graph. Its `moves-to` and `linked-from` lists name only the layers that the small graph has, and `pending` moves only to `intent`.

## Areas and groups

An area is a folder under `requirements/`. The key `areas` lists the areas. `init` proposes them when it detects 2 or more. Two names of `areas` never differ only in case. A group is a named set of areas:

```toml
areas = ["valves", "sensors", "reports"]

[groups]
operations = ["valves", "sensors"]
reporting = ["reports"]
```

A project has no group by default, and `init` never proposes one. A group is a view. The options `--group <name>` of `status`, `stats` and `context` read it. A group never makes a folder, an id or a requirement key.

## The table `change`

```toml
[change]
policy = "three-class"                      # "three-class", "two-class" or "none"
gate = "block"                              # "block", "warn" or "off"
high-risk-layers = ["interfaces", "rules"]  # layers whose documents make a change higher-risk
high-risk-paths = []                        # globs from the repository root
max-fanout = 8                              # documents that one change can touch before SD905
max-task-files = 3                          # paths that one task of an approved plan lists before SD924 (1 to 20)
approval = "branch"                         # "base" or "branch": where an approval can be (default: "base")
approvers = ["owner@example.test"]          # the author e-mails of the approval commits
base = "main"                               # a local branch, for a repository with no remote (default: none)
```

The table `[change]` sets the change process: the change classes that the project uses, and the strictness of the gate. [The change guide](guides/change.md) gives the process. The key `policy` is required in the table. A project with no `[change]` table has the policy `none`, so an upgrade adds no new gate.

`spec-dna init` writes the table with the 5 keys `policy`, `gate`, `high-risk-layers`, `high-risk-paths` and `max-fanout` for a new project (policy `three-class`, gate `block`), and `init --reconfigure` keeps their values ([init.md](init.md)). It also writes `approval` and `approvers`: see [Approval in the branch](#approval-in-the-branch). `init` never proposes the key `base`, but `init --reconfigure` keeps it. An unknown key is an error (exit code 3).

`policy` has these values:

| Policy | Classes | Meaning |
| --- | --- | --- |
| `three-class` | `small-fix`, `routine`, `higher-risk` | The default for a new project. |
| `two-class` | `routine`, `higher-risk` | Each change has an intent. It runs the same checks as `three-class`. |
| `none` | none | No change process: the layers and their checks only. |

With `policy = "none"`, the codes of the group `changes` (SD905 and SD910 to SD925) are off. spec-dna disables them before it reads `[rules]`, so a rule of the file cannot enable them. SD921 (a requirement of a `forever` layer changed with no intent folder) is one of them. In `lite` it is off by default, so a project that sets a policy and runs `lite` can enable it with `[rules] changes = "warning"`.

The key `gate` sets the severity of SD910. The order of the sources for SD910 is:

1. The policy `none` disables it.
2. A rule for SD910 in the table of a root, or in `[rules]` (the group `changes` also sets it).
3. The key `gate`: `block` is an error, `warn` is a warning, `off` is off.
4. The profile ([profiles.md](profiles.md)): a warning in `lite`, an error in `standard` and `strict`.

With `gate = "block"`, a change to the artifact of a stage needs the approval of the earlier artifact. With `approval = "base"`, the base tree must show it. With `approval = "branch"`, an approval commit of an approver in the branch also counts.

`high-risk-layers` and `high-risk-paths` set the minimum class of a change. SD917 reads them. `max-fanout` is the limit of SD905. `max-task-files` is the limit of SD924, and it is the size of a task that a person can review. `init` does not write it, and `init --reconfigure` keeps the value that the file has. An absent key means 3.

The lifecycle checks compare the head with the merge base of a base revision and `HEAD`. The base revision is the first of these:

1. `--base`.
2. `refs/remotes/origin/HEAD`. `git remote set-head origin <branch>` sets it.
3. `refs/heads/<base>`, where `<base>` is the key `base`.

spec-dna never guesses a branch: it reads only the branch that the file names, and it uses no network. A repository with no remote has no `origin/HEAD`, so set `base` there. If the branch does not exist and there is no `origin/HEAD`, the base is unknown. This is not an error, because a CI clone often has no local branch. If the branch exists but has no common history with `HEAD`, the note says so. The commands print a note, `check` runs only the checks of one tree, and `--base <rev>` gives the base. `status`, `next`, `finish` and `check` read the key. `finish` reads it for each policy.

A change of `base` edits `spec-dna.toml`, so SD503 applies: make it a change of its own.

### Approval in the branch

`approval` sets where SD910 looks for the approval of the earlier artifact of a change:

| Value | Meaning |
| --- | --- |
| `base` | Only the base tree holds approvals. This is the default and the rule of 0.6. |
| `branch` | An approval commit of an approver in the branch also counts, if it comes before the work that it approves. |

`approvers` lists the author e-mails of such a commit. The compare ignores case and needs an exact match, so a person who hides an e-mail on the code host lists the `noreply` address too. An e-mail is not a proof of identity. [ADR 0012](adr/0012-approval-in-the-branch.md) states the limit: the code host is the control.

These values are an error (exit code 3):

- `approval = "branch"` with no `approvers`, or with an empty list.
- `approval = "branch"` with `profile = "strict"`. `strict` reads the approvals of the base only.
- An item of `approvers` that is empty, that holds `*`, or that has no `@`. A wildcard matches each author, so ADR 0012 rejects it.

`approval = "branch"` needs the full history of the branch. A shallow clone gives the result of `base`, and when the merge base is not reachable SD910 reports reason 5. Use `fetch-depth: 0` in a workflow that runs `check`.

`init` writes `approval = "branch"` and your `git config user.email` as the one approver for `lite` and `standard`. It writes `approval = "base"` for `strict`. If git has no `user.email`, it writes `approval = "base"` and no `approvers`, and it prints a note. An upgraded project keeps `base`. A change of these keys edits `spec-dna.toml`, so SD503 applies.

## The table `templates`

```toml
[templates]
dir = ".spec-dna/templates"
```

`spec-dna new <type> <id> --template <name>` reads `<dir>/<name>.md` first, and then the embedded template of that name. Without `--template`, `new` uses the default template of the type: `<dir>/<default>.md` if the file exists, and the embedded template if not. A file `madr.md` in the folder replaces the embedded template of a decision. [docs/types.md](types.md#templates) lists the embedded templates.

- A project template names its `type` in its front matter. `new` stops with exit code 2 for a template with no `type`, a template of another type and an unknown template name.
- `dir` is a path relative to the repository root. An absolute path, a `..` segment, a `\` and a `:` are errors (exit code 3).
- The folder is never part of a root, also for a root `.`. `check` does not read a file there as a document, so you need no `exclude`. `check` does not read a project template for its content. The document that `new` writes gets the same checks as any other document.

## The table `evals`

```toml
[evals]
dir = "evals"
min-pass-rate = 0.8
runs = 3
max-total-usd = 20
```

An eval is a task with checks that tests the agent setup of a project ([ADR 0008](adr/0008-harness-and-evals.md)). An eval is a file `<dir>/<id>.json`. The table is optional: with no table, there are no evals. The table enables `spec-dna eval run`, SD506 and SD918. `init` writes the table for a new project, unless the harness level or the answer to question 14 is `none`. [evals.md](evals.md) gives the eval file and the runner.

- `dir` is a path relative to the repository root. spec-dna drops a leading `./` and a final `/`. An absolute path, a `..` or `.` segment, a `\` and a `:` are errors (exit code 3).
- `dir` and `templates.dir` must not overlap: one folder is not the other folder, and it is not inside the other folder. If they overlap, the exit code is 3.
- The folder is never part of a root, also for a root `.`. `check` does not read a file there as a document, so you need no `exclude`. For the same reason, `REVIEW.md` and the skills that `[harness]` writes are never part of a root.
- `min-pass-rate`, `runs` and `max-total-usd` are the defaults of a run. An eval file can set `runs`. The command `eval run` can replace each of them. `pass-at` is a key of an eval file only. In `[evals]` it is an unknown key.

## The table `harness`

The table `[harness]` configures the harness of the Claude Code adapter of spec-dna ([agents.md](agents.md)). [ADR 0004](adr/0004-harness.md) and [ADR 0008](adr/0008-harness-and-evals.md) give the decisions. [harness.md](harness.md) gives the generated files and the hooks.

An unknown key in `[harness]` is an error, as in the rest of the file. `spec-dna init` writes the table, and `spec-dna check` compares the generated files with it (SD501, SD502, SD504 and SD505).

### Parts

These keys select the parts of the harness that `init` writes besides the agents. `check` compares each generated file with them (SD501, SD502 and SD504). A value that is not in the lists of the table is an error (exit code 3). `init` writes the 3 keys. A file that has none of them gets no skill, no `REVIEW.md` and no hook entry.

| Key | Values | Meaning |
| --- | --- | --- |
| `skills` | a list of the 11 names | The skills of the workflow pack ([harness.md](harness.md#the-workflow-pack-and-reviewmd)). Each one is the file `.claude/skills/spec-dna-<name>/SKILL.md`. `[]` writes none. `init` proposes the 7 skills that are on by default when `[change] policy` is not `none` (also the 4 others at the level `full`), and `[]` when it is `none`. An old name `intent`, `spec` or `plan` is an error (exit code 3): the message names the new names and `spec-dna upgrade`. |
| `review-md` | `true` or `false` | `true` writes `REVIEW.md` in the repository root. |
| `protect-tests` | `"fix-branches"`, `"always"` or `"off"` | It selects when the hook `protect-tests` asks before an edit of a tracked test file. `"fix-branches"` asks only on a branch that starts with `fix/`. `"always"` removes the branch test. `"off"` writes no hook. An empty `[evidence] tests` list also writes no hook. |

`init --reconfigure` keeps a part that the file sets. A part with no key gets the default of a new project.

### Roles

`[harness.roles.<role>]` has these keys. The roles are `orchestrator`, `advisor`, `verifier`, `researcher`, `implementer`, `code-simplifier` and `helper`. The keys `reviewer` and `spec-writer` of spec-dna 0.5 give exit code 3. The role `reviewer` is now `verifier`. The role `spec-writer` ended.

| Key | Roles | Meaning |
| --- | --- | --- |
| `claude-code` | all | The model for Claude Code: an alias (`opus`, `sonnet`, `haiku`, `fable`), a model id, `inherit` or `none`. `none` disables the role. `inherit` gives the agent file `model: inherit`; for the orchestrator and the helper it writes no key. A later adapter adds its own key, for example `codex`. |
| `effort` | all but `helper` | `low`, `medium`, `high`, `xhigh`, `max` or `inherit` (no effort is written). For the advisor, it is the effort of the `fallback-advisor` agent. |
| `agent` | `advisor` | The model of the `fallback-advisor` agent: a model or `none` (no agent file). Default `opus`. |
| `fallback` | `orchestrator` | The models of `fallbackModel` in `.claude/settings.json`, in order. Default `["opus", "sonnet"]`. `[]` writes no `fallbackModel`. |

The defaults:

| Role | Model | Effort | Levels |
| --- | --- | --- | --- |
| `orchestrator` | `opus` | `xhigh` | `lean`, `full` |
| `advisor` | `fable` (agent: `opus`) | `xhigh` (agent) | `lean`, `full` |
| `verifier` | `opus` | `high` | `lean`, `full` |
| `researcher` | `haiku` | `high` | `lean`, `full` |
| `implementer` | `sonnet` | `high` | `full` |
| `code-simplifier` | `sonnet` | `high` | `full` |
| `helper` | `haiku` | none | `lean`, `full` |

A role that the level does not include is an error, for example `harness.roles.implementer`: the role is in the level `full`. These rules apply to the values:

- A model is a string that is not empty and has no spaces.
- The orchestrator cannot be `none`, and its effort cannot be `max`. The advisor model cannot be `inherit`, and cannot be `haiku` in any letter case.
- A value in `[harness.efforts]` cannot be `max`.
- For `modelSettings`, spec-dna maps an alias to a model id. A value with a context suffix, such as `opus[1m]`, and the values `opusplan`, `default` and `best` have no id. For such a main model, spec-dna writes no effort, and `init` reports it.
- The advisor should not be a weaker model than the orchestrator. If it is weaker, the configuration is still valid, and `init` prints a warning. spec-dna ranks the known aliases from strong to weak: `fable`, `opus`, `sonnet`, `haiku`.

## The table `spec-dna-sha256`

[ADR 0003](adr/0003-distribution.md) (amendment "phase 2") gives the form of this table. Each release has the file `spec-dna-pin.toml`. It has the key `spec-dna` and the full table of that release. Save the file outside the repository, and run `spec-dna upgrade --pin-file ../spec-dna-pin.toml`. The command moves the pin and writes the table into `spec-dna.toml`. You can remove the lines of the targets that your project does not use.

```toml
spec-dna = "0.6.1"

[spec-dna-sha256]
"spec-dna-0.6.1-aarch64-apple-darwin" = "<64 lowercase hexadecimal characters>"
"spec-dna-0.6.1-aarch64-pc-windows-gnullvm.exe" = "<...>"
"spec-dna-0.6.1-aarch64-unknown-linux-gnu" = "<...>"
"spec-dna-0.6.1-x86_64-apple-darwin" = "<...>"
"spec-dna-0.6.1-x86_64-pc-windows-gnu.exe" = "<...>"
"spec-dna-0.6.1-x86_64-unknown-linux-gnu" = "<...>"
"spec-dna-0.6.1-x86_64-unknown-linux-musl" = "<...>"
```

- If the pin is the version of the binary, each command validates the table. Each key must be a release asset of that version: one of the 7 targets, with `.exe` for the 2 Windows targets. Each value must be 64 lowercase hexadecimal characters. A table error is a configuration error (exit code 3).
- If the pin is a different version, the binary does not validate the table. It reads only the sum of its own asset, for the message of the version mismatch. The asset names contain the version, so a table of an earlier version never matches the pin.
- The binary does not compute the sum of its own file. The install step verifies the sum. The install step is one of the install commands of the release, or the CI job that `init` writes ([docs/quickstart.md](quickstart.md)).
- `init --reconfigure` keeps the table and writes it again in the same form.
- The message of a version mismatch can give the URL and the sum of the pinned asset for this binary. It gives them if the file has that sum and has no `install-source`. Otherwise, it gives the `cargo install` command. If the table exists, the instruction to move the project to the new version names `spec-dna-pin.toml` of that release and the command `spec-dna upgrade --pin-file ../spec-dna-pin.toml`.

## The pin comes first

`spec-dna.toml` can fail to parse while its key `spec-dna` is a different version from the binary. Then the binary gives the message of the version mismatch (exit code 3), not the parse error. A file of a later version can have keys that this binary does not know. The binary finds the key `spec-dna` with a line scan of the lines before the first table header. A binary of a version before 0.4.0 does not have this rule. It reports ``unknown key `spec-dna-sha256` `` for a pin with sums.

## Budgets

The default budgets are (soft and hard, in characters):

- `product`, `system`, `roadmap` and `questions`: 8,000 and 16,000
- `terms`, `requirements`, `interface` and `spec`: 10,000 and 20,000
- `procedure`: 14,000 and 28,000
- `decision`: 7,000 and 14,000
- `rule`: 3,000 and 6,000
- `incident` and `intent`: 6,000 and 12,000
- `research` and `plan`: none. A plan of small tasks grows with the size of the change, and `finish` deletes it. `[budgets.plan]` sets a budget

A budget counts the Unicode characters of the body, without front matter and code blocks. The text of amendment sections counts.

`soft = 0` turns off SD701 and SD703 for a type. `hard = 0` turns off SD702 and SD704, and it needs `soft = 0`, because a `hard` below `soft` is a configuration error (exit code 3). So `soft = 0` and `hard = 0` give a type no budget. `[budgets.plan]` with 2 values above 0 gives a plan the rules of SD701 to SD704. A project that has an explicit `[budgets.plan]` keeps it. Delete the table to take the new default.

A document over the soft budget gives a warning (SD701). If it also grew against the base, it gives SD703. The hard budget compares the document with the base ([ADR 0001](adr/0001-information-model.md), "Amendments"). These rules apply:

- A new document over the hard budget is an error (SD702). The exception: a split moved it out of a document that was over the hard budget in the base (its section aliases name that document). Then it gives SD704.
- A document that was under the hard budget in the base and is over it now is an error (SD702).
- A document that was over the hard budget in the base and grew by more than `budgets.growth` characters (default 4,000) is an error (SD702). Growth up to that allowance gives the warning SD703. The comparison uses the body characters of the 2 versions.
- A document that was over the hard budget in the base and did not grow gives a warning (SD704).

A stub that `mv --stub` leaves and an index that `spec-dna index` generates have no budget. In a root with `enforce-budgets = false`, each of the 4 cases above gives SD704. `check --all` uses the same base, so a document that did not change does not give SD702. The profile sets the severity of each budget code ([docs/profiles.md](profiles.md)).

## Sections of a decision

An accepted decision has 3 kinds of section:

| Kind | Key | After acceptance | `context` | `mv`, `history` |
| --- | --- | --- | --- | --- |
| Locked text | (all other sections) | Cannot change (SD405). Supersede the decision to change its outcome. | The title and the outcome are the summary. | Stays in place. |
| Amendment section | `amendment-sections` (default `Amendments`) | Can change and grow. New text that states a requirement, a BCP 14 keyword or a status word gives SD605. | Served with the outcome. | Moves into a `research` document. |
| Live section | `live-sections` (default `Status`) | Can change in place, with no SD605. Use it for a current state, such as a status or a progress section. | Served as a section of the document. | Stays in place. |

A history section (`history-sections`) is not locked text either, and `history` moves it. If you change the name in a heading, the kind of its section can change. A renamed live heading is locked text again.

## Evidence

A requirement has evidence if a test or a verification record links to it. `check` resolves 3 links:

- `verified-by` in the front matter: paths that cover every requirement of the document
- an inline marker at the end of a requirement: `- **renew**: The library SHALL ... <!-- verified-by: tests/renew.py -->`
- a `covers: <doc-id>#<key>` tag in a test file (`evidence.tests`)

A tag `covers: partial <doc-id>#<key>` records that a test proves a part of a requirement. It is not evidence ([A partial tag](#a-partial-tag)).

The tag is plain text, so it works in a comment of any language. Examples: `// covers: loans#renew` and `# covers: loans#renew, loans#fines`. The ids follow the tag, separated by commas or spaces. The first word that is not an id ends the tag.

### A partial tag

The word `partial` comes first after `covers:` (in any ASCII case), and it applies to each id of the tag: `# covers: partial loans#renew, loans#fines`. The word `partial` after an id is not this form: it ends the tag, as any other word, and the tag stays full.

- Alone, a partial tag does not clear SD608 or SD916, and it does not clear the stage `test` of `next` and `status`. The messages name it.
- A full tag or a `verified-by` of the same requirement is evidence, also next to a partial tag.
- A partial tag that names no requirement gives SD105, as a full tag does.
- `show` lists the partial tags apart from the sources, `status` gives the token `partial <n>`, and `stats --sdlc` gives the share of requirements that have a partial tag and no full evidence.
- A binary before 0.7.0 reads no id after the word `partial`, so it reads no evidence from such a tag.

A `verified-by` path that is not in the git tree gives SD103. A tag that names no requirement gives SD105. An active requirement (of a `requirements`, `interface` or `rule` document with the status `active`) without evidence gives SD608. The profile sets the severities.

`show` lists the evidence of each requirement. A moved requirement keeps its tags, because its old id is an alias. spec-dna does not run the tests.

## Patterns and globs

- A pattern (`legacy-ids`, `legacy-keys`) is literal text with exactly one `{n}`, which matches one or more digits, and any number of `*`. The alias does not keep the zeros at the start of the number. For example, `ADR-012-cache` gives `ADR-12`. A reference to `ADR-012` or `ADR-12` resolves to it.
- A glob (`exclude`, `applies-to`) matches a full `/`-separated path. `**` matches across directories. `*` and `?` match only inside one segment. In `applies-to`, a value without `*` or `?` is a namespace or directory, and it matches everything under it.

## Links by URL

A link with a scheme (`https:`) or a `//` prefix is external, unless it starts with one of the `self-urls`. In that case, the rest of the URL is a path from the repository root: `https://example.org/acme/specs/blob/main/docs/a.md#b` is `docs/a.md#b`. A link to a directory, by URL or by relative path, refers to the `README.md` or `index.md` document in that directory, if there is one. If a self URL names no path in the tree, spec-dna does not report it, because the URL can point to another branch or commit.

## Documents

A document is a `.md` or `.markdown` file under a root and outside every exclude. It can be a tracked or an untracked file, but not an ignored file. `check --base <rev>` reads tracked files only, as CI does. An untracked document is not part of that gate until you run `git add`. Its id is its path under the root, without the extension.

A `README.md` or `index.md` file takes the id of its directory. If that file is directly in a root, it takes the name of the root directory.

In a root of type `research`, an id starts with a namespace. The namespace is the shortest suffix of the root path (whole segments) that no other research root has, such as `history/rollout` for `docs/history/rollout.md`. Thus the ids of 2 research roots never collide. The id without the namespace (the id of 0.2) stays a fallback name. It resolves if no other id has it and if it names one document.

A section that `mv` moved keeps its old id as an alias: the destination lists `<old-id>#<anchor>` in its `aliases`. A stub (front matter `moved-to: <id>`) has no id of its own. A generated index has the front matter field `generated: spec-dna index h:<hash>`. [ADR 0001](adr/0001-information-model.md), section "Metadata", describes the front matter.
