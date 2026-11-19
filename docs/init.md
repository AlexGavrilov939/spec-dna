# `spec-dna init`

This page describes `spec-dna init`: what it detects, the questions that it asks (at most 14), and the files that it writes. Read it before you adopt spec-dna in a repository, or when you change the setup of one. `init` shows a preview and writes only after a confirmation.

`init` never edits a document. In an instruction file (`AGENTS.md`, `CLAUDE.md`), it changes only its own generated lines: the spec-dna line, and the model-team line of the [Claude Code harness](harness.md). Each generated line carries the version of the generator and a hash of its text, so `check` finds a manual edit.

The spec-dna line has 400 bytes or fewer, with its marker. It names the 2 commands of each task (`spec-dna context` and `spec-dna check`), the 5 rules of the disciplines, the command `spec-dna skill show <name>` and the approval rule: an agent writes an approval commit only when the owner says so explicitly in the chat ([agents.md](agents.md)). `init` prints the bytes that the adapters add to each request, so you see the cost of the line.

```sh
spec-dna init                   # interactive on a terminal
spec-dna init --dry-run         # the preview only
spec-dna init --non-interactive # the detected defaults
spec-dna init --answers a.toml  # the answers of a file
spec-dna init --reconfigure     # the questions again, with the current values as defaults
spec-dna init --approver you@example.test   # the approver of the branch rule; the flag can repeat
```

