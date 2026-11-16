---
status: accepted
---

# 0014. Import from Spec Kit and OpenSpec

## Context and Problem Statement

A project that uses Spec Kit or OpenSpec keeps its specs in the layout of that tool. To adopt spec-dna, the team moves each file by hand and rewrites each requirement. Nothing then proves that each sentence and each id stayed. How does such a project move into the layout of spec-dna in one reviewable change, with a proof?

Constraints:

- [ADR 0001](0001-information-model.md): ids and hashes stay, and a gate needs only spec-dna, git and 2 trees. spec-dna uses no network.
- [ADR 0005](0005-layout-and-layers.md): the current truth has one place, and the place gives the type. The layer `research` has `context = "on-request"`.
- [ADR 0006](0006-change-lifecycle.md), "Rejected", option 1: a finished change does not stay as a document. An old plan is a search hit next to the current truth.
- [ADR 0009](0009-upgrade-from-0-5.md) gives the model of a command that moves a project: a clean tree, a plan in memory, a proof, `git mv` and no commit.
- spec-dna copies no text of the 2 tools. Rule zero: the fixtures use the public layouts only.

Evidence:

- The sources, from the research of 2026-11-16 ([design, section 1](../design/0.7.0.md#the-3-tools-at-their-current-versions)):
  - Spec Kit v1.4.1, 2026-11-16, MIT. The rules of a project are in `.specify/memory/constitution.md`. Each feature has a folder `specs/NNN-name/`. A requirement line starts with `- **FR-001**:`. A task has the form `- [ ] T010 [P] [US1] text in path`. The marker `[NEEDS CLARIFICATION]` marks an open question.
  - OpenSpec v2.0.0, 2026-11-11, MIT. The current truth is in `openspec/specs/<domain>/spec.md`, with the headings `### Requirement:` and `#### Scenario:`. A change is a folder `openspec/changes/<name>/` with `proposal.md`, `design.md`, `tasks.md` and `specs/`. A delta spec has the headings `## ADDED|MODIFIED|REMOVED Requirements`. `archive` moves a change to `changes/archive/YYYY-MM-DD-<name>/`. The project files are `config.yaml` and the legacy `project.md`.
  - OpenSpec documents `RENAMED` (the lines `FROM` and `TO`).
- The proof of `upgrade` compares hashes, because the text keeps its form. An import changes the form: OpenSpec puts a requirement in a heading, and spec-dna puts it in a list item.
- Agents search the repository ([evidence of earlier versions](../design/0.7.0.md#evidence-from-earlier-versions)). 2 copies of one truth give 2 search hits.

## Considered Options

1. A copy-only import with no proof.
2. An import by an agent skill or a language model.
3. Keep both trees, and link the documents of spec-dna to the source files.
4. `init --from <tool>`.
5. `spec-dna import`, a command of the binary. It builds a plan in memory, proves it, and moves the files with `git mv`.
6. A two-way export between spec-dna and each tool.

## Decision Outcome

Option 5. The design holds the tables: [the command](../design/0.7.0.md#d21-the-command), [the proof](../design/0.7.0.md#d22-the-proof), [Spec Kit](../design/0.7.0.md#d23-spec-kit-141) and [OpenSpec](../design/0.7.0.md#d24-openspec-200). It also holds [an unknown layout](../design/0.7.0.md#d25-an-unknown-layout-stops-the-import), [the archive](../design/0.7.0.md#d26-the-archive-and-the-other-files) and [the files that it does not read](../design/0.7.0.md#d27-what-the-import-does-not-read). This ADR gives the rules and their reasons.

### Command

```text
spec-dna import speckit|openspec [--dry-run] [--skip-open] [--areas <file>] [--keep none|research] [--json]
```

- `import` needs a project that `init` made, else it stops with exit code 3. Thus the pin, the roots and the layers exist before the first move.
- It refuses a tree that is not clean, also with `--dry-run`, as `upgrade` does. Thus `git reset --hard` and `git clean -fd` always restore the tree.
- It moves a document with `git mv`, so git keeps its history. It never deletes a file and never commits, because the author must read the diff first.
- The exit codes are these:
  - 0: a write, or nothing to change.
  - 1: a tree that is not clean, a stop or a failed proof.
  - 2: a usage error.
  - 3: an unknown layout, no project or a bad file.
  - 4: a git or write error. The message gives the commands that restore the tree.
- `--dry-run` builds the same plan and runs the same proof as a write, and it writes nothing.
- `--areas` has the form of `upgrade --layout --areas`: a TOML table `[areas]` maps a source name to an area folder. Only a person can choose an area.
- If the plan needs a change of `spec-dna.toml`, the first run writes only that file. Such a change is a legacy key pattern for `FR-nnn`, an area or a type. SD503 rejects it together with documents. The person commits it in a pull request of its own and runs `import` again. The second run writes the documents.
- An open feature or an open change becomes a draft `intent.md`, `spec.md` and `plan.md` in one diff. The base has no approval of them, so `check` gives SD910, and `gate = "block"` fails the pull request. `--skip-open` leaves the open changes in place and imports the rules and the current truth only. The report lists the SD910 findings.
- `--keep research` needs the layer `research`. If the graph has none, the command stops with exit code 2 and names `spec-dna layer add research`.

### Proof

The proof states what it proves. It does not claim byte equality, because OpenSpec moves a requirement from a heading to a list item. The write stops with exit code 1 if one of these 5 steps fails:

1. Each requirement of a source spec has one target requirement, or one target delta line for a change, and their normalized sentences are equal. The normal form removes the markup of the heading or the bold key, and it joins the white space. The report lists each pair: the old place and the new id.
2. Each source file has exactly one state in the report: moved (`git mv`), written, left in place, or skipped. A written file is a new file that holds the text of the source. The source stays, and the report prints the `git rm` command for it. A skipped file names its reason.
3. No id of the project before the import disappears.
4. `check --all` on the new tree gives no new error of the codes SD101 to SD106, SD201 to SD204, SD301 to SD306 and SD401 to SD405 (references, ids, front matter, statuses). The proof ignores the codes of evidence, budgets and changes, because the imported text has no `covers:` tag and no intent yet. The report lists them.
5. A second run writes no file and moves no file. The files that the report lists as left in place stay.

The hash of a requirement covers its sentence only. Thus no hash covers a scenario. The proof counts the scenario headings of the source and of the target, and they must be equal.

### Spec Kit

- The constitution becomes `rules/constitution.md`, a `rule` document. Each principle heading becomes a requirement key. The import skips a file that still has the template placeholders.
- A feature with a `tasks.md` and no open task is done. It becomes `requirements/<name>.md`, its `FR-nnn` lines become requirements through a legacy key pattern, and `NNN-<name>` is an alias.
- A feature with an open task, or with no `tasks.md`, becomes an intent folder: `intent.md` (status `draft`, class `higher-risk`), `spec.md` and `plan.md`. The user stories become the intent. Each `FR-nnn` line becomes a delta line `ADD requirements/<name>#fr-nnn`.
- A task `T001 [P] [US1] text in path` becomes `- [ ] t001: text. Files: path.` The box `[x]` stays. The source gives no check for a task, so the task has no `Verify:` label, and SD923 reports it after the approval.
- A marker `[NEEDS CLARIFICATION: text]` is inside an `FR-nnn` line. It becomes an item of `Open questions` in the draft `spec.md`, with the key of the line. SD922 then holds the approval until a person answers.
- The `tasks.md` of a done feature stays in place, and the report lists it.

### OpenSpec

- `openspec/specs/<domain>/spec.md` becomes `requirements/<domain>.md`, and `<domain>` is an alias. Each `### Requirement: <Name>` becomes one requirement: the key is the slug of the name, and the scenarios stay below it as a nested list. In a delta, a line has no nested list, so the scenarios go to a section `Scenarios` after `Delta`, grouped by key.
- A change becomes an intent folder. `proposal.md` gives `intent.md`, `design.md` gives the section `Design` of `plan.md`, and `tasks.md` gives `Order`. The class is `higher-risk` if the change has delta specs.
- In a delta, ADDED becomes `ADD`, MODIFIED becomes `CHANGE`, and REMOVED becomes `REMOVE`. `CHANGE` and `REMOVE` get the hash of the imported current text. The hash fixes the text that the author read (ADR 0006), and here that text is the imported requirement.
- A delta line with no base requirement keeps the whole change in place, with a report line. A part of a change would give a delta that no person wrote.
- By default, the import does not move an archived change, because the specs already hold its final text.

### Unknown layout

`import` stops with exit code 3 and writes nothing in 3 cases. The source folder is absent, the import finds no requirement, or the layout belongs to another major version. The message names the file, the failed rule and the supported versions. A guess moves files to a wrong place, and the proof cannot see a file that the plan does not read.

### Archive and other files

ADR 0006, "Rejected", option 1, rejected a status that keeps finished changes, because an old plan is a search hit next to the current truth. The import honors this rejection by default. With `--keep none`, no archived change and no extra document of a feature becomes a document of spec-dna. They stay where they are, and the report counts them.

`import` never deletes. Thus with `--keep none` too, these files stay search hits until a person deletes them, and git keeps them. The default honors ADR 0006 only in one sense: no old plan enters the corpus, so `check`, `context` and the ids never read it.

`--keep research` is not the default. It moves each archived change or extra document to `research/imported/`, as one `research` document. The layer `research` is on request, so `context` never serves it. The files stay search hits.

### Not read

The delta of spec-dna has no rename verb. `RENAMED` of OpenSpec would need a `REMOVE` and an `ADD`, which change a key with no proof of the references. So a change with `RENAMED` stays in place, with a report line. The import does not read `.openspec.yaml`, `openspec/schemas/` or the extensions of Spec Kit. The tool files also stay in place: the `.specify/` scripts and templates, `init-options.json`, `config.yaml` and `project.md`.

### Rejected

- Option 1, a copy with no proof: 2 copies give 2 search hits, and nothing shows that a sentence or an id survived. A copy also loses the history that `git mv` keeps.
- Option 2, an agent skill or a language model: a model can change a sentence, and no check can prove the result.
- Option 3, both trees with links: 2 truths are 2 search hits. The source tool keeps its own tree, so the trees drift, and `check` reads only one of them.
- Option 4, `init --from <tool>`: `init` has 14 questions already. `init` never edits a document (ADR 0001, amendment of 0.2.0), and an import moves each document. ADR 0009 rejected the same mix for `upgrade`.
- Option 6, a two-way export: it needs a writer for each version of each tool, and these tools change their files often. An export also keeps 2 truths alive.
- A proof of byte equality: OpenSpec moves a requirement from a heading to a list item, so the bytes cannot stay.
- `--keep research` as the default: it puts old plans into the corpus, against ADR 0006.
- A `REMOVE` and an `ADD` for `RENAMED`: the key changes, and no proof covers the references to it.
- A write before the proof, an automatic commit, or a deletion of the source files: the reasons of ADR 0009 apply.

### Consequences

- Good: one command and one diff move a project, and a second run changes nothing.
- Good: `git mv` keeps the history of each moved file. By default, no archive enters the corpus.
- Bad: "done" is a heuristic for Spec Kit. A feature with each task checked but not shipped becomes current truth. A finished feature with no `tasks.md` becomes an intent.
- Bad: Spec Kit has no merged view of the truth. A later feature can change a requirement of an earlier one, and each done feature becomes its own document. The proof checks sentences, so it cannot see a contradiction. The report says so, and a person reviews the documents.
- Bad: an OpenSpec requirement with scenarios becomes one long list item. The hash of a requirement covers its sentence and not a nested list (ADR 0001). Thus the proof counts the scenario headings only.
- Bad: MODIFIED needs its base requirement. A delta with no base keeps the whole change in place, and a person moves it.
- Bad: the parse of `text in path` is a heuristic, and the marks `[P]` and `[US1]` have no place in the task grammar. A person writes the check of each task.
- Bad: the fixtures use the public layouts at the versions above, and no real project proves these rules yet. A new major version of a tool gives exit code 3 until a release supports it.
- Bad: the first run can write only `spec-dna.toml`, so an import takes 2 runs and 2 pull requests when the configuration needs a change.
- Neutral: the import adds a command and a `--json` shape. It adds no type, no diagnostic code and no key.

## Amends

This ADR amends no accepted text. ADR 0001, ADR 0005, ADR 0006 and ADR 0009 stay as they are. The import reuses `git mv`, the proof pattern of ADR 0009 and the layer `research` of ADR 0005. Its default honors ADR 0006, "Rejected", option 1.

It adds a command, so the change sets "Command, option, help text or exit code" and "`--json` shape" of `.claude/agents/ownership.md` apply.

## Amendments

### 2026-11-16: drafts

A `draft` artifact needs no approval (ADR 0006, amendment of 2026-11-16). Thus an imported open change gives no SD910, and `--skip-open` only leaves it in place.
