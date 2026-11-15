# Commands

This page gives each command of spec-dna, its options and the exit codes. [quickstart.md](quickstart.md) gives the steps in the sequence of a new project.

## Summary

```sh
spec-dna init                       # detection, up to 14 questions, a preview, then spec-dna.toml, the adapters and the harness
spec-dna init --dry-run             # the preview only (also --non-interactive, --answers <file>, --reconfigure)
spec-dna new decision pump-choice   # a new document from the template of its type
spec-dna layer add interfaces --types interface --moved-from intent   # a new layer in spec-dna.toml
spec-dna layer list                 # each layer, its level and its documents
spec-dna context src/valves.rs      # the sections that cite a code file
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna check --lint               # also list each prose lint warning (any profile)
spec-dna check --staged             # check the git index, as a pre-commit hook does
spec-dna context irrigation/valves --path src/valves.rs
spec-dna context irrigation/valves --query "close a valve after a sensor timeout"
spec-dna show irrigation/valves#sensor-timeout
spec-dna mv irrigation/valves irrigation/valve-rules   # the old id becomes an alias
spec-dna mv irrigation/valves#sensor-timeout irrigation/sensors
spec-dna mv irrigation/valves#flush irrigation/valves#sensor-timeout irrigation/sensors   # several keys
spec-dna mv irrigation/valves#drip irrigation/drip-lines  # a whole section into a new spec
spec-dna mv guides/deploy#rollback runbooks/rollback --type runbook   # a section of a guide into a new runbook
spec-dna history decisions/cache docs/history/cache.md   # the amendment and history sections into a research document
spec-dna index docs/incidents --replace         # a generated index in place of a hand-kept list
spec-dna next dose-limit                        # the checklist to the next gate of a change
spec-dna status                                 # each change, its stage and its next gate
spec-dna status --group operations              # the changes of a group of areas
spec-dna log                                    # the finished and the closed changes, from the history of git
spec-dna log --since v0.6.0                     # only the changes that finished after a revision
spec-dna stats --sdlc                           # the indicators of the playbook, from git and the files
spec-dna stats --sdlc --group operations        # the same for the changes of a group of areas
spec-dna finish dose-limit --dry-run            # the diff of the finish of a change, and nothing written
spec-dna finish dose-limit                      # apply the delta, delete the folder, stage the paths
spec-dna mv guides/old guides/new --stub        # a stub at the old path for tools that read by path
spec-dna eval run --dry-run                     # the evals, their runs and the estimate; no process starts
spec-dna eval run                               # run each eval with headless Claude Code; exit 1 below min-pass-rate
spec-dna eval run example --runs 1 --out evals/results   # one eval, one run, and a result file
spec-dna transcript ~/.claude/projects/<project> --expect irrigation/valves
spec-dna hook context-meter --levels 300000,600000 --subagent 250000   # a hook of a coding agent
spec-dna hook fanout-cap --cap 10   # a hook of a coding agent
spec-dna hook protect-tests --mode fix-branches --test '**/tests/**'   # a hook of a coding agent
spec-dna --help                     # commands and options (-h); --version (-V) prints the version
spec-dna mv --help                  # the help of one command
```

Each command accepts `--json`. spec-dna does not use the network. It reads the documents with local `git` commands. The only command that starts a program that uses the network is `eval run`: it starts `claude`, and spec-dna itself still makes no network call.

A target is a document id, an alias such as `ADR-12`, `<id>#<anchor or key>`, or a repository path.

## init

```text
spec-dna init [--non-interactive | --answers <file>] [--reconfigure] [--dry-run] [--root <dir>]
```

`init` examines the repository and finds these items:

- the document roots and their types, the ids, the requirement keys and the statuses;
- the languages of the headings;
- the vendored, generated and large reference directories;
- the budgets, the stack, the tests, the agent files, CI, the hooks and the default branch.

Then `init` asks at most 14 questions (13 in this version, 14 after step 4), with the found values as defaults. It shows how it applied each answer, the full `spec-dna.toml`, and the diff of each file that it will create or change. It writes them after you confirm. `--dry-run` shows this preview only. With a change policy other than `none`, the preview has a note with the `CODEOWNERS` line for the specs. `--json` has the field `codeowners`. `init` never writes the `CODEOWNERS` file. Without a terminal, `init` uses the defaults. `init` never edits a document. [init.md](init.md) gives the details.

`init` writes these adapters:

- one line in `AGENTS.md`, if the file exists or if neither file exists;
- one line in `CLAUDE.md`, unless `CLAUDE.md` imports such an `AGENTS.md`;
- the skill `.claude/skills/spec-dna/SKILL.md`.

The line never imports another file. Each adapter has the version of its generator and a hash. `check` reports a manual edit or an old version. A second run of `init` updates the adapters. It also replaces the blocks of many lines that earlier versions wrote.

The questions 9 and 10 set the [agent harness](harness.md). `init` writes it for Claude Code: agent files, a model-team line in `CLAUDE.md` and a merge into `.claude/settings.json`. The default level is `lean`.

If `spec-dna.toml` exists, `init` updates the adapters, and the harness files if the file has a `[harness]` table. With `--reconfigure`, it asks the questions again. `init` does not run if the pin in `spec-dna.toml` is a different version.

At the end, `init` prints the bytes that the adapters add to the prompt prefix of each agent request (`--json`: `prefix_bytes`). It also prints the `git add` command for the files that it wrote.

## check

```text
spec-dna check [--base <rev>] [--all] [--lint] [--staged]
```

`check` runs the controls of ADR 0001 on the working tree, or with `--staged` on the git index. With `--base`, the base is the merge base of `<rev>` and `HEAD`. Without `--base`, the base is `HEAD`.

- Without `--all`, `check` reports only the diagnostics that are new against the base.
- With `--all`, it uses the same base, but it reports each diagnostic and fails on any error.
- Without `--base`, `check` also reads the untracked documents that git does not ignore.
- With `--base`, it reads only tracked files, as CI does.
- With `--staged`, the head tree is the git index: the tree that the next commit records. `check` reads `spec-dna.toml`, the documents, the test files, the code and the harness files (SD501, SD502, SD504, SD505) from the index. Untracked files and edits that are not staged do not count. `--staged` works with `--base` and `--all`, and the base does not change. `check` stops with exit code 4 for an unmerged path in the index, and with exit code 3 if `spec-dna.toml` is not in the index.

The hook that `init` writes runs `spec-dna check --staged`. A commit records the index, so a check of the working tree can pass while the committed tree fails (SD101).

`check` also compares the requirements of the lifecycle base and the head. If a requirement of a `forever` layer changed and no file of the `intent` folder changed, it gives SD921. This check runs only with a `[change] policy` other than `none`, and only if git can name the lifecycle base.

With a `[change] policy` other than `none`, `check` finds the lifecycle base. This base is the merge base of `--base` (or, without it, of `origin/HEAD`) with `HEAD`.

Without `--base`, the base that decides which diagnostics are new stays `HEAD`. The lifecycle base is only for the codes SD905 (a change with no delta), SD910, SD913, SD917 and SD921 ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). Their diagnostics are always new. With `--staged`, they diff the git index against the lifecycle base.

