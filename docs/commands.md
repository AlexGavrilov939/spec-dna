# Commands

This page gives each command of spec-dna, its options, its behavior and the exit codes. It is for the person who runs the commands or writes a script for them. [quickstart.md](quickstart.md) gives the steps in the sequence of a new project.

## Summary

| Command | Purpose | Section |
| --- | --- | --- |
| `init` | Examine the repository, ask up to 14 questions, write `spec-dna.toml`, the adapters and the harness | [init](#init) |
| `upgrade` | Move a project that pins 0.4.0 or later to the version of the binary, and move the pin | [upgrade](#upgrade) |
| `import` | Move a project of Spec Kit 1.4.1 or OpenSpec 2.0.0 into the layout of spec-dna, with a proof | [import](#import) |
| `check` | Run the controls of the gate | [check](#check) |
| `evidence stamp` | Stamp the test reports with the hash of the code tree | [evidence](#evidence) |
| `context` | Print the sections that an agent needs | [context](#context) |
| `show` | Print a document, a section or a requirement | [show](#show) |
| `new` | Write a document from a template, also the 3 documents of a change | [new](#new) |
| `next` | Print the checklist to the next gate of a change | [next](#next) |
| `approve` | Set the status of one artifact of a change and commit that file alone (for a person) | [approve](#approve) |
| `accept` | Write the acceptance record of a change, or print its brief | [accept](#accept) |
| `skill list`, `skill show` | Print the 12 skills of the workflow pack, or the text of one skill | [skill](#skill) |
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
spec-dna upgrade --dry-run --pin-file ../spec-dna-pin.toml   # the diff of the move from 0.5 to 0.6; writes nothing
spec-dna upgrade --pin-file ../spec-dna-pin.toml             # write the move, then commit it as one change
spec-dna import speckit --dry-run   # the plan and the proof of the move from Spec Kit; writes nothing (also openspec)
spec-dna import openspec --skip-open   # the move from OpenSpec, with no open change
spec-dna new decision pump-choice   # a new document from the template of its type
spec-dna new intent sensor-timeout --template prd   # the first document of a change
spec-dna next sensor-timeout        # the checklist to the next gate of a change
spec-dna approve sensor-timeout intent   # a person: set the status of the intent and commit that file alone
spec-dna accept sensor-timeout --brief   # the brief for the acceptor: give it, unchanged, as the prompt of the agent
spec-dna accept sensor-timeout --session "$ID"   # the acceptor: write the skeleton of acceptance.toml
spec-dna skill list                 # the 12 skills, their kind, stage, state and default
spec-dna skill show write-plan      # the text of a skill, also if the skill is off
spec-dna finish sensor-timeout --dry-run   # the diff of the finish of a change, and nothing written
spec-dna finish sensor-timeout      # apply the delta, delete the folder, stage the paths
spec-dna status                     # each change, its class, its stage and its next gate
spec-dna status --group operations  # the changes of a group of areas
spec-dna log                        # the finished and the closed changes, from the history of git
spec-dna log --since v0.6.0         # only the changes that finished after a revision
spec-dna stats --sdlc               # the indicators of the playbook, from git and the files
spec-dna stats --sdlc --group operations   # the same for the changes of a group of areas
spec-dna layer add interfaces --types interface --moved-from intent   # a new layer in spec-dna.toml
spec-dna layer list                 # each layer, its level and its documents
spec-dna context src/valves.rs      # the sections that cite a code file
spec-dna context src/valves.rs --group operations   # the same, with no document of an area outside the group
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna check --lint               # also list each prose lint warning (any profile)
spec-dna check --staged             # check the git index, as a pre-commit hook does
spec-dna check --reports --base origin/master   # in CI after the tests: also read the stamped test reports
spec-dna evidence stamp             # after the tests: write <report>.stamp for each test report
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
spec-dna eval run --dry-run         # the evals, their runs and the estimate; no process starts
spec-dna eval run                   # run each eval with a headless agent; exit 1 below min-pass-rate
spec-dna eval run example --runs 1 --out evals/results   # one eval, one run, and a result file
spec-dna transcript ~/.claude/projects/<project> --expect requirements/valves/closing   # a Claude Code session
spec-dna hook context-meter --levels 300000,600000 --subagent 250000   # a hook of a coding agent
spec-dna hook fanout-cap --cap 10   # a hook of a coding agent
spec-dna hook protect-tests --mode fix-branches --test '**/tests/**'   # a hook of a coding agent: ask before an edit of a test file
spec-dna hook protect-approvals --intent docs/intent   # a hook of a coding agent: deny an approval by the agent
spec-dna --help                     # commands and options (-h); --version (-V) prints the version
spec-dna mv --help                  # the help of one command
```

Each command accepts `--json`. spec-dna reads the documents with local `git` commands, and it makes no network call. Only `eval run` starts a program that uses the network: `claude`, the agent of the Claude Code adapter.

A target is a document id, an alias such as `ADR-12`, `<id>#<anchor or key>`, or a repository path.

## init

```text
spec-dna init [--non-interactive | --answers <file>] [--reconfigure] [--dry-run] [--root <dir>] [--approver <e-mail>]
```

`init` examines the repository and finds these items:

- the document roots and their types, the ids, the requirement keys and the statuses;
- the languages of the headings;
- the vendored, generated and large reference directories;
- the budgets, the stack, the tests, the agent files, CI, the hooks and the default branch.

Then `init` asks at most 14 questions, with the found values as defaults. Questions 12 to 14 set the harness and the evals. It shows how it applied each answer, the full `spec-dna.toml`, and the diff of each file that it will create or change. It writes them after you confirm.

`--dry-run` shows this preview only. Without a terminal, `init` uses the defaults. `init` never edits a document. [init.md](init.md) gives the details.

`--approver <e-mail>` gives an approver for `[change] approvers`. The flag can repeat, and `--approver=<e-mail>` is the same. A new configuration and `--reconfigure` read it. The e-mail of an agent or a bot, such as `noreply@anthropic.com`, gives exit code 2 and writes nothing. The answers key `approvers` has the same effect. Without a flag, an answer or a person to list, `init` writes `approval = "base"` and a note ([init.md](init.md#the-approvers)).

With a change policy other than `none`, the preview has a note with the `CODEOWNERS` line for the specs. `--json` has the field `codeowners`. `init` never writes the `CODEOWNERS` file.

`init` writes these adapters:

- one line of 400 bytes or fewer in `AGENTS.md`, if the file exists or if neither file exists. It names the 5 rules of the disciplines, `spec-dna skill show <name>` and the approval rule;
- one line in `CLAUDE.md`, unless `CLAUDE.md` imports such an `AGENTS.md`;
- the skill `.claude/skills/spec-dna/SKILL.md` of the Claude Code adapter.

The line never imports another file. Each adapter has the version of its generator and a hash. `check` reports a manual edit or an old version. A second run of `init` updates the adapters. It also replaces the multi-line blocks that earlier versions wrote.

Questions 12 and 13 set the [agent harness](harness.md). `init` writes it for the Claude Code adapter: agent files, the skills of the workflow pack, `REVIEW.md`, a model-team line in `CLAUDE.md` and a merge into `.claude/settings.json`. The default level is `lean`.

Question 14 sets the evals ([evals.md](evals.md)). The answer `skeleton` writes the table `[evals]`, `evals/check.sh` and an example eval. For the CI answer `github`, it also writes the workflow `.github/workflows/agent-evals.yml`. It starts no run. A new project gets `skeleton`. An upgraded project keeps `none`.

If `spec-dna.toml` exists, `init` updates the adapters, and the harness files if the file has a `[harness]` table. With `--reconfigure`, it asks the questions again. `init` and `init --reconfigure` do not run if the pin in `spec-dna.toml` is a different version. They write nothing and exit with code 3. For a pin from 0.4.0 up to the version of the binary, the message names `spec-dna upgrade`.

At the end, `init` prints the bytes that the adapters add to the prompt prefix of each agent request (`--json`: `prefix_bytes`, when `spec-dna.toml` already exists). It also prints the `git add` command for the files that it wrote.

## check

```text
spec-dna check [--base <rev>] [--all] [--lint] [--staged] [--reports]
```

`check` runs the controls of ADR 0001 on the working tree, or with `--staged` on the git index. With `--base`, the base is the merge base of `<rev>` and `HEAD`. Without `--base`, the base is `HEAD`.

- Without `--all`, `check` reports only the diagnostics that are new against the base.
- With `--all`, it uses the same base, but it reports each diagnostic and fails on any error.
- Without `--base`, `check` also reads the untracked documents that git does not ignore.
- With `--base`, it reads only tracked files, as CI does.
- With `--staged`, the head tree is the git index: the tree that the next commit records. `check` reads `spec-dna.toml`, the documents, the test files, the code and the harness files (SD501, SD502, SD504, SD505) from the index. Untracked files and edits that are not staged do not count. `--staged` works with `--base` and `--all`, and the base does not change. `check` stops with exit code 4 for an unmerged path in the index, and with exit code 3 if `spec-dna.toml` is not in the index.

- With `--reports` and a `[evidence] reports` list that is not empty, `check` also reads the test reports from the disk, also when git ignores them ([Test reports](config.md#test-reports)). It reports a missing, stale or unreadable report (SD613 to SD615), and a requirement whose tag binds to no passed test case (SD616 with `--all`, SD617 for a change that the diff finishes). It also reports a requirement of a finished change that passed fewer times than `[evidence] reliable-runs`, when `[evidence] reliable-for` covers the change (SD620). `--reports` does not combine with `--staged` (exit code 2), because the hook of a commit needs no report. With no key, or with an empty list, `--reports` changes nothing. `--reports` is an option of `check` only. The codes of the reports are off in `lite`.

The hook that `init` writes runs `spec-dna check --staged`. A commit records the index, so a check of the working tree can pass while the committed tree fails (SD101).

`check` reads the pin of the base. This also applies without `--base` and with `--staged`, where the base is `HEAD`. These rules apply:

- If the base pins a version from 0.4.0 to before 0.6.0, `check` plans the upgrade of the base tree in memory. The plan is the plan of `spec-dna upgrade` without `--layout`. Then `check` compares the head with that plan. It writes nothing to git. Thus the first pull request after an upgrade keeps the gate, and the 0.5 documents of the base give no new diagnostic. [The upgrade guide](guides/upgrade-0.5.md#open-the-first-pull-request) gives the steps.
- If the base pins a version before 0.4.0, `check` exits with code 3. The message says to move the base to 0.4.2 first.
- If `upgrade` cannot plan the base, `check` exits with code 3. The message names the base, its pin and the reason.
- SD503 does not apply if the base pins a version before 0.6.0. One pull request can hold the configuration and the documents. A later change of the configuration keeps SD503, also a change of the pin inside 0.6.

`check` also compares the requirements of the lifecycle base and the head. If a requirement of a `forever` layer changed and no file of the `intent` folder changed, it gives SD921. This check runs only with a `[change] policy` other than `none`, and only if git can name the lifecycle base.

With a `[change] policy` other than `none`, `check` finds the lifecycle base. This base is the merge base of `--base` (or, without it, of `origin/HEAD`, or, if the repository has no `origin/HEAD`, of the local branch that `[change] base` names) with `HEAD`.

Without `--base`, the base that decides which diagnostics are new stays `HEAD`. The lifecycle base is only for the change codes that compare the head with a base ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). Their diagnostics are always new. With `--staged`, they diff the git index against the lifecycle base.

These rules apply to the lifecycle base:

- `--json` has the field `change_base`: the commit id of this base, or `null` if git cannot name it. With `none`, the field is not in the output, and `check` reads no second tree and no history for it.
- If there is no base, the text output prints this note on stderr:

  ```text
  note: no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base
  ```

  Then `check` runs only the checks of one tree.
- The refs that `check` reads for this base are `origin/HEAD` and the branch of `[change] base`, in this order. It never guesses a local `main` or `master`. If `[change] base` names a branch that does not exist, and there is no `origin/HEAD`, the note is ``no base for the change checks: no origin/HEAD, and the branch `<b>` of [change] base does not exist; pass --base <rev>``. This is not an error, because a CI clone often has no local branch. If the branch exists but has no common history with `HEAD`, the note is ``no base for the change checks: the branch `<b>` of [change] base has no common history with HEAD; pass --base <rev>``.
- If the lifecycle base pins a version before 0.6.0, it counts as no lifecycle base. Git cannot give a diff from the planned base. `change_base` is `null`, and `check` runs the checks of one tree. The text output prints this note on stderr:

  ```text
  note: the base `main` pins `0.5.0`, before 0.6.0, so the change checks have no lifecycle base
  ```

  The note names the base: the value of `--base`, `HEAD`, or the first 12 characters of the merge base. With the policy `none`, `check` prints no note.
- A shallow clone can fail to find the merge base. Then `change_base` is `null`.
- With `--base` and an `origin/HEAD`, `check` also reads the merge base of `origin/HEAD` and `HEAD`. If the merge base of `--base` and `HEAD` is an older ancestor of it, the change checks and SD503 use the newer merge base, and `change_base` is that commit. With the policy `none`, only SD503 uses the newer merge base, and no note prints. The text output prints this note on stderr:

  ```text
  note: `--base v0.7.0` is older than the merge base of HEAD and origin/HEAD (<12 characters>), so the change checks use that merge base: the commits between them are merged pull requests, and a squash merge keeps no approval commit
  ```

  The commits between the 2 bases are merged pull requests. A squash merge keeps no approval commit, so an older base gave SD910 (reason 4) for work that was approved. The document checks and the choice of the new diagnostics keep `--base`. `next`, `status` and `finish` follow the same rule, with no note. Only `origin/HEAD` starts this rule, not `[change] base`. A repository with no `origin/HEAD` keeps the older base.
- For a stacked pull request, pass the branch below it as `--base`. SD910 trusts `--base`.

The checks of one tree are SD905 (the delta part), SD912, SD914, SD919, SD920 (the absent decision), SD922, SD923, SD924 and SD925. They need no base. SD923 and SD924 skip an `approved` plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0, with a note on stderr ([SD923](diagnostics.md#checks-of-the-change-lifecycle)). SD914 is an error only for a spec that the diff touches against the lifecycle base. Without that base, it is a warning.

A change can remove or rename a path or an identifier that a document cites. Then `check` gives a warning that names the document (SD606, SD607). In an accepted decision, an amendment section that cites the new path or identifier clears these warnings.

The text output gives the errors first, then SD606 and SD607, then the other warnings. Each prose lint code (SD801 to SD805) gets one line with a count. Another warning code with more than 10 diagnostics also gets one line with a count. At the end, the output gives the fix of each listed code once.

`--lint` lists each prose lint warning. It also runs the lint in a profile that has it off (`lite`, `standard`), and it overrides a `lint = "off"` rule. `--lint` does not run the lint when `language` is not English. The findings are warnings, so they never fail the gate. Without `--all`, `--lint` lists the new findings only.

An accepted decision cannot change its locked text (SD405), so `check` gives no SD101 and no SD102 for a link or a reference there. After `finish` deletes an intent, such a link is dead. With `--all`, `check` prints one info line for each of these locked links, after the diagnostics and before the count line:

```text
docs/decisions/timer.md:8: info: the locked text links `../intent/dose/intent.md`, which no longer exists; add an amendment that names `../intent/dose/intent.md` and points to the change `dose` (finished in commit 9b446b2c2bf3 on 2026-11-16)
```

The line is not a diagnostic. It has no code, no severity and no rule, it is not in the counts, and it does not change the exit code. If the target was in the folder of an intent, the line names the change, and the commit that `log` gives for it ("finished in" or "closed in"). If the history has no such commit (a shallow clone, or a deletion that is not committed yet), the line says "the change `<slug>` (`spec-dna log` names its commit)". For another target, it says "the new place of the text, or the reason for its removal". Add an amendment to the decision that names the target. An amendment section clears the line when a code span or a word of prose with a `/` in the section equals the target as written (with no `#` fragment), the id or the path of the place that is gone (from the root of the documents or of the repository, with or without `.md`), or, for a change folder, when the slug of the change is in a code span. A link to the target in the amendment gives SD102, because that text can change. Without `--all`, `check` prints no such line. A reference to a document that exists, with an absent section, is not a locked link.

The JSON output lists each diagnostic. It also lists each document with its number of recognized requirements. The field `locked_links` is always in the output, with or without `--all`. It is a list of the locked links, in the order of path and line. Each entry has `path`, `line`, `target`, `pointer` (the text after "points to" in the info line) and `commit` (the full id of the commit that the pointer names, or `null`). The list is empty if there is no locked link. The field `approvals` is always in the output too, after `locked_links`. It lists the approval commits that cleared a gate of SD910, in the order of the walk. Each entry has `commit` (the full id), `path` (the artifact that the commit approved) and `author` (the author e-mail). The list is empty with `[change] approval = "base"`, and also when no gate needed an approval commit ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). The profile sets the severity of each code ([profiles.md](profiles.md)). [diagnostics.md](diagnostics.md) gives each code.

## evidence

```text
spec-dna evidence stamp [--json]
```

`evidence stamp` binds the test reports to the code tree ([Test reports](config.md#test-reports)). It reads each file that `[evidence] reports` matches, and it writes `<report>.stamp` next to it:

```text
spec-dna-stamp 1
tree 3f2a9c1b7e0d4a11
report 9c1b7e0d4a113f2a
```

`tree` is the hash of the code tree now. `report` is the hash of the bytes of the report. Run the command after the tests, in the same job, so the stamp is exact. It prints `stamped <n> report(s) at tree <hash>: <p> pass, <f> fail, <s> skip`. With `--json`, the output has `stamped`, `tree`, `pass`, `fail`, `skip` and `reports`: for each report, its `path`, `format` (`junit` or `lines`) and counts.

It reads the whole set before it writes. If one report is unreadable, nothing gets a stamp. A report that git tracks is a refusal too, because a committed report can be a hand-written one. Exit codes: 0 stamped; 1 a report is unreadable, git tracks a report, or no file matches (nothing got a stamp); 2 usage (no `stamp`); 3 configuration (no `[evidence] reports`, a glob of it that is absolute or has `..`, or a different pin); 4 git or file error (also a folder of reports that exists and cannot be read).

## context

```text
spec-dna context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--group <name>] [--include-inactive]
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

`--group <name>` serves no document of an area outside the group. A group is a name of `[groups]` and its areas ([Areas and groups](design/0.6.0.md#areas-and-groups)). A document has an area in 2 cases: it is a `requirements` document under `<folder of the layer>/<area>/`, or it is an `intent` with `areas`. A document with no area, such as a rule, a decision or a procedure, is in every group. A target that names a document of another area serves it. `context` counts the documents that it does not serve in one line after the listed sections:

```text
<!-- group operations: 2 documents of another area not served; name a document as target to serve it -->
```

The line says `1 document` for one document. It is not in the output if the count is 0. The `--json` output has no count. An unknown group, or a file with no `[groups]` table, is a usage error (exit code 2).

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
- the evidence of each requirement: its `verified-by` paths and the full `covers:` tags of the tests. The partial tags (`covers: partial <doc-id>#<key>`) follow after `partial:`, as in `- loans#fines: none; partial: tests/test_fines.py:1`. `--json` gives each item of `evidence` the field `sources` (the full evidence) and the field `partial` (a list of `<path>:<line>`, empty if there is none).

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
- The area of a new path is its first folder under the folder of the layer of `requirements`. If it equals a name of `areas` in lowercase ASCII and differs from it in bytes, `new` stops with exit code 2 and writes nothing. The message names the area and the name of `areas`. With an empty `areas`, `new` checks no area.

`--template <name>` selects a template. `new` reads `<dir>/<name>.md` first, where `<dir>` is `[templates] dir` ([config.md](config.md#the-table-templates)). Then it reads the embedded template of that name. For example, `new intent <slug> --template prd` writes a PRD, and `new procedure <slug> --template runbook` writes a runbook. A file in `<dir>` with the name of an embedded template replaces it, including the default template. For example, a file `madr.md` replaces the default template of `decision`.

`new` stops with exit code 2 and writes nothing in these cases:

- an unknown name
- a template of another type
- a project template with no `type` in its front matter
- a name with a path in it

`new` copies the template and makes 4 changes:

- It writes the H1 from the id. For `intent`, `spec` and `plan`, it writes the placeholder `{{The title of the change.}}` in place of a title from the slug. `check` gives SD306 until the author writes the title, and `status` shows the placeholder until then.
- It sets the first status of the type.
- It removes `was-type` from the copy.
- It removes `type:` where the place gives the type.

`new` writes no `type:` line where the place gives the type. It uses `--root <dir>`, or the first root of that single type, or the first mixed root. A type name of 0.5 is an error (exit code 2) that names the new command. For example, `new contract x` says to use `new interface`, and `new change x` says to use `new intent`.

## approve

```text
spec-dna approve <slug> intent|spec|plan [--json]
```

`approve` is the command of a person for an approval commit. It sets the `status` line of one artifact of the change `<slug>` to its approval value (`accepted` for an intent, `approved` for a spec or a plan), and it commits that file alone. The commit changes the `status` line and no other byte, so it is an approval commit of the branch rule ([config.md](config.md#approval-in-the-branch)). The command makes no check of the person: run it in a terminal of your own. The hook `protect-approvals` ([harness.md](harness.md#spec-dna-hook-protect-approvals)) denies it for an agent.

The steps, in this order. A refusal gives exit code 1, and nothing changes:

1. The pin is the version of the binary, and the project has a layer of the type `intent`. Else exit code 3.
2. The folder `<intent folder>/<slug>/` and the file of the artifact exist. Else exit code 5.
3. The file is tracked, and it has the same text in `HEAD`, in the index and in the working tree. Commit the text first.
4. The status is `draft`. If it is the approval value already, the command prints that and ends with exit code 0, with no commit. Another status is a refusal.
5. An intent needs a valid `class`. Set the class in a commit of its own first.
6. The earlier artifact has its approval value: a spec needs an `accepted` intent. A plan needs an `approved` spec for a higher-risk change, and an `accepted` intent for a routine change.
7. With `approval = "branch"`, the e-mail of the author (`git var GIT_AUTHOR_IDENT`, so `GIT_AUTHOR_EMAIL` counts) is in `[change] approvers`. The compare ignores case. With `approval = "base"`, the command does not check the author, because the merge is the approval.
8. The command replaces the value of the `status` line in place. It keeps the indent, the quotes, a comment after the value and the line end (`\n` or `\r\n`).
9. The command runs `git commit --only -m "docs: approve the <artifact> of <slug>" -- <path>`. Other staged changes stay staged and out of the commit. The command passes no `--no-verify` and no `--no-gpg-sign`, so the hooks and the signing of the person apply. A project whose commit rules reject this message gets exit code 4 with the text of git, and the person makes the commit by hand.
10. If the commit fails, the command writes the old text back and exits with code 4.

The output is `approved <path> in <12 characters of the commit> (author <e-mail>)`. With `--json`: `{"slug", "artifact", "path", "status", "commit", "author"}`. `commit` is the full id, or `null` when the artifact already had the approval value.

Exit codes: 0 committed, or already approved; 1 a refusal of steps 3 to 7; 2 usage (the slug or the artifact name is bad, or an option is unknown); 3 configuration; 4 git or file error; 5 no folder or no file.

## accept

```text
spec-dna accept <slug> [--role <name>] [--session <id>] [--base <rev>] [--json]
spec-dna accept <slug> --brief [--role <name>] [--base <rev>] [--json]
```

`accept` is the command of the acceptance record. The acceptor is an agent that did not write the code of the change `<slug>`. It tries to break the change, and it writes one record in the file `acceptance.toml` in the folder of the change. The command works in each profile. The codes SD927 to SD935 decide only whether `finish` and `check` need a record ([diagnostics.md](diagnostics.md#the-acceptance-record-sd927-to-sd935)). A change that has a plan needs a record in `standard` and `strict`.

Both forms refuse (exit code 1) while the folder of the change, except `acceptance.toml`, or `spec-dna.toml` has an edit that `HEAD` does not have, in the index or in the working tree. An untracked file in the folder counts. The message says: commit the work, then accept. Both forms also refuse (exit code 1) while a code path that `scope` matches has an edit that `HEAD` does not have (a staged edit, an unstaged edit or a deleted file), or is untracked. The message says: commit or remove the path, then accept. Thus the brief is a function of the committed state, and the brief, the compare of `finish` and the compare of `check` read the same files. A path that `scope` does not match, and a file that is not code, never causes a refusal. Both forms also refuse a change that has no `plan.md` or no `intent.md`.

**The brief** (`--brief`). The form writes no file. It prints one text that is a function of the repository state only:

1. The first line is `spec-dna-brief 1 <slug> <role>`. The role is `acceptor` when `--role` is absent. A role is `[a-z][a-z0-9-]*`, at most 40 characters, or the exit code is 2.
2. The fixed text of the brief format 1 (about 3 KB): the task of the acceptor and the rules of the record.
3. The files `intent.md`, `spec.md` (if it exists) and `plan.md` of the folder, whole, as `HEAD` has them. Each starts with a line `=== <file> ===`.
4. `scope`, `paths` and `tree`, as the record writes them (see below), and the sections `invariants`, `deviations` and `roles` when they have an item.
5. The last line is `brief <hash>`: the first 8 bytes of SHA-256 of the text before it, as 16 hexadecimal digits. The hash covers the bytes, so an edit of the spacing of a file changes it.

The builder gives the whole output, unchanged, as the prompt of the acceptor, and writes no word of its own. The acceptor copies the hash of the last line into `brief` of its entry. The hash makes the input of the acceptor recomputable. It does not show that the acceptor got the text, because anyone can compute the brief.

**The skeleton** (no `--brief`). The command writes `<folder>/acceptance.toml` with the computed fields, one `[[acceptor]]` entry and one `[[proof]]` entry for each requirement that the change adds or changes. The acceptor fills in the rest. The file has no `acknowledged-by` entry:

```toml
format = 1
change = "sensor-timeout"
scope = ["src/sensors.rs", "src/valves/**", "tests/test_valves.py"]   # the plan scope and [change] shared-paths
paths = ["src/sensors.rs", "tests/test_valves.py"]                    # the code paths of the diff in scope
tree = "3f2a9c1b7e0d4a11"                                             # the hash of the content of paths
result = "pending"                                                    # accepted or rejected
acknowledged-by = []

[[acceptor]]
role = "acceptor"                # a role name of [owners], or "acceptor"
brief = "pending"                # the hash on the last line of the brief
session = "6c0e2f1a"             # a claim: the value of --session, or "unknown"
agent = "unknown"                # a claim: the agent id of a subagent, or "unknown"
result = "pending"

[[proof]]
requirement = "requirements/valves/closing#sensor-timeout"
state = "tagged"                 # reliable, passed-once, verified, tagged, owed or none
cases = []

[[attempt]]                      # the acceptor adds these
acceptor = 1                     # the number of its [[acceptor]] entry, from 1
text = "Sent a NaN value after the timeout."
outcome = "held"                 # held or broke

[[finding]]
acceptor = 1
severity = "nit"                 # important or nit
path = "src/valves.rs"
line = 88                        # optional
text = "The constant TIMEOUT has no unit in its name."
```

- `scope` is the paths and globs of the plan (the items of `Files` and the `Files:` of the tasks, as SD913 reads them) and the globs of `[change] shared-paths`, sorted.
- `paths` is the code paths of the diff of the change against the base that `scope` matches, sorted. A code path is a path of the code tree ([config.md](config.md#test-reports)). A deleted code path stays in the list. Another open change does not change the list, because only the paths of `scope` count. When the diff has no such path, as in a pull request after the code merged, `paths` is the code paths that changed since the commit that added the plan. The base is the merge base of `--base` (default: `origin/HEAD`, then `[change] base`) and `HEAD`.
- `tree` is the hash of the content of `paths` only: one line `<mode> <object id>\t<path>` for each path, or `absent\t<path>` for a path that the tree does not have. A Markdown edit and a change to another path change nothing.
- `[[proof]]` lists the requirements that the change adds or changes (the requirements of SD916), with their proof state ([config.md](config.md#proof-levels-and-the-proof-debt)). A requirement that is not in the tree yet, such as the target of an `ADD` line, has the state of its `covers:` tags. A change with no delta lists the requirements whose hash differs from the base.
- With `--session`, `session` is the value. Without it, or with an empty value, it is `unknown`. `agent` is always `unknown`: no tool gives a subagent its agent id.
- A second run adds one `[[acceptor]]` entry, with the same role or another, and keeps each other byte of the file. It prints `acceptor: <n>`: write this number as `acceptor` in each of your attempts and findings.
- If the code changed since the record (the `scope`, the `paths` or the `tree` differ), the command refuses: delete the record and accept again. An uncommitted `acceptance.toml` alone is no refusal.

A valid record has these rules (SD928): the file is TOML with `format = 1`, no unknown key and at most 16,384 bytes; `change` is the slug and `tree` has 16 hexadecimal digits; `paths` is sorted, has no duplicate, and holds code paths that `scope` matches; no value is `pending`; each `[[acceptor]]` has `brief`, `session` and `agent`; each `[[attempt]]` and `[[finding]]` names the number of an `[[acceptor]]` entry, and each acceptor has at least one attempt; a `not-affected` or `violated` answer has a `reason`; each finding has a path from the root and a text; the top `result` is `accepted` only when each acceptor accepted, no finding is `important`, no invariant is `violated` and no deviation is `rejected`. No other table is valid.

The output of the skeleton form is `wrote <path>`, then `acceptor: <n>`, then a line `next:`. With `--json`: `{"slug", "path", "acceptor", "role", "session", "created"}`. With `--brief --json`: `{"slug", "role", "brief", "text"}`, where `brief` is the hash.

What the record does not prove. `session` and `agent` are claims, so the record cannot show that the acceptor is not the builder. It cannot show that the acceptor read the brief, or that it tried hard. The person who acknowledges the record and the evals of the skill are the root of the trust.

Exit codes: 0 written or printed; 1 a refusal (an edit that `HEAD` does not have, no `plan.md` or `intent.md`, or changed code); 2 usage (a bad slug or role, `--session` with `--brief`); 3 configuration or a different version pin; 4 git or file error; 5 no folder for the slug.

## next

```text
spec-dna next <slug> [--base <rev>]
```

`next` prints the checklist to the next gate of the change `<slug>`, the folder `<intent folder>/<slug>/`. The first 2 lines give the change, its class, its stage and its base. An intent with no valid class (the field is absent, empty or invalid) has no class: the first line then says `(class not set)`. The other lines are the checklist, at most 10. Each line says done (`[x]`) or open (`[ ]`), and ends with `[skill: <name>]`. The last line is the edit or the command that ends the stage, for example ``set `status: accepted` in docs/intent/dose-limit/intent.md``. The output has at most 12 lines.

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
| `plan` | `intent.md` is absent, or its status is not `accepted`, or its `class` is not set | Set `status: accepted` in `intent.md`, or write the `class` |
| `design` | The class is `higher-risk`, and `spec.md` is absent or not `approved` | Set `status: approved` in `spec.md` |
| `build` | `plan.md` is absent or not `approved`, or no file of `Files` differs from the base, or a task of `Order` is open (not `[x]`, and with no reason in `Skip:`, and not a task that closes with `finish`). A plan that lists no files has no test of the diff, so only an open task keeps it in `build` | Set `status: approved` in `plan.md`, change a file of `Files`, or do the next open task |
| `test` | A requirement that the change adds or changes has no full evidence | Add a `covers:` tag in a test file |
| `deploy` | Each such requirement has evidence | Run `spec-dna finish <slug>` |

- In the stage `deploy`, `next` names the item `accept` after `review` and before `finish`, with the skill `accept`, only for a change that needs an acceptance record ([accept](#accept)). The rule is the rule of `finish` and `check`: the acceptance is on (not in the profile `lite`), and the plan is new (a plan that a pin before 0.8.0 added, or a repository with no lifecycle base, has no item). The item is open (`[ ]`) while the folder of the change has no record. It is done (`[x]`) when the record is valid, fresh and `accepted`. It is open again when the record is stale because the code changed after the acceptance, when the record says `rejected`, and when the record is not valid. A change that needs no record has the lines of 0.7.1, byte for byte. The last line stays the line of `finish`.
- An intent with no `class` stays in the stage `plan`, because the class decides the next artifact. If its status is `draft`, the last line asks for the class first and then for the status. If its status is `accepted`, the last line asks for the class, and says that `check` reads the change as `routine` until then. `check` does not change: it reads a change with no class as `routine`.
- A line that asks to approve a spec or a plan appears only when the file exists. With no file, the last line asks to write it with `spec-dna new`.
- With `[change] approval = "branch"`, each line that asks for an approval also says that an approver commits the `status` line alone. A commit that changes more than the `status` line is no approval commit.
- If the plan lists no path because the items of `Files` are not paths, the last line says so and shows the first such item. A path is a code span, or a plain path ([Files and paths](types.md#files-and-paths)).
- A routine change with an approved plan that lists no files has no step for the diff. The plan states this with one `Files` item that starts with `none` ([Files and paths](types.md#files-and-paths)). The line `[x] the plan lists no files: the change edits no file` replaces the step of the files. The stage passes `build` when each task of `Order` is closed, or when `Order` has no task, with or without a base. A higher-risk change keeps the step of the files. A paragraph under `Files` states the same when its first word is `none`, followed by `:`, `.` or the end of the paragraph (`None.`, `None: research only.`).
- If the `Files` section of the plan has a paragraph and no list item, and the paragraph is not in that `none` form, the plan lists no path. The line is "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`" ([A paragraph under `Files`](types.md#a-paragraph-under-files)). With an `approved` plan, it is the last line. With a `draft` plan, it comes first among the lines of the plan, and the line of the approval stays the last. `status` shows it in the same way.
- A file of `Files` differs from the base if the diff against the base has a path that a `Files` item matches. The diff is the diff of the working tree. A file of the folder of the change does not count. With no base, the diff is unknown. Then a change with an approved plan stays in `build`.
- A change that has a delta uses its `ADD` and `CHANGE` lines as the requirements to prove. A `REMOVE` line needs no evidence. A change with no delta line uses the requirements whose hash differs from the base. If the diff has a changed file of another change, only the documents that `Files` lists count. This is the rule of step 5 of `finish`.
- A requirement has evidence in the cases of [evidence](config.md#evidence). A `covers:` tag can name a requirement of an `ADD` line before `finish` writes it.
- A partial tag (`covers: partial <doc-id>#<key>`) is not evidence ([A partial tag](config.md#a-partial-tag)). Suppose a partial tag names a requirement that has no full evidence. Then the summary line of `next` says "no full evidence" in place of "no evidence", and it ends with "; N of them have a partial covers: tag only" (N is a number). The item of that requirement names its id, and it says "has a partial covers: tag only", the place of the tag, and "add a test that covers all of it". With no partial tag, the lines are the lines of 0.6.
- The change that the default branch finished is no longer in the tree, so it has no stage. `spec-dna log` lists it.
- The stage Maintain is not the stage of a change. `status` lists an open follow-up of an incident ([status](#status)).

In the `build` stage, `next` also names the next open task of the plan, also if the plan is a draft. It gives the id, the text, the files and the verification of the task. The task list is the `Order` section of `plan.md` ([Tasks of a plan](types.md#tasks-of-a-plan)). `next` counts the tasks that are done and the tasks that are skipped (a task with a reason in `Skip:`, which is not needed and is closed). With a skipped task, the line of the tasks ends with `, <n> not needed`. A task whose text names `spec-dna finish <slug>` with the slug of the change closes with `finish`: it is not open, and the line of the tasks gets ``, <n> for `finish` `` after the part `not needed`. A task of a person is an open task that an agent does not tick ([Tasks of a plan](types.md#tasks-of-a-plan)). Its line ends with `[person]` in place of the skill. When each open task is a task of a person, and no step of an agent is open (for example a file of `Files` to change), the last line of `next` is `wait for a person: task <id> is a task of a person; an agent does not tick it`, and the stage stays `build`. If a step of an agent is open, it stays the last line. A draft plan keeps the approval as its last line. When each task that does not close with `finish` is closed, the stage leaves `build`, and its last line names `spec-dna finish <slug>`. The first open task of an agent is the next task. A task of a person is the next task only if each open task is a task of a person.

### Skills

Each line of `next` has the name of the skill for its step. The list is fixed and has 10 names, in this order: `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify`, `accept` and `finish`. With `--json`, each item has the field `skill`. An item of a person (an open task of a person, and the line that waits for it) has the field `"person":true` too, and its text line ends with `[person]` in place of the skill. The skill of such an item stays `execute`, but an agent does not do the item.

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
| The stage `deploy`, the acceptance record of a change that needs one | `accept` |
| The stage `closed`, or a change that is ready for `finish` | `finish` |

A line that is done has the skill of its own step. `spec-dna skill show <name>` prints the text of a skill, also for an agent with no skill folder. `next` never names `debug` and `post-mortem`: a failure and an incident are events, not stages of a change.

### Base and output

The base is the merge base of `--base` (default: `origin/HEAD`, then `[change] base`) and `HEAD`. `next` prints it. If git cannot name a base, `next` prints a note. The note is `note: no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base`. The stage then stops at `build`, because the diff against the base is unknown.

With `[change] policy = "none"`, `next` says that the change process is off and reads no git data. It exits with 0.

With `--json`, the output is one object:

```json
{"slug":"dose-limit","class":"routine","stage":"build","base":"3f2a9c1b7e0d4a11...","items":[{"done":true,"text":"the intent is accepted","skill":"brainstorm"}],"next_task":{"id":"t2","text":"Close the valve","files":["src/valve.py"],"verify":"`pytest`"},"next":"change a file that `Files` of ...","notes":[]}
```

`base` is a SHA, or `null`. `class` is `null` if the intent has no valid class, and with the policy `none`. `next_task` is `null` outside the `build` stage, and it is `null` if the plan has no open task. A draft plan can have one. `next` is the text of the last item. An item of a person has `"person":true`; the other items have no such field. `notes` lists the notes, for example the note for no base. `next` warns on a pin mismatch, as `status` does.

Exit codes: 0 done; 2 usage; 3 configuration, or a `--base` that is not a commit; 4 git; 5 no folder for the slug.

## skill

```text
spec-dna skill list [--json]
spec-dna skill show <name> [--json]
```

`skill` prints the skills of the workflow pack ([harness.md](harness.md#the-workflow-pack-and-reviewmd)). It reads no document, writes nothing and starts no program. The text of each skill is a constant of the binary, so the pin fixes it.

`skill list` prints one line for each of the 12 skills, in the order of the table of the pack. A line has 5 columns: the name, the kind (`stage` or `event`), the stage (`-` for an event skill), the state in the project and the default.

```text
brainstorm  stage plan   on  off
clarify     stage design on  on
execute     stage build  off off
debug       event -      off off
```

- The state comes from `[harness] skills` of `spec-dna.toml`: `on` if the list has the name, else `off`. A project with a `spec-dna.toml` and no `[harness]` table has each skill `off`. With no `spec-dna.toml`, the state is `-`.
- The default follows the rule of [ADR 0023](adr/0023-default-skills-from-the-release-table.md): it reads the 8 skills of the design (the 7 of D2 and `accept`), the verdict of the skill in the table of the release, and a constant of the release. With the constant `true`, the default is `on` only for a skill with the verdict `on`. With `false`, it is also `on` for a skill of the 8 whose verdict is not `off`. `init`, `upgrade` and SD507 use the same default.
- A configuration error gives exit code 3. A pin of another version gives the warning of `next` on stderr.

`skill show <name>` prints the body of one skill, with no front matter and no marker. It is the text that `init` writes into `.claude/skills/spec-dna-<name>/SKILL.md`. It works for a skill that is off, and with no `spec-dna.toml`, so an agent with no skill folder can read the text of the step that `next` names. The 12 names are the only names. The adapter skill `spec-dna`, a folder name such as `spec-dna-review` and the names `intent`, `spec` and `plan` of 0.6 give exit code 2.

With `--json`, `list` prints one object, and `show` prints one object with the body:

```json
{"skills":[{"name":"brainstorm","kind":"stage","stage":"plan","state":"on","default":"on"}]}
{"name":"write-plan","kind":"stage","stage":"build","body":"When to use: ..."}
```

`stage` is `null` for an event skill. `state` is `null` with no `spec-dna.toml`.

Exit codes: 0 done; 2 usage or an unknown name; 3 configuration.

## finish

```text
spec-dna finish <slug> [--dry-run] [--base <rev>]
```

`finish` ends a change ([Finish](design/0.6.0.md#finish)). It applies the delta of the spec of `<intent folder>/<slug>/` to the requirements, checks the evidence, and deletes the folder. The author runs it in the pull request of the code. The review then sees the code and the new current truth in one diff. It stops at the first step that fails. A stop writes nothing.

1. Step 0: no task of `Order` in `plan.md` is open (a task that is not `[x]` and has no reason in `Skip:`), and no document outside the folder links into it. An open task stops `finish` and the stop lists the open tasks (the first 10 with their text, then the ids of the others). An open task whose text names `spec-dna finish <slug>` with the slug of the change is not open: `finish` closes it, and its `Hours:` counts. If another open task holds `spec-dna finish` with no slug or with another slug, the reason ends with ``; a task closes with `finish` only when its text names `spec-dna finish <slug>` `` (with the real slug). A duplicate id of a task is not counted, as for `next` ([types.md](types.md#tasks-of-a-plan)). A change with the intent status `closed` is abandoned, so its tasks do not count. The link check reads a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` and `from`. It skips a stub, a generated index and the locked text of an accepted decision. It lists each reference and stops, also in the profile `lite`. `check` runs the same search for each change folder and reports a reference as SD612 ([diagnostics.md](diagnostics.md#links-drift-and-quality-sd601-to-sd612)), so the author sees the stop before `finish` runs.
2. Step 1: the intent is `accepted` and the plan is `approved`. The spec is `approved` for a `higher-risk` change and for a spec that has a delta line. An intent with the status `closed` runs steps 0 and 1, and then `finish` deletes the folder. It applies no delta.
3. Step 2: no open concern (SD919), no open question (SD922), and each decision of `Decisions` is `accepted` (SD920).
4. Step 3: each delta line is valid against the tree (SD914). A `CHANGE` or `REMOVE` hash is the hash of the requirement now. The delta is the delta of the base tree: an approved delta must not change. The pull request changes each interface document of `Interfaces`, or a file that it cites.
5. Step 4: it applies the delta in memory.
    - `ADD` writes a list item after the outermost requirement item that holds the last requirement item. So a nested last requirement gives a sibling of the top-level items. The item has the indent and the marker of that item (an ordered list gets the next number, counted after the removals).
    - With no item, `ADD` writes into the section `Requirements` (`Rules` for a rule), and it adds the section if needed.
    - `finish` writes a document that does not exist at `<root>/<id>.md`. The document has the status `active`, the H1 from the id, a `Requirements` section (`Rules` for a rule) and the items. It has the line `type: requirements` only where the place gives no type.
    - `CHANGE` replaces the sentence on one line. It keeps the key, the nested lists and the `aliases` and `verified-by` markers (also a marker inside a sentence that wraps).
    - `REMOVE` deletes the lines of the item. If the item holds a nested requirement, the delta must have a `REMOVE` line for each nested key too. Else `finish` stops at step 4 and names the key.
    - `finish` stops at step 4 if the result would drop a requirement that no `REMOVE` line names.
    - A document id that ends in `.md` gives SD914: write the id without `.md`.
    - A relative link of a delta line moves to the path of the target document.
    - The result keeps CRLF, a BOM and the final newline of each file.
    - A line that the tree applies already is skipped, so a second run changes nothing.
6. Step 5: each requirement that the delta adds or changes has evidence (SD916). A partial `covers:` tag alone is not evidence. For a change with no delta, each requirement whose hash differs from the base has it. The result may add no error. The only exception is SD305 for a new document. With `[evidence] reports`, step 5 has 2 more parts, and the numbers of the steps stay. Step 5.1 runs first: each test report must be present, readable and fresh (SD613 to SD615). Step 5.4 is the check of the evidence: a requirement that has a tag also needs a bound test case that passed in the reports (SD617). With `[evidence] reliable-for`, a requirement of a covered change that passed fewer times than `[evidence] reliable-runs` stops step 5.4 with SD620 ([Proof levels and the proof debt](config.md#proof-levels-and-the-proof-debt)). A proof debt (`proof-owed`) is no evidence for SD916, SD617 and SD620. Run the tests and `spec-dna evidence stamp` before `finish`, in the same checkout ([Test reports](config.md#test-reports)). The steps are off in `lite`.
7. Steps 5.2 and 5.5: the acceptance record ([accept](#accept)). They run in `standard` and `strict`, and in `lite` only with a rule that turns the group `acceptance` on. A change needs a record when its plan is new: the branch added it, or the commit that added it on the base pinned 0.8.0 or later ([diagnostics.md](diagnostics.md#the-acceptance-record-sd927-to-sd935)). A closed change and a change with no plan need none. For a plan of an older pin, `finish` prints on stderr ``note: `<plan>` was added when spec-dna.toml pinned <pin>, before 0.8.0, so `finish` needs no acceptance record``, and it runs no step 5.2. Step 5.2 comes after step 5.1 and before the evidence (step 5.4). It reads `<folder>/acceptance.toml` of `HEAD`, and it stops at the first code that is not off: SD927 (no record), SD928 (the record is not valid, or the `brief` of an entry is not the brief of its adding commit), SD929 (the code that the record lists changed), SD930 (the result is `rejected`), SD931 (2 acceptors with one session and one agent) and SD933 (too few acceptors). Step 5.5 comes last. It moves each record that the folder has, also in `lite` and for a plan of an older pin, where no rule needs it. With no record and no need, it writes nothing. It moves the record to `.spec-dna/accepted/<slug>.toml`, or to `<slug>-2.toml`, `<slug>-3.toml` and so on if the file exists. The archive file is one of the files that `finish` writes, so `--dry-run` lists it, and `finish` stages it. If git ignores the path, step 5 stops before it writes, and the message names the rule (`git check-ignore -v`). The numbers of the steps and the value 6 of `--json` on success stay as they are.

If no step stops, `finish` writes all the files in one pass. Then it deletes the tracked files of the folder and stages each path with `git add -A`. It makes no commit: the author reads the diff and commits it with the code.

`finish` adds the label `Hours:` (the estimate) of each task of the plan that is not a duplicate and is not skipped, and the label `Actual:` (the real hours) of each task that is not a duplicate, also a skipped one, because its time was spent ([types.md](types.md#tasks-of-a-plan)). It prints the line `Hours: <low>` or `Hours: <low>-<high>` (also with `--dry-run`), with no zero after the last digit (`3.5`, `4`). If a task has `Actual:`, the line is `Hours: <estimate>, actual <n>`, or `Hours: actual <n>` if no task has an estimate. A plan with no `Actual:` prints the line as before. A task with a number counts as the range of equal ends, so a plan with `Hours: 1-2` and `Hours: 1.5` gives `Hours: 2.5-3.5`. The next line says to add this line to the commit message of the change and to keep it in the squash message, because `finish` makes no commit. No task and no value of `Hours:` stops `finish`. It prints these notes:

- A task has a value that is no number and no range, for example `soon` or `3-1`: the note ``the task `<id>` has the value "<value>" in `Hours:`, which is no number of hours and no range; no total``, one for each such task, and no total of the estimates. The line `Hours: actual <n>` still prints if a task has `Actual:`.
- A task has no `Hours:`, and no value is invalid: the note ``the tasks `<id>`, `<id>` have no `Hours:`; the total leaves them out``. The total of the other tasks still prints. For one task, the note is ``the task `<id>` has no `Hours:`; the total leaves it out``.
- A task has an `Actual:` that is no number (a range is not a number), for example `soon` or `1-2`: the note ``the task `<id>` has the value "<value>" in `Actual:`, which is no number of hours; no actual total``, one for each such task. The line has no actual part.
- A done task has no `Actual:`, and another task has one: the note ``the done tasks `<id>`, `<id>` have no `Actual:`; the actual total leaves them out``. For one task, the note is ``the done task `<id>` has no `Actual:`; the actual total leaves it out``. A skipped task needs no `Actual:`.
- No task has `Hours:` or `Actual:`: no line and no note.

`log` and `stats --sdlc` read the line from the commit that deletes `intent.md`. Nothing checks the value: the line is a record. The section `Hours` of a plan is not read.

These rules apply to `Hours:`:

- The label `Hours:` of a task has a number or a range ([Tasks of a plan](types.md#tasks-of-a-plan)). The number can end with `h` (`1.5h`). The line `Hours:` of a commit message has no `h`: `Hours: 3.5h` is not read, and the value is absent. The same holds for the number after `actual`.
- A number of the line `Hours:` of a commit message has up to 7 digits before the `.` and up to 2 after it. A longer number is not read. A range of the line is `<a>-<b>` with `a` not above `b`, and spaces or tabs can be around the `-`.
- `log` reads the commit that deletes `intent.md`. For a merge-commit merge, that is the commit of the branch, so the line must be in that commit and not only in the message of the merge commit.

These rules apply to the files:

- `finish` writes a generated index again if it lists a deleted file, so `check` gives no SD610.
- `finish` needs the folder to be tracked and equal to `HEAD`. An untracked file in the folder, or an edit that is not committed, stops it.
- It also stops if it must write a file that has an edit that is not committed, because it stages each file that it writes.
- `finish` writes and deletes no path that has a symbolic link, also as the last part of the path, and no path outside the repository.
- It checks all the paths before it writes the first file, in `--dry-run` too.

The base is the merge base of `--base` (default: `origin/HEAD`, then `[change] base`) and `HEAD`. An `accepted` change needs it. A `closed` change needs none. The base of step 3 does not depend on `[change] policy`, and the stops do not follow the profile or a `[rules]` entry.

`--dry-run` writes nothing. It prints the diff of each file that `finish` would write, and `remove <path>` for each file that it would delete. With `--json`, the output is one object:

```json
{"slug":"dose-limit","ok":true,"dry_run":true,"step":6,"diagnostics":[],"files":[{"path":"docs/requirements/valves.md","action":"write","diff":"--- a/..."},{"path":"docs/intent/dose-limit/intent.md","action":"remove","diff":""}],"hours":{"low":2.5,"high":3.5},"actual":4}
```

`hours` is the total of the `Hours:` labels of the tasks as `{"low","high"}` (equal for a number), or `null`. `actual` is the total of the `Actual:` labels as a number, or `null`. A stop has `"ok":false`, the `step` that failed, a `reason`, and the `diagnostics` of the stop. `files` is then empty, and the output has no `hours` and no `actual`. [diagnostics.md](diagnostics.md#stops-of-finish) lists the stops.

`finish` is the one command that deletes documents. It deletes only the files of the intent folder, and only after the steps. Git is the archive.

## status

```text
spec-dna status [--group <name>] [--base <rev>]
```

`status` lists each change of the working tree. For each change it gives the class, the stage, the statuses of the intent, spec and plan, the areas and the next gate. It also gives the progress of the tasks (`done/total`, `skipped <n>` if a task is skipped, `finish <n>` if a task closes with `finish`, and the id of the next open task, with `[person]` after it if it is a task of a person), and the token `partial <n>` when `<n>` requirements of the change have a partial `covers:` tag and no full evidence (`status --json`: the integer field `partial`, 0 if there is none). The line `next:` of `status` ends with `[person]` when the last item is an item of a person. A started change (an `accepted` intent) that has had no commit for `stale-days` days is `stale` (SD911). `status` also gives the number of started changes and the limit `max-started`. When a requirement of an active document has a proof debt and no evidence, and a code of the group `proof` is on (not in `lite`, unless a rule turns one on), `status` ends with the line `proof: 3 owed (manual 1, environment 2)`: the count, then the count of each reason that has a debt ([Proof levels and the proof debt](config.md#proof-levels-and-the-proof-debt)). `status --json` has the object `proof` at its end, only when a requirement has a `proof-owed` marker (a project with no marker gets the JSON of 0.7.1): `{"owed":3,"by_reason":{"manual":1,"environment":2,"external":0,"pending":0}}`. `status` reads no report, so it counts the debts only. With the policy `none`, `status` has no `proof`.

Then `status` lists each open follow-up of an incident. A follow-up is an item `- [ ]` in the `Follow-ups` section. The list gives the incident id, the path, the line and the text, and it ends with `[skill: post-mortem]`. This is the stage Maintain. The follow-ups are not part of a group, because an incident has no areas.

```text
policy three-class, base 3f2a9c1b7e0d, 1 started (max 3)
dose-limit routine build intent accepted spec - plan approved tasks 1/3 next-task t2 areas valves
  next: change a file that `Files` of docs/intent/dose-limit/plan.md lists: none differs from the base
follow-up incidents/valve-stuck: docs/incidents/valve-stuck.md:13 Add an alarm for a stuck valve. [skill: post-mortem]
```

`--group <name>` lists only the changes that are in the group. A group is a name of `[groups]` and its areas ([Areas and groups](design/0.6.0.md#areas-and-groups)). A change is in the group in one of these cases:

- An area of the group is in the `areas` of its intent.
- An area of the group is the area of a delta target.

The area of a delta target is the first folder under the folder of the layer of `requirements`. For example, the target `requirements/valves/closing` is in the area `valves`. The counts of started changes are the counts of the project. An unknown group is a usage error (exit code 2). [context](#context) has the option `--group` too.

The base and the policy `none` work as for [next](#next). The output has `notes` for the no-base note. With `--json`, the output is one object:

```json
{"version":"0.6.0","policy":"three-class","base":"3f2a9c1b7e0d4a11...","started":1,"max_started":3,"notes":[],"changes":[{"slug":"dose-limit","path":"docs/intent/dose-limit","title":"Dose limit","class":"routine","statuses":{"intent":"accepted","spec":null,"plan":"approved"},"approved_in":{"intent":"base","spec":null,"plan":"base"},"stage":"build","next":"change a file that ...","stale":false,"areas":["valves"],"tasks":{"done":1,"skipped":0,"finish":0,"total":3,"next_task":{"id":"t2","text":"Close the valve","files":["src/valve.py"],"verify":"`pytest`"}},"partial":0}],"follow_ups":[{"incident":"incidents/valve-stuck","path":"docs/incidents/valve-stuck.md","line":13,"text":"Add an alarm for a stuck valve.","skill":"post-mortem"}]}
```

A status is `null` if the file is absent. `class` is `null` if the intent has no valid class, and the text shows the token `none`. `approved_in` gives the source of the approval of each artifact: `"base"` (the base tree has the same status), `"branch <short sha>"` (the approval commit that set the status, with `[change] approval = "branch"`) or `null` (the status is not `accepted` or `approved`, the file is absent, or no approval counts). The short sha has 12 characters. With `approval = "branch"`, the text adds `(base)` or `(branch <short sha>)` after each `accepted` or `approved` status. With `approval = "base"`, the text does not change. Each item of `follow_ups` has `"skill":"post-mortem"`. `max_started` is `null` if the layer has no limit. `areas` lists the areas of the intent, then the areas of the delta targets. `tasks.done` counts the tasks with `- [x]` and no reason in `Skip:`, `tasks.skipped` counts the skipped tasks, and `tasks.finish` counts the open tasks that close with `finish` (a number). `tasks.next_task` is the next open task whatever the stage is, and it is `null` if the plan has none.

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

For each change, `log` gives the date of the commit (UTC), the result, the slug and the class. It also gives the counts of the delta lines of its spec (`add`, `change` and `remove`), the hours, the commit and the title. The counts and the class come from the parent tree too. The hours are the last line `Hours: <number>`, `Hours: <a>-<b>`, `Hours: <number or range>, actual <n>` or `Hours: actual <n>` of the message of that commit. Spaces or tabs can be around the `,`. The line can be in any paragraph, because a host can write the messages of a squash as a list. A line with other text after the number is not such a line. `finish` prints the line, and the person or the agent that commits puts it in the message. The text shows `hours <estimate> actual <n>`, for example `hours 5-9 actual 4.5` (`4.5` or `5-9` for the estimate). In `--json`, `hours` is `{"low","high"}`, with equal ends for a number, and `actual` is a number. An absent value is `-` in the text and `null` in `--json`: a line with no `actual` part gives `actual -`, and the line `Hours: actual <n>` gives `hours -`. `stats --sdlc` reads the estimate only. A script that splits the text of `log` on spaces must read the new pair. One git process reads the messages of all the listed commits. A line that has no valid form is not counted. The newest change comes first.

```text
2024-04-05 finished dose-limit higher-risk add 1 change 2 remove 0 hours 3.5 actual 4 3f2a9c1b7e0d Dose limit
2024-03-18 closed old-idea routine add 0 change 0 remove 0 hours - actual - 91be40d27c5a Old idea
```

`--since <rev>` limits `log` to the commits after `<rev>` (`<rev>..HEAD`). A `<rev>` that is not a commit is exit code 3.

A change has an entry only if the folder came to the default branch in an earlier commit. Assume that a pull request holds the intent, the approval and the code together. A squash merge then brings it to the default branch, and the default branch never had the folder. Then the change has no entry.

A rename of the folder out of the `intent` folder reads as a deletion, because git cannot follow the file outside of its path limit. A rename inside the `intent` folder is not a deletion. The entry comes later, under the new name.

A move of `intent.md` out of the `intent` folder does not end a change. `log` gives no entry for it, because the file still exists. For each deleted `intent.md`, `log` asks git for the diff of the commit against its first parent, with no path limit. It runs one git process for each deleted file. The deletion is a move out only if all 3 conditions are true:

1. The diff renames the deleted `intent.md` to a new path.
2. The rename is `R100`, which means that the new file has the same text. Else the front matter `aliases` of the new file holds the old id of the intent. `spec-dna mv` and `upgrade` write it. `finish` never writes it.
3. The same diff has no other record that deletes or renames a file of the change folder.

A plain `git mv` of `intent.md` is `R100`, so it is a move. A `git mv` of a whole folder out of the `intent` folder renames each file of the folder. If the folder holds a spec or a plan too, condition 3 is false: the folder left the layer, and the change ended. If the folder holds only `intent.md`, the move of the folder is the same single `R100` rename as a plain `git mv`, so it is a move. A commit of `finish` that removes the spec and the plan ends the change in the same way, also if it adds a near copy of the intent with the old id in its `aliases`. A near copy with no alias is not `R100`, so it does not meet condition 2.

2 cases stay as they were in 0.6.1, and both read as a move. In both, the folder holds only `intent.md`, so the commit has no other record of the folder and `log` gives no entry for that change. The first case is a commit of `finish` that adds a near copy with the old id in its `aliases`. The second case is a `git mv` of the whole folder out of the `intent` folder. A move that changes more than half of the text is no rename for git, so it ends the change.

In a shallow clone, `log` can miss the changes that the clone does not have. It prints the note `this is a shallow clone; older changes may be missing`. With `--json`, the field `shallow` is `true`. With `[change] policy = "none"`, `log` says that the change process is off and lists nothing. It warns on a pin mismatch, as `status` does.

With `--json`, the output is one object:

```json
{"changes":[{"slug":"dose-limit","class":"higher-risk","title":"Dose limit","result":"finished","delta":{"add":1,"change":2,"remove":0},"hours":{"low":3.5,"high":3.5},"actual":4,"commit":"3f2a9c1b7e0d4a11...","date":"2024-04-05"}],"shallow":false}
```

Exit codes: 0 done; 2 usage; 3 configuration, or a `--since` that is not a commit; 4 git.

## stats

```text
spec-dna stats --sdlc [--group <name>]
```

`stats --sdlc` prints the leading and the lagging indicators of the playbook under the playbook names. It reads the history of git and the committed files. It reads no working tree, no pull request data and no clock. The one exception is the count of the proof states: with `[evidence] reports` and a code of the group `proof` on, `stats` also reads the report files from the disk. `stats` without `--sdlc` is a usage error (exit code 2).

The command rebuilds the life of each change from the commits that touched its `intent.md`, `spec.md` and `plan.md`:

- A life ends at the commit that deleted the `intent.md`. A change that is still in the history of `HEAD` is open.
- The order of the commits is the order of the ancestry, not the order of the dates.
- A span that would be negative is not counted.
- The durations use the committer dates.
- A rename of the folder inside the `intent` folder keeps the life under the new name.
- A move of `intent.md` out of the `intent` folder does not end the life, because the file still exists. `stats` drops that life: it is not finished, closed or open. The [`log`](#log) section gives the 3 conditions of a move out, and the 2 cases that read as a move although the folder left the layer.

| Play | Kind | Indicator | Value |
| --- | --- | --- | --- |
| Plan | leading | Time from first conversation to a committed `intent.md` | The median days from the first commit of the folder to the first commit that sets `accepted`. The conversation is not in git. |
| Plan | lagging | Survival rate | The percent of intents that are `accepted` rather than `closed`. A deleted intent counts as in `log`. A draft that is still open does not count. |
| Design | leading | Time between the `intent.md` commit and the `spec.md` commit | The median days from the first commit of the intent to the first commit of the spec. |
| Design | lagging | Requirements rework after build starts | The mean of the `spec.md` commits after the first `plan.md` commit, for each change that has a spec and a plan. |
| Build | leading | Time from plan approval to the merged pull request | The median days from the first commit that sets the plan `approved` to the commit that deleted the folder. It counts finished changes. |
| Build | leading | Share of changes that merge from the first implementation pass | `n/a`: it needs pull request metadata. |
| Build | lagging | How often the merged diff still matches the committed `plan.md` | The percent of finished changes whose diff has no path that SD913 reports for the scope of the plan. |
| Build | lagging | Hours of work per finished change | The median of the hours of the finished changes. The hours are the estimate of the last line `Hours: <number>`, `Hours: <a>-<b>` or `Hours: <estimate>, actual <n>` of the message of the commit that deleted the `intent.md` (the commit that [`log`](#log) lists). The actual hours of the line are not read here. A range counts as its midpoint. Unit `hours`. The count is the number of finished changes that have such a line, and the note gives the total (a range `<low>-<high>` if a change has a range). A change with no line does not count. |
| Build | lagging | Rework cycles per change | `n/a`: it needs pull request metadata. |
| Test | leading | The eval pass rate over time | The `pass_rate` of the newest result file in `<[evals] dir>/results`, in percent. `n/a` without `[evals]`. |
| Test | leading | How long a production incident takes to become a permanent eval | The median days from the commit that added an incident to the commit that added the eval that its `Eval` section names. `n/a` without `[evals]`. |
| Test | lagging | Regressions caught in CI against regressions found in production | `n/a`: it needs the incident tracker. |
| Test | lagging | Share of requirements with partial evidence only | The percent of the requirements that need evidence (an `active` `requirements`, `interface` or `rule` document) in the tree of `HEAD` that have a partial `covers:` tag and no full evidence. Unit `percent`. The count is the number of such requirements, and the note gives the partial and the bare count. `n/a` if the tree has no active requirement, or if `[evidence] tests` is empty. |
| Maintain | leading | Time from band breach to an `intent.md` in the triage queue | `n/a`: spec-dna has no bands. |
| Maintain | lagging | Repeat incidents of the same class | `n/a`: an incident has no class field. |
| Deploy | both | The other indicators of the playbook | `n/a`: they need CI logs, telemetry or an incident tracker. |

The commit that "deleted the folder" is the first-parent commit of the deletion. For a merge commit it is the merge commit, and for a squash merge it is the squash commit. A rebase merge shows only its last commit. For such a change the time and the diff can be smaller than the real ones.

The diff of "the merged diff still matches the plan" is the diff of that commit against its first parent. The scope is the scope of SD913 for the approved plan of the parent tree. It has the folder, the `Files` patterns, the `Files:` paths of the tasks, and the documents that the delta and `Interfaces` name. These rules apply to the scope:

- `stats` reads the last tree of the life in the history.
- A squash merge keeps no commit of the branch, so `stats` cannot see a plan that the pull request added or edited.
- When one commit finishes more than one change, the scope is the union of their scopes, as SD913 reads the changes of one diff.
- `stats` finds the document of an id by its place under each root, and it does not read aliases.
- A generated index is in scope.
- A change with no plan is not counted.

A shallow clone can miss old changes, so each indicator that it computes from the history has this note in `reason`. The eval pass rate reads only the tree of `HEAD`, so it has no note. An indicator with no data has no value, and the reason says what is missing.

`--group <name>` counts only the changes that are in the group. A group is a name of `[groups]` and its areas. A change is in the group by the rule of [status](#status). The areas come from the last tree of the change. An unknown group is a usage error (exit code 2).

With `[change] policy = "none"`, `stats` says that the change process is off and gives no indicator. It warns on a pin mismatch.

`stats` also counts the requirements of the active documents of `HEAD` for each proof state ([Proof levels and the proof debt](config.md#proof-levels-and-the-proof-debt)). The text has the line `proof: reliable 0, passed-once 1, verified 2, tagged 40, owed 1, none 3`. The line appears only when a code of the group `proof` is on (not in `lite`, unless a rule turns one on), and a requirement is owed or `[evidence] reports` is set. A project that uses neither gets the output of 0.7.1. With `--json`, the object `proof` follows `indicators`, only when a requirement has a `proof-owed` marker, or when a code of the group `proof` is on and `[evidence] reports` is set. It has one count for each state: `reliable`, `passed_once`, `verified`, `tagged`, `owed` and `none`. With `[evidence] reports` and a code of the group `proof` on, `stats` reads the reports from the disk and binds the tags, as `check --reports` does, to tell `reliable` and `passed-once` from `tagged`. In `lite`, `stats` reads no report. A report that is missing, stale or unreadable gives no finding in `stats`: its tags are `tagged`. A folder of reports that cannot be read is no report, and it is no error. With the policy `none`, the output has no `proof`.

The 2 leading Test indicators, the share of partial evidence and the count of the proof states read the tree of `HEAD`, not the working tree, so a file that is not committed does not count. The 2 leading rows are `n/a` with the reason "the configuration has no [evals] table" if the configuration has no `[evals]`. `--group` does not change them, because a result file, an incident and a requirement belong to no change. The share of partial evidence needs no `[evals]`, and a shallow clone adds no note to it.

The pass rate reads each `*.json` file that is directly in `<[evals] dir>/results`.

- A file is a complete result when it has a string `started`, a number `pass_rate` from 0 to 1, and `"stopped": null`.
- A run that the cost cap stopped has no pass rate, so its file is not complete.
- The value is the `pass_rate` of the file with the latest `started`, in percent.
- The count is the number of complete files.
- The note names the `started` of the newest file and the number of files that the indicator skipped.
- If there is no complete file, the reason says to run `spec-dna eval run --out <[evals] dir>/results` and to commit the file.
- The project keeps the files, so they are the history of the pass rate.

The time pairs each `incident` document of `HEAD` with an eval file.

- The eval file is the first path `<[evals] dir>/<id>.json` that is a file of `HEAD`. The path is in the section `Eval` of the incident, also in its subsections.
- The case and the level of the heading `Eval` do not matter.
- A link such as `../../evals/x.json` or `/evals/x.json` names the path.
- A path that continues a longer path on its left, such as `docs/evals/x.json`, does not count.
- The span is the days from the oldest commit that added the incident to the oldest commit that added the eval file. The order of the ancestry decides which commit is the oldest.
- A span below 0 does not count.
- The value is the median, and the count is the number of incidents with a span.
- The text line says "result files" and "incidents" for the count, not "changes".

The text output has a summary line, then one line for each indicator. A value that has a note gets a second line that starts with `note:`. An indicator that git cannot give shows `n/a` and its reason:

```text
4 changes in the history: 2 finished, 1 closed, 1 open
Plan lagging: Survival rate: the share of intent.md files that are accepted rather than closed: 66.7 percent (3 changes)
Build leading: Share of changes that merge from the first implementation pass: n/a (it needs pull request metadata)
```

With `--json`, the output is one object with the 16 indicators of the table, in its order. The table of 0.6 had 15 rows, so a script reads a row by its `name`, not by its place:

```json
{"indicators":[{"play":"Plan","kind":"lagging","name":"Survival rate: the share of intent.md files that are accepted rather than closed","value":66.7,"unit":"percent","count":3,"reason":null},{"play":"Build","kind":"lagging","name":"Rework cycles per change","value":null,"unit":null,"count":null,"reason":"it needs pull request metadata"}]}
```

- `value` is a number with 1 decimal, or `null` for an indicator with no value. The row of the hours has up to 2 decimals and no zero after the last digit (`2.25`, `3.5`, `6`).
- `unit` is `days`, `percent`, `commits per change` or `hours`.
- `count` is the number of changes that the value counts. For the 2 leading Test indicators, it is the number of result files and incidents. For the share of partial evidence, it is the number of requirements that need evidence. It is `null` for an indicator that git cannot give.
- `reason` is `null` for a full value. Each `null` value has a reason.

Exit codes: 0 done; 2 usage, an unknown group, or no `--sdlc`; 3 configuration; 4 git.

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
- A `types` list that `--types` must grow and that the command cannot edit in place, such as a list on many lines.
- A value that is not valid. Examples are a name with a character other than a letter, a digit, `-` or `_`, and an unknown type. Other examples are a numeric option with a value that is not a number, and a `--dir` that leaves the root. A `--keep` or `--context` value that is not in the list is not valid. `--files` with `--dir` is not valid, and a folder that another layer has is not valid.
- A file layout where the new table has no place, for example `layers = { ... }` as an inline table of the root.

The command reads its own result before it writes it. The result must give the layers that the command expects.

If `spec-dna.toml` has a top-level `types` list, `--types` also appends each type that the list lacks. The command writes it in the same write as the layer, and each comment stays. Without a `types` key, or with an empty list, each type is valid already, and the command changes no `types` list. A list that the command cannot grow in place, such as a list on many lines, gives exit code 2. The file keeps its bytes. Add the types to the list by hand, and run the command again.

The change goes into `spec-dna.toml` only. The output ends with `next:` lines, one action for each line. There are 3 lines at most:

1. `next: commit spec-dna.toml as a change of its own (SD503)`. `check` gives SD503 for a change of `spec-dna.toml` that has other files ([diagnostics.md](diagnostics.md)).
2. `next: run spec-dna init to update the agent skill, which lists the types of the project`. The command writes this line only when the new types change the types of the project, and `.claude/skills/spec-dna/SKILL.md` exists.
3. `next: run spec-dna new <first type> <id> to write the first document of the layer`. The command writes this line only when the layer has a type. A layer with `--files` holds top files, and its line has no `<id>`.

When the command appended types, a line that starts with `added the types` comes before the `next:` lines.

With `--json`, the output is one object. `types_added` lists the appended types. `next` lists the `next:` lines without their prefix.

```json
{"layer":"interfaces","level":0,"path":"spec-dna.toml","wrote_implied_graph":false,"moved_from":["intent"],"types_added":[],"next":["commit spec-dna.toml as a change of its own (SD503)","run spec-dna new interface <id> to write the first document of the layer"]}
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

These rules apply to the move of a whole document:

- If the new id is an alias of the moved document, the move drops that alias, and the old id becomes an alias. The output has `(alias dropped: <alias>)`, and `--json` has the field `alias_dropped` (the alias, or `null`). A file at the new path, the id of another document or an ambiguous alias stops the move with exit code 2.
- In a mixed root, the new place can give the type ([Place gives the type](types.md#place-gives-the-type)). Then `mv` removes the `type` line. The move can change the type of the document, because the person chose the place. The output names the change: `type: intent -> roadmap (the place gives the type)`.
- If the document has requirements and the type of the new place holds none, `mv` stops with exit code 2 and changes nothing. Move the requirements first, with the second form.
- An accepted decision keeps its type. `mv` stops with exit code 2 if the new place gives another type.
- If the new type has no status, `mv` removes the `status` line. If the status is not a status of the new type (after `status-map`), `mv` writes the first status of the new type, and the output names it: `status: accepted -> draft`. All other fields stay.
- If the new place gives no type, and the old type came from the old place, `mv` writes the `type` line, unless `default-type` of the new root is that type. A move never changes a type with no line in the output.
- In a root of one type, `mv` changes no `type` and no `status` line.
- The area of the new path is its first folder under the folder of the layer of `requirements`. If it equals a name of `areas` in lowercase ASCII and differs from it in bytes, `mv` stops with exit code 2 and changes nothing. The message names the area and the name of `areas`. This also applies to the new document of a split. With an empty `areas`, `mv` checks no area.
- After the move, `mv` removes each folder that the move left empty, from the deepest folder to the highest. A folder with a file stays, and so does a folder with an ignored file. The root folder of the document stays. With `--stub`, the stub keeps the folder.

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

`index` writes a generated index of the documents under a directory. The index has one row for each entry, with its name, title and status. A cell with no text, such as a missing status, is written as `| |`. `--replace` replaces a file that a person keeps manually. Then the links to a section of the old file that names an entry go to the entry. `check` reports a stale or manually edited index (SD610).

`index` gives the file the type of its place, as `check` reads it: the `type` line that the old file keeps, the type of the root, the type of the layer folder, or the `default-type` of the root. If that type needs a status, and the old file has none, and the root gives none, `index` writes the first status of the type before the line `generated`. That is `draft`, or `proposed` for a decision. A status that the old file has stays. A root with `default-type` and `status` gets no line. If the place gives no type (for example `index docs` in a mixed root with no `default-type`), `index` stops with exit code 2 and writes nothing. The message is ``the place `<dir>` gives no type; index a folder of a layer, or set `default-type` of the root``. For the folder of a layer that holds more than one type, such as the `intent` folder (`intent`, `spec` and `plan`), the message names the layer and its types, because the folder gives no single type. The index that `finish` writes again follows the same rule, and for a place with no type it keeps the front matter as it is.

## eval

```text
spec-dna eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate] [--harness <arm>]
spec-dna eval run --ab <skill|all|agents-line> [--harness <arm>] [--runs <n>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate | --no-isolate] [--resume <result file>] [--json]
spec-dna eval run --arms <arm>,<arm>[,<arm>] [<id>...] [--runs <n>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--resume <result file>] [--json]
spec-dna eval pack
spec-dna eval verdict [--release] [--json]
spec-dna eval verdict --compare <treatment>,<control> <result file> [--json]
spec-dna eval records <result file>
```

`eval run` tests the agent setup of the project ([ADR 0008](adr/0008-harness-and-evals.md)). It starts the headless agent of the Claude Code adapter (`claude -p`) on a copy of a fixture. Then it runs the checks of the eval. It needs the table `[evals]` in `spec-dna.toml`. `eval` with no subcommand, or with another subcommand than `run`, `pack`, `verdict` and `records`, is a usage error. [evals.md](evals.md) is the guide to the start, the cost and the CI job.

spec-dna itself makes no network call. `claude` makes network calls, and each run costs money. A pin that differs from the version of the binary stops the command, as it stops `check`. The runner has no time limit of its own. In CI, `timeout-minutes` of the workflow limits the job.

[evals.md](evals.md) gives the eval file, the steps of a run, the command of `claude`, the output and the result file.

### Options

| Option | Meaning |
| --- | --- |
| `<id>...` | The evals to run |
| `--runs <n>` | Replaces the `runs` of each selected eval and `[evals] runs`. An integer from 1 to 100 |
| `--min-pass-rate <r>` | Replaces `[evals] min-pass-rate`. A number from 0 to 1 |
| `--max-total-usd <n>` | Replaces `[evals] max-total-usd` for the cap. A number above 0 |
| `--out <dir>` | Writes the result to `<dir>/eval-<YYYYMMDD>T<HHMMSS>Z.json` (the UTC time of the start). If the file exists, the name gets the next free suffix from `-2`. `<dir>` is relative to the repository root, and `eval run` makes it before the first run, so a bad value stops the command before any cost. It cannot be `[evals] dir`, because the check SD506 reads each `*.json` file there. For example, use `<[evals] dir>/results` |
| `--dry-run` | Checks the evals and prints each eval with its runs and the estimate. It starts no process, and writes no file. Exit code 0, or 3 if the estimate is above the cap |
| `--isolate` | Adds `--setting-sources project --strict-mcp-config`, so a run does not load the user and local settings or the MCP servers of the person. It is off by default, so CI and a laptop run the same command. A test with `claude` 2.1.325 (probe P2 of the design page) showed the effect. The flag removes the user skills, the user agents and the plugins that the user settings enable. The test found no trace of the user `CLAUDE.md`. This proof is weak, because it uses the answer of the model. The flag keeps the plugins that `CLAUDE_CODE_PLUGIN_DIRS` names. To remove these plugins, unset the variable before the command. `--strict-mcp-config` removes the MCP servers of the account of the person. A clean `CLAUDE_CONFIG_DIR` also removes the login |
| `--json` | Prints the result as one JSON object, and no text |
| `--ab <target>` | Runs the evals of the pack in 2 arms, with the skill and without it. See [eval run --ab](#eval-run---ab) |
| `--harness <arm>` | Gives the copy of each run a harness arm (`none`, `lean`, `lean-no-advisor`, `full` or `project`) in place of the harness files of the project. It isolates the runs, so `--no-isolate` with it is a usage error. See [eval run --arms](#eval-run---arms) |
| `--arms <arm>,<arm>[,<arm>]` | Runs the plain evals in 2 or 3 harness arms, in waves. See [eval run --arms](#eval-run---arms) |

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | The pass rate is at least `min-pass-rate`. Also `--dry-run` |
| 1 | The pass rate is lower |
| 2 | Usage error: a bad option value, an unknown id, `--out` that is `[evals] dir`, or no `run` subcommand. Also an unknown harness arm, 1 arm or more than 3 with `--arms`, `--arms` with `--ab`, and `--no-isolate` with `--arms` or `--harness` |
| 3 | No `[evals]` table, an invalid eval file (also `change` with no `branch`), no eval, an absent `check.sh`, a different version pin, an estimate above the cap, a stop by the cap, or a `claude` that is absent or cannot start |
| 4 | A git or file error of `eval run` itself: the temporary folder, the copy, the base commit, the commit of the change or the result file |

A plain `eval run` with `--harness` keeps the meaning of exit code 1: the pass rate is below `min-pass-rate`. A run of `--arms` gives no pass rate, so it never exits with 1.

### eval run --ab

`eval run --ab <target>` measures a skill against no skill ([ADR 0016](adr/0016-skill-evals.md)). The target is a skill, `all` or `agents-line`. It reads the pack in `<[evals] dir>/pack/`, runs each eval in the arms `with` and `without` in pairs, and writes `<[evals] dir>/results/skills-<YYYYMMDD>T<HHMMSS>Z.json` (or the folder of `--out`). The result has the pass rates, the costs, the difference and the cost ratio with a 90% interval, and a verdict. [evals.md](evals.md#the-pack-and-the-measure-of-a-skill) gives the arms, the rules of the verdict and the result file.

`--ab` accepts `--runs`, `--max-total-usd`, `--out`, `--dry-run`, `--json`, `--isolate` (the default), `--no-isolate` and `--resume <result file>`. `--resume` goes on in a result file of the same target: it skips the pairs that the file holds, and it writes one merged file. `--resume <file> --runs <n>` with `n` above the `runs` of the file adds runs to each eval. SIGHUP, SIGINT and SIGTERM write the partial result, with `"stopped": "interrupted"`, and exit with 129, 130 and 143. The result has the key `runs`. `--resume` rejects a lower `--runs`, and `--runs` with a file that kept the runs of each eval. An estimate above the cap does not stop the command: it prints a note and starts, and the check before each run stops it at the cap. `--dry-run` prints the estimate also when it is above the cap, and exits with 0. `--min-pass-rate` with `--ab` is a usage error. `--ab` needs `<[evals] dir>/check.sh`. An attempt that the API refuses (401, 403, 429, a 5xx status or a page of Cloudflare) is excluded: no result counts it, and the runner tries it again after 60, 120, 240, 480, 960 and 1,920 seconds. The variable `SPEC_DNA_EVAL_BACKOFF` (an integer from 0 to 3,600) sets the first wait in seconds. After the last wait, the command stops with `"stopped": "api-error"` and writes the partial result.

| Code | Meaning of `eval run --ab` |
| --- | --- |
| 0 | The target is complete, with any verdict. Also `--dry-run`, also with an estimate above the cap |
| 2 | Usage error: no target, more than one target, an unknown target, `--min-pass-rate`, a bad value of `SPEC_DNA_EVAL_BACKOFF`, an unknown harness arm, `--no-isolate` with `--harness`, or a `--resume` file that has another harness |
| 3 | No `[evals]` table, an invalid file of the pack, no eval for the target, an absent `check.sh`, a different version pin, a stop by the cap, a stop by `api-error`, or a `claude` that is absent or cannot start |
| 4 | A git or file error: the temporary folder, the copy, the base commit or the result file. After the first run, the command writes the partial result (`"stopped": "error"`) before it exits |
| 129, 130, 143 | SIGHUP, SIGINT or SIGTERM. The partial result is written first |

### eval run --arms

`eval run --arms <arm>,<arm>[,<arm>] [<id>...]` runs the plain evals of `[evals] dir` in 2 or 3 harness arms ([ADR 0030](adr/0030-harness-arms-time-record-and-verdict-floor.md)). An arm is `none`, `lean`, `lean-no-advisor`, `full` or `project`. The runner writes the harness of the arm into the copy of each run, and passes `--model` and `--effort` of the orchestrator of `lean` to each arm, also `project`. It runs in waves: wave `r` runs each eval once in each arm, and the order of the arms rotates by `(eval index + r) mod arms`. It writes `<[evals] dir>/results/arms-<time>.json`, with one side for each arm of each eval and run. A stop by the cap keeps the complete waves. The command accepts `--runs`, `--max-total-usd`, `--out`, `--dry-run`, `--json` and `--resume <result file>` (the same arms in the same order, and the evals of the file). It always isolates the runs. [evals.md](evals.md#harness-arms) gives the arms, the result file and the records.

| Code | Meaning of `eval run --arms` |
| --- | --- |
| 0 | Each planned run is done. Also `--dry-run`. The command never exits with 1, because it gives no pass rate |
| 2 | Usage error: an unknown arm, 1 arm or more than 3, an arm twice, `--arms` with `--ab` or `--harness`, `--min-pass-rate`, `--no-isolate`, an unknown id, an id that the `--resume` file does not have, or a `--resume` file with other arms |
| 3 | No `[evals]` table, an invalid eval file, no eval, an absent `check.sh`, a different version pin, a stop by the cap or by `api-error`, or a `claude` that is absent or cannot start |
| 4 | A git or file error. The command writes the partial result before it exits |
| 129, 130, 143 | SIGHUP, SIGINT or SIGTERM. The partial result is written first |

### eval pack

`eval pack` writes the 57 evals of the pack and their fixtures to `<[evals] dir>/pack/`. It writes each file once, and it keeps each file that exists. The output has one line `wrote <path>` or `kept <path>` for each file. With `--json` it prints `{"written":[...],"kept":[...]}`. Exit code 0, 3 for no `[evals]` table, or 4 for a write error. It takes no argument and no other option.

### eval verdict

`eval verdict` prints one line for each of the 12 skills and for `agents-line`: the name, the verdict (`on`, `off`, `inconclusive` or `unmeasured`) and the source (`project <file>`, `release` or `-`). For each name, it reads the newest result file of `<[evals] dir>/results` that holds a verdict of that name, and then the table of the release that the binary holds. It starts no agent and needs no `[evals]` table. With `--json`, it prints `{"verdicts":[{"name","verdict","source","file"}]}`. Exit code 0, or 3 for a configuration error.

`eval verdict --release` prints the table of the release instead: one line for each name that has a row, with the verdict, the rule, the runs and passes of each arm, the costs, the 2 intervals, the batch and the version of Claude Code. With `--json`, it prints `{"release":[...]}`. It reads the binary only, and it needs no project. [evals.md](evals.md#the-records-of-a-release) gives the form.

`eval verdict --compare <treatment>,<control> <result file>` gives the verdict of 2 arms of a result of `eval run --arms`, with the bootstrap, the rules and the floor of the verdict of a skill, on the complete waves. It prints both intervals and the number of waves, and with `--json` the item `{"treatment","control","waves","planned_waves","evals","pairs","with","without","difference","cost_ratio","verdict","rule","below_floor"}`. Exit code 0; 2 for names that are not 2 different arms of the file, a missing file argument, or `--release`; 3 for a file that cannot be read or that is no result of `--arms` (a file that names an arm, an eval or a run twice, or has a negative cost, is none). It needs no project. A verdict needs 5 evals and 15 complete pairs: below that, it is `inconclusive` with `"below_floor": true`, and the item of `eval verdict --json` and of `--ab` has the same key.

### eval records

`eval records <result file>` reads one result file of `eval run --ab` or `eval run --arms`, and prints one record for each run of each arm, as JSON Lines on stdout. A record has the 9 keys `skill`, `eval`, `run`, `arm`, `pass`, `cost`, `tokens`, `models` and `claude_code`, and no other key. A result of 0.8.0 gives 3 more keys after them: `harness`, `turns` and `seconds`. The command builds each record from these values (12 for a result of 0.8.0), so no path and no text of the file reaches the output. It computes each item again from its records, and it compares the result with the item of the file. It prints the number of records on stderr. It takes one file and no option. Exit code 0, 2 for a usage error, or 3 for a file that is not a result of `--ab` or `--arms`, an item with no verdict, a value outside its form or a result that the records do not repeat. [evals.md](evals.md#the-records-of-a-release) gives the form of a record.

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
spec-dna hook protect-tests [--mode <fix-branches|always>] [--test <glob>]...
spec-dna hook protect-approvals [--intent <dir>]...
```

The hooks use the event names of Claude Code, so they belong to its adapter ([agents.md](agents.md)). A coding agent runs `hook`. The agent starts it on each tool call and sends one JSON object on stdin. spec-dna reads at most 1 MiB of it.

A hook never reads `spec-dna.toml`, never checks the pin and never scans the repository. It reads at most one more file. `protect-tests` reads no file, and runs `git` at most 2 times, only for a path that matches a test glob. `protect-approvals` runs `git` only for a command whose `git` has the subcommand `commit`, and then 3 times, and 2 more times for each artifact file that the working tree changes.

A hook ignores unknown flags and bad values. The exit code is always 0. An error gives no output. A hook prints JSON only when the agent must see a message.

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

`protect-tests` runs before an `Edit` or `Write` tool call (the `PreToolUse` event). It asks the person before an edit of a test file that git tracks. It reads `tool_name`, `tool_input.file_path` and `cwd` of the event, and `CLAUDE_PROJECT_DIR` of the environment. The hook does not see an edit through a `Bash` call.

`init` writes the command from `[harness] protect-tests` and `[evidence] tests`, with one `--test <glob>` for each glob ([`[harness] protect-tests`](config.md#parts)). The generated command ends with `2>/dev/null || true`, so an error gives no output and the edit continues. An empty `[evidence] tests` list writes no hook.

`--mode` is `fix-branches` or `always`. An absent or unknown value has the same effect as `fix-branches`. With no `--test`, the hook prints nothing. The steps, in this order:

1. The tool is `Edit` or `Write`.
2. The root is `CLAUDE_PROJECT_DIR`, else `cwd`. `file_path` is absolute and below the root. If it is not, the hook resolves the links of the root and of the folder of the file, and tries again. A path with a `..`, `.` or empty segment stops the hook.
3. The path relative to the root matches a `--test` glob. With no match, the hook stops, and no process starts.
4. For `fix-branches`: `git symbolic-ref --short -q HEAD` gives a name that starts with `fix/`. A detached HEAD stops the hook.
5. `git ls-files --error-unmatch` finds the file in the index. A new file stops the hook.
6. The hook answers `permissionDecision: "ask"` with a reason. The reason has the relative path and, for `fix-branches`, the branch.

Each step that fails gives no output. `git` runs with `GIT_OPTIONAL_LOCKS=0`. The hook stores nothing. A run with `claude -p` cannot ask the person. The agent of the Claude Code adapter then denies the edit, and the model reads the reason.

`protect-approvals` runs before a `Bash` tool call (the `PreToolUse` event). It denies a call of an agent that approves a document. It reads `tool_name` and `tool_input.command` of the event, `cwd`, and `CLAUDE_PROJECT_DIR` of the environment. `init` writes the command when `[harness] protect-approvals` is `true`, with one `--intent <dir>` for each folder of the `intent` layer from the repository root, for example `docs/intent` ([`[harness] protect-approvals`](config.md#the-table-harness)). The generated command ends with `2>/dev/null || true`. The hook never reads `spec-dna.toml`.

The hook joins a line that ends with a backslash to the next line. It splits the command into segments at `;`, `&`, `|`, brackets, backticks and line feeds, and each segment into words at white space. It removes the quotes, so `sh -c "..."` gives its inner words. This is a text scan, not a shell parser. The rules, in this order:

1. A segment with a word whose last path part is `spec-dna` and a later word `approve`: deny.
2. A segment with the word `git` and its subcommand `commit` that has a word, before the word `git` in the call, with `accepted` or `approved`: deny, and the reason says to run the edit and the commit as 2 calls. The test ignores the path `.spec-dna/accepted/`. This is for an edit and a commit in one call. With no `--intent`, the hook skips this rule and the next one.
3. A segment with the word `git` and its subcommand `commit` (the options of `git`, such as `-C <dir>`, can come before it): the hook reads the repository. The folder is `cwd`, else `CLAUDE_PROJECT_DIR`. A `cd <dir>` segment before the commit, and `git -C <dir>`, move it. The hook expands `~`, `$PWD` and `$CLAUDE_PROJECT_DIR` (also with braces) at the start of a dir. For another dir with `$` or `~`, the hook inspects the repository of `cwd` and of `CLAUDE_PROJECT_DIR` too. The old tree is `HEAD`, or `HEAD~1` with `--amend`. A tree that is not there is the empty tree. The candidates are the artifact files `<dir>/<slug>/intent.md`, `spec.md` and `plan.md` that the working tree changes against the old tree (`git diff --name-only`), and the untracked ones. For each candidate, the hook compares the old status with the status in the index and in the working tree. It denies the call if a new status is the approval value, and the old status is not.

4. In the same repository check as rule 3, the candidates also include the record `<dir>/<slug>/acceptance.toml`. The hook reads `acknowledged-by` in the old tree, in the index and in the working tree. It denies the call if the index or the working tree has an entry that the old tree does not have. The reason tells the agent to remove the entry: only a person acknowledges a record. The candidates also include the archive files under `.spec-dna/accepted/`. The hook denies a new archive file whose bytes are not those of a record that the same commit deletes from a change folder, and it denies a change of an archive file that the old tree has (a delete passes). So the commit of `finish` passes, because it deletes the record and adds the same bytes.

A deny is `permissionDecision: "deny"` with a reason. The reason of rule 3 names the path and the status, and tells the agent to set the status back and to ask the person to run `spec-dna approve <slug> <artifact>` in a terminal of their own. Bad JSON, no git, a git error and a path that the hook cannot read give no output. [harness.md](harness.md#spec-dna-hook-protect-approvals) gives the limits.

## upgrade

```text
spec-dna upgrade [--dry-run] [--layout [--areas <file>]] [--pin-file <file>]
```

`upgrade` moves a project that pins spec-dna 0.4.0 or later to the version of the binary that you run. It is also the command that moves the pin. It has no `--to` option, because the binary embeds the standard of its own version. A project with an older pin first moves to 0.4.2 and then runs `upgrade`. [The upgrade guide](guides/upgrade-0.5.md) gives the procedure for 0.5, and [the guide for 0.6](guides/upgrade-0.6.md) the procedure for 0.6.

| Option | Meaning |
| --- | --- |
| `--dry-run` | Print the diff of each file and write nothing. It needs a clean tree, as a write does |
| `--layout` | In a mixed root, also move the folders of the other 0.5 namespaces into the 0.6 folders, and move the documents that have the name of a top file to the top file of their root |
| `--areas <file>` | With `--layout` only. A TOML file with the table `[areas]`. Each key is a document id of the 0.5 tree, and each value is an area: `"specs/pumps" = "pumps"`. Each mapped document moves to `<root>/requirements/<area>/<slug>.md`. Save the file outside the repository, as the file of `--pin-file`. A 0.6 project gets exit code 2 |
| `--pin-file <file>` | The file `spec-dna-pin.toml` of the release, as a path from the current folder. `upgrade` copies its sums into `[spec-dna-sha256]`. The binary cannot know its own checksum |

Save the file of `--pin-file` outside the repository, for example in the folder above it: `--pin-file ../spec-dna-pin.toml`. A file inside the repository makes the tree not clean, so `upgrade` refuses it (exit code 1). Verify the file with `gh attestation verify`. The command prints nothing when its output goes to a file or a pipe, so check its exit code (0 means verified).

`upgrade` always reads and checks this file. The key `spec-dna` must be the version of the binary, and `[spec-dna-sha256]` must have valid sums. If not, `upgrade` exits with code 3. A project that has `[spec-dna-sha256]` needs the flag when the pin changes. A project with no table gets the table from the file, and the report says so.

`upgrade` reads the committed tree (`HEAD`), and it starts only from a clean tree. A staged change, an unstaged change and an untracked file that git does not ignore give exit code 1. The message gives the count and the first 5 paths. It also tells how to keep the work: stash the changes with `git stash -u`, run `spec-dna upgrade`, commit its paths, then run `git stash pop`. A repository with no commit also gives exit code 1. `--dry-run` has the same rules.

`upgrade` computes the whole plan in memory, runs the proof, and writes nothing before the proof passes. It does these 7 steps and stops at the first failure:

| Step | What it does |
| --- | --- |
| 1. Check | Refuses a tree that is not clean. Reads the old configuration. Builds the whole plan in memory |
| 2. Configuration | Renames the old names: the types (`change` gives the 3 types `intent`, `spec` and `plan`), the statuses, the keys of `[budgets.*]` and the role `reviewer`. Removes `spec-writer`. Writes `[change] policy = "none"` and the layers that the types in use need, with no limits. Sets the pin to the new version, with the sums of `--pin-file`. Keeps each comment and each line ending. Writes the keys below |
| 3. Front matter | Rewrites each `type:` and each `status:` that changed. A `proposed` or `accepted` change becomes an `intent` that is `draft` or `accepted`, and a `rejected` change becomes a `closed` intent. A `done` change and a former `note` become `research`. `upgrade` writes `was-type` (`change` or `note`) on each such document, and it adds a front matter if the document has none. It removes the `status` line of a `done` change |
| 4. Harness | If `[harness]` exists, renames the role keys, renames the skills of 0.6 in `skills` (`intent` is `brainstorm`, `spec` is `specify` and `clarify`, `plan` is `write-plan`, [harness.md](harness.md#upgrade-from-06)), leaves out a new name that is off by default and names it in the report, adds to a list that is not empty each skill that is new in 0.7.0 and that `init` proposes for the profile and the level (the report names each one added, [profiles.md](profiles.md#what-init-writes-for-each-profile)), and writes the new parts as off: `skills = []`, `review-md = false` and `protect-tests = "off"`. A project at level `full` gets `code-simplifier = { claude-code = "none" }` in `[harness.roles]`. Deletes an unedited generated file under its old name. Keeps an edited file, and `check` reports it (SD501). Never replaces a file that has no marker. For a 0.5 project, it disables that role |
| 5. Changes and layout | Always: moves each `proposed`, `accepted` or `rejected` change with `git mv` to `<root>/intent/<slug>/intent.md`, in its own root. Writes its old id into `aliases`. The slug is the file stem, or the folder of an `index.md` or a `README.md`. With `--layout` only: moves the 6 folders of a mixed root that the next paragraph lists. Writes the old ids into `aliases`. A `done` change and a 0.5 spec stay where they are, because only a person can choose the area of a spec: `--areas` gives the choice. The top files and the areas are in the paragraphs below |
| 6. Proof | Checks the items below. If the proof fails, `upgrade` writes nothing and exits with code 1 |
| 7. Report | Lists the proposals to delete, the decisions to check, the files that it kept, the links that it did not rewrite and the changes of behavior |

In step 2, `upgrade` also writes the keys that keep a 0.5 document as it was:

- `required-sections = { intent = [] }` in each mixed root with a migrated change;
- `[budgets.intent]` of 10,000 and 20,000, if a migrated change is over 12,000 characters;
- `live-sections = { decision = [] }` in each root that has no `live-sections` and can hold a decision. Thus no 0.5 decision gets a live section by default.

If the 0.5 key `types` has `change`, the new list has `intent`, `spec` and `plan` at its place, because the intent layer holds the 3 types together. A list of many lines that needs this change is a hand edit (exit code 3), because the plan keeps the comments of such a list only if its length stays.

A 0.6 project gets 2 more edits of `spec-dna.toml` beyond the pin. Both add values only, both follow an exact rule, and the report names each of them:

- The file has the key `types`, `[change] policy` is not `none`, and the list lacks `intent`, `spec` or `plan`. `upgrade` adds the absent types. With the policy `none`, no command writes a `spec` or a `plan`, so the list stays as it is. An empty list (`types = []`) means that every type is valid, so it also stays as it is.
- `linked-from` of `[layers.pending]` is exactly `["intent"]`, the value that `init` 0.6.0 wrote. `upgrade` adds `requirements` and `decisions`, for each of them that the graph has. Another value stays.

These edits change `spec-dna.toml`. A new `types` list also changes the generated skill, because the skill lists the types of the project. SD503 does not count the skill, so the pull request of the pin can hold these edits. If a list has many lines, `upgrade` does not edit it. It prints a warning that names the hand edit.

`--layout` moves the folders `contracts`, `invariants`, `guides`, `runbooks`, `reference` and `notes` of a mixed root. They go to `interfaces`, `rules`, `procedures`, `procedures`, `research` and `research`, with each tracked file under them. A root of one type keeps its path.

Also with `--layout`, a document of a mixed root becomes the top file of the root when its file stem is in this table. The stem is lowercase, and `_` is read as `-`:

| File stem | Top file |
| --- | --- |
| `roadmap` | `roadmap.md` |
| `product-overview`, `overview` | `product.md` |
| `open-questions`, `questions` | `questions.md` |
| `glossary`, `terms` | `terms.md` |
| `architecture`, `system` | `system.md` |

A document qualifies only if all these are true:

- Its 0.5 type is `note`, `reference`, or a `change` that is not `done`. A `change` named `roadmap` becomes the top file, not an intent.
- It has no requirement: no item with a legacy key, and no item with a slug key and a BCP 14 word.
- It is directly in the root, or directly in `notes/`, `reference/` or `changes/`.
- Its size is not above the hard budget of the type of the top file. A `note` or a `reference` has no hard budget, and the types `product`, `system`, `terms`, `roadmap` and `questions` have one. A document over it would give SD702.

The moved file has no `type` line, no `status` line and no `was-type` line, because its place gives the type. Its old id is an alias, the key `types` (if the project has it) gets the type, and the graph gets the layer `foundation` or `pending`. If 2 documents want the same top file, or the top file exists, or the id of the top file is the id or an alias of another document (also in another root), or the id of the document names more than one document already, or a document is over the budget, that document is no top file. The report has one `kept` line that names the paths, and each such document follows the other rules of `upgrade`: a change moves to the intent layer, and `--layout` renames its folder. The upgrade does not stop. A 0.6 project gets no move to a top file, because SD503 forbids a move of a document in the pull request of the pin. For a 0.6 project, see [After the upgrade: top files and areas](guides/upgrade-0.5.md#after-the-upgrade-top-files-and-areas).

`--areas <file>` moves the documents that the file maps. The file has one table, `[areas]`, and each entry is `"<id of the document in the 0.5 tree>" = "<area>"`. A mapped document goes to `<root>/<folder of the requirements layer>/<area>/<slug>.md` in its own root, with its old id as an alias. The plan adds each area to the key `areas`, sorted, and writes the key if the file has none. An unknown id, a document that is not a `requirements` document in 0.6, an area that is not one folder name (letters, digits, `-` and `_`), 2 areas that differ only in case, a target that exists and 2 documents with one target stop the plan (exit code 1), and nothing is written. A file that does not parse, that has no table `[areas]` or that has a value that is not a string is exit code 3. `--areas` without `--layout` is exit code 2. A 0.6 project with `--areas` is exit code 2: `` `--areas` moves documents of a 0.5 project; in a 0.6 project, run `spec-dna mv` in a pull request of its own ``.

After the moves, `upgrade` removes each folder that a move left empty, from the deepest folder to the highest. A folder with any file stays, also an ignored file, and the root folder of the document stays. Git does not track folders, so the output does not list them. `--dry-run` removes no folder.

The plan compares the paths of a target in ASCII lowercase, because some file systems fold the case of a name (macOS, Windows). Thus `docs/Roadmap.md` and `docs/roadmap.md` are one target on each file system: a top file that exists under another case keeps the document, and 2 files with targets that differ only in case are 2 files with one target.

A stop of step 5 is exit code 1, and the message names each path. These are the causes: a slug that is not valid, 2 moves with one target, a target that exists, a target in another root, an `exclude` glob that matches a moved path, and each cause of `--areas` above.

A configuration with no `[harness]` table, or with the level `none`, gets no harness file. `upgrade` writes the 0.6 adapters, as `init` does. It changes a document body only for the link destinations of a move. It never renames the file stem of a decision, so `legacy-ids` patterns still resolve. The one change of a stem is the move of a change into `intent/<slug>/intent.md`.

When 2 old types with explicit keys merge into one, `upgrade` keeps the weaker value. `upgrade` never commits and never deletes a document. A second run after the commit changes nothing and prints `spec-dna upgrade: nothing to change`. A second run before the commit gives exit code 1.

The proof reuses the preservation check of `mv`. It checks these items:

- Each id, alias, legacy id, section anchor and requirement key of the old tree resolves to the same object in the new tree.
- The hash of each requirement and each section is the same. The locked hash of each decision is the same too.
- Each link of a document goes to the same file as before.
- Each document has the 0.6 type that the mapping gives. A document that became a top file has the type of the top file.
- The new configuration parses, and the layer graph has no cycle.
- No error of the rule groups `references` (SD101, SD102, SD104, SD106) and `ids` (SD201 to SD204) is new.

A pointer of `[conventions]` that names a document by its path is not edited. With `--layout`, such a pointer breaks (SD106), and the proof fails. Write the pointer as the id of the document, commit the change, and run `upgrade` again.

The text output has one line for each file: `moved <from> -> <to>`, `wrote <path>`, `removed <path>` or `kept <path>: <reason>`. With `--dry-run`, the verbs are `would move`, `would write` and `would remove`, and the diff of the file follows each line. Then come the proof line, the report and the `git add` command that stages the result:

```text
to commit: git add -A -- :/spec-dna.toml :/docs/intent/dose-limit/intent.md
```

The paths start with `:/`, so the command works from each folder. The command lists the new path of a move and not the old path, because `git mv` has already staged the rename.

One `next:` line follows the command. SD503 does not count `spec-dna.toml` when only its pin, its table `[spec-dna-sha256]` and the new skill names in the list `skills` change to a later version ([diagnostics.md](diagnostics.md#files-that-sd503-counts)). In that case the line says that the commit can also hold other work. If `spec-dna.toml` has other edits, or the project is at 0.5 or earlier, the line says that the commit goes in a pull request of its own. A plan that leaves `spec-dna.toml` as it is has no `next:` line. If the project runs the pre-commit hook `typos` and has no setting for the hashes of the markers, a `note:` line follows ([init.md](init.md#spell-checkers)).

The report lists these items:

- each `done` or `rejected` change that no document references, as a proposal to delete;
- each decision with no `status` in its front matter. spec-dna does not read a `Status` section;
- each locked decision that cites a migrated change, because `finish` does not stop on a reference in locked text;
- each kept harness file, each `kept` line of a top file, and each ended role with its model;
- the first 5 links of each kind that `upgrade` did not rewrite (see the limits below);
- each edit of `spec-dna.toml` beyond the pin (the `types` list and the `pending` layer of a 0.6 project);
- the table `[spec-dna-sha256]`, if `--pin-file` added it, each warning of the harness configuration and each hand edit of `spec-dna.toml` that the plan cannot make;
- the changes of behavior in the list below, for a project of 0.5 or earlier only;
- the count of each error code of `check --all` on the new tree, without SD506. The check uses the new tree as its own base, as `check --all` does after the commit of the upgrade. Thus an old excess of a budget (SD702) is not an error. The check of a pull request compares with the merge base, so it can report other new findings.

For a project of 0.5 or earlier, `upgrade` reports these changes of behavior, and the CHANGELOG records them. The report of a 0.6 project does not list them. The field `behavior` of `--json` is the same for each source:

- A former `note` is now `research`, with `keep = "forever"`. SD202 allows the deletion of a document that has `was-type: note` or `was-type: change`, as 0.5 did. This is the only exemption in a `forever` layer.
- `context` listed incidents and notes. It now counts them in one line.
- The context order follows the levels of the graph. `foundation` comes before `rules`.
- The `--json` field `name` of SD405 and SD611 changed.
- 0.6.0 renames types, keys and commands, so it is a minor release that breaks compatibility. Semantic Versioning allows this before 1.0.

With `--json`, the output is one object with these fields:

| Field | Content |
| --- | --- |
| `version`, `from`, `to`, `dry_run` | The version of the binary, the old pin, the new pin and a flag |
| `files` | One entry for each file: `path`, `action` (`write`, `move`, `remove` or `keep`), `note` and `diff`. A `move` entry also has `from`, the old path |
| `proof` | `ok`, `ids`, `requirements`, `sections`, `types` and `failures` |
| `report` | `proposals`, `no_status`, `locked_cites` (`decision` and `change`), `kept`, `ended_roles` (`role` and `model`), `behavior`, `errors` (`code` and `count`), `sums_added`, `outside_links`, `folder_links`, `warnings` and `edits` (the edits of a 0.6 `spec-dna.toml` beyond the pin, as lines of text) |

`outside_links` and `folder_links` are counts. A failed proof gives the object on stdout and exit code 1. A second run gives `files: []`.

| Exit code | Meaning |
| --- | --- |
| 0 | `upgrade` wrote the changes, or nothing needed a change |
| 1 | The tree is not clean, or the repository has no commit. A stop of a move (see step 5). A symbolic link, or a file in the place of a folder, in a path that `upgrade` writes. A path that `upgrade` writes and git ignores. A file that is a document and also an instruction file or a harness file. The proof failed |
| 2 | A usage error, also `--areas` without `--layout` and `--areas` for a 0.6 project |
| 3 | No git repository or no `spec-dna.toml`. A pin before 0.4.0 or after the binary. A file that neither reader accepts. A form that `upgrade` cannot edit (a hand edit). A file that is not UTF-8 and needs an edit. `[spec-dna-sha256]` with no valid `--pin-file`, or a bad `--pin-file` or `--areas` file |
| 4 | A git error, or a write that failed |

A write that fails prints "to restore the tree, run `git reset --hard` and `git clean -fd`". To undo an upgrade before the commit, run these 2 commands. After the merge, a revert needs 2 pull requests, because SD503 of the old binary rejects a configuration and documents together.

If `spec-dna.toml` has a form that `upgrade` cannot edit without a loss of text, `upgrade` exits with code 3 and writes nothing. Two examples are a budget with dotted keys (`guide.soft`) and a `types` list of many lines that must change its length. The message gives the key and the form to write.

For example: "`budgets.guide`: upgrade cannot edit this form; write it as a table with a header, such as `[budgets.guide]`, ... by hand, commit the change, then run `spec-dna upgrade` again". Edit the file with the 0.5 names, because the 0.5 reader of `upgrade` refuses a 0.6 name. Commit the edit, because `upgrade` reads `HEAD`.

These limits apply:

- A link from a file that is not a document stays as it is. Examples are `README.md`, an instruction file and a file outside the roots. The link goes to a moved file. `upgrade` counts these links (`outside_links`) and does not rewrite them. `check` does not read them. Search for the old paths, and fix the links.
- A link to a folder stays too, if the folder of a change moves or the document that the folder gives changes (`folder_links`).
- A file with CRLF line endings that moves gets the new block of `aliases` with LF line endings. `mv` does the same.
- `git log --follow` can lose the history of a small change that `upgrade` moves, because the same commit edits its front matter and links. [The upgrade guide](guides/upgrade-0.5.md#commit-the-result) gives the advice.
- If the first write step fails, empty folders can stay. Git does not show them.
- `status`, `next` and `finish` read a lifecycle base that pins a version before 0.6.0 with the rules of 0.6. `check` does not.

## import

```text
spec-dna import speckit|openspec [--dry-run] [--skip-open] [--areas <file>] [--keep none|research] [--json]
```

`import` moves a project of Spec Kit 1.4.1 or OpenSpec 2.0.0 into the layout of spec-dna, in one diff that a person can read. The reasons are in [ADR 0014](adr/0014-import.md). It needs a project that `init` made. Without `spec-dna.toml` it stops with exit code 3 and says `run spec-dna init first`. It reads the committed tree (`HEAD`), and it starts only from a clean tree, also with `--dry-run`. It computes the whole plan in memory, runs the proof, and then writes. It never deletes a file and never commits. A second run after the commit prints `spec-dna import: nothing to change`.

| Option | Meaning |
| --- | --- |
| `--dry-run` | Print the same plan and run the same proof, and write nothing. The lines say `would move` and `would write` |
| `--skip-open` | Do not read an open feature or an open change. They stay in place, and each of their files is `skipped`. An open feature that is still a template then does not stop the import |
| `--areas <file>` | A TOML table `[areas]` that maps the name of a source folder to one area folder: `"001-photo-albums" = "media"` for Spec Kit, and `auth = "media"` for OpenSpec. The target is `<requirements folder>/<area>/<name>.md`, and the delta lines name the same id. The file stays outside the repository. A pair of areas that differ only in case, also against `areas` of `spec-dna.toml`, is exit code 1 |
| `--keep none` | The default. An archived change and the extra files of a feature stay in place, and the report counts them |
| `--keep research` | Move each Markdown file of an archived change, and each extra Markdown file of a feature, with `git mv` to `<research folder>/imported/<source folder>/<file>`, as one `research` document. A file that is not Markdown stays, and the report names it. The layer `research` must exist. Without it, the command stops with exit code 2 and names `spec-dna layer add research --types research`. The moved files stay search hits, but `context` never serves them |

### What moves and what is written

A source that gives one target moves with `git mv`, and the plan then edits the file in place. It keeps its old id as an alias if it was a document before (its root covers it). A target goes to the folder of the layer that holds its type: in a root of that type (for example the root `specs` of the type `requirements` that `init` writes for Spec Kit), else in the first mixed root. If a layer holds a type and no root can hold it, the first run adds a root `docs` of the type `mixed` to `spec-dna.toml`. With no such layer, the command stops with exit code 2 and names `spec-dna layer add <layer> --types <type>`. The current truth gets `status: active`. A source that is already a document of the project (when `init` made its folder a root) and that the plan writes, not moves, gets the line `moved-to` in its front matter, and its first target gets its id as an alias. The `git rm` of the report then loses no id (SD202).

| Source | Target |
| --- | --- |
| Spec Kit: `.specify/memory/constitution.md` | `rules/constitution.md`. `Core Principles` becomes `Rules`, and each principle (a level 3 heading) is a requirement with the slug of its heading as key, with no leading numeral. A file with the template placeholders, or with no `Core Principles` heading, is skipped with its reason |
| Spec Kit: `specs/<digits>-<name>/` with a `tasks.md` that has a task and no open task (a done feature) | `requirements/<name>.md`, moved from `spec.md`. The `FR-nnn` lines stay as they are, through the legacy key `FR-{n}`. `<digits>-<name>` is an alias. `tasks.md` stays |
| Spec Kit: a feature with an open task, or with no task line, or with no `tasks.md` (an open feature) | `intent/<name>/intent.md` (status `draft`, class `higher-risk`), `spec.md` and `plan.md`, written. The user stories are the intent, and each other section is a sub-section. Each `FR-nnn` line is a delta line `ADD requirements/<name>#fr-nnn`. A marker `[NEEDS CLARIFICATION: text]` goes from the sentence to an item of `Open questions`. Each task `T001 [P] [US1] text in path` is `- [ ] t001: text. Files: path.`, and the task has no `Verify:` label. The sources stay |
| OpenSpec: `openspec/specs/<domain>/spec.md` | `requirements/<domain>.md`, moved. Each `### Requirement: <Name>` is a requirement with the slug of the name as key. Each `#### Scenario:` is a nested item below it. `<domain>` is an alias |
| OpenSpec: `openspec/changes/<name>/` | `intent/<name>/intent.md`, `spec.md` and `plan.md`, written. `Why` is `Why now`, `What Changes` and `Impact` are the intent, and another heading is a sub-section. `design.md` is the section `Design`, and `tasks.md` is `Order` (a task `1.1` is `t1-1`). The delta spec gives `ADD`, `CHANGE` and `REMOVE` lines, with the hash of the imported text, and the section `Scenarios` holds the scenarios, the reason and the migration of a removed block, and a `Purpose`. The sources stay |
| OpenSpec: a change with `RENAMED`, or with a block that has no base requirement | Stays in place, with a report line |
| OpenSpec: `archive/`, `config.yaml`, `project.md`, `.openspec.yaml`, `schemas/`; Spec Kit: `.specify/` files | Stay in place. `--keep research` moves the Markdown files of an archive |

The import changes no sentence. A line that the target rejects stops the proof: for example `- **SC-001**: ... MUST ...` gives SD203, because the key is no slug and no legacy key. The message names the line and 2 fixes: add the pattern to `legacy-keys`, or remove the bold key in the source and commit. A link that is broken in the source gives SD102, and it stays broken. The plan edits each relative link of a moved or written file so that it goes to the same file, and each link of a document of the project to a moved file. A link of a file that is no document stays, and the report counts it.

If the plan needs a change of `spec-dna.toml` (the legacy key `FR-{n}`, an area of `--areas`, or a type that the `types` list lacks), the first run writes only that file, because SD503 rejects it together with other files. Commit it in a pull request of its own, and run the command again. An open feature or an open change becomes a draft intent, spec and plan. A draft needs no approval, so it gives no SD910.

### The proof

The proof has 6 steps and stops the write (exit code 1) if one fails. It does not claim byte equality.

1. Each requirement of a source has one target requirement or one target delta line, and the normal forms of the sentences are equal. The normal form joins the white space and removes the link destinations, the alias markers and a marker `[NEEDS CLARIFICATION: ...]`. The proof also checks that the open question holds the text of the marker. It reads the sentence of the target from the parse of the planned text.
2. Each source file has exactly one state: `moved`, `written`, `kept` or `skipped`. A kept or skipped file is not moved and not written.
3. Each id of the project before the import resolves to the same document.
4. `check` on the new tree gives no new error of the codes SD101 to SD106, SD201 to SD204, SD301 to SD306 and SD401 to SD405. The report counts the other new findings, such as SD608.
5. The plan on the planned tree is empty. A second run writes nothing.
6. No line of a source vanishes. A line of a source, with its link destinations, its bold markers and its backticks removed and its white space joined, must be in one line or one paragraph of a target, or in the sentence of a requirement. A line with a word of BCP 14 (`MUST`, `SHALL`, `SHOULD`, `MAY`, also with `NOT`) that no target holds stops the write, and the message names the file and the line. Another line that no target holds is listed in the report as `dropped`. A heading needs no copy. A task line is compared on its text, because the reader moves its id, its marks and its path (a final group in parentheses stays in the text). The reader keeps a line of a `tasks.md` that is no task, the text before the title of a spec, and the text of a delta spec outside its blocks, as plain text.

The proof also counts the scenario headings of the source and of the target of an OpenSpec spec, because no hash covers a scenario.

### Output

The text has one line for each source file: `moved <from> -> <to>`, `wrote <to> (from <sources>)`, `kept <path>: <reason>` or `skipped <path>: <reason>` (`would move` and `would write` for `--dry-run`). Then come one line `pair <from> -> <id>` for each requirement, the line `proof: ok (<n> requirements, <n> scenarios, <n> ids)`, the report, `to commit: git add -A -- <paths>` and `to remove the sources: git rm -r -- <paths>`. The output has no diff: run `git diff` after the write. The report says when a moved file has an edit that is too large for a git rename (a similarity below one half). This happens for most OpenSpec domain specs, because each requirement heading becomes a list item. The constitution is a small case. `git rm` names the files that the plan wrote, and a folder only if it holds no kept file. A source file that is not valid UTF-8, and a target that differs from an existing file only in case, stop the plan (exit code 1). With `strict`, SD608 is an error for each imported active requirement, and with `--skip-open` SD917 and SD921 are errors too. The report names them.

With `--json`, the command prints one object: `version`, `source` (`speckit` or `openspec`), `dry_run`, `config_only`, `sources` (each with `path`, `state`, `to` as a list and `reason`), `pairs` (each with `from` as `<path>:<line>`, `id` and `delta`, the slug of the change or `null`), `proof` (`ok`, `requirements`, `scenarios`, `ids` and `failures`) and `report` (`errors` with `code` and `count`, `outside_links` and `notes`). A run that writes only `spec-dna.toml` has `config_only: true`, and its `sources` and `pairs` are empty. A failed proof prints the object, and the exit code is 1.

### Known limits

- The check of the lines (step 6) compares text as it is in a target line or paragraph. A line that is a part of a longer line of a target counts as held, so it can hide a lost line with the same words, for example the text of 2 sections that have the same heading.
- Step 6 cannot see a change of the text of a task that the reader makes on purpose: a word group such as `in v2.0` after ` in ` goes to `Files` when it looks like a path.
- A moved done feature or domain spec keeps the status of its own front matter. Only a file with no `status` gets `active`. The lines of an `aliases` list that the import rewrites can show as `dropped` in the report.
- The text before the first title of a spec stays in the intent. The text under a second level 1 heading of a spec is not part of a section, so the report can list it as `dropped`.
- A second run on a project whose tool folder is gone (all files of `.specify/` or `openspec/` moved or removed) stops with exit code 3, because no layout is left to read.
- A project that holds an import of an older build (renames committed, edits lost) cannot be repaired by a new run, because the sources are gone.

An unknown layout stops with exit code 3 and writes nothing: `.specify/` or `specs/` is absent for Spec Kit; `openspec/` is absent, or `openspec/specs/` has files and no domain folder (another major version) for OpenSpec; or a spec file that the import reads has no requirement (no `FR-nnn` line, no `### Requirement:` heading). The message names the file, the rule and the supported version. An `openspec/` folder with no `specs/`, and a `.specify/` folder with no `specs/`, are no stop, so a second run after the move of each spec or after the `git rm` of the report changes nothing.

| Code | Meaning of `import` |
| --- | --- |
| 0 | A write, a dry run, or nothing to change |
| 1 | A tree that is not clean, a stop of the plan (2 sources with one target, a target that exists with other text, a key that cannot be made, a pair of areas), or a failed proof |
| 2 | Usage error: no source, another source, another value of `--keep`, or a type with no layer (`--keep research` with no layer `research`) |
| 3 | An unknown layout, no project, a bad `spec-dna.toml` or a bad file of `--areas` |
| 4 | A git or write error. The message gives the commands that restore the tree |

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error). For `eval run`: the pass rate is at least `min-pass-rate`. `hook` always exits with 0 |
| 1 | `check` found a new error (with `--all`: any error), `evidence stamp` found an unreadable report or no report, the preservation check of `mv`, `history` or `index` refused the change, a step of `finish` stopped, `approve` refused (steps 3 to 7), `accept` refused (an edit that `HEAD` does not have, a change with no plan or no intent, or a changed code), `upgrade` or `import` refused or its proof failed, or the pass rate of `eval run` is below `min-pass-rate` |
| 2 | Usage error. Also a refusal of `layer add` (it changes nothing), an unknown `--group` of `status`, `stats` or `context`, `stats` without `--sdlc`, an unknown id or an unknown harness arm of `eval run`, `--arms` with 1 arm or more than 3, `--arms` with `--ab`, `--no-isolate` with `--arms` or `--harness`, a `--compare` that is not 2 arms of its file, an `index` of a place that gives no type, a `new` or `mv` into an area that differs from a name of `areas` only in case, and an `import` with a missing layer for a type that it writes or for `--keep research` |
| 3 | Configuration error: no valid `spec-dna.toml` (also 2 names in `areas` that differ only in case, `evidence stamp` with no `[evidence] reports`, a glob of `[evidence] reports` that is absolute or has `..`, or `[evidence] reliable-for` other than `none` with no `reports`), a different version pin, an unknown base revision (for `log`: an unknown `--since`), or no git repository. For `check`: also a base that pins a version before 0.4.0, or that `upgrade` cannot plan. For `upgrade`: also a form that it cannot edit, or a bad `--pin-file`. For `finish`: also an `accepted` change with no base. For `import`: no project, an unknown layout or a bad file of `--areas`. For `eval run`: also an invalid eval, an estimate above the cap, a stop by the cap, or a `claude` that is absent. For `eval verdict --compare`: a file that is no result of `eval run --arms` |
| 4 | Git or file system error. For `finish`: a write failed, and the message lists the files. For `approve`: the commit failed, and the file has its old text. For `import`: a failed `git mv` or write, and the message gives the commands that restore the tree. For `eval run`: the temporary folder, the copy, the base commit or the result file. For `evidence stamp` and `check --reports`: a folder of reports that exists and cannot be read |
| 5 | A target names no document, section or requirement. For `finish`, `next`, `approve` and `accept`: no folder for the slug, and for `approve` also no file of the artifact |

A command that writes to a closed pipe, such as `spec-dna check --json | head -1`, ignores the write error. It ends with its own exit code, and it prints no error. This applies to every command.

## Cache

spec-dna keeps the parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, a commit never holds the cache, and the cache stays after a branch switch. On each run, spec-dna parses the files that differ from the git index, and the untracked files. It writes the cache to a temporary file and then renames that file. You can delete the cache at any time, or use `--no-cache`.
