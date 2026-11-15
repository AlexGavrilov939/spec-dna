# Commands

This page gives each command of spec-dna, its options, its behavior and the exit codes. It is for the person who runs the commands or writes a script for them. [quickstart.md](quickstart.md) gives the steps in the sequence of a new project.

## Summary

| Command | Purpose | Section |
| --- | --- | --- |
| `init` | Examine the repository, ask up to 14 questions, write `spec-dna.toml`, the adapters and the harness | [init](#init) |
| `upgrade` | Move a project from 0.4 or 0.5 to 0.6, and move the pin | [upgrade](#upgrade) |
| `check` | Run the controls of the gate | [check](#check) |
| `context` | Print the sections that an agent needs | [context](#context) |
| `show` | Print a document, a section or a requirement | [show](#show) |
| `new` | Write a document from a template, also the 3 documents of a change | [new](#new) |
| `next` | Print the checklist to the next gate of a change | [next](#next) |
| `finish` | Apply the delta of a change, check the evidence, delete the folder of the change | [finish](#finish) |
| `status` | Print each change, its class, its stage and its next gate | [status](#status) |
| `log` | Print the finished changes | [log](#log) |
| `stats --sdlc` | Print the indicators of the playbook | [stats](#stats) |
| `layer add`, `layer list` | Add a layer to the configuration, or list the layers | [layer](#layer) |
| `mv`, `history`, `index` | Move a document, move history sections, generate an index | [mv](#mv), [history](#history), [index](#index) |
| `transcript` | Count the tokens and the reads of an agent transcript | [transcript](#transcript) |
| `hook` | The hooks that a coding agent runs | [hook](#hook) |
| `eval run` | Run the evals with a headless agent | [eval](#eval) |

```sh
spec-dna init                       # detection, up to 14 questions, a preview, then spec-dna.toml, the adapters and the harness
spec-dna init --dry-run             # the preview only (also --non-interactive, --answers <file>, --reconfigure)
spec-dna upgrade --dry-run --pin-file spec-dna-pin.toml   # the diff of the move from 0.5 to 0.6; writes nothing
spec-dna upgrade --pin-file spec-dna-pin.toml             # write the move, then commit it as one change
spec-dna new decision pump-choice   # a new document from the template of its type
spec-dna new intent sensor-timeout --template prd   # the first document of a change
spec-dna next sensor-timeout        # the checklist to the next gate of the change
spec-dna finish sensor-timeout --dry-run   # the diff that finish would write
spec-dna status                     # each change, its class, its stage and its next gate
spec-dna log --since origin/master  # the changes that finished since a revision
spec-dna stats --sdlc               # the indicators of the playbook that git can give
spec-dna layer add interfaces --types interface --moved-from intent   # a new layer in spec-dna.toml
spec-dna layer list                 # each layer, its level and its documents
spec-dna context src/valves.rs      # the sections that cite a code file
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna check --lint               # also list each prose lint warning (any profile)
spec-dna check --staged             # check the git index, as a pre-commit hook does
spec-dna context requirements/valves/closing --path src/valves.rs
spec-dna context requirements/valves/closing --query "close a valve after a sensor timeout"
spec-dna show requirements/valves/closing#sensor-timeout
spec-dna mv requirements/valves/closing requirements/valves/shutoff   # the old id becomes an alias
spec-dna mv requirements/valves/closing#sensor-timeout requirements/sensors/timeouts
spec-dna mv requirements/valves/closing#flush requirements/valves/closing#sensor-timeout requirements/sensors/timeouts   # several keys
spec-dna mv requirements/valves/closing#drip requirements/valves/drip-lines   # a whole section into a new document
spec-dna mv procedures/deploy#rollback procedures/rollback --type procedure   # a section of a procedure into a new procedure
spec-dna history decisions/cache docs/history/cache.md   # the amendment and history sections into a research document
spec-dna index docs/incidents --replace         # a generated index in place of a hand-kept list
spec-dna mv procedures/old procedures/new --stub    # a stub at the old path for tools that read by path
spec-dna transcript ~/.claude/projects/<project> --expect requirements/valves/closing   # a Claude Code session
spec-dna hook context-meter --levels 300000,600000 --subagent 250000   # a hook of a coding agent
spec-dna hook fanout-cap --cap 10   # a hook of a coding agent
spec-dna hook protect-tests         # a hook of a coding agent: ask before an edit of a test file
spec-dna eval run --max-total-usd 10   # the evals, with a cost cap
spec-dna --help                     # commands and options (-h); --version (-V) prints the version
spec-dna mv --help                  # the help of one command
```

Each command accepts `--json`. spec-dna itself makes no network call. `eval run` starts an agent, which uses the network. It reads the documents with local `git` commands.

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

Then `init` asks at most 14 questions, with the found values as defaults. It shows how it applied each answer, the full `spec-dna.toml`, and the diff of each file that it will create or change. It writes them after you confirm.

`--dry-run` shows this preview only. Without a terminal, `init` uses the defaults. `init` never edits a document. [init.md](init.md) gives the details.

`init` writes these adapters:

- one line in `AGENTS.md`, if the file exists or if neither file exists;
- one line in `CLAUDE.md`, unless `CLAUDE.md` imports such an `AGENTS.md`;
- the skill `.claude/skills/spec-dna/SKILL.md` of the Claude Code adapter.

The line never imports another file. Each adapter has the version of its generator and a hash. `check` reports a manual edit or an old version. A second run of `init` updates the adapters. It also replaces the multi-line blocks that earlier versions wrote.

Questions 12 and 13 configure the [agent harness](harness.md). `init` writes it for Claude Code: agent files, the stage skills, `REVIEW.md`, a model-team line in `CLAUDE.md` and a merge into `.claude/settings.json`. The default level is `lean`. Question 14 writes the eval skeleton ([evals.md](evals.md)).

If `spec-dna.toml` exists, `init` updates the adapters, and the harness files if the file has a `[harness]` table. With `--reconfigure`, it asks the questions again. `init` does not run if the pin in `spec-dna.toml` is a different version. The message names `spec-dna upgrade`, which is the command that moves a pin.

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

With a `[change]` policy other than `none`, `check` also runs the lifecycle codes SD905 and SD910 to SD921. They compare the base and the head. Without `--base`, the base of these codes is the merge base with the default branch that `origin/HEAD` names. If git cannot name that branch, `check` prints a note that it cannot decide SD910 locally. SD921 reports a requirement of `rules`, `requirements` or `interfaces` that changed in a diff with no intent folder.

If the pin of the base is before 0.6.0, `check` first runs the plan of [`upgrade`](#upgrade) on the base tree in memory. Then it compares the head with the result, and SD503 does not apply. A base pin before 0.4.0 gives exit code 3.

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

The order of the layers sets the order of the first tier. It also orders the documents of one citation class (cite the file, cite an identifier, or cite a directory). The layers come by level, from 0 up. The layers of one level keep the order of the `[layers.*]` tables in `spec-dna.toml`, or the order of the default graph. In the default graph, `foundation` comes before `rules`. `spec-dna layer list` shows the level of each layer ([The table `layers`](config.md#the-table-layers)).

`context` does not serve the documents of a layer with `context = "on-request"`, unless a target names them. `show` always reads them. `context` counts the documents of these layers that the request reaches, in one line after the listed sections, and lists no file:

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

`--template <name>` selects a template. `new` reads `<dir>/<name>.md` first, where `<dir>` is `[templates] dir` ([config.md](config.md#the-table-templates)). Then it reads the embedded template of that name. For example, `new intent <slug> --template prd` writes a PRD, and `new procedure <slug> --template runbook` writes a runbook. A file in `<dir>` with the name of an embedded template replaces it, including the default template. For example, a file `madr.md` replaces the default template of `decision`.

`new` stops with exit code 2 and writes nothing in these cases:

- an unknown name
- a template of another type
- a project template with no `type` in its front matter
- a name with a path in it

`new` copies the template and makes 4 changes:

- It writes the H1 from the id.
- It sets the first status of the type.
- It removes `was-type` from the copy.
- It removes `type:` where the place gives the type.

`new` writes no `type:` line where the place gives the type. It uses `--root <dir>`, or the first root of that single type, or the first mixed root. A type name of 0.5 is an error (exit code 2) that names the new command. For example, `new contract x` says to use `new interface`, and `new change x` says to use `new intent`.

## next

```text
spec-dna next <slug>
```

`next` prints the checklist from the stage of a change to its next gate. Each line says `done` or `open`. The last line gives the edit that approves the document of the stage, for example `status: approved` in `plan.md`. `next` derives the stage as [status](#status) does. [The change guide](guides/change.md) gives the process.

## finish

```text
spec-dna finish <slug> [--dry-run]
```

`finish` ends a change. It works in the working tree. It applies the delta, checks the evidence, and deletes the folder `docs/intent/<slug>/`. It stops at the first step that fails:

1. It checks that no document outside the intent folder links into it. Such a link breaks when `finish` deletes the folder (SD101 for an id, SD102 for a Markdown link). `finish` lists each link in unlocked text and stops. A reference in the locked text of a decision does not stop `finish`, because that text cannot change.
2. It checks that the intent is `accepted`, that the plan is `approved`, and, for a `higher-risk` change, that the spec is `approved`.
3. It checks each concern (SD919) and each needed decision (SD920).
4. It checks each `CHANGE` and `REMOVE` hash against the requirement text of the head tree (SD914). It checks that each interface document of the `Interfaces` section, or a file that it cites, changed in the diff (SD914).
5. It applies the delta. `ADD` appends a list item. `CHANGE` replaces the sentence. `REMOVE` deletes the list item.
6. It checks the evidence of each requirement that the delta adds or changes (SD916). For a `routine` change, which has no delta, it checks each requirement whose hash differs from the base.
7. It deletes the intent folder.

In step 5, `ADD` appends the item at the end of the last list of requirements in its document. If the document does not exist, `finish` writes a minimal one. It has the front matter with the status `active`, the H1, a `Requirements` section and the `ADD` items. It has no placeholder. It has a `type: requirements` line when the place of the file does not give the type. If a root requires more sections of `requirements`, the new document gets SD305, and the author adds them.

`--dry-run` prints the diff and writes nothing. `finish` does not commit. The author commits the result in the same pull request as the code.

For an intent with the status `closed`, `finish` checks only that status (step 2) and deletes the folder. To drop a started change, a first pull request sets `closed`. A later pull request runs `finish`. `check` reports SD915 for a pull request that deletes a folder with an unapplied delta and a base status other than `closed`.

`finish` is the one command that deletes documents. It deletes only the intent folder, and only after the checks. Git is the archive.

| Exit code | Meaning |
| --- | --- |
| 0 | `finish` completed, or `--dry-run` printed the diff |
| 1 | A check failed |
| 3 | A configuration error |

`finish` uses the merge base with the default branch that `origin/HEAD` names. If git cannot name it, `finish` prints a note that it cannot decide SD910 locally.

## status

```text
spec-dna status [--group <name>]
```

`status` prints each change in the tree: its class, its stage and its next gate. It also lists each open follow-up of an incident. `--group <name>` reads one group of [`areas`](config.md#areas-and-groups). `status` reads the head tree, the base tree and git. It writes nothing.

A change has no stage field. `status` derives the stage from the files, their statuses and git. The stage is the first row of this table that matches, from the top:

| Stage | The change is in this stage when |
| --- | --- |
| Closed | `intent.md` is `closed`. The next step is `finish`, which deletes the folder |
| Plan | `intent.md` is `draft` |
| Design | The class is `higher-risk`, and `spec.md` is absent or `draft` |
| Build | `plan.md` is absent or `draft`, or no file of `Files` differs from the base |
| Test | At least one requirement that the diff adds or changes has no evidence |
| Deploy | Each such requirement has evidence, and the intent folder still exists |
| Maintain | An `incident` has an open item (`- [ ]`) in its `Follow-ups` section |

For a `higher-risk` change, the requirements of the Test row are the `ADD` and `CHANGE` lines of the delta. For a `routine` change, they are the requirements whose hash differs from the base. A `small-fix` has no folder, so it has no stage. The tree no longer has a change that the default branch finished, and [log](#log) lists it. The incident of a Maintain row gets its own intent with `from:`. The author checks the item (`- [x]`) when the follow-up is complete.

## log

```text
spec-dna log [--since <rev>]
```

`log` prints the finished changes. It reads each commit of the history that deleted an `intent.md`. For each one, it prints the slug, the class, the title, the counts of the delta, the commit and its date. `--since <rev>` limits the commits to those after `<rev>`.

`log` counts a change as finished when the parent tree has the status `accepted`. A change with another status in the parent tree is closed. A squash merge writes only the net diff. Thus the folder must reach the default branch in an earlier pull request. If one pull request holds the intent, the approval and the code, the default branch never holds the folder. Then `log` has no entry for it.

## stats

```text
spec-dna stats --sdlc [--group <name>]
```

`stats --sdlc` prints the leading and lagging indicators of the [AI-native SDLC playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook), under its names. It reads git and the files only. It prints `n/a` for an indicator that git cannot give, and the output says why. `--group <name>` reads one group.

| Stage | Indicator | From git |
| --- | --- | --- |
| Plan | Time to a committed `intent.md` | Partly: from the first commit of the folder to the commit that sets `accepted` |
| Plan | Survival rate: the share of intents that are accepted, not closed | Yes |
| Design | Time between the `intent.md` commit and the `spec.md` commit | Yes |
| Design | Requirements rework: `spec.md` commits after the first `plan.md` commit | Yes |
| Build | Time from plan approval to the merged pull request | Yes: from the approval commit to the commit that deletes the folder |
| Build | How often the merged diff matches the committed `plan.md` | Yes: SD913 for the finished change |
| Build | Share of changes that merge from the first pass, and rework cycles for each change | `n/a`: needs pull request data |
| Test | The eval pass rate over time | Yes: from the files of `eval run --out` that the project keeps |
| Test | Time from an incident to a permanent eval | Yes: from the commit of the incident to the commit of the eval |
| Test | Regressions caught in CI against regressions found in production | `n/a`: needs the incident tracker |
| Maintain | Time from a band breach to an `intent.md`, and repeat incidents of one class | `n/a`: spec-dna has no bands, and an incident has no class |
| Deploy, other plays | The other indicators of the playbook | `n/a`: need CI logs, telemetry or an incident tracker |

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
- A layer with no `moves-to` key that has dotted keys or an inline table gives exit code 2. Add `moves-to` to it manually.

If `spec-dna.toml` has no `[layers]` table, the project uses the default graph ([config.md](config.md#the-table-layers)). In this case, `layer add` first writes the tables of that graph. It applies `types`: it drops the layers that hold none of the types, and the edges to them. It then adds the new layer.

The command refuses these cases with exit code 2. It writes nothing, and the file keeps its bytes:

- A cycle in the `moves-to` edges.
- A name that a layer already has.
- A type that another layer has.
- A layer in an edge (`--moves-to`, `--linked-from`, `--moved-from`) that the graph does not have.
- A value that is not valid. Examples are a name with a character other than a letter, a digit, `-` or `_`, and an unknown type. Other examples are a numeric option with a value that is not a number, and a `--dir` that leaves the root. A `--keep` or `--context` value that is not in the list is not valid. `--files` with `--dir` is not valid, and a folder that another layer has is not valid.
- A file layout where the new table has no place, for example `layers = { ... }` as an inline table of the root.

The command reads its own result before it writes it. The result must give the layers that the command expects.

The change goes into `spec-dna.toml` only. Commit it as a change of its own: `check` gives SD503 for a change of `spec-dna.toml` that has other files ([diagnostics.md](diagnostics.md)). With `--json`, the output is one object:

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
- Links to a moved key or section follow it. This rule also applies in accepted decisions, because a link destination is not locked text.
- A decision keeps its locked text. Only its amendment and history sections move, into a research document. `mv` lists the moved amendments that can change the outcome. These amendments are candidates for a new decision that supersedes the old one.

Before `mv` changes a file, a preservation check compares the result with the current tree:

- each requirement has the same hash (without link destinations);
- each requirement id resolves to the same requirement;
- the locked text of an accepted decision does not change;
- there is no new error.

If a check fails, `mv` changes nothing. It names each requirement that the move changes, and it exits with code 1.

## history

```text
spec-dna history <doc-id> [<to>]
```

`history` moves the history sections of a document into a research document. The history sections are the `history-sections`, and the amendment sections of a decision. `history` leaves a pointer for each section, and it adds a backlink. The default destination is `<name>-history.md` next to the document, in a mixed root.

If no layer has the type `research`, `history` stops with exit code 2 and changes nothing. The message gives the command to run: `spec-dna layer add research --types research`. A new project of the small graph has no `research` layer. `mv` does the same for any type of a new document that no layer has.

## index

```text
spec-dna index <dir or file> [--replace]
```

`index` writes a generated index of the documents under a directory. The index has one row for each entry, with its name, title and status. `--replace` replaces a file that a person keeps manually. Then the links to a section of the old file that names an entry go to the entry. `check` reports a stale or manually edited index (SD610).

## transcript

```text
spec-dna transcript <file or dir>... [--expect <id>]... [--format <name>]
```

`transcript` reads agent transcripts. The format comes from an adapter. The adapter `claude-code` reads Claude Code JSONL. For each session and episode, it reports these counts:

- the input, cache-creation, cache-read and output tokens;
- the tool calls;
- the documents and sections that the agent read.

It prints counts and ids only. It never prints the text of a prompt, a command or a file. [transcript.md](transcript.md) gives the details.

## hook

```text
spec-dna hook context-meter [--levels <a,b,...>] [--subagent <n>]
spec-dna hook fanout-cap [--cap <n>]
spec-dna hook protect-tests
```

The hooks use the event names of Claude Code, so they belong to its adapter ([agents.md](agents.md)). A coding agent runs `hook`. The agent starts it on each tool call and sends one JSON object on stdin. spec-dna reads at most 1 MiB of it.

A hook never reads `spec-dna.toml`, never checks the pin and never scans the repository. It reads at most one more file. A hook ignores unknown flags and invalid values. The exit code is always 0. An error gives no output. A hook prints JSON only when the agent must see a message.

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

`protect-tests` runs before an `Edit` or `Write` tool call (the `PreToolUse` event). It asks the person before an edit of a test file that git tracks, when the branch name starts with `fix/`. It answers `permissionDecision: "ask"` with a reason. A new test file does not trigger it, because a fix can add a test.

The generated command contains the test globs of `[evidence] tests`. The hook runs `git` only for a path that matches a test glob. The generated command ends with `2>/dev/null || true`, so an error gives no output and the edit continues. The hook does not see an edit through a `Bash` call. [`[harness] protect-tests`](config.md#parts) selects the branch test (`"fix-branches"`), no branch test (`"always"`) or no hook (`"off"`). An empty `[evidence] tests` list also writes no hook.

## eval

```text
spec-dna eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run]
```

`eval run` runs the evals of `[evals] dir` with a headless agent. It starts the Claude Code command `claude -p` for each run, so it needs the `claude` binary on the `PATH`. spec-dna itself makes no network call, but the agent that it starts does. [evals.md](evals.md) gives the eval file, the steps of a run and the CI job.

| Option | Meaning |
| --- | --- |
| `<id>...` | The evals to run. Without an id, `eval run` runs each eval |
| `--runs <n>` | The runs of each eval. It overrides the key `runs` of an eval, which overrides `[evals] runs` |
| `--min-pass-rate <r>` | The least share of evals that must pass. It sets the value of `[evals] min-pass-rate` for this run |
| `--max-total-usd <n>` | The cost cap of this command. It sets the value of `[evals] max-total-usd` for this run |
| `--out <dir>` | Write a result file with the counts, the costs and the check names only. It has no prompt text and no output text |
| `--dry-run` | Start no agent run |

An eval passes when at least `pass-at` of its runs pass. The suite passes when the share of evals that pass is at least `min-pass-rate`. The command prints a table of the results. `--json` prints the same result.

`eval run` refuses to start when `runs` times `max-budget-usd` of all evals is more than `max-total-usd`. It adds the `total_cost_usd` of each run, and it starts no more runs when the sum is more than `max-total-usd`. A run does not use `--bare`, so each run loads the `CLAUDE.md`, the skills and the hooks of the project. The [exit codes](#exit-codes) of `eval run` are 0, 1 and 3.

## upgrade

```text
spec-dna upgrade [--dry-run] [--layout] [--pin-file <file>]
```

`upgrade` moves a project that pins spec-dna 0.4.0 or later to the version of the binary that you run. It is also the command that moves the pin. It has no `--to` option, because the binary embeds the standard of its own version. A project with an older pin first moves to 0.4.2 and then runs `upgrade`. [The upgrade guide](guides/upgrade-0.5.md) gives the procedure.

| Option | Meaning |
| --- | --- |
| `--dry-run` | Print the diff of each file and write nothing. It needs a clean tree, as a write does |
| `--layout` | Also move the folders of the other 0.5 namespaces into the 0.6 folders |
| `--pin-file <file>` | The file `spec-dna-pin.toml` of the release. `upgrade` copies its sums into `[spec-dna-sha256]`. The binary cannot know its own checksum |

If `spec-dna.toml` has `[spec-dna-sha256]`, `--pin-file` must name the `spec-dna-pin.toml` of the new version. The person verifies that file with `gh attestation verify`. A table with no valid `--pin-file` gives exit code 3.

`upgrade` starts only from a clean tree. A staged change, an unstaged change and an untracked file that git does not ignore give exit code 1. It computes the whole plan in memory, runs the proof, and writes nothing before the proof passes. It does these 7 steps and stops at the first failure:

| Step | What it does |
| --- | --- |
| 1. Check | Refuses a tree that is not clean. Reads the old configuration. Builds the whole plan in memory |
| 2. Configuration | Writes the new keys and renames the old ones: the types, `reviewer` and `spec-writer`. Writes `[change] policy = "none"` and the layers that the types in use need, with no limits. Sets the pin to the new version |
| 3. Front matter | Rewrites each `type:` and each `status:` that changed, and `roots.status` and each `status-map` target. A `proposed` or `accepted` change becomes an `intent` that is `draft` or `accepted`, and a `rejected` change becomes a `closed` intent. A `done` change and a former `note` become `research`. `upgrade` writes `was-type` (`change` or `note`) on each such document. It removes the `status` line of a `done` change. For each root that has no `live-sections`, it writes `live-sections = {}`, so that no 0.5 decision gets a live section by default |
| 4. Harness | If `[harness]` exists, renames the role keys and writes the new parts (`skills`, `review-md` and `protect-tests`) as off. A project at level `full` gets `code-simplifier = "none"`. Deletes an unedited generated file under its old name. Keeps an edited file, and `check` reports it (SD501). Never replaces a file that has no marker: it disables that role |
| 5. Changes and layout | Always: moves each `proposed`, `accepted` or `rejected` change to `intent/<slug>/intent.md` with `git mv`, and writes its old id into `aliases`. With `--layout` only: moves the folders `contracts`, `invariants`, `guides`, `runbooks`, `reference` and `notes` to the 0.6 folders, and writes the old ids into `aliases`. It stops if 2 moves have the same target. A 0.5 spec stays where it is, because only a person can choose its area |
| 6. Proof | Checks that each old id resolves in the new tree and that the hash of each requirement and each section is the same. If the proof fails, `upgrade` writes nothing and exits with code 1 |
| 7. Report | Lists the deletion proposals, each decision with no `Status` section, and the changes of behavior |

`upgrade` also writes the 0.6 adapters, as `init` does. It changes a document body only for the link destinations of a move. It never renames the file stem of a decision, so `legacy-ids` patterns still resolve. The one change of a stem is the move of a change into `intent/<slug>/intent.md`.

When 2 old types with explicit keys merge into one, `upgrade` keeps the weaker value. A second `upgrade` changes nothing. `upgrade` never commits and never deletes a document. It prints the `git add` command at the end.

The proof reuses the preservation check of `mv`. It also checks these items:

- Each document has the 0.6 type that the mapping gives.
- The new configuration parses.
- The layer graph has no cycle.

The report lists these items:

- each `done` or `rejected` change that no document cites, as a proposal to delete, and each decision with no `Status` section;
- each locked decision that cites a migrated change, because `finish` does not stop on a reference in locked text;
- each kept harness file, and each ended role with its model;
- the changes of behavior in the list below;
- the count of each error code of `check --all` on the new tree.

`upgrade` reports these changes of behavior, and the CHANGELOG records them:

- A former `note` is now `research`, with `keep = "forever"`. SD202 allows the deletion of a document that has `was-type: note` or `was-type: change`. This is the only exemption in a `forever` layer.
- `context` listed incidents and notes. It now counts them in one line.
- The context order follows the levels of the graph. `foundation` comes before `rules`.
- The `--json` field `name` of SD405 and SD611 changed. A consumer must read `code`.
- 0.6.0 renames types, keys and commands, so it is a minor release that breaks compatibility.

With `--json`, the output has the fields `from`, `to`, `files`, `proof` and `report`. Each entry of `files` has `path`, `action` (`write`, `move`, `remove` or `keep`) and `diff`, as in `init --json`.

| Exit code | Meaning |
| --- | --- |
| 0 | `upgrade` wrote the changes, or nothing needed a change |
| 1 | The tree is not clean, or the proof failed |
| 2 | A usage error |
| 3 | A configuration error: no `spec-dna.toml`, a pin before 0.4.0 or after the binary, a file that neither schema reads, or `[spec-dna-sha256]` with no valid `--pin-file` |
| 4 | A git error |

To undo an upgrade before the commit, run `git reset --hard`. Then run `git clean -fd`. After the merge, a revert needs 2 pull requests, because SD503 of the old binary rejects a configuration and documents together.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error). For `eval run`: the pass rate is at least `min-pass-rate`. `hook` always exits with 0 |
| 1 | `check` found a new error (with `--all`: any error), or the preservation check of `mv`, `history` or `index` refused the change. Also a failed check of `finish`, a refusal or a failed proof of `upgrade`, and a pass rate of `eval run` below `min-pass-rate` |
| 2 | Usage error. Also a refusal of `layer add`: it changes nothing |
| 3 | Configuration error: no valid `spec-dna.toml`, a different version pin, an unknown base revision, or no git repository. Also, for `eval run`, a stop by the cost cap or an absent `claude` binary |
| 4 | Git or file system error |
| 5 | A target names no document, section or requirement |

A command that writes to a closed pipe, such as `spec-dna check --json | head -1`, ignores the write error. It ends with its own exit code, and it prints no error. This applies to every command.

## Cache

spec-dna keeps the parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, a commit never holds the cache, and the cache stays after a branch switch. On each run, spec-dna parses the files that differ from the git index, and the untracked files. It writes the cache to a temporary file and then renames that file. You can delete the cache at any time, or use `--no-cache`.