`--json` has the field `change_base`, the commit id of this base, or `null` if git cannot name it. With `none`, the field is not in the output, and `check` reads no second tree and no history for it. If there is no base, the text output prints this note on stderr: `no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>`. Then `check` runs only the checks of one tree. `origin/HEAD` is the only ref that `check` reads for this base. It does not use a local `main` or `master`. A shallow clone can fail to find the merge base. Then `change_base` is `null`. For a stacked pull request, pass the branch below it as `--base`. SD910 trusts `--base`.

The checks of one tree are SD905 (the delta part), SD912, SD914, SD919, SD920 and SD922. They need no base. SD914 is an error only for a spec that the diff touches against the lifecycle base. Without that base, it is a warning.

A change can remove or rename a path or an identifier that a document cites. Then `check` gives a warning that names the document (SD606, SD607). In an accepted decision, an amendment section that cites the new path or identifier clears these warnings.

The text output gives the errors first, then SD606 and SD607, then the other warnings. Each prose lint code (SD801 to SD805) gets one line with a count. Another warning code with more than 10 diagnostics also gets one line with a count. At the end, the output gives the fix of each listed code once.

`--lint` lists each prose lint warning. It also runs the lint in a profile that has it off (`lite`, `standard`), and it overrides a `lint = "off"` rule. `--lint` does not run the lint when `language` is not English. The findings are warnings, so they never fail the gate. Without `--all`, `--lint` lists the new findings only.

The JSON output lists each diagnostic. It also lists each document with its number of recognized requirements. The profile sets the severity of each code ([profiles.md](profiles.md)). [diagnostics.md](diagnostics.md) gives each code.

## context

```text
spec-dna context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--include-inactive]
```

`context` prints the sections that an agent needs, in this order:

