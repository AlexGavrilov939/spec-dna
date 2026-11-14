# `spec-dna init`

`init` makes the configuration of a repository. It detects the facts of the repository. It asks at most 10 questions with the detected defaults, and it shows a preview. It writes only after a confirmation. It never edits a document. In an instruction file (`AGENTS.md`, `CLAUDE.md`), it changes only its own generated lines: the spec-dna line, and the model-team line of the [agent harness](harness.md).

```sh
spec-dna init                   # interactive on a terminal
spec-dna init --dry-run         # the preview only
spec-dna init --non-interactive # the detected defaults
spec-dna init --answers a.toml  # the answers of a file
spec-dna init --reconfigure     # the questions again, with the current values as defaults
```

Without a terminal (for example, when an agent runs `init`), `init` uses the detected defaults, as `--non-interactive` does. If `spec-dna.toml` exists and you do not set `--reconfigure`, `init` updates the agent adapters, as in 0.1. If the file has a `[harness]` table, it also updates the harness files ([Upgrade](#upgrade)). Run `init` in this way after an upgrade.

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
| Harness defaults | The `model` and the `advisorModel` of an existing `.claude/settings.json` become the main model and the advisor. `init` adds `NotebookEdit` to `deny` if git tracks no `*.ipynb` file. A file in `.claude/agents` that is not generated and has the name of a role file makes that role `none`. The name is the `name` of the front matter, or the file name |

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
| 9 | The agent harness: `none`, `lean` or `full` ([harness.md](harness.md)). The question says what each level installs, and that the files are for Claude Code | `lean`. In `--reconfigure`, the current level; for a configuration with no `[harness]` table, `lean` |
| 10 | The models: `role=model[:effort]` words, such as `reviewer=sonnet:medium`. `advisor=<model>/<agent model>[:effort]` sets the advisor and the model of `fallback-advisor`. `none` disables a role. The question is not asked if the level is `none` | `orchestrator=opus:xhigh advisor=fable reviewer=opus:high researcher=haiku:high helper=haiku`, and for `full` also `implementer=sonnet:high spec-writer=sonnet:high`. The detected values replace the main model and the advisor |

A language other than `en` sets `[rules] lint = "off"`, because the requirement lint reads English only. A type that you do not choose makes its detected roots `note` roots. A new project gets one mixed root (`docs`, or the directory of `--root <dir>`). In this root, `spec-dna new` puts each type into its own namespace, such as `decisions/<id>`. Thus the ids of different types cannot clash.

## Preview and files

The preview shows the full `spec-dna.toml` and each file to create or change:

- `spec-dna.toml`
- the generated line in `AGENTS.md` and `CLAUDE.md`, and the skill `.claude/skills/spec-dna/SKILL.md`
- for `github`: `.github/workflows/spec-dna.yml` (the job of [quickstart.md](quickstart.md#ci)). If the file exists, `init` does not change it.
- a local hook at the end of `.pre-commit-config.yaml` (only if `repos:` is its last top-level key), or one line in `.husky/pre-commit`

- the harness files, if the level is not `none`: `.claude/agents/*.md`, `.claude/agents/model-team.md`, the model-team line in `CLAUDE.md` and `.claude/settings.json`. The harness block of the preview shows each file with `create`, `change` or `remove` and a note. It also shows the warnings, the reasons for disabled roles and the line `Each teammate needs spec-dna 0.5.0 or later on PATH for the hooks.` It shows the bytes that the harness adds to each request of the main session. After a write, `init` tells you to restart Claude Code.

For GitLab CI and lefthook, the preview gives the text to add manually. `init` checks that it can read the new `spec-dna.toml` before it writes a file. `--json` has a `harness` object with the level, the files (path, action and note), the kept notes, the warnings and the bytes.

`init` writes the harness files as [harness.md](harness.md) says. It changes `.claude/settings.json` by a merge, and it keeps each key that it does not set. It does not replace a file that it did not generate.

`init` refuses the harness and exits with code 2 in these cases:

- git ignores `.claude/agents` or `.claude/settings.json`. A team cannot commit the files.
- An answer sets a role to a model, and a file that `init` did not generate has the agent name of that role. Set the role to `none`, or rename the agent.
- `.claude/settings.json` is not valid JSON.

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
harness = "lean"                # "none", "lean" or "full"
harness-models = ["orchestrator=opus:xhigh", "reviewer=sonnet:medium", "advisor=fable/opus"]
harness-deny = ["NotebookEdit"] # tools that no agent gets; replaces the default
write = true                    # false: the preview only

[[roots]]                       # optional: these roots replace the mapping
path = "docs"
type = "mixed"
default-type = "spec"
```

The `[[roots]]` tables use the keys of `spec-dna.toml` ([config.md](config.md)).

`harness-models` has one `role=model[:effort]` word for each role that you change. A role that is not in the list keeps its default. A role that the level does not include is an error. The model `none` disables a role, and `inherit` writes no model. A suffix after `:` that has only letters, or none, must be an effort. A suffix with a digit, such as `:0` in a cloud model id, stays part of the model. The advisor model cannot be haiku.

## Reconfigure

`init --reconfigure` asks the questions again. The defaults are the current values: the profile, the types, the evidence rule and test globs, and the roots of `spec-dna.toml`. `init` does not change the keys that no question asks for:

- `install-source`, `self-urls`, `[repos]`, `[lint]` and `[context]`
- every rule of `[rules]`. An answer replaces only the rule of its group: `evidence`, `budgets`, and `lint` for a language other than English.
- the test globs
- the `[budgets.<type>]` tables, unless the budget level changes
- `[conventions]`
- the section keys of each root (`required-sections`, `amendment-sections`, `history-sections`, `companion-sections`, `live-sections`), and its budget, profile and rules keys

The harness has the current `[harness]` table as its defaults. A smaller level removes the generated files that it no longer produces, if you did not edit them. It removes the model-team line and the hook handlers too. It keeps each key of `.claude/settings.json` that the new configuration no longer sets, and it prints one `kept` line for each key. Remove these keys by hand.

`init` does not keep the comments of the old file. The preview shows the new file before `init` writes it.

## Upgrade

`init` with an existing `spec-dna.toml` and no `--reconfigure` is the upgrade path:

- A configuration with no `[harness]` table (spec-dna 0.4 or older) gets no harness. Run `init --reconfigure` to add it.
- `init` regenerates the harness files of a configuration with a `[harness]` table. It writes each generated file that you did not edit, and each file that is missing. It changes the model-team line and the settings entries that differ. It prints each path that it writes.
- `init` keeps a generated file that you edited, or that has text after its marker, and it prints `kept <path>: edited by hand (SD501)`. `check` reports the file. Delete it and run `init` to generate it again.
- `init` never removes a file on this path. Only `--reconfigure` removes files.
