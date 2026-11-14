# `spec-dna init`

`init` makes the configuration of a repository. It detects the facts of the repository. It asks at most 8 questions with the detected defaults, and it shows a preview. It writes only after a confirmation. It never edits a document. In an instruction file (`AGENTS.md`, `CLAUDE.md`), it changes only its own generated line.

```sh
spec-dna init                   # interactive on a terminal
spec-dna init --dry-run         # the preview only
spec-dna init --non-interactive # the detected defaults
spec-dna init --answers a.toml  # the answers of a file
spec-dna init --reconfigure     # the questions again, with the current values as defaults
```

Without a terminal (for example, when an agent runs `init`), `init` uses the detected defaults, as `--non-interactive` does. If `spec-dna.toml` exists, `init` updates only the agent adapters, as in 0.1, unless you set `--reconfigure`. Run `init` in this way after an upgrade.

## Detection

`init` reads the file list of git (tracked files and untracked files that git does not ignore) and the Markdown files. It finds these facts:

| Fact | How |
| --- | --- |
| Greenfield or brownfield | A repository is brownfield if it has documents to map or 20 files or more |
| Document roots | The highest directories that are mostly Markdown, or that have a documentation name (`docs`, `doc`, `documentation`, `wiki`, `handbook`) |
| Types of roots | The name of the directory. See [Types of roots](#types-of-roots) |
| Id patterns | The names of the files. See [Id patterns](#id-patterns) |
| Types of numbered directories | The files of the directory. See [Types of numbered directories](#types-of-numbered-directories) |
| Requirement keys | Bold keys at the start of list items, such as `- **R-12** text`. They give `legacy-keys = ["R-{n}"]`. If some keys have a letter suffix, the pattern is `R-{n}*` |
| Statuses | The status words in the front matter of each root. A word that the type does not allow gets a `status-map` entry. For example, `shipped` and `implemented` map to `active`, and `approved` maps to `accepted` for a decision. An unknown word maps to the default status of the root |
| Heading languages and aliases | The scripts of the headings. Known translations of `Decision Outcome` and `Amendments` in decisions (for example `Entscheidung`, `Decisión`, `Nachtrag`) become `required-sections` and `amendment-sections` |
| Exclusions | Vendored directories (`vendor`, `node_modules`, `third_party`), generated directories (`dist`, `build`, `generated`), files that say that a generator wrote them, sibling directories with a vendor prefix (`vendor-*`, `external-*`), and the files of a decision root that are not decisions (a README without an outcome, `*-implementation-plan.md`) |
| Large reference directories | A directory with 100 Markdown files or more, or 1,000,000 characters or more, and with no typed directory in it. It becomes a `reference` root |
| Required sections | If most documents of a type in a root do not have the required sections, the root removes the requirement |
| Budgets | The documents over the hard budget, at 3 levels |
| Stack and apps | Manifests such as `Cargo.toml`, `package.json`, `go.mod` and `pyproject.toml`, down to depth 3 |
| Tests | The test globs that match files, for `covers:` tags |
| Agent files | `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.claude/skills`, `.cursor/rules` and other agent files |
| CI and hooks | `.github/workflows`, `.gitlab-ci.yml` and other CI files; pre-commit, husky and lefthook |
| Default branch | `origin/HEAD`. If it is absent, a local `main` or `master`. If both are absent, the current branch |

Detection skips vendored, generated and test fixture directories (`fixtures`, `testdata`, `__snapshots__`, `golden`). In a root `.`, it excludes the root files of the repository (`README.md`, `AGENTS.md`, `CLAUDE.md`, `CHANGELOG.md` and similar files). In a root that holds `.claude/skills`, it excludes the generated skill `spec-dna/`. Detection only reads files. It does not use the network.

### Types of roots

The name of a directory gives the type of the root:

- `adr`, `adrs` and `decisions` give `decision`.
- `specs` gives `spec`.
- `contracts` gives `contract`.
- `guides` and `guidelines` give `guide`.
- `runbooks` gives `runbook`.
- `incidents`, `incident`, `postmortems` and `postmortem` give `incident`.
- `rfcs`, `tasks` and `plans` give `change`.
- `research`, `references`, `meetings` and `archive` give `reference`.

This rule also applies to a top-level directory, such as `rfcs/`. The names `incident` and `postmortem` are also common names of code directories. They give a type only inside a documentation tree, or to a directory that is mostly Markdown. Agent directories (`.claude/skills`, `.claude/agents`, `.cursor/rules`) give `guide`. A root becomes mixed if the front matter of its documents names another type.

### Id patterns

- File names such as `ADR-012-cache.md` give `legacy-ids = ["ADR-{n}"]`. A directory of such files becomes a root.
- Decision files such as `0001-use-postgres.md` (adr-tools, for example in `doc/adr`) give `{ alias = "ADR-{n}", file = "{n}-*" }`. These conditions apply:
  - each number of the root is unique;
  - no file of the root starts with a date;
  - no other root makes the same alias.
- Files with a number and no prefix make a root only for decisions and changes.

### Types of numbered directories

The files of the directory give its type. `init` examines these facts in this order:

1. The prefix: `ADR` gives `decision`, `RFC` gives `change` and `INC` gives `incident`.
2. The front matter `type` of most files.
3. The content of most files:
    - `Decision`, or `Context` and `Consequences`, gives `decision`.
    - `Root cause` as a heading or a table field, or `Timeline` and `Impact`, gives `incident`.
    - `Steps` gives `runbook`.
    - Requirements with a BCP 14 keyword give `spec`.

If the files show no type, the type is the type of the enclosing root, if that root has a type. In a mixed root, the type is `note`. A mixed documentation tree takes the type of its numbered files only if it has no other documents.

## Questions

| # | Question | Default |
| --- | --- | --- |
| 1 | The profile: `lite`, `standard` or `strict` ([profiles.md](profiles.md)) | `standard` |
| 2 | The knowledge types of the project. A shorter list writes `types` into `spec-dna.toml`. Then a document of another type is an error | Every type (no `types` key). The question shows the detected types |
| 3 | The evidence policy (`profile`, `off`, `warning`, `error`) and the link style (`covers`, `verified-by`, `both`) | `profile, both` |
| 4 | The normative language and the heading aliases: `en; outcome: Entscheidung; amendments: Nachtrag` | The detected aliases |
| 5 | The mapping of each detected root: press Enter to accept it, enter `drop` to drop it, or edit it (`type=spec status=active exclude=a/**,b.md`) | Accept |
| 6 | The budget level: `default`, `relaxed` (2 times), `tight` (half) or `off`. The question shows the number of documents over the hard budget at each level | `default` |
| 7 | The agent adapters: `agents-md`, `claude-md`, `skill` or `none` | The instruction files that exist, and the skill |
| 8 | The CI step (`github`, `gitlab`, `none`), the hook (`pre-commit`, `husky`, `lefthook`, `none`) and the base branch | The detected CI system, hook framework and default branch |

A language other than `en` sets `[rules] lint = "off"`, because the requirement lint reads English only. A type that you do not choose makes its detected roots `note` roots. A new project gets one mixed root (`docs`, or the directory of `--root <dir>`). In this root, `spec-dna new` puts each type into its own namespace, such as `decisions/<id>`. Thus the ids of different types cannot clash.

## Preview and files

The preview starts with the 8 answers and how `init` applied each one. A line gives the value, whether it is the default of the question or changed from it, and what it writes. An answer that equals the default and writes no key or file says `writes nothing`. For example, the default types (all 10) write no `types` key.

Then the preview shows the full `spec-dna.toml` and each file to create or change:

- `spec-dna.toml`
- the generated line in `AGENTS.md` and `CLAUDE.md`, and the skill `.claude/skills/spec-dna/SKILL.md`
- for `github`: `.github/workflows/spec-dna.yml` (the job of [quickstart.md](quickstart.md#ci)). If the file exists, `init` does not change it.
- a local hook at the end of `.pre-commit-config.yaml` (only if `repos:` is its last top-level key), or one line in `.husky/pre-commit`. The hook runs `spec-dna check --staged`, which checks the git index: a commit records the index, not the working tree. A hook that 0.4.1 wrote (`spec-dna check`) reads the working tree. `init` changes that exact line in place when it runs again (also with an existing `spec-dna.toml`), and the preview shows the diff. It does not change any other text of a hook.

After the list, the preview gives the diff of each file other than `spec-dna.toml`. A file that does not exist yet shows as a diff against `/dev/null`. `--dry-run` shows the same diffs, even for a project that has a `spec-dna.toml` already (`init` then updates the adapters only). With `--json`, each entry of `files` has a `diff`, and `answers` has the 8 answers.

The skill lists the types of the project (`types` of `spec-dna.toml`), or all 10 types if the project has no `types` list. If you change `types` later, run `spec-dna init` to update the skill, or `check` reports SD502.

For GitLab CI and lefthook, the preview gives the text to add manually. `init` checks that it can read the new `spec-dna.toml` before it writes a file.

## Answers file

An answers file is TOML. Each key is optional. An absent key keeps its default.

```toml
profile = "standard"
types = ["spec", "decision", "invariant", "runbook", "incident", "reference"]
evidence = "warning"            # "profile", "off", "warning" or "error"
evidence-style = "both"         # "covers", "verified-by" or "both"
language = "en"
outcome-headings = ["Entscheidung"]
amendment-headings = ["Nachtrag"]
mapping = "accept"
drop = ["docs/archive"]         # detected roots to drop
budgets = "default"             # "default", "relaxed", "tight" or "off"
adapters = ["agents-md", "claude-md", "skill"]
ci = "github"                   # "github", "gitlab" or "none"
hook = "pre-commit"             # "pre-commit", "husky", "lefthook" or "none"
base = "main"
write = true                    # false: the preview only

[[roots]]                       # optional: these roots replace the mapping
path = "docs"
type = "mixed"
default-type = "spec"
```

The `[[roots]]` tables use the keys of `spec-dna.toml` ([config.md](config.md)).

## Reconfigure

`init --reconfigure` asks the questions again. The defaults are the current values: the profile, the types, the evidence rule and test globs, and the roots of `spec-dna.toml`. `init` does not change the keys that no question asks for:

- `install-source`, `self-urls`, `[repos]`, `[lint]` and `[context]`
- every rule of `[rules]`. An answer replaces only the rule of its group: `evidence`, `budgets`, and `lint` for a language other than English.
- the test globs
- the `[budgets.<type>]` tables, unless the budget level changes
- `[conventions]`
- the section keys of each root (`required-sections`, `amendment-sections`, `history-sections`, `companion-sections`, `live-sections`), and its budget, profile and rules keys

`init` does not keep the comments of the old file. The preview shows the new file before `init` writes it.