1. The sections that `[conventions]` names for the type of each target ([show](#show)).
2. The first tier: each rule whose `applies-to` matches an entry or a path, and each rule with no `applies-to` (it applies to the whole project). Also each top file (`product`, `system` or `terms`) whose `applies-to` matches. A top file with no `applies-to` is not in a task context, but a target can name it.
3. The entry documents or sections.
4. For a code path: the sections that cite the file by its path, or by an identifier that is in the file ([diagnostics.md](diagnostics.md#code-citations)). A code path is a target or a `--path`, and it can be an untracked file. The documents do not need `applies-to`.
5. The requirement sections and the summary of the best documents that cite the file.
6. The other normative documents whose `applies-to` matches an entry or a path, and the sections that cite only a directory of the path.
7. The `depends-on` targets at depth 1, and the linked accepted decisions as summaries.
8. The other sections.

The order of the layers orders the first tier. It also orders the documents of one citation class (cite the file, cite an identifier, or cite a directory). The layers come by level, from 0 up. The layers of one level keep the order of the `[layers.*]` tables in `spec-dna.toml`, or the order of the default graph. In the default graph, `foundation` comes before `rules`. `spec-dna layer list` shows the level of each layer ([Layer graph](design/0.6.0.md#layer-graph)).

A layer with `context = "on-request"` is held back. `context` serves its documents only if a target names them. `show` always reads them. `context` counts the documents of these layers that the request reaches, in one line after the listed sections, and lists no file:

```text
<!-- on-request, not served: research 1, intent 1, incidents 2; name a document as target to serve it -->
```

The layers are in the order of `context`. The line is not in the output if the request reaches no such document. In the default graph, `research`, `intent`, `incidents` and `pending` are `on-request`. A layer with `context = "always"` can fill the plan without a named target. `context` serves no `research` document in any layer, unless a target names it. The `--json` output has no count.

`context` serves at most 8 documents and 5 summaries that cite a file. It lists the other sections that cite the file.

In a layer with `context = "always"`, `context` lists an intent, an incident or a roadmap that cites the file. It does not serve that document.

In a document with requirements, the requirement sections come first, then the title section. `context` lists the other sections of that document.

`context` does not serve a superseded, retired or rejected document. It lists the document with its status and its successor. A target that names the document, or `--include-inactive`, makes `context` serve it.

If a section does not fit the budget, `context` lists its id, hash and size. Each section has its id and hash.

With `--query` (the task in plain words), the sections that match its words come first. BM25 ranks them, without a network or a model. Use this option for long documents without requirement lists.

## show

```text
spec-dna show <id>[#<anchor>]
spec-dna show --conventions <type>
```

`show <id>` prints a document, a section with its subsections, or one requirement, with ids and hashes. For a requirements, interface or rule document, it also prints these items:

- the number of recognized requirements (`--json`: their keys);
- the evidence of each requirement: its `verified-by` paths and the `covers:` tags of the tests.

`show --conventions <type>` prints the conventions for documents of the type. These are the sections of project documents that `[conventions]` in `spec-dna.toml` names. The output also gives the names of the amendment and live sections of the type. When a target of `context` is a document of that type, `context` serves the same sections first.

## new

```text
spec-dna new <type> [<id>] [--root <dir>] [--template <name>]
```

`new` writes a new document from a template ([types.md](types.md#templates)). Without `--template`, it uses the default template of the type. The place follows the layer of the type ([Place gives the type](types.md#place-gives-the-type)):

- A top file type (`product`, `system`, `terms`, `roadmap`, `questions`) takes no id. `new terms` writes `<root>/terms.md`. An id gives exit code 2.
- `new intent <slug>` writes `<root>/intent/<slug>/intent.md`. `new spec <slug>` and `new plan <slug>` write `spec.md` and `plan.md` into that folder. Without the folder, they stop with exit code 5 and write nothing.
- Another type writes `<root>/<dir>/<id>.md` in a mixed root. `<dir>` is the folder of the layer of the type, and the id is relative to it. For example, `new requirements valves/closing` writes `<root>/requirements/valves/closing.md`.
- In a root of one type, the file is `<root>/<id>.md`.
- A type that no layer has gives exit code 2. The message says to run `spec-dna layer add`.

`--template <name>` selects a template. `new` reads `<dir>/<name>.md` first, where `<dir>` is `[templates] dir` ([config.md](config.md#the-table-templates)). Then it reads the embedded template of that name. For example, `new intent <slug> --template prd` writes a PRD, and `new procedure <slug> --template runbook` writes a runbook. A file in `<dir>` with the name of an embedded template replaces it, also for the default: a file `madr.md` replaces the default template of `decision`. `new` stops with exit code 2 and writes nothing in these cases:

- an unknown name
- a template of another type
- a project template with no `type` in its front matter
- a name with a path in it

`new` copies the template and makes 4 changes:

- It writes the H1 from the id.
- It sets the first status of the type.
- It removes `was-type` from the copy.
- It removes `type:` where the place gives the type.

`new` writes no `type:` line where the place gives the type. It uses `--root <dir>`, or the first root of that single type, or the first mixed root. A type name of 0.5 is an error (exit code 2) that names the new command. For example, `new contract x` says to use `new interface`.

## layer

```text
spec-dna layer add <name> [--types t1,t2] [--paths g1,g2] [--files f1,f2] [--dir d]
  [--moves-to a,b] [--moved-from a,b] [--linked-from a,b]
  [--keep forever|until-finished] [--context always|on-request]
  [--max-docs n] [--max-chars n] [--max-started n] [--stale-days n]
spec-dna layer list
```

A layer is a named set of documents ([The table `layers`](config.md#the-table-layers)). `layer add` adds one to `spec-dna.toml`. `layer list` shows the layers.

### layer add

`layer add` writes `[layers.<name>]` at the end of `spec-dna.toml`, so the new layer is last in the file order. The options give the keys of the table ([config.md](config.md#the-table-layers)): `--types`, `--paths`, `--files`, `--dir`, `--moves-to`, `--linked-from`, `--keep`, `--context`, `--max-docs`, `--max-chars`, `--max-started` and `--stale-days`. A list option takes names or globs that a comma separates. The table has only the keys that the command line gives.

`layer add` writes the line ending of the file: a file with CRLF line endings keeps them. An entry of `--files` follows the same rule as the key `files`: a relative path inside the root, which no other layer has.

`--moved-from a,b` adds the new layer to the `moves-to` list of the layers `a` and `b`. The command edits that list in place. It inserts the new name and changes no other byte of the file, so each comment stays.

- A layer that has a `moves-to` key gets the name at the end of the list. This works for a table and for dotted keys.
- A layer with no `moves-to` key that has a table form (`[layers.a]`) gets a new `moves-to` line below its header.
- A layer with no `moves-to` key that has dotted keys or an inline table gives exit code 2. Add `moves-to` to it by hand.

If `spec-dna.toml` has no `[layers]` table, the project uses the default graph ([config.md](config.md#the-table-layers)). In this case, `layer add` first writes the tables of that graph. It applies `types`: it drops the layers that hold none of the types, and the edges to them. It then adds the new layer.

The command refuses these cases with exit code 2. It writes nothing, and the file keeps its bytes:

- A cycle in the `moves-to` edges.
- A name that a layer already has.
- A type that another layer has.
- A layer in an edge (`--moves-to`, `--linked-from`, `--moved-from`) that the graph does not have.
- A value that is not valid. Examples are a name with a character other than a letter, a digit, `-` or `_`, and an unknown type. Others are a number that is not a number, a `--keep` or `--context` value that is not in the list, and a `--dir` that leaves the root. `--files` with `--dir` and a folder that another layer has are also not valid.
- A file layout where the new table has no place, for example `layers = { ... }` as an inline table of the root.

The command reads its own result before it writes it. The result must give the layers that the command expects.

The change goes into `spec-dna.toml` only. Commit it as a change of its own: `check` gives SD503 for a change of `spec-dna.toml` that comes with other files ([diagnostics.md](diagnostics.md)). With `--json`, the output is one object:

```json
{"layer":"interfaces","level":0,"path":"spec-dna.toml","wrote_implied_graph":false,"moved_from":["intent"]}
```

### layer list

`layer list` prints one line for each layer, in the order of the file:

- The name and the level. Level 0 is the most stable layer.
- The selection: `types`, `files` and `paths`, if the layer has them. It also gives `dir` if the file sets it.
- The edges: `moves-to` and `linked-from`.
- `keep` and `context`.
- The limits that the layer sets: `max-docs`, `max-chars`, `max-started` and `stale-days`.
- `docs`: the number of documents of the layer, and the number that each rule put there. The rules are the type of the layer (`type`), a glob of `paths` (`paths`) and a top file of the layer (`files`). A glob of `paths` comes first: a document that matches belongs to that layer, also when its type is in another layer.

A document of a type that no layer has is in no layer. A document with no type (SD303) is also in no layer. A last line gives their number. `layer list` reads the same documents as `check`. If the file has no `[layers]` table, the first line says so, and the list is the default graph.

With `--json`, the output is one object with the keys `declared`, `layers`, `documents` and `unplaced`. Each entry of `documents` has `path`, `id`, `type`, `layer` and `rule`. `layer` and `rule` are `null` for a document that is in no layer.

## mv

```text
spec-dna mv <from> <to> [--stub]
spec-dna mv <doc-id>#<key or anchor>... <doc-id or path> [--type <type>]
```

The first form moves a document with `git mv`. It adds the old id to the `aliases` of the document, and it changes the relative links to the document. `--stub` leaves a stub at the old path for tools that read a file by its path. The stub has the front matter `moved-to`, no id and no budget.

The second form splits a document of any type. It moves requirements, or whole sections with their subsections, into one of these documents:

- an existing document of any type;
- a new document. `mv` writes its front matter and its title. Its type is the type of its root, `--type`, `research` for the history of a decision, or the type of the source.

These rules apply to a split:

- Requirements move only into a requirements, interface or rule document.
- Each moved section of `HEAD` keeps its old id as an alias: `<old-id>#<anchor>` in the `aliases` of the destination.
- Each moved requirement whose id is in `HEAD` gets an inline marker `<!-- aliases: ... -->` with that id. A key that is new in the uncommitted change gets no alias.
- The rows of `companion-sections`, such as a coverage table, move with their requirements. If a row names a key that moves and a key that stays, `mv` refuses the move.
- Each moved section leaves its heading in the source, with a one-line pointer to its new place.
- Links to a moved key or section follow it. This rule also applies in accepted decisions, because a link destination is not frozen text.
- A decision keeps its frozen text. Only its amendment and history sections move, into a research document. `mv` lists the moved amendments that can change the outcome. These amendments are candidates for a new decision that supersedes the old one.

Before `mv` changes a file, a preservation check compares the result with the current tree:

- each requirement has the same hash (without link destinations);
- each requirement id resolves to the same requirement;
- the frozen text of an accepted decision does not change;
- there is no new error.

If a check fails, `mv` changes nothing. It names each requirement whose text would change, and it exits with code 1.

## history

```text
spec-dna history <doc-id> [<to>]
```

`history` moves the history sections of a document into a research document. The history sections are the `history-sections`, and the amendment sections of a decision. `history` leaves a pointer for each section, and it adds a backlink. The default destination is `<name>-history.md` next to the document, in a mixed root. If no layer has the type `research`, `history` stops with exit code 2 and changes nothing. The message gives the command to run: `spec-dna layer add research --types research`. A new project of the small graph has no `research` layer. `mv` does the same for any type of a new document that no layer has.

## index

```text
spec-dna index <dir or file> [--replace]
```

`index` writes a generated index of the documents under a directory. The index has one row for each entry, with its name, title and status. `--replace` replaces a file that a person keeps manually. Then the links to a section of the old file that names an entry go to the entry. `check` reports a stale or manually edited index (SD610).

## finish

```text
spec-dna finish <slug> [--dry-run] [--base <rev>]
```

`finish` ends a change ([Finish](design/0.6.0.md#finish)). It applies the delta of the spec of `<intent folder>/<slug>/` to the requirements, checks the evidence, and deletes the folder. The author runs it in the pull request of the code. The review then sees the code and the new current truth in one diff. It stops at the first step that fails. A stop writes nothing.

1. Step 0: no document outside the folder links into it. It reads a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` and `from`. It skips a stub, a generated index and the locked text of an accepted decision. It lists each reference and stops, also in the profile `lite`.
2. Step 1: the intent is `accepted`, the plan is `approved`, and (for a `higher-risk` change) the spec is `approved`. An intent with the status `closed` runs steps 0 and 1, and then `finish` deletes the folder. It applies no delta.
3. Step 2: no open concern (SD919), no open question (SD922), and each decision of `Decisions` is `accepted` (SD920).
4. Step 3: each delta line is valid against the tree (SD914). A `CHANGE` or `REMOVE` hash is the hash of the requirement now. The delta is the delta of the base tree: an approved delta must not change. The pull request changes each interface document of `Interfaces`, or a file that it cites.
5. Step 4: it applies the delta in memory. `ADD` writes a list item after the outermost requirement item that holds the last requirement item, so a nested last requirement gives a sibling of the top-level items. The item has the indent and the marker of that item (an ordered list gets the next number, counted after the removals). With no item, it writes into the section `Requirements` (`Rules` for a rule), and it adds the section if needed. `finish` writes a document that does not exist at `<root>/<id>.md`. The document has the status `active`, the H1 from the id, a `Requirements` section (`Rules` for a rule) and the items. It has the line `type: requirements` only where the place gives no type. `CHANGE` replaces the sentence on one line. It keeps the key, the nested lists and the `aliases` and `verified-by` markers (also a marker inside a sentence that wraps). `REMOVE` deletes the lines of the item. If the item holds a nested requirement, the delta must have a `REMOVE` line for each nested key too, or `finish` stops at step 4 and names the key. `finish` stops at step 4 if the result would drop a requirement that no `REMOVE` line names. A document id that ends in `.md` gives SD914: write the id without `.md`. A relative link of a delta line moves to the path of the target document. The result keeps CRLF, a BOM and the final newline of each file. A line that the tree applies already is skipped, so a second run changes nothing.
6. Step 5: each requirement that the delta adds or changes has evidence (SD916). For a change with no delta, each requirement whose hash differs from the base has it. The result may add no error. The only exception is SD305 for a new document.

If no step stops, `finish` writes all the files in one pass. Then it deletes the tracked files of the folder and stages each path with `git add -A`. It makes no commit: the author reads the diff and commits it with the code. `finish` writes a generated index again if it lists a deleted file, so `check` gives no SD610. `finish` also needs the folder to be tracked and equal to `HEAD`. An untracked file in the folder, or an edit that is not committed, stops it. It also stops if it must write a file that has an edit that is not committed, because it stages each file that it writes. `finish` writes and deletes no path that has a symbolic link, also as the last part of the path, and no path outside the repository. It checks all the paths before it writes the first file, in `--dry-run` too.

The base is the merge base of `--base` (default: `origin/HEAD`) and `HEAD`. An `accepted` change needs it. A `closed` change needs none. The base of step 3 does not depend on `[change] policy`, and the stops do not follow the profile or a `[rules]` entry.

`--dry-run` writes nothing. It prints the diff of each file that `finish` would write, and `remove <path>` for each file that it would delete. With `--json`, the output is one object:

```json
{"slug":"dose-limit","ok":true,"dry_run":true,"step":6,"diagnostics":[],"files":[{"path":"docs/requirements/valves.md","action":"write","diff":"--- a/..."},{"path":"docs/intent/dose-limit/intent.md","action":"remove","diff":""}]}
```

A stop has `"ok":false`, the `step` that failed, a `reason`, and the `diagnostics` of the stop. `files` is then empty. [diagnostics.md](diagnostics.md#stops-of-finish) lists the stops.

`finish` is the one command that deletes documents. It deletes only the files of the intent folder, and only after the steps. Git is the archive.

## next

```text
spec-dna next <slug> [--base <rev>]
```

`next` prints the checklist to the next gate of the change `<slug>`, the folder `<intent folder>/<slug>/`. The first 2 lines give the change, its class, its stage and its base. The other lines are the checklist, at most 10. Each line says done (`[x]`) or open (`[ ]`), and ends with `[skill: <name>]`. The last line is the edit or the command that ends the stage, for example ``set `status: accepted` in docs/intent/dose-limit/intent.md``. The output has at most 12 lines.

```text
change dose-limit (routine), stage build
base 3f2a9c1b7e0d
[x] the intent is accepted [skill: brainstorm]
[x] the plan is approved [skill: write-plan]
[ ] tasks of `Order`: 1 of 3 done [skill: execute]
[ ] task t2: Close the valve. Files: `src/valve.py`. Verify: `pytest` [skill: execute]
[ ] change a file that `Files` of docs/intent/dose-limit/plan.md lists: none differs from the base [skill: execute]
```

### Stages

A change has no stage field. The stage is the first row of this table that matches. [Stage derivation](design/0.6.0.md#stage-derivation) gives the reason for each row.

| Stage | The change is in this stage when | The last line of `next` |
| --- | --- | --- |
| `closed` | `intent.md` is `closed` | Run `spec-dna finish <slug>` |
| `plan` | `intent.md` is absent, or its status is not `accepted` | Set `status: accepted` in `intent.md` |
| `design` | The class is `higher-risk`, and `spec.md` is absent or not `approved` | Set `status: approved` in `spec.md` |
| `build` | `plan.md` is absent or not `approved`, or no file of `Files` differs from the base | Set `status: approved` in `plan.md`, or change a file of `Files` |
| `test` | A requirement that the change adds or changes has no evidence | Add a `covers:` tag in a test file |
| `deploy` | Each such requirement has evidence | Run `spec-dna finish <slug>` |

- If the plan lists no path because the items of `Files` are not paths, the last line says so and shows the first such item. A path is a code span, or a plain path ([Files and paths](types.md#files-and-paths)).
- A file of `Files` differs from the base when the diff of the working tree against the base has a path that a `Files` item matches. A file of the folder of the change does not count. With no base, the diff is unknown. Then a change with an approved plan stays in `build`.
- A change that has a delta uses its `ADD` and `CHANGE` lines as the requirements to prove. A `REMOVE` line needs no evidence. A change with no delta line uses the requirements whose hash differs from the base. If the diff has a changed file of another change, only the documents that `Files` lists count. This is the rule of step 5 of `finish`.
- A requirement has evidence in the cases of [evidence](config.md#evidence). A `covers:` tag can name a requirement of an `ADD` line before `finish` writes it.
- The change that the default branch finished is no longer in the tree, so it has no stage. `spec-dna log` lists it.
- The stage Maintain is not the stage of a change. `status` lists an open follow-up of an incident ([status](#status)).

In the `build` stage, `next` also names the next open task of the plan, also if the plan is a draft. It gives the id, the text, the files and the verification of the task. The task list is the `Order` section of `plan.md` ([Tasks of a plan](types.md#tasks-of-a-plan)). `next` counts the tasks that are done. The first open task is the next task.

### Skills

Each line of `next` has the name of the skill for its step. The skills come in 0.7.0. Until then, `next` prints the names only, as data: a fixed list of 9 names, in this order: `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify` and `finish`. With `--json`, each item has the field `skill`.

| Step | Skill |
| --- | --- |
| The stage `plan` | `brainstorm` |
| The stage `design`, a spec to write or to approve | `specify` |
| An open concern (SD919) or an open question (SD922) | `clarify` |
| The stage `build`, a plan to write or to approve | `write-plan` |
| The stage `build`, an open task or a change of the code | `execute` |
| The stage `test`, a requirement with no evidence | `tdd` |
| The stage `deploy`, run the proof of the plan | `verify` |
| The stage `deploy`, review the diff | `review` |
| The stage `closed`, or a change that is ready for `finish` | `finish` |

A line that is done has the skill of its own step.

### Base and output

The base is the merge base of `--base` (default: `origin/HEAD`) and `HEAD`. `next` prints it. If git cannot name a base, `next` prints the note `no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>`. The stage then stops at `build`, because the diff against the base is unknown. With `[change] policy = "none"`, `next` says that the change process is off and reads no git data. It exits with 0.

With `--json`, the output is one object:

```json
{"slug":"dose-limit","class":"routine","stage":"build","base":"3f2a9c1b7e0d4a11...","items":[{"done":true,"text":"the intent is accepted","skill":"brainstorm"}],"next_task":{"id":"t2","text":"Close the valve","files":["src/valve.py"],"verify":"`pytest`"},"next":"change a file that `Files` of ...","notes":[]}
```

`base` is a SHA, or `null`. `next_task` is `null` outside the `build` stage, and it is `null` if the plan has no open task. A draft plan can have one. `next` is the text of the last item. `notes` lists the notes, for example the note for no base. `next` warns on a pin mismatch, as `status` does.

Exit codes: 0 done; 2 usage; 3 configuration, or a `--base` that is not a commit; 4 git; 5 no folder for the slug.

## status

```text
spec-dna status [--group <name>] [--base <rev>]
```

`status` lists each change of the working tree. For each change it gives the class, the stage, the statuses of the intent, spec and plan, the areas and the next gate. It also gives the progress of the tasks (`done/total` and the id of the next open task). A started change (an `accepted` intent) that has had no commit for `stale-days` days is `stale` (SD911). `status` also gives the number of started changes and the limit `max-started`.

Then `status` lists each open follow-up of an incident. A follow-up is an item `- [ ]` in the `Follow-ups` section. The list gives the incident id, the path, the line and the text. This is the stage Maintain. The follow-ups are not part of a group, because an incident has no areas.

```text
policy three-class, base 3f2a9c1b7e0d, 1 started (max 3)
dose-limit routine build intent accepted spec - plan approved tasks 1/3 next-task t2 areas valves
  next: change a file that `Files` of docs/intent/dose-limit/plan.md lists: none differs from the base
follow-up incidents/valve-stuck: docs/incidents/valve-stuck.md:13 Add an alarm for a stuck valve.
```

`--group <name>` lists only the changes that are in the group. A group is a name of `[groups]` and its areas ([Areas and groups](design/0.6.0.md#areas-and-groups)). A change is in the group if an area of the group is in the `areas` of its intent, or is the area of a delta target. The area of a delta target is the first folder under the folder of the layer of `requirements`. For example, the target `requirements/valves/closing` is in the area `valves`. The counts of started changes are the counts of the project. An unknown group is a usage error (exit code 2). `context --group` is not in this version.

The base and the policy `none` work as for [next](#next). The output has `notes` for the no-base note. With `--json`, the output is one object:

```json
{"version":"0.6.0","policy":"three-class","base":"3f2a9c1b7e0d4a11...","started":1,"max_started":3,"notes":[],"changes":[{"slug":"dose-limit","path":"docs/intent/dose-limit","title":"Dose limit","class":"routine","statuses":{"intent":"accepted","spec":null,"plan":"approved"},"stage":"build","next":"change a file that ...","stale":false,"areas":["valves"],"tasks":{"done":1,"total":3,"next_task":{"id":"t2","text":"Close the valve","files":["src/valve.py"],"verify":"`pytest`"}}}],"follow_ups":[{"incident":"incidents/valve-stuck","path":"docs/incidents/valve-stuck.md","line":13,"text":"Add an alarm for a stuck valve."}]}
```

A status is `null` if the file is absent. `max_started` is `null` if the layer has no limit. `areas` lists the areas of the intent, then the areas of the delta targets. `tasks.next_task` is the next open task whatever the stage is, and it is `null` if the plan has none.

Exit codes: 0 done; 2 usage, or an unknown group; 3 configuration, or a `--base` that is not a commit; 4 git.

## log

```text
spec-dna log [--since <rev>]
```

`log` lists the changes that left the default branch. A finished change leaves no folder, so the history of git is the only record of it. `log` reads each commit that deleted an `intent.md` under the folder of the `intent` layer. It uses `git log -M --diff-filter=D`, with no `--first-parent` and no `-m`. Thus a squash merge, a merge commit and a rebase merge each give one entry for one deleted file. The entry is the commit that deleted the file, and it is never a merge commit.

The status of the intent in the parent of that commit gives the result:

| Status in the parent tree | Result |
| --- | --- |
| `accepted` | `finished` |
| any other status, also `draft` | `closed` |

For each change, `log` gives the date of the commit (UTC), the result, the slug and the class. It also gives the counts of the delta lines of its spec (`add`, `change` and `remove`), the commit and the title. The counts and the class come from the parent tree too. A line that has no valid form is not counted. The newest change comes first.

```text
2024-04-05 finished dose-limit higher-risk add 1 change 2 remove 0 3f2a9c1b7e0d Dose limit
2024-03-18 closed old-idea routine add 0 change 0 remove 0 91be40d27c5a Old idea
```

`--since <rev>` limits `log` to the commits after `<rev>` (`<rev>..HEAD`). A `<rev>` that is not a commit is exit code 3.

A change has an entry only if the folder came to the default branch in an earlier commit. If a pull request holds the intent, the approval and the code together, and a squash merge brings it to the default branch, the default branch never had the folder. Then the change has no entry. A rename of the folder out of the `intent` folder reads as a deletion, because git cannot follow the file outside of its path limit. A rename inside the `intent` folder is not a deletion. The entry comes later, under the new name.

In a shallow clone, `log` can miss the changes that the clone does not have. It prints the note `this is a shallow clone; older changes may be missing`. With `--json`, the field `shallow` is `true`. With `[change] policy = "none"`, `log` says that the change process is off and lists nothing. It warns on a pin mismatch, as `status` does.

With `--json`, the output is one object:

```json
{"changes":[{"slug":"dose-limit","class":"higher-risk","title":"Dose limit","result":"finished","delta":{"add":1,"change":2,"remove":0},"commit":"3f2a9c1b7e0d4a11...","date":"2024-04-05"}],"shallow":false}
```

Exit codes: 0 done; 2 usage; 3 configuration, or a `--since` that is not a commit; 4 git.

## stats

```text
spec-dna stats --sdlc [--group <name>]
```

`stats --sdlc` prints the leading and the lagging indicators of the playbook under the playbook names. It reads the history of git and the committed files. It reads no working tree, no pull request data and no clock. `stats` without `--sdlc` is a usage error (exit code 2).

The command rebuilds the life of each change from the commits that touched its `intent.md`, `spec.md` and `plan.md`. A life ends at the commit that deleted the `intent.md`. A change that is still in the history of `HEAD` is open. The order of the commits is the order of the ancestry, not the order of the dates. A span that would be negative is not counted. The durations use the committer dates. A rename of the folder inside the `intent` folder keeps the life under the new name.

| Play | Kind | Indicator | Value |
| --- | --- | --- | --- |
| Plan | leading | Time from first conversation to a committed `intent.md` | The median days from the first commit of the folder to the first commit that sets `accepted`. The conversation is not in git. |
| Plan | lagging | Survival rate | The percent of intents that are `accepted` rather than `closed`. A deleted intent counts as in `log`. A draft that is still open does not count. |
| Design | leading | Time between the `intent.md` commit and the `spec.md` commit | The median days from the first commit of the intent to the first commit of the spec. |
| Design | lagging | Requirements rework after build starts | The mean of the `spec.md` commits after the first `plan.md` commit, for each change that has a spec and a plan. |
| Build | leading | Time from plan approval to the merged pull request | The median days from the first commit that sets the plan `approved` to the commit that deleted the folder. It counts finished changes. |
| Build | leading | Share of changes that merge from the first implementation pass | `n/a`: it needs pull request metadata. |
| Build | lagging | How often the merged diff still matches the committed `plan.md` | The percent of finished changes whose diff has no path that SD913 reports for the scope of the plan. |
| Build | lagging | Rework cycles per change | `n/a`: it needs pull request metadata. |
| Test | leading | The eval pass rate over time | `n/a`: evals come in step 4 of 0.6.0. |
| Test | leading | How long a production incident takes to become a permanent eval | `n/a`: evals come in step 4 of 0.6.0. |
| Test | lagging | Regressions caught in CI against regressions found in production | `n/a`: it needs the incident tracker. |
| Maintain | leading | Time from band breach to an `intent.md` in the triage queue | `n/a`: spec-dna has no bands. |
| Maintain | lagging | Repeat incidents of the same class | `n/a`: an incident has no class field. |
| Deploy | both | The other indicators of the playbook | `n/a`: they need CI logs, telemetry or an incident tracker. |

The commit that "deleted the folder" is the first-parent commit of the deletion. For a merge commit it is the merge commit, and for a squash merge it is the squash commit. A rebase merge shows only its last commit. For such a change the time and the diff can be smaller than the real ones.

The diff of "the merged diff still matches the plan" is the diff of that commit against its first parent. The scope is the scope of SD913 for the approved plan of the parent tree. It has the folder, the `Files` patterns, the `Files:` paths of the tasks, and the documents that the delta and `Interfaces` name. `stats` reads the last tree of the life in the history. A squash merge keeps no commit of the branch, so `stats` cannot see a plan that the pull request added or edited. When one commit finishes more than one change, the scope is the union of their scopes, as SD913 reads the changes of one diff. `stats` finds the document of an id by its place under each root, and it does not read aliases. A generated index is in scope. A change with no plan is not counted.

A shallow clone can miss old changes, so each indicator that it computes has this note in `reason`. An indicator with no data has no value, and the reason says what is missing.

`--group <name>` counts only the changes that are in the group. A group is a name of `[groups]` and its areas. A change is in the group if an area of the group is in the `areas` of its intent, or is the area of a delta target, as for [status](#status). The areas come from the last tree of the change. An unknown group is a usage error (exit code 2). With `[change] policy = "none"`, `stats` says that the change process is off and gives no indicator. It warns on a pin mismatch.

The text output has a summary line, then one line for each indicator. A value that has a note gets a second line that starts with `note:`. An indicator that git cannot give shows `n/a` and its reason:

```text
4 changes in the history: 2 finished, 1 closed, 1 open
Plan lagging: Survival rate: the share of intent.md files that are accepted rather than closed: 66.7 percent (3 changes)
Build leading: Share of changes that merge from the first implementation pass: n/a (it needs pull request metadata)
```

With `--json`, the output is one object with the 14 indicators of the table, in its order:

```json
{"indicators":[{"play":"Plan","kind":"lagging","name":"Survival rate: the share of intent.md files that are accepted rather than closed","value":66.7,"unit":"percent","count":3,"reason":null},{"play":"Build","kind":"lagging","name":"Rework cycles per change","value":null,"unit":null,"count":null,"reason":"it needs pull request metadata"}]}
```

`value` is a number with 1 decimal, or `null` for an indicator with no value. `unit` is `days`, `percent` or `commits per change`. `count` is the number of changes that the value counts, and it is `null` for an indicator that git cannot give. `reason` is `null` for a full value. Each `null` value has a reason.

Exit codes: 0 done; 2 usage, an unknown group, or no `--sdlc`; 3 configuration; 4 git.

## eval

```text
spec-dna eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate]
```

`eval run` tests the agent setup of the project ([ADR 0008](adr/0008-harness-and-evals.md)). It starts headless Claude Code (`claude -p`) on a copy of a fixture, and then runs the checks of the eval. It needs the table `[evals]` in `spec-dna.toml`. `eval` with no subcommand, or with another subcommand, is a usage error.

spec-dna itself makes no network call. `claude` does, and each run costs money. A pin that differs from the version of the binary stops the command, as it stops `check`.

### An eval file

An eval is the file `<dir>/<id>.json`, where `<dir>` is `[evals] dir`. A JSON file directly in `<dir>` is an eval. A file in a subfolder is not. The file has at most 64 KiB, it is UTF-8, and it is one JSON object. A key that the table does not list is an error. An integer is a JSON number with no fraction. `eval run` reads the working tree: the tracked files, and the untracked files that git does not ignore. It checks the keys in the order of this table, and it reports the first rule that fails.

| Key | Required | Default | Rule |
| --- | --- | --- | --- |
| `id` | yes | - | `[a-z0-9][a-z0-9-]{0,63}`, equal to the name of the file without `.json` |
| `task` | yes | - | The prompt. Not empty, at most 8,000 bytes, and it does not start with `-` |
| `fixture` | yes | - | A folder under `<dir>/`, but not under `<dir>/results/`. It has `/` separators and no `..`, `.`, empty segment or `\`. It holds at least one file |
| `branch` | no | none | A git branch name of at most 100 characters of `A-Z`, `a-z`, `0-9`, `.`, `_`, `/` and `-`. It starts with a letter or a digit, has no `..` or `//`, and does not end with `/` or `.lock`. The branch `main` is the base branch, so it gives no switch |
| `allowed-tools` | no | `[]` | A list of strings. Each one is not empty and has no control character |
| `max-turns` | no | `20` | An integer from 1 to 1,000 |
| `max-budget-usd` | yes | - | A number above 0, and at most `[evals] max-total-usd` |
| `runs` | no | `[evals] runs` | An integer from 1 to 100 |
| `pass-at` | no | the runs of the eval | An integer from 1 to the runs of the eval |
| `checks` | yes | - | A list that is not empty. Each item has exactly `name` (`[a-z0-9][a-z0-9-]{0,63}`, once in the file) and `run` (not empty) |

```json
{
  "id": "fix-keeps-tests",
  "task": "Fix the failing renew test by changing the code, not the test.",
  "fixture": "evals/fixtures/renew",
  "branch": "fix/renew",
  "allowed-tools": ["Read", "Edit", "Bash(cargo test*)"],
  "max-turns": 20,
  "max-budget-usd": 2,
  "runs": 3,
  "pass-at": 2,
  "checks": [
    { "name": "tests-unchanged", "run": "tests_unchanged" },
    { "name": "tests-pass", "run": "cargo test --quiet" }
  ]
}
```

`<dir>/check.sh` is a file of the project. It defines the shell functions that the checks call. `eval run` stops with exit code 3 if the file is absent, or if `<dir>` holds no eval.

### The steps

1. It reads the configuration and checks each eval file. An invalid file gives exit code 3 and the message names the path and the rule. It selects the evals of the ids. With no id, it selects each eval, in the order of the id. An unknown id is a usage error.
2. The estimate is the sum of `runs x max-budget-usd` of the selected evals. If the estimate is above `max-total-usd`, it stops with exit code 3 and starts no process.
3. It runs `claude --version` once. If `claude` is absent or cannot start, it stops with exit code 3. The message names `claude`.
4. For each eval and each run (each run runs; there is no early stop):
    1. It makes a temporary folder, `spec-dna-eval-<pid>-<n>` in the temporary directory of the system (mode 700 on Unix).
    2. It copies the files of the fixture into the root of the folder. Then it copies `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and the files of `.claude/`, except `.claude/settings.local.json`, from the working tree. A file of the project replaces a fixture file at the same path. The copy uses the list of git: a file that git ignores is not copied. It copies no symbolic link, and a warning names it.
    3. It starts a git repository in the folder. The base commit has the author `spec-dna <eval@spec-dna.invalid>`, the date `2000-01-01T00:00:00Z`, no signature and no hook. If `branch` is set and is not `main`, it switches to a new branch of that name.
    4. It starts `claude` in the folder. The arguments are `-p`, the task, then `--allowedTools` with each tool as its own argument (only for a list that is not empty), then `--output-format json --max-turns <n> --max-budget-usd <usd> --permission-mode dontAsk`, then with `--isolate` `--setting-sources project`. It never passes `--bare`, because an eval must load `CLAUDE.md`, the skills and the hooks. The environment is the environment of `eval run`, without `CLAUDE_PROJECT_DIR`, `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`. A session that starts `eval run` keeps its `CLAUDECODE`: `claude -p` starts under it (Claude Code 2.1.325). `eval run` keeps at most 16 MiB of the output of `claude` in memory.
    5. It reads `total_cost_usd` and `subtype` from the output. The output is one JSON object, or a list of events that ends with the object of type `result`. If the cost is absent, the spend counts `max-budget-usd` for the run.
    6. It runs each check with `sh -c` in the folder. The script sources `check.sh` of the project (never the copy, so the agent cannot change its checks), and then runs the `run` text of the check. The environment has `SPEC_DNA_CHECK_SH`, the absolute path of that file, and `SPEC_DNA_EVAL_BASE`, the id of the base commit. The checks also run when `claude` failed. A check passes on exit code 0. A run passes when `claude` exits with code 0 and each check passes.
    7. It deletes the folder, also after an error.
    8. It adds the cost to the spend. If the spend is above `max-total-usd` and a run is left, it stops. Claude Code checks `--max-budget-usd` against an estimate of the client, so the real cost can be higher than the budget.
5. An eval passes when its passed runs are at least `min(pass-at, runs)`. The pass rate is the share of the evals that pass, not of the runs. The suite passes when the pass rate is at least `min-pass-rate`.

### Options

| Option | Meaning |
| --- | --- |
| `<id>...` | The evals to run |
| `--runs <n>` | Replaces the `runs` of each selected eval and `[evals] runs`. An integer from 1 to 100 |
| `--min-pass-rate <r>` | Replaces `[evals] min-pass-rate`. A number from 0 to 1 |
| `--max-total-usd <n>` | Replaces `[evals] max-total-usd` for the cap. A number above 0 |
| `--out <dir>` | Writes the result to `<dir>/eval-<YYYYMMDD>T<HHMMSS>Z.json` (the UTC time of the start; if the file exists, `-2`, `-3` and so on). `<dir>` is relative to the repository root, and `eval run` makes it. It cannot be `[evals] dir`, because the check SD506 reads each `*.json` file there. Use for example `<dir>/results` |
| `--dry-run` | Checks the evals and prints each eval with its runs and the estimate. It starts no process, and writes no file. Exit code 0, or 3 if the estimate is above the cap |
| `--isolate` | Adds `--setting-sources project`, so a run does not load the user and local settings of the person. It is off by default, so CI and a laptop run the same command. spec-dna does not test what else a run still loads from the folder of the user |
| `--json` | Prints the result as one JSON object, and no text |

### Output

```text
eval run: 2 evals, 4 runs, estimate 7.00 USD, cap 20.00 USD
example: 1 of 1 runs passed, pass-at 1: pass, 0.12 USD
fix-keeps-tests: 1 of 3 runs passed, pass-at 2: fail, 1.80 USD
pass rate 50.0 percent (1 of 2 evals), min-pass-rate 80.0 percent: fail
spend 1.92 USD
```

A stop by the cap adds `stopped: the spend <s> USD is above max-total-usd <c> USD; <n> runs did not start`, and the line of the pass rate is absent. An eval that did not run all its runs does not pass. The progress of each run goes to stderr.

The result of `--json` and the result file hold the same object. The file is indented. With `--dry-run`, `--json` prints one object: `spec_dna`, `dry_run` (`true`), `evals_dir`, `max_total_usd`, `estimate_usd`, and `evals`, a list with the `id`, `runs`, `pass_at`, `max_budget_usd` and `estimate_usd` of each eval.

```json
{"spec_dna":"0.5.0","claude_code":"2.1.325","started":"2026-11-15T10:15:00Z","evals_dir":"evals","min_pass_rate":0.8,"max_total_usd":20,"estimate_usd":1,"spend_usd":0.12,"stopped":null,"pass_rate":1,"passed":true,"evals":[{"id":"example","runs":1,"pass_at":1,"max_budget_usd":1,"passed_runs":1,"passed":true,"spend_usd":0.12,"results":[{"claude_exit":0,"subtype":"success","cost_usd":0.12,"checks":[{"name":"notes-done","passed":true}]}]}]}
```

- `claude_code` is the first word of `claude --version`, or `null`. `stopped` is `null` or `"max-total-usd"`. After a stop, `pass_rate` is `null`, `passed` is `false`, and an eval that did not start has `"results":[]`.
- `claude_exit` is the exit code of `claude`, or -1 if a signal ended it. `cost_usd` is `null` when `claude` gave no cost. `spend_usd` is the sum that the cap uses.
- The result has no task text, no check command, no output text of `claude`, no path of the machine and no value of the environment. A project can keep the files in `<dir>/results`, and the pass-rate indicator of `stats --sdlc` reads them. Claude Code can keep its own session files on the machine. spec-dna does not read or copy them.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | The pass rate is at least `min-pass-rate`. Also `--dry-run` |
| 1 | The pass rate is lower |
| 2 | Usage error: a bad option value, an unknown id, `--out` that is `[evals] dir`, or no `run` subcommand |
| 3 | No `[evals]` table, an invalid eval file, no eval, an absent `check.sh`, a different version pin, an estimate above the cap, a stop by the cap, or a `claude` that is absent or cannot start |
| 4 | A git or file error of `eval run` itself: the temporary folder, the copy, the base commit or the result file |

## transcript

```text
spec-dna transcript <file or dir>... [--expect <id>]... [--format <name>]
```

`transcript` reads agent transcripts. The first format is Claude Code JSONL. For each session and episode, it reports these counts:

- the input, cache-creation, cache-read and output tokens;
- the tool calls;
- the documents and sections that the agent read.

It prints counts and ids only. It never prints the text of a prompt, a command or a file. [transcript.md](transcript.md) gives the details.

## hook

```text
spec-dna hook context-meter [--levels <a,b,...>] [--subagent <n>]
spec-dna hook fanout-cap [--cap <n>]
spec-dna hook protect-tests [--mode <fix-branches|always>] [--test <glob>]...
```

A coding agent runs `hook`. The agent starts it on each tool call and sends one JSON object on stdin. spec-dna reads at most 1 MiB of it. A hook never reads `spec-dna.toml`, never checks the pin and never scans the repository. It reads at most one more file. `protect-tests` reads no file, and runs `git` at most 2 times, only for a path that matches a test glob. A hook ignores unknown flags and bad values. The exit code is always 0. An error gives no output. A hook prints JSON only when the agent must see a message.

`context-meter` runs after a tool call (the `PostToolUse` event of Claude Code). It finds the size of the context in the transcript of the agent: the input, cache-creation and cache-read tokens of the last assistant message. The hook reads the file from its end, 256 KiB at a time, up to 4 MiB. When the size reaches a level of `--levels`, the hook tells the agent once, with `additionalContext`.

In a subagent, the transcript is `<directory of transcript_path>/<session_id>/subagents/agent-<agent_id>.jsonl`, and `--subagent` gives the only level. A level of 0 or an empty list disables the meter. The `session_id` and the `agent_id` must have 1 to 64 characters of `A-Za-z0-9_-`.

The hook keeps one empty file for each level that the context reached, in `<temporary directory>/spec-dna-meter/`. When the size falls below 2/3 of the first level (the agent compacted its context), the hook removes the files of the session. Then the hook can report the levels again. The hook also removes files that are older than 3 days.

`fanout-cap` runs before a `Workflow` tool call (the `PreToolUse` event). It reads the script of the workflow from the first of these sources that exists:

1. `tool_input.script`.
2. The file `tool_input.scriptPath`, in the `cwd`.
3. `.claude/workflows/<name>.js` or `.mjs`, in `$CLAUDE_PROJECT_DIR`.

The hook finds an upper limit of the `agent(` calls: the call sites, multiplied by the sizes of the loops around them. The hook recognizes the loops `map`, `filter`, `forEach` and the other array methods, `for`, `while`, `do`, `pipeline` and `Array.from`. It recognizes a list that is a literal, a variable with a literal, `.slice` with numbers, and `Array.from({ length: N })`. Any other list has no limit.

A named function counts once for each mention of its name. A call of itself and a `workflow(` call have no limit. The hook allows a script that it cannot read (for example, when its brackets do not match).

When the limit is above the cap, or if there is no limit, the hook answers `permissionDecision: "ask"`, and the user decides. The cap is `SPEC_DNA_FANOUT_CAP` if it is set. Otherwise the cap is `--cap`, and its default is 10. A cap of 0 disables the hook.

`protect-tests` runs before an `Edit` or `Write` tool call (the `PreToolUse` event). It reads `tool_name`, `tool_input.file_path` and `cwd` of the event, and `CLAUDE_PROJECT_DIR` of the environment. `init` writes the command from `[harness] protect-tests` and `[evidence] tests`, with one `--test <glob>` for each glob. `--mode` is `fix-branches` or `always`. An absent or unknown value acts as `fix-branches`. With no `--test`, the hook prints nothing. The steps, in this order:

1. The tool is `Edit` or `Write`.
2. The root is `CLAUDE_PROJECT_DIR`, else `cwd`. `file_path` is absolute and below the root. If it is not, the hook resolves the links of the root and of the folder of the file, and tries again. A path with a `..`, `.` or empty segment stops the hook.
3. The path relative to the root matches a `--test` glob. With no match, the hook stops, and no process starts.
4. For `fix-branches`: `git symbolic-ref --short -q HEAD` gives a name that starts with `fix/`. A detached HEAD stops the hook.
5. `git ls-files --error-unmatch` finds the file in the index. A new file stops the hook.
6. The hook answers `permissionDecision: "ask"` with a reason. The reason has the relative path and, for `fix-branches`, the branch.

Each step that fails gives no output. `git` runs with `GIT_OPTIONAL_LOCKS=0`. The hook stores nothing. In a run that cannot ask the person (`claude -p`), Claude Code denies the edit, and the model reads the reason.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error). `hook` always exits with 0 |
| 1 | `check` found a new error (with `--all`: any error), the preservation check of `mv`, `history` or `index` refused the change, a step of `finish` stopped, or the pass rate of `eval run` is below `min-pass-rate` |
| 2 | Usage error. Also a refusal of `layer add` (it changes nothing), an unknown `--group` of `status` or `stats`, `stats` without `--sdlc`, and an unknown id of `eval run` |
| 3 | Configuration error: no valid `spec-dna.toml`, a different version pin, an unknown base revision (for `log`: an unknown `--since`), or no git repository. For `finish`: also an `accepted` change with no base. For `eval run`: also an invalid eval, an estimate above the cap, a stop by the cap, or a `claude` that is absent |
| 4 | Git or file system error. For `finish`: a write failed, and the message lists the files. For `eval run`: the temporary folder, the copy, the base commit or the result file |
| 5 | A target names no document, section or requirement. For `finish` and `next`: no folder for the slug |

A command that writes to a closed pipe, such as `spec-dna check --json | head -1`, ignores the write error. It ends with its own exit code, and it prints no error. This applies to every command.

## Cache

spec-dna keeps the parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, the cache never goes into a commit, and it stays after a branch switch. On each run, spec-dna parses the files that differ from the git index, and the untracked files. It writes the cache to a temporary file and then renames that file. You can delete the cache at any time, or use `--no-cache`.