Without a terminal (for example, when an agent runs `init`), `init` uses the detected defaults, as `--non-interactive` does. If `spec-dna.toml` exists and you do not set `--reconfigure`, `init` updates the agent adapters, as in 0.1. If the file has a `[harness]` table, it also updates the harness files ([Upgrade](#upgrade)). `spec-dna upgrade` writes the same files when it moves the pin.

`init` and `init --reconfigure` do not run if the pin in `spec-dna.toml` is a different version. They stop with exit code 3 and write nothing. For a pin from 0.4.0 up to the version of the binary, the message names `spec-dna upgrade`. For a 0.5 project, run `spec-dna upgrade` first ([upgrade guide](guides/upgrade-0.5.md)).

## Detection

`init` reads the file list of git (tracked files and untracked files that git does not ignore) and the Markdown files. It finds these facts:

| Fact | How |
| --- | --- |
| Greenfield or brownfield | A repository is brownfield if it has documents to map or 20 files or more |
| Document roots | The highest directories that are mostly Markdown, or that have a documentation name (`docs`, `doc`, `documentation`, `wiki`, `handbook`) |
| Types of roots | The name of the directory. See [Types of roots](#types-of-roots) |
| Size | The number of documents in the detected roots. Fewer than 30 documents, also in a new project, give the `small` graph. 30 documents or more give the `default` graph |
| Areas | The top folders of the roots that hold requirements, with the most documents first. If they give fewer than 2 names, the folders of the apps (the directories of the manifests). At most 8 names, and only if there are 2 or more. A name is one visible folder name. Archive, template, image and vendored folders are no areas. If 2 folders differ only in case, `init` proposes one name (the most documents, then the name), and the preview has a note that names the other folder, because the reader of `spec-dna.toml` stops such a pair |
| Id patterns | The names of the files. See [Id patterns](#id-patterns) |
| Types of numbered directories | The files of the directory. See [Types of numbered directories](#types-of-numbered-directories) |
| Requirement keys | Bold keys at the start of list items, such as `- **R-12** text`. They give `legacy-keys = ["R-{n}"]`. If some keys have a letter suffix, the pattern is `R-{n}*` |
| Statuses | The status words in the front matter of each root. A word that the type does not allow gets a `status-map` entry. For example, `shipped` and `implemented` map to `active`, and `approved` maps to `accepted` for a decision. An unknown word maps to the default status of the root |
| Heading languages and aliases | The scripts of the headings. Known translations of `Decision Outcome` and `Amendments` in decisions (for example `Entscheidung`, `Decisión`, `Nachtrag`) become `required-sections` and `amendment-sections` |
| Exclusions | Vendored directories (`vendor`, `node_modules`, `third_party`), generated directories (`dist`, `build`, `generated`), files that say that a generator wrote them, sibling directories with a vendor prefix (`vendor-*`, `external-*`), and the files of a decision root that are not decisions (a README without an outcome, `*-implementation-plan.md`) |
| Large reference directories | A directory with 100 Markdown files or more, or 1,000,000 characters or more, and with no typed directory in it. It becomes a `research` root |
| Required sections | If most documents of a type in a root do not have the required sections, the root removes the requirement |
| Budgets | The documents over the hard budget, at 3 levels |
| Stack and apps | Manifests such as `Cargo.toml`, `package.json`, `go.mod` and `pyproject.toml`, down to depth 3 |
| Tests | The test globs that match files, for `covers:` tags |
| Agent files | `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `.claude/skills`, `.cursor/rules` and other agent files |
| CI and hooks | `.github/workflows`, `.gitlab-ci.yml` and other CI files; pre-commit, husky and lefthook |
| Default branch | `origin/HEAD`. If it is absent, a local `main` or `master`. If both are absent, the current branch |
| Harness defaults (Claude Code adapter) | The `model` and the `advisorModel` of a current `.claude/settings.json` become the main model and the advisor. `init` adds `NotebookEdit` to `deny` if git tracks no `*.ipynb` file. A file in `.claude/agents` that is not generated and has the name of a role file makes that role `none`. The name is the `name` of the front matter, or the file name |

Detection skips vendored, generated and test fixture directories (`fixtures`, `testdata`, `__snapshots__`, `golden`). In a root `.`, it excludes the root files of the repository (such as `README.md`, `AGENTS.md`, `CLAUDE.md` and `CHANGELOG.md`). In a root that holds `.claude/skills`, it excludes the generated skill `spec-dna/`. Detection only reads files. It does not use the network.

### Types of roots

The name of a directory gives the type of the root:

- `adr`, `adrs` and `decisions` give `decision`.
- `specs` and `requirements` give `requirements`.
- `contracts` and `interfaces` give `interface`.
- `guides`, `guidelines`, `runbooks` and `playbooks` give `procedure`.
- `incidents`, `incident`, `postmortems` and `postmortem` give `incident`.
- `invariants` and `principles` give `rule`.
- `rfcs`, `proposals`, `changes`, `tasks`, `plans` and `enhancements` give `research`.
- `research`, `references`, `meetings` and `archive` give `research`.

`init` never proposes the types `intent`, `spec` and `plan`, because they are valid only as `intent/<slug>/intent.md`. A flat folder of proposals or changes (`rfcs`, `proposals`, `changes`, `tasks`, `plans`, `enhancements`) and the prefixes `RFC`, `RFD`, `KEP` and `PEP` give `research`. The preview then says: to start an open proposal, run `spec-dna new intent <slug>`.

`init` writes no root for a type that no layer of the chosen graph has. The `small` graph has no layer for `interface`, `procedure` and `research`. The preview gives a note for each such root. A mixed root whose detected default type has no layer loses its `default-type`. If `init` writes no root, the project gets the root of a new project. Choose `default` at question 2 to keep these roots.

This rule also applies to a top-level directory, such as `rfcs/`. The names `incident` and `postmortem` are also common names of code directories. They give a type only inside a documentation tree, or to a directory that is mostly Markdown. Agent directories (`.claude/skills`, `.claude/agents`, `.cursor/rules`) give `procedure`. A root becomes mixed if the front matter of its documents names another type.

### Id patterns

- File names such as `ADR-012-cache.md` give `legacy-ids = ["ADR-{n}"]`. A directory of such files becomes a root.
- Decision files such as `0001-use-postgres.md` (adr-tools, for example in `doc/adr`) give `{ alias = "ADR-{n}", file = "{n}-*" }`. These conditions apply:
  - each number of the root is unique;
  - no file of the root starts with a date;
  - no other root makes the same alias.
- Files with a number and no prefix make a root only for decisions.

### Types of numbered directories

The files of the directory give its type. `init` examines these facts in this order:

1. The prefix: `ADR` gives `decision`, `RFC` gives `research` and `INC` gives `incident`.
2. The front matter `type` of most files.
3. The content of most files:
    - `Decision`, or `Context` and `Consequences`, gives `decision`.
    - `Root cause` as a heading or a table field, or `Timeline` and `Impact`, gives `incident`.
    - `Steps` gives `procedure`.
    - Requirements with a BCP 14 keyword give `requirements`.

If the files show no type, the type is the type of the root that contains the folder, if that root has a type. In a mixed root, the type is `research`. A mixed documentation tree takes the type of its numbered files only if it has no other documents.

## Questions

`init` asks the questions in this order. It skips a question that does not apply. Questions 12, 13 and 14 configure the Claude Code adapter: the [harness](harness.md) and the [evals](evals.md). They write files for Claude Code only. For another agent, answer `none` to question 12. Then `init` does not ask the questions 13 and 14.

Question 5 has 2 prompts: the gate and the approvers. Question 12 has 2 prompts: the level and the parts.

| # | Question | Default |
| --- | --- | --- |
| 1 | The profile: `lite`, `standard` or `strict` ([profiles.md](profiles.md)). In a new project, the answer also sets the defaults of the questions 4, 5, 12 and 14 ([The preset of a profile](#the-preset-of-a-profile)) | `standard` |
| 2 | The layer graph: `small`, `default` or `custom`. The question shows the number of documents and the types that the small graph has no layer for. `custom` keeps the `[layers]` tables of the current `spec-dna.toml`, so it is only for `--reconfigure` | By the number of documents: fewer than 30 gives `small`, 30 or more gives `default`. In `--reconfigure`, `custom` |
| 3 | The areas of `requirements/`: folder names, or `none` for one folder. The question shows the detected areas | The detected areas, at most 8. `none` if fewer than 2 are detected |
| 4 | The change policy: `three-class`, `two-class` or `none` ([config.md](config.md#the-table-change)) | `three-class` in a new project, and `two-class` for `lite`. In `--reconfigure`, the current policy; `none` for a file with no `[change]` table |
| 5 | The gate: `block`, `warn` or `off`. It sets the severity of SD910. Then the approvers: the e-mails whose approval commits count, or `none` ([The approvers](#the-approvers)). `init` does not ask the question if the policy is `none`, and it does not ask the approvers for the profile `strict` | `block` in a new project, and `warn` for `lite`. In `--reconfigure`, the current gate. For the approvers, see [The approvers](#the-approvers) |
| 6 | The evidence policy (`profile`, `off`, `warning`, `error`) and the link style (`covers`, `verified-by`, `both`) | `profile, both` |
| 7 | The language of normative text, as a BCP 47 tag, and the heading aliases: `en; outcome: Entscheidung; amendments: Nachtrag` | The detected language and aliases. `en` if detection finds none |
| 8 | The mapping of each detected root: press Enter to accept it, enter `drop` to drop it, or edit it. An edit has the form `type=requirements status=active exclude=a/**,b.md` | Accept |
| 9 | The budget level: `default`, `relaxed` (2 times), `tight` (half) or `off`. The question shows the number of documents over the hard budget at each level | `default` |
| 10 | The agent adapters: `agents-md`, `claude-md`, `skill` or `none`. `agents-md` writes the line into `AGENTS.md`, which Codex, Cursor and many other agents read. `claude-md` writes it into `CLAUDE.md`, which Claude Code reads. `skill` writes the skill for Claude Code | `claude-md` and `skill`, and `agents-md` when `AGENTS.md` exists or `CLAUDE.md` does not exist |
| 11 | The CI step (`github`, `gitlab`, `none`), the hook (`pre-commit`, `husky`, `lefthook`, `none`) and the base branch | The detected CI system, hook framework and default branch |
| 12 | The Claude Code harness: `none`, `lean` or `full` ([harness.md](harness.md)). The question says what each level installs, and that the files are for Claude Code. Then the parts, as words `skills=<names or none>`, `review-md=<true or false>` and `protect-tests=<fix-branches, always or off>`, in any order. A word that you omit keeps its default | `lean`. In `--reconfigure`, the current level; for a configuration with no `[harness]` table, `lean`. For the parts, see [Parts of the harness](#parts-of-the-harness) |
| 13 | The models: `role=model[:effort]` words, such as `verifier=sonnet:medium`. `advisor=<model>/<agent model>[:effort]` sets the advisor and the model of `fallback-advisor`. `none` disables a role. The question is not asked if the level is `none` | `orchestrator=opus:xhigh advisor=fable verifier=opus:high researcher=haiku:high helper=haiku`, and for `full` also `implementer=sonnet:high code-simplifier=sonnet:high`. The words `reviewer=` and `spec-writer=` of spec-dna 0.5 give exit code 3: the role `reviewer` is now `verifier`, and the role `spec-writer` ended. The detected values replace the main model and the advisor |
| 14 | The evals: `none` or `skeleton` ([Evals](#evals)). The question says what `skeleton` writes. It is not asked for the harness level `none`. The answer is then `none` in a new project, and the current state in `--reconfigure` | `skeleton` in a new project, `none` for the profile `lite`, and `none` for the level `none`. In `--reconfigure`, `skeleton` if the file has the table `[evals]`, else `none`, also for the level `none` |

The numbers are the numbers of the question in the `--json` output.

Question 2 writes the tables `[layers.<name>]` of the graph. The layers give the types of the project, so `init` writes no `types` key. Question 3 writes `areas = [...]`, if the answer has a name. `init` never proposes a group: add a `[groups]` table manually.

Questions 4 and 5 write the table `[change]` with 6 keys (and the 2 keys of the approval, below): `policy`, `gate`, `high-risk-layers`, `high-risk-paths` (`[]`), `shared-paths` and `max-fanout` (`8`). The default of `high-risk-layers` is `["interfaces", "rules"]`, without the layers that the chosen graph does not have. The `small` graph has no `interfaces` layer, so it gives `["rules"]`. A new project gets the table.

`shared-paths` lists the documents that each change can edit, so SD913 does not report them. No question asks for it. For a new project, `init` writes the top file of each layer that has the type `roadmap` or `questions`, as `spec-dna new` writes it, and each Markdown file whose name is `roadmap`, `questions` or `open-questions`, in lowercase and without `.md`. The list is sorted and has no repeated path. The graphs `small` and `default` give `["docs/questions.md", "docs/roadmap.md"]`.

The table also has the keys `approval` and `approvers` (see [Approval in the branch](config.md#approval-in-the-branch)). The second prompt of question 5 asks for them, as [The approvers](#the-approvers) says.

### The preset of a profile

The answer to question 1 sets the defaults of a new project. The answers to the other questions replace them. [profiles.md](profiles.md#what-init-writes-for-each-profile) gives the table of the keys. In short:

- `lite` asks for `two-class` and `warn`. It writes no `REVIEW.md`, `protect-tests = "off"`, no `[evals]` table and no starter file. Its skills are `brainstorm`, `write-plan` and `finish`, each one only if it is on by default.
- `standard` asks for `three-class` and `block`. It writes `REVIEW.md`, `protect-tests = "fix-branches"` and the starter files of the evals. Its skills are the skills that are on by default.
- `strict` is as `standard`, with `approval = "base"` and `protect-approvals = true`. `init` writes no eval pack for it: the pack has 57 evals and 812 files. With the answer `skeleton`, the preview and the result print a note that names `spec-dna eval pack` and the folder `<dir>/pack/`. With `--json`, the note is an item of `notes`.
- The level `full` of question 12 proposes the 11 skills, for each profile.

A skill is on by default when `spec-dna skill list` says so ([ADR 0023](adr/0023-default-skills-from-the-release-table.md)). The rule reads the 7 skills of the design, the verdict of each skill in the table of the release, and one constant. With the constant `true`, a skill is on only with the verdict `on`. With `false`, a skill of the 7 stays on unless its verdict is `off`. A skill outside the 7 is on only with the verdict `on`. The constant is `false` in 0.7.0, so `execute`, `verify` and `debug` are off by default and the 8 other skills are on. `init` never reads a table of the project for this: the default is the same in each project of a release.

`--reconfigure` keeps the current values and applies no preset. A part that the file does not set gets the default of the profile of the file. If the answer to question 1 differs from the file, `init` changes the severities and, for `strict`, the keys of ADR 0018: `approval = "base"` (also when the file has `branch`, because `strict` reads the approvals of the base only; the list `approvers` stays in the file, and a note says so) and `protect-approvals = true` (only in a file that has no such key). It prints this note: "the profile `<p>` sets other defaults only for a new project; answer questions 4, 5, 12 and 14 to change them".

### The approvers

`approvers` lists the e-mails of the persons whose approval commits count. `init` never writes the address of an agent or a bot, because an agent can commit with it. The list of these addresses is in the binary: `noreply@anthropic.com`, and each address that ends with `[bot]@users.noreply.github.com`. The hidden address of a person, such as `1234+name@users.noreply.github.com`, is not on the list.

`init` takes the approvers from the first of these sources that gives a list:

1. `--approver <e-mail>`. The flag can repeat, and `--approver=<e-mail>` is the same.
2. The answers key `approvers`, an array of strings. `[]` means no approvers.
3. The answer to the second prompt of question 5: e-mails with commas, or `none`. The default is the list of the next sources.
4. For `--reconfigure` of a file that has the key `approval`: the current list.
5. `git config user.email`, if it is set and it is not the address of an agent or a bot.

An item of the sources 1 to 3 must have an `@` and no `*`. If it is the address of an agent or a bot, `init` stops with exit code 2 and writes nothing. The prompt runs for `lite` and `standard`, and only when the policy is not `none`. Without a terminal, `init` acts as `--non-interactive`: it asks nothing.

| Profile | A list from the sources 1 to 5 | No list |
| --- | --- | --- |
| `lite`, `standard` | `approval = "branch"` and `approvers = [<the list>]` | `approval = "base"`, no `approvers`, and a note |
| `strict` | `approval = "base"`. A list from a source 1 to 3 is not written, and a note says so | `approval = "base"` |

The note for no list says why. If the e-mail of git is the address of an agent or a bot, the note names it and gives the command `spec-dna init --reconfigure --approver <your e-mail>`. If git has no e-mail, the note names the 2 keys. `strict` reads the approvals of the base only, so `init` never writes `branch` for it. An empty `approvers` with `approval = "branch"` is an error, so `init` never writes that pair.

With `--reconfigure`, a file that has the key `approval` keeps `approval` and `approvers`, except that `strict` writes `base`. A list from a source 1 to 3 replaces `approvers` and sets `approval = "branch"`, but not for `strict`. A kept list with an agent address gives a note, and `check` gives SD926. A file with no `approval` key, such as a file of 0.6, gets the keys of its profile. `--approver` with a `spec-dna.toml` that exists and no `--reconfigure` changes nothing, as `--answers` does. The workflow that `init` generates to run `check` uses `fetch-depth: 0`, because the rule of `branch` reads the commits of the branch.

Run the approval of each document with `spec-dna approve <slug> <artifact>` in a terminal of your own ([commands.md](commands.md#approve)).

A file that has no `[change]` table has the policy `none`. For it, the answer `none` writes no table, and another policy writes the table. With `--reconfigure`, the current values are the defaults. `init` keeps `high-risk-paths`, `shared-paths`, `max-fanout` and the layers of `high-risk-layers` that the graph has. If the file has a `[change]` table with no `gate`, the profile sets SD910. Question 5 then proposes the gate with the same severity: `warn` for `lite`, `block` for the other profiles.

A policy other than `none` needs the `intent` layer, because a change is the folder `intent/<slug>/`. The graphs `small` and `default` have it. If the layers of `custom` have no `intent` layer, the preview says so, and `init` writes the policy `none`.

With a policy other than `none`, `init` prints the `CODEOWNERS` line `docs/intent/*/spec.md @OWNER` in a note of the preview. The path follows the first mixed root and the folder of the `intent` layer. With `--json`, the field `codeowners` has the line, or `null` for the policy `none`. `init` never writes the `CODEOWNERS` file.

Only a higher-risk change has a spec, so the line protects its approval. A higher-risk plan has the same path as a routine plan, so a path rule cannot select it. The repository process must require the owner for a higher-risk plan, for example with branch protection.

Question 7 writes the top key `language = "<tag>"`. The answer `en` writes nothing, because `en` is the default. A tag that is not a BCP 47 tag, such as `German`, stops `init` with exit code 2, and `init` writes no file. The word lists of the lint are English ([config.md](config.md)). When `language` is not English, the checks that use them are off for the whole project (SD801 to SD805 and SD904).

Version 0.5 wrote `[rules] lint = "off"` for this purpose, but `init` does not write it now. A `lint` rule in the current file stays as it is.

Detection proposes the language from the heading aliases that it found: `Entscheidung` gives `de`, and `Decisión` gives `es`. If it finds no alias, it uses the script that most headings have: `Greek` gives `el`. Several languages share the scripts `Cyrillic` (`ru`), `Arabic` (`ar`), `Devanagari` (`hi`) and `Han` (`zh`). A tag from such a script is a guess, and a note says so. An alias can decide between the languages: `Рішення` gives `uk`.

Latin headings with no alias give `en`. If a script has no tag, or the aliases belong to 2 languages, the proposal is `en` and a note in the preview says why. Enter the tag of your language to change it.

A new project gets one mixed root (`docs`, or the directory of `--root <dir>`) with no `default-type`. The folder of a layer gives the type of a document there. For example, `docs/requirements/loans.md` is a `requirements` document. `spec-dna new <type> <id>` writes a document into the folder of its layer. A document with no type and no layer folder gives SD303.

### Parts of the harness

The second prompt of question 12 sets 3 keys of `[harness]`. An omitted word keeps the default below.

| Part | Writes | Default of a new project |
| --- | --- | --- |
| `skills` | The skills `.claude/skills/spec-dna-<name>/SKILL.md`. The 11 names are `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify`, `finish`, `debug` and `post-mortem` ([harness.md](harness.md#the-workflow-pack-and-reviewmd)) | The skills that are on by default (`spec-dna skill list`; [ADR 0023](adr/0023-default-skills-from-the-release-table.md)) at the level `lean`, and all 11 at the level `full`, if the answer to question 4 is not `none`, else `none`. `lite` proposes only `brainstorm`, `write-plan` and `finish` of them |
| `review-md` | `REVIEW.md` | `true`, and `false` for `lite` |
| `protect-tests` | The hook `protect-tests` in `.claude/settings.json` | `fix-branches`. `off` if `[evidence] tests` is empty, which is the link style `verified-by`, and `off` for `lite` |

- The defaults follow the final answers, so a later answer to question 4 or 6 changes them.
- The descriptions of the 7 skills of the design (D2) add 1,250 bytes or fewer to each request, and the 11 skills add 1,900 bytes or fewer. A release that turns on fewer skills adds fewer bytes. `REVIEW.md` adds none. `init` prints the total bytes that the harness adds, and the bytes that the adapters add (the line of 400 bytes or fewer, and the description of the adapter skill).
- In `--reconfigure`, a part that the current `[harness]` table sets keeps its value. A part with no key in the table gets the default of the profile of the file. So `init --reconfigure` offers the new parts to a project that has none of them.
- A `REVIEW.md` with no harness marker gives `review-md=false`. A skill file with no marker leaves that skill out of the default. A note names each such file, and `init` does not change it. A skill file of 0.6 for a name that ended (`spec-dna-intent`, `spec-dna-spec` or `spec-dna-plan`) is different: if it has a marker and you did not edit it, `init` removes it. If you edited it, `init` keeps it and names it with `kept <path>: edited by hand (SD501)`. An answer that asks for such a file stops `init` with exit code 2.
- The answer keys are `harness-skills` (an array of names, `[]` for none), `harness-review-md` (`true` or `false`) and `harness-protect-tests`. They need a harness level other than `none`.
- The key `protect-approvals` is no part of the prompt, and no answers key sets it. `init` writes `protect-approvals = true` for the profile `strict` in a new configuration, and in a file that has no such key. A file that has the key keeps its value. The key adds the hook that denies an approval by an agent ([harness.md](harness.md#spec-dna-hook-protect-approvals)).

### Evals

The answer `skeleton` of question 14 writes the starter files of the evals. [evals.md](evals.md#start) lists them. `init` starts no run. It adds no root and no text to an agent request, so the output of `context` does not change.

- `init` writes each starter file once and never again. A file that exists stays, and a note names it. You can edit each file.
- The folder is `evals`. If git lists a file under `evals/`, the folder is `.spec-dna/evals`, and a note says so.
- `skeleton` with a current `[evals]` table keeps the table as it is and writes no file. `none` with a current table removes the table and keeps the files, and a note says so.
- No starter file is Markdown. The folder of the evals is never part of a root, so a later fixture can hold Markdown files.
- `init` writes the workflow only for the CI answer `github` of question 11. CI runs no eval until you set the repository variable `SPEC_DNA_EVALS` ([evals.md](evals.md#the-ci-job)).
- After a write that created the workflow, `init` prints "To run the evals in CI, set the secret and the variable:". Then it prints `gh secret set ANTHROPIC_API_KEY` and `gh variable set SPEC_DNA_EVALS --body true`.
- `init --json` has the field `evals_commands` with these 2 commands, or an empty array.
- The check SD918 (each incident needs an eval) needs the table `[evals]`.

## Preview and files

The preview starts with the 14 answers and how `init` applied each one. A line gives the number of the question, the value, and what it writes. It also says if the value is the default of the question or a change. An answer that equals the default and writes no key or file says `writes nothing`. For example, answer 3 (`none`) writes no `areas` key.

Answer 2 writes the `[layers.<name>]` tables. Answers 4 and 5 write the `[change]` table. Answer 12 (the harness level) writes the `[harness]` table and the harness files. Answer 13 (the models) writes the `[harness.roles]` table. Answer 14 (the evals) writes the table `[evals]` and the starter files.

If the level is `none`, answer 13 shows `-` and says `writes nothing`. A `note:` line follows the answers for each root that `init` did not write, and for a folder of proposals.

Then the preview shows the full `spec-dna.toml` and each file to create or change:

- `spec-dna.toml`
- the generated line in `AGENTS.md` and `CLAUDE.md`, and the skill `.claude/skills/spec-dna/SKILL.md`. The line is the same for each agent. It never imports another file
- for `github`: `.github/workflows/spec-dna.yml` (the job of [quickstart.md](quickstart.md#ci)). If the file exists, `init` does not change it.
- for the answer `skeleton`: the starter files of the [evals](#evals), and for `github` also `.github/workflows/agent-evals.yml`
- a local hook at the end of `.pre-commit-config.yaml` (only if `repos:` is its last top-level key), or one line in `.husky/pre-commit`. The hook runs `spec-dna check --staged`, which checks the git index: a commit records the index, not the working tree. A hook that 0.4.1 wrote (`spec-dna check`) reads the working tree. `init` changes that exact line in place when it runs again (also when `spec-dna.toml` exists), and the preview shows the diff. It does not change any other text of a hook.
- the harness files of the Claude Code adapter, if the level is not `none`. They are `.claude/agents/*.md`, `.claude/agents/model-team.md`, the skills `.claude/skills/spec-dna-<name>/SKILL.md`, `REVIEW.md`, the model-team line in `CLAUDE.md` and `.claude/settings.json`

The harness block of the preview shows each harness file with `create`, `change` or `remove` and a note. It also shows the warnings and the reasons for disabled roles. When `init` creates or changes `.claude/agents/Explore.md`, a note says that the file replaces the built-in `Explore` subagent of Claude Code. The note also says how to remove it: `researcher = { claude-code = "none" }` and `init --reconfigure`.

The block shows the bytes that the harness adds to each request of the main session. After a write, `init` tells you to restart Claude Code. The block also has a note on the hooks. The note names 0.7.0 if `protect-approvals` writes its hook entry, which is the default of the profile `strict`. It names 0.6.0 if `protect-tests` writes its hook entry, which is the default of a new project. Otherwise, the note is `Each teammate needs spec-dna 0.5.0 or later on PATH for the hooks.`

After the list, the preview gives the diff of each file other than `spec-dna.toml`. This includes the harness files and the files that `init` removes. A new file shows as a diff against `/dev/null`. A file that `init` removes shows as a diff to `/dev/null`.

`--dry-run` shows the same diffs, even for a project that has a `spec-dna.toml` already (`init` then updates the adapters only). With `--json`, each entry of `files` has a `diff`.

A new project and `--reconfigure` also give `notes`, `warnings`, `codeowners` and `answers` with the 14 answers. The field `number` of an answer is the number of the question. The field `default_type` of a detected root is `null` for a mixed root with no `default-type`. With a `spec-dna.toml` that exists, `--dry-run --json` gives `files` with the same entries, and `harness`.

The skill lists the types of the project: the types that a layer has. If `spec-dna.toml` has the key `types`, the skill lists only the types of that key. If you change the layers later, for example with `spec-dna layer add`, run `spec-dna init` to update the skill. Otherwise `check` reports SD502.

For GitLab CI and lefthook, the preview gives the text to add manually. `init` checks that it can read the new `spec-dna.toml` before it writes a file.

`--json` has a `harness` object. Its fields, in this order, are `level`, `files` (each has `path`, `action` and `note`), `kept` (the `kept` lines), `warnings`, `notes` and `request_bytes`. `notes` is a list of text. It holds the notes of the harness block, for example the note that `.claude/agents/Explore.md` replaces the built-in `Explore` subagent. The list is empty if `init` has no note to give.

The adapter skill `.claude/skills/spec-dna/SKILL.md` also has the block of the 5 disciplines and the rule that a spec-dna skill replaces a general skill of the same stage ([harness.md](harness.md#disciplines)).

`init` writes the harness files as [harness.md](harness.md) says. It changes `.claude/settings.json` by a merge, and it keeps each key that it does not set. It does not replace a file that it did not generate.

If `.claude/settings.json` of the project has `enabledPlugins` with a plugin whose name has the word `superpowers`, `init` prints a note for the harness. The note says that a spec-dna skill replaces a general skill of the same stage, that nothing proves the choice of Claude Code, and that `init` does not move a file that the plugin writes, such as `docs/superpowers/specs/`: exclude that folder from the roots. `init` reads this file of the project only, never a file of the home folder, and `init` changes no other part of the plugin setup. The note is in the `notes` of the `harness` object of `--json` too.

`init` refuses the harness and exits with code 2 in these cases:

- git ignores `.claude/agents` or `.claude/settings.json`. A team cannot commit the files.
- An answer sets a role to a model, and a file that `init` did not generate has the agent name of that role. Set the role to `none`, or rename the agent.
- `.claude/settings.json` is not valid JSON.

## Answers file

An answers file is TOML. Each key is optional. An absent key keeps its default.

```toml
profile = "standard"
graph = "default"               # "small", "default" or "custom"
areas = ["valves", "sensors"]   # folders of requirements/; [] or "none": one folder
change-policy = "three-class"   # "three-class", "two-class" or "none"
gate = "block"                  # "block", "warn" or "off"; not used with the policy "none"
approvers = ["you@example.test"]  # persons only; [] for none; an agent address gives exit code 2
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
harness-models = ["orchestrator=opus:xhigh", "verifier=sonnet:medium", "advisor=fable/opus"]
harness-deny = ["NotebookEdit"] # tools that no agent gets; replaces the default
harness-skills = ["brainstorm", "specify", "clarify", "write-plan", "review", "finish", "post-mortem"]  # [] for none; the default follows the policy and the level
harness-review-md = true
harness-protect-tests = "fix-branches"  # "fix-branches", "always" or "off"
evals = "skeleton"              # "none" or "skeleton"
write = true                    # false: the preview only

[[roots]]                       # optional: these roots replace the mapping
path = "docs"
type = "mixed"
default-type = "requirements"
```

The `[[roots]]` tables use the keys of `spec-dna.toml` ([config.md](config.md)).

The answers key `types` of 0.5 does not exist in 0.6. An answers file with this key gives an error that names `graph`. The key `types` of `spec-dna.toml` stays: it narrows the types of the layers. `graph = "custom"` is only for `--reconfigure`, because a new project has no layers to keep.

`harness-models` has one `role=model[:effort]` word for each role that you change. A role that is not in the list keeps its default. A role that the level does not include is an error. The model `none` disables a role, and `inherit` writes no model.

The text after the last `:` is an effort if it has no digit, also when it is empty. An unknown effort is an error: `opus:x-high` and `opus:` give `unknown effort`. Text with a digit is part of the model name.

A suffix with a digit, such as `:0` in a cloud model id, stays part of the model. The advisor model cannot be haiku, in any letter case.

## Reconfigure

`init --reconfigure` asks the questions again. The defaults are the current values. They are the profile, the graph (`custom`), the areas, the evidence rule, the test globs, the roots and the evals state (question 14).

With `custom`, `init` writes the current `[layers.<name>]` tables again, in the same order. A file that has no such tables stays without them. Another answer replaces the tables, and a root of a type that the new graph has no layer for is not written.

`init` does not change the keys that no question asks for:

- `install-source`, `self-urls`, `types`, `[repos]`, `[lint]`, `[context]`, `[groups]` and `[templates]`
- the table `[evals]`, as question 14 says: `skeleton` keeps it as it is
- the keys of `[change]` that no question asks for: `high-risk-layers`, `high-risk-paths`, `shared-paths`, `max-fanout`, `max-task-files`, `approval`, `approvers` and `base`. `max-task-files` stays only if the file sets it. `base` is the local branch of the change checks. `init` never asks for it, and it stays as you set it. `high-risk-layers` stays only if it is not the default of the current graph. If it is that default, `init` writes the default of the new graph, so a file that changes from the small graph to the default graph gets `["interfaces", "rules"]`. In each case, `init` writes only the layers that the new graph has. A list that you set, such as `[]`, stays. The answers of questions 4 and 5 replace `policy` and `gate`
- `language`: it is the default of question 7. The answer `en` removes the key
- every rule of `[rules]`. An answer replaces only the rule of its group: `evidence` and `budgets`.
- the test globs
- the `[budgets.<type>]` tables, unless the budget level changes
- `[conventions]`
- the section keys of each root (`required-sections`, `amendment-sections`, `history-sections`, `companion-sections`, `live-sections`), and its budget, profile and rules keys. A root with no `default-type` keeps none. A section key that equals its default (`live-sections = { decision = ["Status"] }`) is not written

The harness has the current `[harness]` table as its defaults. `--reconfigure` also offers the parts that a project has not yet, such as the skills, `REVIEW.md` and `protect-tests`.

A smaller level removes the generated files that it no longer produces, if you did not edit them. These files include the skills and `REVIEW.md`. It removes the model-team line and the hook handlers too. It keeps each key of `.claude/settings.json` that the new configuration no longer sets, and it prints one `kept` line for each key. Remove these keys manually.

`init` does not edit the key `types`. If the answer to question 4 is not `none`, and the `types` list lacks `intent`, `spec` or `plan`, `init` prints `warning: the policy <p> needs the types intent, spec and plan; add <list> to types in spec-dna.toml`. A file with no `types` key allows each type, so it gets no warning. With `--json`, the warning is in the top-level field `warnings`.

`init` does not keep the comments of the old file. The preview shows the new file before `init` writes it.

## Upgrade

`init` with a `spec-dna.toml` that exists and no `--reconfigure` updates the generated files of the pinned version. `spec-dna upgrade` moves the pin, and it writes the same files. These rules apply to `init`:

- `init` updates the adapters and the skill. The skill lists the types of the layers of the project. `init` also changes the exact hook line that 0.4.1 wrote, `spec-dna check`, to `spec-dna check --staged`. The harness upgrade below comes after these changes.
- A configuration with no `[harness]` table (spec-dna 0.4 or older) gets no harness. Run `init --reconfigure` to add it.
- `init` regenerates the harness files of a configuration with a `[harness]` table. These are the agent files, the skills of `[harness] skills`, and `REVIEW.md` if `review-md` is `true`. It writes each generated file that you did not edit, and each file that is absent. It changes the model-team line and the settings entries that differ. It prints each path that it writes.
- `init` keeps a generated file that you edited, or that has text after its marker, and it prints `kept <path>: edited by hand (SD501)`. `check` reports the file. Delete it. Then run `init` to generate it again.
- `init` never removes a file on this path. Only `--reconfigure` removes files.
- `init --dry-run` on this path shows the diff of each file, also of the harness files. With `--json`, `files` has a `diff` for each file.
