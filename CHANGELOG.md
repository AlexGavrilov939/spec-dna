# Changelog

This file lists the changes of each version. The project uses [Semantic Versioning](https://semver.org). Diagnostic codes are in [docs/diagnostics.md](docs/diagnostics.md).

## 0.5.0 - 2026-11-14

An optional agent harness for Claude Code: `init` installs a small team of models, and `check` keeps its files true ([ADR 0004](docs/adr/0004-harness.md), [docs/harness.md](docs/harness.md)).

### Added

- `[harness]` in `spec-dna.toml`: the level (`none`, `lean`, `full`), the model and the effort of each generic role (orchestrator, advisor, reviewer, researcher, implementer, spec-writer, helper), the denied tools and the hook values.
- `init` asks 2 more questions: the harness level and the models of the roles. The answers file has the keys `harness`, `harness-models` and `harness-deny`. A new configuration gets `lean`.
- Generated Claude Code files:
  - Agent files `.claude/agents/checker.md`, `Explore.md` (it replaces the built-in Explore) and `fallback-advisor.md`. The level `full` also has `implementer.md` and `spec-writer.md`.
  - The page `.claude/agents/model-team.md`.
  - A model-team line in `CLAUDE.md`.
  - Entries in `.claude/settings.json`: `model`, `modelSettings`, `fallbackModel`, `advisorModel`, `env.CLAUDE_CODE_SUBAGENT_MODEL`, `permissions.deny` and 2 hooks. spec-dna keeps all other keys of the settings file.
- `spec-dna hook context-meter`: tells an agent its context size once at each level.
- `spec-dna hook fanout-cap`: asks the person before a workflow whose agent count can exceed the cap.
- SD504 `harness-missing` and SD505 `harness-settings-invalid`. SD501 and SD502 also apply to the harness files.

### Upgrade notes

- A configuration of 0.4 or older has no `[harness]` table, so `init` adds no harness on upgrade. Run `spec-dna init --reconfigure` to add it.
- The hooks need spec-dna 0.5.0 or later on the `PATH` of each teammate. Without it, the hooks do nothing.

## 0.4.1 - 2026-11-14

One example domain, greenhouse irrigation, for the tests, the fixtures, the documentation and the texts for agents.

### Changed

- `init` no longer gives the type `reference` to the directory names `validation`, `validations` and `briefs`. It no longer excludes sibling directories with the prefix `ext-`. For such a directory, set the root type or an `exclude` glob in `spec-dna.toml`.
- The fix texts of SD405 and SD805 and the skill text of `init` use neutral examples. After the upgrade, run `spec-dna init` to update the adapters.
- The README says what spec-dna is, why a team needs it and how to use it in a project.

## 0.4.0 - 2026-11-13

The first public release: release binaries for 7 targets, with SHA-256 sums that a project pins in `spec-dna.toml` (phase 2 of ADR 0003).

### Added

- Release binaries for macOS (`aarch64-apple-darwin`, `x86_64-apple-darwin`), Linux (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`) and Windows (`x86_64-pc-windows-gnu`, `aarch64-pc-windows-gnullvm`). Each release also has `SHA256SUMS`, `spec-dna-pin.toml` and a build-provenance attestation for each file. A tag `v<version>` starts `.github/workflows/release.yml`. [docs/quickstart.md](docs/quickstart.md#install) gives the install and the verification commands.
- The table `[spec-dna-sha256]` in `spec-dna.toml`: the sums of the release binaries, copied from `spec-dna-pin.toml`. If the pin is the version of the binary, each command validates the table (exit code 3 for an error). `init --reconfigure` keeps the table.
- The CI job that `init` writes downloads the pinned Linux x86-64 binary and verifies its sum, if `spec-dna.toml` has that sum and no `install-source`. Otherwise, it builds the pin from source, as before.
- `SECURITY.md`, and [docs/evidence.md](docs/evidence.md): the method, the results and the limits of the agent experiments.

### Changed

- The message of a version mismatch gives the download URL and the sum of the pinned binary when the table has them. To move a project to the version of the binary, it names `spec-dna-pin.toml` of that release.
- If `spec-dna.toml` does not parse and its pin is a different version, `check` and `init` give the version mismatch, not the parse error. A configuration of a later version can have keys that an earlier binary does not know.
- CI builds the 7 targets on each push and pull request. It also runs each binary on a runner of its own platform, with these commands: `--version`, `init`, `check`, `show` and `context` (`scripts/smoke.sh`). Before, CI built `aarch64-apple-darwin` only.
- ADR 0003 has the design of phase 2. The Windows binaries keep the GNU ABI. There is no dispatch shim. A tool manager such as mise or aqua can keep several versions at the same time.
- The README is shorter: an example, the install, the start, the status and the links. [docs/commands.md](docs/commands.md) has the reference of each command. `CONTRIBUTING.md` has the repository layout, the setup and the dependencies.
- The documentation, the code comments, the schema descriptions and the text for agents follow ASD-STE100.
- Releases are immutable. After the publication, nobody can change the assets or the tag of a release.

### Upgrade notes

- A binary of a version before 0.4.0 does not know the table. For a configuration with the table, it reports ``unknown key `spec-dna-sha256` `` in place of the version mismatch. Install the pinned version.
- `init` does not change a CI job that exists, so the job of an earlier `init` still builds from source. To download the binary in CI, copy the job from [docs/quickstart.md](docs/quickstart.md#ci).

## 0.3.3 - 2026-11-13

Convention files outside the documents, and fixes from a review of the open defects.

### Added

- A pointer of `[conventions]` can name a Markdown file that is not a document. Such a file is outside every root, or an `exclude` glob removes it, for example the `README.md` of a decision directory. `context` serves its sections first in the convention tier (`"type": "file"` in `--json`). `show --conventions <type>` prints them. `show <path>#<anchor>` prints one section. No check, budget or id applies to the file. SD106 also reports a file that is absent.

### Changed

- `check` prints a warning code with more than 10 diagnostics as one count, as it does for the prose lint. On a mature corpus without test links, the SD608 lines no longer hide the other output. `--json` lists each diagnostic, and the exit code does not change.
- `scripts/cross-build.sh` names the zig version of CI when zig is absent. It also warns when another zig version is on `PATH`.

### Fixed

- `init --reconfigure` removed `[conventions]` and the root keys `history-sections`, `companion-sections` and `live-sections`. It now keeps them. If you ran `init --reconfigure` with 0.3.0 to 0.3.2, compare `spec-dna.toml` with its last commit.
- spec-dna reads a convention file of the working tree from disk on every run, without the parse cache. It also adds the file to the git status pathspec. Thus a changed file, or a symlink whose target changed, gives the current text. spec-dna does not read a symlink whose target is outside the repository (SD106).

## 0.3.2 - 2026-11-13

Live sections and project conventions. A replay of real changes of a corpus found 2 gaps. First, agents left the progress section of an accepted decision stale, because spec-dna froze that text. Second, agents did not see a convention for contract changes that only the project documents state.

### Added

- `live-sections` per root and type: the sections that record a current state, such as a progress section, in any language. In an accepted decision, a live section can change in place without SD405, and new text in it gives no SD605. The frozen text and the amendment sections keep their rules. [docs/config.md](docs/config.md), section "Sections of a decision", compares the 3 kinds of section.
- `[conventions]`: for each type, pointers to the sections of project documents that state the conventions of the project for documents of that type. `context` serves them first when a target is a document of that type. `show --conventions <type>` prints them, with the names of the amendment and live sections of the type.
- SD106 `convention-unresolved` (error, rule group `references`): a pointer of `conventions` names no document or section.

### Changed

- The skill tells agents to run `spec-dna show --conventions <type>` before they edit a document of that type. It also tells them to update a live section of an accepted decision in place when a change makes the section stale.
- The message and the fix of SD405 name the live sections.

## 0.3.1 - 2026-11-11

### Changed

- `mv` of a section leaves the heading of the section with a one-line pointer to its new place, for every destination type. Before, only a move into a `reference` document left a pointer. A review of 10 splits on a real corpus found a problem: a reader of the old document could not find the moved text. Prose references such as "see section 1" or "(Releases, below)" had the same problem.

## 0.3.0 - 2026-11-11

Hygiene functions: split, history extraction, generated indexes and stubs for every document type, and lower cost of code tasks.

### Added

- `mv <doc-id>#<section>... <to>` moves sections out of a document of any type into a new or an existing document of any type. `--type <type>` sets the type of a new destination. Each moved section keeps its old id as an alias (`<old-id>#<anchor>` in the `aliases` of the destination), and links follow it. The preservation check also requires that every moved section resolves at its old id.
- A move into a `reference` document leaves a one-line pointer in place of each moved section, and the new document links back to its source.
- Decisions: only the amendment sections and the history sections move, and only into a `reference` document. The frozen text stays. `mv` lists the moved amendments that state a requirement, a BCP 14 keyword or a status word as candidates for a superseding decision. It never writes a decision.
- `companion-sections` per root and type: the sections that belong to the requirements, such as a coverage table. When requirements move, their table rows and list items move with them. If a row has a key that moves and a key that stays, `mv` stops and gives that reason.
- `history-sections` per root and type. `spec-dna history <doc-id> [<to>]` moves the history sections (and the amendment sections of a decision) into a reference document. In a decision, a history section is not frozen text.
- `spec-dna index <dir> [--replace]` writes a generated index of the documents of a directory (default `<dir>/README.md`). The index has the mark `generated: spec-dna index h:<hash>`. With `--replace`, it replaces a hand-kept list, and links to a section of the old file that names an entry go to the entry. SD610 (warning): a stale index, or an index with a manual edit.
- `mv <from> <to> --stub` leaves a stub at the old path (front matter `moved-to`) for tools that read a file by its path. A stub has no id and no budget.
- SD503 (error): a change edits `spec-dna.toml` together with documents or code. A configuration change must be a change of its own.
- `[budgets] growth` (default 4,000 characters).
- A split can move text out of a document over its hard budget. The new document gives the warning SD704, not SD702, because it holds no new text.

### Changed

- A document can be over its hard budget in the base and grow by at most `[budgets] growth` characters. Then it gives the warning SD703, not the error SD702. In the code tasks of a real corpus, the error made agents split or shorten unrelated documents.
- The skill gives these rules, and the fix texts of SD701 to SD704 say the same:
  - Do not split, move or shorten a document for a size warning.
  - For a size error, put the new text into a new document, or move whole sections with one `mv`.
  - Do not edit `spec-dna.toml`.
  - Run `check` once, or run the project target that runs it.
- The ids of a root of type `reference` start with a namespace from the root path, so two reference roots never collide (D11-1). The id of 0.2 stays a fallback name when it is unique.

## 0.2.1 - 2026-11-08

Fewer agent requests and a better detection of common layouts.

### Added

- `spec-dna <command> --help` and `spec-dna help <command>` print the help of one command only.
- `init` detects more layouts. A top-level type directory such as `rfcs/`, `adr/` or `runbooks/` keeps its type. The new type words are `validation` and `validations` (`reference`), and `incident` and `postmortem` (`incident`). A directory with one of these names must be in a documentation tree, or it must be mostly Markdown. Decision files such as `0001-use-postgres.md` (adr-tools, `doc/adr`) give `{ alias = "ADR-{n}", file = "{n}-*" }`. These conditions apply:
  - each number of the root is unique;
  - no file of the root starts with a date;
  - no other root makes the same alias.

### Changed

- A directory of numbered files takes its type from its own files. These facts give the type:
  - the prefix (`ADR`, `RFC`, `INC`)
  - the front matter `type` of most files
  - the content of most files: `Decision`, or `Context` and `Consequences`; `Root cause` or `Timeline` and `Impact`; `Steps`; requirements with a BCP 14 keyword

  If the files show no type, the type is the type of the enclosing root, if that root has a type. In a mixed root, the type is `note`. Before, the directory took the default type of the enclosing root. A mixed tree takes the type of its numbered files only if it has no other documents. Files with a number and no prefix (`0001-*.md`) make a root only for decisions and changes.
- The skill: split a document only for a size error (SD702, and in the `strict` profile also SD701, SD703 and SD704). A size warning needs no split. A split document needs no second split. The fix texts of SD701 and SD703 no longer ask for a split.

## 0.2.0 - 2026-11-08

Profiles, 4 knowledge types, requirement evidence and an interactive `init`. The goal is that a corpus that gets larger stays consistent, so agents stay fast and cost little.

### Added

- `profile = "lite" | "standard" | "strict"` sets the default severity of each code. `[rules]` and the `rules` of a root override it by rule group (`references`, `ids`, `budgets`, `drift`, `evidence`, `lint`) or by code. A root can have its own `profile`. [docs/profiles.md](docs/profiles.md) gives the table.
- 4 document types. Each type has a template, required sections and budgets. [docs/types.md](docs/types.md) gives the details.
  - `invariant`: project-wide rules with stable ids and an `applies-to` scope. `context` serves a matching invariant first.
  - `runbook`: an operational procedure.
  - `incident`: it must link the requirements, decisions or runbooks that it involves (SD609).
  - `reference`: material that agents seldom read. It has no budget. `context` never serves it unless a target names it.
- `spec-dna new <type> <id>` writes a document from the template of its type. In a mixed root, each type gets its own namespace. SD306 (error): a template placeholder (`{{text}}`) is still in a document.
- `types` in `spec-dna.toml`: the document types of the project. A document of another type is an error (SD303).
- `required-sections` accepts a list of lists: several required sections for a type.
- Evidence: a requirement links to tests by `verified-by`, in the front matter or in an inline `<!-- verified-by: <path> -->` marker. A test file of any language (`[evidence] tests`) can also have a `covers: <doc-id>#<key>` tag. SD105 (error): a tag names no requirement. SD608 (by profile): an active requirement has no evidence. `show` lists the evidence of each requirement.
- `init` detects the repository and asks at most 8 questions with the detected defaults. The questions are about these items:
  - the profile, the knowledge types, and the evidence policy and link style;
  - the language and heading aliases, and the root map;
  - the budget level, the agent adapters, and the CI step, hook and base branch.
  It shows the full `spec-dna.toml` and the files to create or change, and writes them only after a confirmation. Modes: `--non-interactive`, `--answers <file>`, `--reconfigure`, `--dry-run`. It can write a GitHub Actions job and a pre-commit or husky hook. [docs/init.md](docs/init.md).

### Changed

- A configuration without `profile` uses `standard`. In it, the requirement lint is off (0.1: warnings) and SD608 is a warning. `[rules] lint = "warning"` and `[rules] evidence = "off"` keep the behavior of 0.1.
- `lint-severity` and `drift-severity` of a root are now the rules `lint` and `drift` of that root.
- The line in `AGENTS.md` and `CLAUDE.md` and the skill ask for one `context` call for a task that edits code or docs. A question needs no call. The skill names `new` and `covers:`, and its body has 1,863 characters (0.1.2: 2,392).
- The text output of `check` gives the fix of each listed code once.
- The default `context` budget is 8,000 characters (was 12,000).
- The parser reads the inline `verified-by` marker. The index format changed (parser version 9), so the first run parses all documents again.

## 0.1.2 - 2026-11-08

Fixes of the defects that a playground product of 12 agent tasks found in 0.1.1.

### Fixed

- `mv` of requirements: a list item can end after a blank line. A cut or an insert at that end joined the next paragraph to an item. Thus the preservation check stopped a valid move. The move now cuts and inserts whole lines without the blank lines after the item.
- `mv` of requirements: the preservation check compares the requirement text without link destinations, because the move rewrites the links to a moved key or section. The index format changed (parser version 8), so the first run parses all documents again.
- If the preservation check stops a move, it names each requirement whose text would change. It gives the text before and after the first difference, or it says that the requirement would be lost or new.
- `mv` of requirements records an alias only for an id of `HEAD`. A key that is new in the uncommitted change gets no alias.
- SD606 and SD607 on the frozen text of an accepted decision: an amendment section clears them if it cites the new path or identifier. Before, no valid change could clear them, except a superseding decision.

### Changed

- The text output of `check` lists the errors first, then the drift warnings (SD606, SD607), then the other warnings. It gives one line with a count for each prose lint code (SD801 to SD805). `--lint` lists each of these warnings. The JSON output did not change.
- The skill tells agents when to amend an accepted decision (the outcome stays) and when to supersede it (the outcome changes). It also tells them to name new code paths of an accepted decision in its amendment section.

## 0.1.1 - 2026-11-08

The first version for a consumer.

### Added

- `mv <doc-id>#<key or section>... <spec-id>` moves several requirements, or whole sections, into an existing spec or a new one. Each moved requirement keeps its old id as an alias, and links follow it. A preservation check runs before any file changes. If the check fails, `mv` changes nothing and exits with code 1.
- SD807 (warning): a list item starts with a bold slug key and has a BCP 14 keyword, but no colon follows the key.
- `install-source` in `spec-dna.toml`: the git source of spec-dna for the message of a version mismatch and for the CI job of the quickstart.
- `drift-severity` per root: SD606 and SD607 as `off`, `warning` (default) or `error`.
- `init` prints the `git add` command for the files that it wrote.

### Changed

- SD405 ignores link destinations. A move that rewrites a link in an accepted decision does not change its frozen text. The index format changed (parser version 7), so the first run parses all documents again.
- The message of a version mismatch says "move the project to", for an upgrade and for a downgrade.
- The skill tells agents to split a document with one `mv`.

## 0.1.0 - 2026-11-08

A local tag for the first demo. It was not published.

- `context <code path>` serves the sections that cite the file: by path, `path:line`, link, directory or identifier.
- SD606 and SD607 (warnings): a change removes or renames a path or an identifier that a document cites.
- `init` writes one line into each root instruction file, never an import, and prints the bytes that it adds to the prompt prefix. It does not run when the pin differs from the binary.
- `transcript` counts the spec-dna commands of an agent.
