---
status: approved
---
# Import of a Spec Kit or OpenSpec project: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the command `import` and its `--json` shape. The rules are in [the design](../../design/0.7.0.md), D21 to D27 and section 15, and the docs of the build state them.

## Interfaces

None. No interface document describes the CLI. The new surface is this.

`spec-dna import speckit|openspec [--dry-run] [--skip-open] [--areas <file>] [--keep none|research] [--json]`. Another source, or another value of `--keep`, is exit code 2. The exit codes are the 5 codes of D21. A missing layer for a target type is exit code 2, and the message names `spec-dna layer add`.

The text output has one line for each source file: `moved <from> -> <to>`, `wrote <to> (from <sources>)`, `kept <path>: <reason>` or `skipped <path>: <reason>`. `--dry-run` writes `would move` and `would write`. Then come one line for each pair, the proof line, the report, `to commit: git add -A -- <paths>` and `to remove the sources: git rm -r -- <paths>`. The output has no diff. A run with nothing to write prints `spec-dna import: nothing to change`.

`--json` prints one object: `version`, `source` (`speckit` or `openspec`), `dry_run`, `config_only`, `sources`, `pairs`, `proof` and `report`. An item of `sources` has `path`, `state` (`moved`, `written`, `kept` or `skipped`), `to` (a list) and `reason`. An item of `pairs` has `from` (`<path>:<line>`), `id` and `delta` (the slug of the change, or `null`). `proof` has `ok`, `requirements`, `scenarios`, `ids` and `failures`. `report` has `errors` (`code` and `count`), `outside_links` and `notes`. A failed proof prints the object and gives exit code 1.

## Decisions

- [[adr/0014-import]]
- [[adr/0009-upgrade-from-0-5]]
- [[adr/0006-change-lifecycle]]
- [[adr/0005-layout-and-layers]]

## Concerns

- [x] Where does a target go? Resolved: to the folder of the layer that holds its type, in the first root that has that layer. A type that the `types` list lacks is a change of `spec-dna.toml` (D21).
- [x] Which source moves, and which is written? Resolved: a source that gives one target moves with `git mv`, then the plan edits it. These are the constitution, a done feature and a domain spec. A moved document keeps its old id as an alias. Each file of an open feature or an open change is the source of a written target, and it stays.
- [x] Which status and aliases does the current truth get? Resolved: `status: active`, the aliases of D23 and D24, and the old id of a moved document.
- [x] A moved file has relative links. Resolved: the plan edits each link so that it goes to the same file, as `mv` and `upgrade` do. A link of a file that is not a document stays, and the report counts it. A link that is broken in the source gives SD102, and the proof stops.
- [x] The constitution needs `## Rules` (SD305). Resolved: `Core Principles` becomes `Rules`. The key of a principle is the slug of its heading with no leading numeral. Other text stays. A file with no `Core Principles` is skipped.
- [x] When is a feature done? Resolved: its `tasks.md` has 1 task or more, and no open task. A task line is a box, then `T` and digits.
- [x] What does an open feature give? Resolved: `intent.md` holds the user stories in `Intent` and the other sections as sub-sections. `spec.md` holds the `ADD` lines and an open question for each marker. `plan.md` lists the paths of the tasks in `Files`, or `- none`. The path of a task is the token after the last ` in ` that has a `/` or a `.`.
- [x] An `SC-nnn` or entity line with a BCP 14 word gives SD203. Resolved: the import changes no sentence. The proof stops, and the message names the line and the 2 fixes.
- [x] With `--skip-open`, an open feature can still stop the import (D25). Resolved: `--skip-open` does not read an open change.
- [x] What does OpenSpec give? Resolved: `Why` becomes `Why now`. `What Changes` and `Impact` become `Intent`, and another heading becomes a sub-section. A task `1.1` becomes `t1-1`. A delta block becomes one paragraph. `Scenarios` also holds the text of a `REMOVED` block and a `Purpose`. The heading `## RENAMED Requirements` keeps the change in place.
- [x] What does `--keep research` move? Resolved: each Markdown file, as one `research` document, to `imported/<source folder>/` under the folder of the research layer. A file that is not Markdown stays.
- [x] What is a key of `--areas`? Resolved: the name of the source folder. The plan of the configuration stops a pair of areas that differ only in case with exit code 1. It uses the compare of D30 (change `limits-061`).
- [x] How does the proof show step 5? Resolved: it plans again on the planned tree in memory, and the plan must be empty. On a second run, a target with the planned text is no write. A target with other text stops the plan.
- [x] The fixtures must hold no text of the 2 tools. Resolved: a test helper writes each project from text in the words of spec-dna. Only the paths, the headings and the line forms follow the public templates.

## Open questions

- none.

## Tests

- Spec Kit: the constitution, and a constitution with a placeholder. 2 done features, with the legacy key and the alias. An open feature with an inline marker. The extra files stay.
- OpenSpec: 2 domain specs with scenarios. A change with `ADDED`, `MODIFIED` and `REMOVED` and their hashes. A change with no base requirement, and a change with `RENAMED`, stay in place. The archive is counted.
- The proof: a changed sentence stops the write (a mutation test). A changed scenario count stops it. An `SC-001` line with `MUST` stops it.
- The command: no project (3), a tree that is not clean (1) and each case of D25 (3). `--keep research` with no layer (2), and with the layer. A pair of areas (1). The text output and `--json`. A dry run and a write give the same plan. The first run writes only `spec-dna.toml`. A second run changes nothing. With the profile `standard`, `gate = "block"` and an `origin/HEAD` base, `check` passes, with and without `--skip-open`. With `strict`, the report names SD608, SD917 and SD921.
