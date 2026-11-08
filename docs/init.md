# `spec-dna init`

`init` makes the configuration of a repository. It detects the facts of the repository, asks at most 8 questions with the detected defaults, and shows a preview. It writes only after a confirmation. It never edits a document. In an instruction file (`AGENTS.md`, `CLAUDE.md`), it changes only its own generated line.

```sh
spec-dna init                   # interactive on a terminal
spec-dna init --dry-run         # the preview only
spec-dna init --non-interactive # the detected defaults
spec-dna init --answers a.toml  # the answers of a file
spec-dna init --reconfigure     # the questions again, with the current values as defaults
```

Without a terminal (for example, when an agent runs `init`), `init` uses the detected defaults, as `--non-interactive` does. If `spec-dna.toml` exists, `init` updates only the agent adapters, as in 0.1, unless `--reconfigure` is set. Use this after an upgrade.

## Detection

`init` reads the file list of git (tracked files and untracked files that git does not ignore) and the Markdown files. It finds:

| Fact | How |
| --- | --- |
| Greenfield or brownfield | Brownfield if there are documents to map or 20 files or more |
| Document roots | The top-most directories that are mostly Markdown, or that have a documentation name (`docs`, `doc`, `documentation`, `wiki`, `handbook`) |
| Types of roots | A directory name: `adr` and `decisions` give `decision`, `specs` gives `spec`, `contracts` gives `contract`, `guides` and `guidelines` give `guide`, `runbooks` gives `runbook`, `incidents` and `postmortems` give `incident`, `rfcs`, `tasks` and `plans` give `change`, `research`, `briefs` and `archive` give `reference`. Agent directories (`.claude/skills`, `.claude/agents`, `.cursor/rules`) give `guide`. A root whose documents name another type in the front matter becomes mixed. |
| Id patterns | File names such as `ADR-012-cache.md`: `legacy-ids = ["ADR-{n}"]`. A directory of such files becomes a root. |
| Requirement keys | Bold keys such as `- **R-12** text` at the start of list items: `legacy-keys = ["R-{n}"]`, or `R-{n}*` if some keys have a letter suffix |
| Statuses | The front-matter status words of each root. A word that the type does not allow gets a `status-map` entry: `shipped` and `implemented` map to `active`, `approved` maps to `accepted` for a decision, and so on. An unknown word maps to the default status of the root. |
| Heading languages and aliases | The scripts of the headings. Known translations of `Decision Outcome` and `Amendments` (for example `Решение`, `Entscheidung`, `Nachtrag`) in decisions become `required-sections` and `amendment-sections`. |
| Exclusions | Vendored directories (`vendor`, `node_modules`, `third_party`), generated directories (`dist`, `build`, `generated`), files that say that a generator wrote them, sibling directories with a vendor prefix (`ext-*`), and the files of a decision root that are not decisions (a README without an outcome, `*-implementation-plan.md`) |
| Large reference directories | A directory with 100 Markdown files or more, or 1,000,000 characters or more, and no typed directory in it: a `reference` root |
| Required sections | If most documents of a type in a root do not have the required sections, the root removes the requirement |
| Budgets | The documents over the hard budget, at 3 levels |
| Stack and apps | Manifests such as `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml` (to depth 3) |
| Tests | The test globs that match files, for `covers:` tags |
| Agent files | `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.claude/skills`, `.cursor/rules` and others |
| CI and hooks | `.github/workflows`, `.gitlab-ci.yml` and other CI files; pre-commit, husky, lefthook |
| Default branch | `origin/HEAD`, else a local `main` or `master`, else the current branch |

Detection reads only. It uses no network.

## Questions

| # | Question | Default |
| --- | --- | --- |
| 1 | The profile: `lite`, `standard` or `strict` ([profiles.md](profiles.md)) | `standard` |
| 2 | The knowledge types of the project | The detected types; a new project gets spec, guide, decision, invariant, runbook, incident and reference |
| 3 | The evidence policy (`profile`, `off`, `warning`, `error`) and the link style (`covers`, `verified-by`, `both`) | `profile, both` |
| 4 | The normative language and the heading aliases: `en; outcome: Решение; amendments: Поправки` | The detected aliases |
| 5 | The mapping: each detected root, to accept (Enter), to drop (`drop`), or to edit (`type=spec status=active exclude=a/**,b.md`) | Accept |
| 6 | The budget level: `default`, `relaxed` (2 times), `tight` (half) or `off`, with the number of documents over the hard budget at each level | `default` |
| 7 | The agent adapters: `agents-md`, `claude-md`, `skill` or `none` | The present instruction files and the skill |
| 8 | The CI step (`github`, `gitlab`, `none`), the hook (`pre-commit`, `husky`, `lefthook`, `none`) and the base branch | The detected CI system, hook framework and default branch |

A language other than `en` sets `[rules] lint = "off"`, because the requirement lint reads English only. A type that is not chosen makes its detected roots `note` roots. A new project gets one mixed root (`docs` or `--root <dir>`): `spec-dna new` puts each type into its own namespace there, such as `decisions/<id>`, so that the ids of different types cannot clash.

## Preview and files

The preview shows the full `spec-dna.toml` and each file to create or change:

- `spec-dna.toml`;
- the generated line in `AGENTS.md` and `CLAUDE.md`, and the skill `.claude/skills/spec-dna/SKILL.md`;
- `.github/workflows/spec-dna.yml` (the job of [quickstart.md](quickstart.md#ci)), for `github`;
- a local hook at the end of `.pre-commit-config.yaml` (only if `repos:` is its last top-level key), or one line in `.husky/pre-commit`.

For GitLab CI and lefthook, the preview gives the text to add by hand. `init` checks that it can read the new `spec-dna.toml` before it writes a file.

## Answers file

An answers file is TOML. Each key is optional, and a missing key keeps its default.

```toml
profile = "standard"
types = ["spec", "decision", "invariant", "runbook", "incident", "reference"]
evidence = "warning"            # "profile", "off", "warning" or "error"
evidence-style = "both"         # "covers", "verified-by" or "both"
language = "en"
outcome-headings = ["Решение"]
amendment-headings = ["Поправки"]
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

`init --reconfigure` asks the questions again. The defaults are the current values: the profile, the types, the evidence rule and test globs, and the roots of `spec-dna.toml`. The keys that no question asks for stay: `install-source`, `self-urls`, `[repos]`, `[lint]`, `[context]`, the `[budgets.<type>]` tables (unless the budget level changes) and the section, budget, profile and rules keys of each root. The comments of the old file are lost. The preview shows the new file before `init` writes it.
