# Changelog

This file lists the changes of each version. The project uses [Semantic Versioning](https://semver.org). Diagnostic codes are in [docs/diagnostics.md](docs/diagnostics.md).

## 0.3.1 - 2026-11-11

### Changed

- `mv` of a section leaves the heading of the section with a one-line pointer to its new place, for every destination type. Before, only a move into a `reference` document left a pointer. A review of 10 splits on a real corpus found that a reader of the old document, and prose references such as "see section 1" or "(Releases, below)", lost the moved text.

## 0.3.0 - 2026-11-11

Hygiene functions: split, history extraction, generated indexes and stubs for every document type, and lower cost of code tasks.

### Added

- `mv <doc-id>#<section>... <to>` moves sections out of a document of any type (guide, runbook, decision, incident, invariant, note, spec, contract, change, reference), into a new or an existing document of any type. `--type <type>` sets the type of a new destination. Each moved section keeps its old id as an alias (`<old-id>#<anchor>` in the `aliases` of the destination), and links follow it. The preservation check also requires that every moved section resolves at its old id.
- A move into a `reference` document leaves a one-line pointer in place of each moved section, and the new document links back to its source.
- Decisions: only the amendment sections and the history sections move, and only into a `reference` document. The frozen text stays. `mv` lists the moved amendments that state a requirement, a BCP 14 keyword or a status word as candidates for a superseding decision. It never writes a decision.
- `companion-sections` per root and type: the sections that belong to the requirements, such as an acceptance map. When requirements move, their table rows and list items move with them. A row of a key that moves and a key that stays refuses the move, with that reason.
- `history-sections` per root and type, and `spec-dna history <doc-id> [<to>]`: moves the history sections (and the amendment sections of a decision) into a reference document. In a decision, a history section is not frozen text.
- `spec-dna index <dir> [--replace]`: writes a generated index of the documents of a directory (default `<dir>/README.md`), marked by `generated: spec-dna index h:<hash>`. With `--replace`, it replaces a hand-kept registry, and links to a section of the old file that names an entry go to the entry. SD610 (warning): a stale or hand-edited index.
- `mv <from> <to> --stub` leaves a stub at the old path (front matter `moved-to`) for tools that read a file by its path. A stub has no id and no budget.
- SD503 (error): a change edits `spec-dna.toml` together with documents or code. A configuration change must be a change of its own.
- `[budgets] growth` (default 4,000 characters).
- A new document that a split moved out of a document over its hard budget gives the warning SD704, not SD702: it holds no new text.

### Changed

- A document that was over its hard budget in the base and grew by at most `[budgets] growth` characters gives the warning SD703, not the error SD702. In the code tasks of a real corpus, the error made agents split or shorten unrelated documents.
- The skill: do not split, move or shorten a document for a size warning; for a size error, put the new text into a new document or move whole sections with one `mv`; do not edit `spec-dna.toml`; run `check` once, or the project target that runs it. The fix texts of SD701 to SD704 say the same.
- The ids of a root of type `reference` start with a namespace from the root path, so two reference roots never collide (D11-1). The id of 0.2 stays a fallback name when it is unique.

## 0.2.1 - 2026-11-08

Fewer agent requests and a better detection of common layouts.

### Added

- `spec-dna <command> --help` and `spec-dna help <command>` print the help of one command only.
- `init` detects more layouts: a top-level type directory such as `rfcs/`, `adr/` or `runbooks/` keeps its type. New type words: `validation` and `validations` (`reference`), `incident` and `postmortem` (`incident`); a directory with one of these names must be in a documentation tree or be mostly Markdown. Decision files such as `0001-use-postgres.md` (adr-tools, `doc/adr`) give `{ alias = "ADR-{n}", file = "{n}-*" }`, if each number of the root is unique, no file of the root starts with a date, and no other root makes the same alias.

### Changed

- A directory of numbered files takes its type from its own files: the prefix (`ADR`, `RFC`, `INC`), the front matter `type` of most files, or their content (`Decision`, or `Context` and `Consequences`; `Root cause` as a heading or a table field, or `Timeline` and `Impact`; `Steps`; requirements with a BCP 14 keyword). If the files show no type, the type is the type of a typed enclosing root, or `note` in a mixed root. Before, it took the default type of the enclosing root. A mixed tree takes the type of its numbered files only if it has no other documents. Files numbered without a prefix (`0001-*.md`) make a root only for decisions and changes.
- The skill: split a document only for a size error (SD702, and in the `strict` profile also SD701, SD703 and SD704). A size warning needs no split, and a split document needs no second split. The fix texts of SD701 and SD703 no longer ask for a split.

## 0.2.0 - 2026-11-08

Profiles, 4 knowledge types, requirement evidence and an interactive `init`. The goal: a growing corpus stays orderly, so agents stay fast and cheap.

### Added

- `profile = "lite" | "standard" | "strict"` sets the default severity of each code. `[rules]` and the `rules` of a root override it by rule group (`references`, `ids`, `budgets`, `drift`, `evidence`, `lint`) or by code. A root can have its own `profile`. [docs/profiles.md](docs/profiles.md) gives the table.
- 4 document types: `invariant` (project-wide rules with stable ids and an `applies-to` scope; `context` serves a matching invariant first), `runbook`, `incident` (it must link the requirements, decisions or runbooks that it involves: SD609) and `reference` (cold material: no budget, and `context` never serves it unless a target names it). Each type has a template, required sections and budgets. [docs/types.md](docs/types.md).
- `spec-dna new <type> <id>` writes a document from the template of its type. In a mixed root, each type gets its own namespace. SD306 (error): a template placeholder (`{{text}}`) is still in a document.
- `types` in `spec-dna.toml`: the document types of the project. A document of another type is an error (SD303).
- `required-sections` accepts a list of lists: several required sections for a type.
- Evidence: a requirement links to tests by `verified-by` (front matter, or an inline `<!-- verified-by: <path> -->` marker) or by a `covers: <doc-id>#<key>` tag in a test file of any language (`[evidence] tests`). SD105 (error): a tag names no requirement. SD608 (by profile): an active requirement has no evidence. `show` lists the evidence of each requirement.
- `init` detects the repository and asks at most 8 questions with the detected defaults: the profile, the knowledge types, the evidence policy and link style, the language and heading aliases, the mapping of each detected root, the budget level, the agent adapters, and the CI step, hook and base branch. It shows the full `spec-dna.toml` and the files to create or change, and writes them only after a confirmation. Modes: `--non-interactive`, `--answers <file>`, `--reconfigure`, `--dry-run`. It can write a GitHub Actions job and a pre-commit or husky hook. [docs/init.md](docs/init.md).

### Changed

- A configuration without `profile` uses `standard`. In it, the requirement lint is off (0.1: warnings) and SD608 is a warning. `[rules] lint = "warning"` and `[rules] evidence = "off"` keep the behavior of 0.1.
- `lint-severity` and `drift-severity` of a root are now the rules `lint` and `drift` of that root.
- The line in `AGENTS.md` and `CLAUDE.md` and the skill ask for one `context` call for a task that edits code or docs, and none for a question. The skill names `new` and `covers:`, and its body has 1,863 characters (0.1.2: 2,392).
- The text output of `check` gives the fix of each listed code once.
- The default `context` budget is 8,000 characters (was 12,000).
- The parser reads the inline `verified-by` marker. The index format changed (parser version 9), so the first run parses all documents again.

## 0.1.2 - 2026-11-08

Fixes of the defects that a playground product of 12 agent tasks found in 0.1.1.

### Fixed

- `mv` of requirements: a list item can end after a blank line. A cut or an insert at that end joined the next paragraph to a moved or a remaining item, so the preservation check refused a valid move. The move now cuts and inserts whole lines without the blank lines after the item.
- `mv` of requirements: the preservation check compares the requirement text without link destinations, because the move rewrites the links to a moved key or section. The index format changed (parser version 8), so the first run parses all documents again.
- A refused move names each requirement whose text would change, and gives the text before and after the first difference, or says that the requirement would be lost or new.
- `mv` of requirements records an alias only for an id of `HEAD`. A key that is new in the uncommitted change gets no alias.
- SD606 and SD607 on the frozen text of an accepted decision: an amendment section that cites the new path or identifier clears them for that decision. Before, no valid change could clear them, except a supersede.

### Changed

- The text output of `check` lists the errors first, then the drift warnings (SD606, SD607), then the other warnings. It gives one line with a count for each prose lint code (SD801 to SD805). `--lint` lists each of these warnings. The JSON output did not change.
- The skill tells agents when to amend an accepted decision (the outcome stays) and when to supersede it (the outcome changes), and to name new code paths of an accepted decision in its amendment section.

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
- The message of a version mismatch says "move the project to", for both directions.
- The skill tells agents to split a document with one `mv`.

## 0.1.0 - 2026-11-08

A local tag for the first demo. It was not published.

- `context <code path>` serves the sections that cite the file: by path, `path:line`, link, directory or identifier.
- SD606 and SD607 (warnings): a change removes or renames a path or an identifier that a document cites.
- `init` writes one line into each root instruction file, never an import, and prints the bytes that it adds to the prompt prefix. It does not run when the pin differs from the binary.
- `transcript` counts the spec-dna commands of an agent.
