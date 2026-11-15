# Changelog

This file lists the changes of each version. The project uses [Semantic Versioning](https://semver.org). Diagnostic codes are in [docs/diagnostics.md](docs/diagnostics.md).

## Unreleased

### Changed

- `spec-dna init` question 7 writes the top key `language = "<tag>"` instead of `[rules] lint = "off"`. The answer `en` writes nothing, and a value that is not a BCP 47 tag is a usage error (exit code 2). Detection proposes a tag from the heading aliases and from the script of the headings (`Entscheidung` gives `de`). If it cannot map them, it proposes `en` with a note. `init --reconfigure` keeps `language`, a `[change]` table and a `[rules] lint` rule as they are. A new project gets no `[change]` table ([docs/init.md](docs/init.md)).
- `history`, and `mv` to a new document of a type that no layer has, stop with exit code 2 and change nothing. The message names the command to run, such as `spec-dna layer add research --types research`: the small graph of a new project has no `research` layer.
- The default of `roots.live-sections` is `{ decision = ["Status"] }`. SD405 does not report a change to the `Status` section of an accepted decision, and `show --conventions decision` lists `Status`. Set `live-sections = { decision = [] }` to keep the 0.5 behavior.
- The document types of 0.6: 15 types. `spec` is now `requirements`, `contract` is `interface`, `invariant` is `rule`, `guide` and `runbook` are `procedure`, `note` and `reference` are `research`, and `change` is `intent`. A type name of 0.5 in `spec-dna.toml` is an error (exit code 3), and in a front matter it gives SD303. The message names the new type. The tier label `invariant` of `context` is now `rule`.
- The statuses of an `intent` are `draft`, `accepted` and `closed`. SD304 names the new status for `proposed`, `rejected` and `done`.
- A `research` document has the behavior of `reference`: no budget, `context` never serves it, and SD202 reports its deletion. A `procedure` has the budget of a guide and no default required section.
- SD202 follows the layer graph. A deleted document of a layer with `keep = "until-finished"` gives no SD202. The `intent` layer is such a layer by default, and so is a custom layer with this value. In a `forever` layer, only the field `was-type: note` or `was-type: change` allows a deletion, as 0.5 did for a `note` and a finished `change`. The status `closed` gives no exemption.
- The place gives the type. In a mixed root, the type of a document comes from these sources, in this order:
  - The `type` field of the front matter.
  - A top file, such as `terms.md`.
  - The folder of a layer.
  - The file name in the `intent` folder.
  - `default-type`.
- `type:` is optional where the place gives the type. A mixed root has no default for `default-type`, so a document with no type gives SD303.
- The types `intent`, `spec` and `plan` are valid only as `intent/<slug>/<type>.md`. A `spec` at another place gives SD303, and its message names `requirements`. A root `type` or `default-type` of one of these types is an error (exit code 3). A document of a type that no layer has gives SD303 ([docs/types.md](docs/types.md)). A document with no type, with an unknown type, or with an intent-only type at a wrong place has no valid type: SD303 is its only report of the type, so it gets no SD304 for its status and no SD305 for a section, and no budget applies to it.
- `spec-dna new` writes each document at the place of its layer. `new requirements loans` writes `requirements/loans.md`, and an id is relative to the folder of the layer. A top file type takes no id (`new terms`). `new spec` and `new plan` stop with exit code 5 when the folder of the change does not exist. `new` writes no `type:` line where the place gives the type.
- `spec-dna init` asks the layer graph at question 2 (`small`, `default` or `custom`) instead of the knowledge types. Fewer than 30 documents, also in a new project, give `small`. 30 documents or more give `default`. `custom` keeps the `[layers]` tables of the current file, so it is the default of `--reconfigure`. The answers key `types` of 0.5 is not valid any more. An answers file with this key gives an error that names `graph`. `init` asks at most 14 questions. It uses the numbers of the design: this version asks 1 to 3 and 6 to 13. `init` writes no root for a type that no layer has, and a note in the preview says why ([docs/init.md](docs/init.md)).
- A new project gets one mixed root `docs` with no `default-type` and no status. The folder of a layer gives the type of a document. `init` writes no `types` key.
- The skill of `init` lists the types that the layers have. After `spec-dna layer add`, run `spec-dna init` to update it, or `check` reports SD502. The skill and the agent `spec-writer` describe the places of `spec-dna new`.
- `init --reconfigure` keeps a mixed root that has no `default-type` (it became `research`). It does not write `live-sections = { decision = ["Status"] }`, which is the default now. It keeps the keys `types`, `[groups]` and `[templates]`.
- The demo and the quickstart use the layout of 0.6: `requirements/loans`, `interfaces/http` and `decisions/`. The demo adds the layer `interfaces` with `layer add`.
- CHANGE OF BEHAVIOR: the order of `context` follows the levels of the layer graph. It lists the layers from level 0 up, and the layers of one level in the order of the configuration. In the default graph, `foundation` comes before `rules`. In 0.5 a rule came first. A top file (`product`, `system` or `terms`) is in the first tier only if its `applies-to` matches, so the top files do not go into each context. The order of the layers also replaces the order of the types among the documents that cite a code path. The `context` order of the tiers does not change ([docs/commands.md](docs/commands.md#context)).
- CHANGE OF BEHAVIOR: a layer with `context = "on-request"` (in the default graph: `research`, `intent`, `incidents` and `pending`) does not serve its documents, unless a target names them. `show` reads them. `context` counts these documents in one line and lists no file. In 0.5 it listed the intents, the incidents and the roadmaps that cite a file. The `--json` output has no count.
- SD405 is `decision-locked` and SD611 is `decision-links-intent`. The codes stay. In `--json`, read `code`: the field `name` changed. The text output shows the new names. The fingerprint and the keys of `[rules]` do not change. The messages and the fix texts say "locked", where 0.5 said "frozen". The fix of SD611 tells the author to name the intent as plain text, not as a link. SD901 and `finish` would stop on a link.
- The requirement lint (SD801 to SD805) is off when `language` has a primary subtag other than `en`, also with `check --lint`.
- SD101 and SD102 give no diagnostic for a reference in the locked text of an accepted decision (the body outside the amendment, history and live sections). The author cannot change that text, and a finished intent leaves no document to link.

### Added

- The command `spec-dna next <slug> [--base <rev>]` and the command `spec-dna status [--group <name>] [--base <rev>]` ([docs/commands.md](docs/commands.md#next)). The stage of a change comes from its files, their statuses, the diff against the lifecycle base and the evidence: `closed`, `plan`, `design`, `build`, `test` or `deploy`. `next` prints a checklist of at most 12 lines to the next gate. Each line says done or open, and the last line gives the edit that approves the artifact. In the `build` stage, it names the next open task of the plan (id, text, files and verification). `status` lists each change with its class, stage, next gate, task progress, areas and staleness (SD911), the number of started changes, and the open follow-ups of the incidents. `--group` lists the changes of a group of `areas`. An unknown group is exit code 2, and `next` gives exit code 5 for a slug with no folder. Without a base, both commands print a note, and the stage stops at `build`. With `[change] policy = "none"`, they say that the change process is off. `--json` gives the fields of both commands. `context --group` is not in this version.
- Each line of `next`, and each item of its `--json`, has the name of a skill: `brainstorm`, `specify`, `clarify`, `write-plan`, `execute`, `tdd`, `review`, `verify` or `finish`. The skills come in 0.7.0. The names are fixed until then.
- The code SD922 (`question-open`): a spec with the status `approved` has an open question, a `- [ ]` item in its `Open questions` section. It is a warning in `lite` and an error in `standard` and `strict`, as SD919. It is in the rule group `changes`, and the policy `none` turns it off. The `spec` template has an `Open questions` section after `Concerns`. `Concerns` and SD919 stay.
- The parser keeps the list items of the sections `Order` and `Open questions`. A task of a plan is an item of `Order` with a task box and a task id: ``- [ ] t1: <text>. Files: `path`. Verify: <text>.`` The model has the tasks of a plan, the first open task, and the open questions of a spec. A duplicate task id is marked in the model and gives no diagnostic. The `plan` template shows 2 example tasks ([docs/types.md](docs/types.md#tasks-of-a-plan)).`PARSER_VERSION` is 13, so the cache is rebuilt once.
- The parser keeps the list items of the sections `Delta`, `Interfaces`, `Decisions`, `Concerns`, `Files` and `Follow-ups`, with the byte range of each item, and the byte range of the sentence of each requirement.The cache is rebuilt once (see the entry above for the parser version). The intent fields `class` (`routine` or `higher-risk`; no value is `routine`; another value, such as `small-fix`, gives SD302), `areas` and `from`. The templates `intent`, `prd` and `rfc` have `class: routine` ([docs/types.md](docs/types.md#intent-spec-and-plan)).
- The embedded templates `prd`, `rfc`, `madr`, `post-mortem` and `runbook`, and `spec-dna new <type> <id> --template <name>` ([docs/types.md](docs/types.md#templates), [docs/commands.md](docs/commands.md#new)). `madr` is the default template of a `decision`, and `post-mortem` is the default of an `incident`. A decision has the sections Considered Options, Status and Amendments, and an incident has the sections Timeline, Links, Follow-ups (a task list) and Eval.
- The table `[templates]` with the key `dir` (default `.spec-dna/templates`) holds the project templates. A file `<name>.md` there, with a `type` in its front matter, replaces the embedded template of that name. `new` stops with exit code 2 for an unknown template and for a template of another type. The folder is never part of a root, also for a root `.`, so it needs no `exclude` ([docs/config.md](docs/config.md#the-table-templates)).
- `spec-dna layer add <name>` writes a layer into `spec-dna.toml`, and `spec-dna layer list` shows each layer, its level, its edges, its limits and the number of documents that each rule put in it. `--moved-from` adds the new layer to the `moves-to` list of the named layers and keeps each comment of the file. `layer add` stops with exit code 2 and does not change the file for each of these: a cycle, a name that exists, a type of another layer, an unknown layer in an edge and a bad value, and a `files` entry that is not a relative path inside the root or that another layer has. It keeps the line endings of the file (CRLF stays CRLF). It first writes the default graph if the file has no `[layers]` table ([docs/commands.md](docs/commands.md#layer)). `layer list --json` gives the rule that put each document in its layer. A document with no type is in no layer.
- The front matter field `was-type` (`note` or `change`). A value of another kind gives SD302. SD202 reads it.
- The types `product`, `system`, `terms`, `roadmap`, `questions`, `spec` and `plan`, with their statuses, required sections, budgets and templates ([docs/types.md](docs/types.md)).
- `spec-dna new intent <slug>`, `new spec <slug>` and `new plan <slug>` write the files of a change into `intent/<slug>/`. `new` names the new type for a type of 0.5.
- The configuration keys `[layers.<name>]`, `areas` and `[groups]` ([docs/config.md](docs/config.md)). A layer has the keys `types`, `dir`, `files`, `paths`, `moves-to`, `linked-from`, `keep`, `context`, `max-docs`, `max-chars`, `max-started` and `stale-days`. These are errors (exit code 3): a cycle in `moves-to`, a type in 2 layers, an absent layer in an edge, `files` with `dir`, 2 layers with the same `dir`, a `files` entry that is not a relative path inside the root, a file that 2 layers have, an unknown key, a bad value, and a group area that is not in `areas`. The place rule, the order of `context`, SD202, `new` and `layer` read the layers.
- `init` question 3 proposes the areas of `requirements/`: the top folders of the existing specs, or the folders of the apps. It proposes at most 8, and only 2 or more. The answers key is `areas`, and `none` gives one folder. `init` never proposes a group.
- `init` says in a note that a flat folder of proposals (`rfcs`, `proposals`, `changes`, `tasks`, `plans`, `enhancements`, or the prefixes `RFC` and `KEP`) is a `research` root. To start an open proposal, run `spec-dna new intent <slug>`. `init --json` has the field `notes`, and `default_type` is `null` for a mixed root with no `default-type`.
- The configuration key `language`, a BCP 47 language tag (default `en`, top level only). A primary subtag other than `en` turns off the checks that read English word lists ([docs/config.md](docs/config.md)).
- The configuration key `[lint] history-terms`, a list of unique phrases. It replaces the default phrases of the check for history in requirements. An absent key and an empty list are different.
- The table `[change]` with the keys `policy`, `gate`, `high-risk-layers`, `high-risk-paths` and `max-fanout` ([docs/config.md](docs/config.md)). `policy` (`three-class`, `two-class` or `none`) is required in the table. `gate` (`block`, `warn` or `off`) sets the severity of SD910. `high-risk-layers` names layers of the graph (default `interfaces` and `rules`, without the names that the graph does not have). `high-risk-paths` has globs from the repository root (default none). `max-fanout` is an integer (default 8). An unknown key of `[change]` is an error (exit code 3). Without the table the policy is `none`. With `none`, the codes SD905 and SD910 to SD921 are off, and no rule in `spec-dna.toml` can turn them on.
- The rule groups `layers` (SD901 to SD904) and `changes` (SD905 and SD910 to SD922, without SD918) for `[rules]`. The word `rules` in `[layers.rules]` is a layer name, not the `[rules]` table.
- The codes SD905, SD910 to SD917, SD919 and SD920 of the change lifecycle, with their names, their profile severities and their fix texts. SD910 follows `[change] gate`, and a rule for SD910 in `[rules]` has priority over it.
- `check --json` has the field `change_base` when `[change] policy` is not `none`: the merge base of `--base` (or of `origin/HEAD`) with `HEAD`, or `null`. The text output prints a note on stderr if there is no base. The base reads only `refs/remotes/origin/HEAD`, never a local branch.
- The lifecycle checks that read one tree, with a `[change] policy` other than `none` ([docs/diagnostics.md](docs/diagnostics.md#checks-of-the-change-lifecycle)). SD914 reports an invalid delta line: a wrong form, an absent target, a layer that `moves-to` of the `intent` layer does not allow, a document of a type with no requirements, a hash that does not match, and an `Interfaces` item that names no interface document. It skips a line that the head already applies. `check` reports it as an error only for a spec that the diff touches. SD919 reports an open concern in an `approved` spec. SD920 reports a decision of `Decisions` that does not exist, in an `approved` spec. SD912 reports more `accepted` intents than `max-started` of the `intent` layer. SD905 reports a delta that touches more documents than `max-fanout`. A closed change gives none of them.
- The checks of the diff against the lifecycle base, with a `[change] policy` other than `none` ([docs/diagnostics.md](docs/diagnostics.md#checks-of-the-change-lifecycle)). SD910 reports an artifact that changes while the base tree has no approval of the artifact before it (`intent.md`, `spec.md`, `plan.md`, then code), and code under the `Files` of a plan that the base tree does not approve. An approval in the same pull request does not count. A change that a commit of the branch added and finished gives SD910 with `gate = "block"`, and it passes with `warn`. SD913 reports a path of the diff that no `Files` item of a change matches. SD917 reports a `routine` intent when a delta line targets a layer of `high-risk-layers`, when the diff changes a requirement of such a layer, or when a path matches `high-risk-paths`. SD905 also counts the requirement documents of a change with no delta line. SD912 is new when the head has more started changes than the lifecycle base. A diff with no change is a small fix, and only SD917 and SD921 apply to it. `check --staged` diffs the index.
- The checks of a finished change and of staleness, with a `[change] policy` other than `none` ([docs/diagnostics.md](docs/diagnostics.md#checks-of-the-change-lifecycle)). They apply when the diff deletes an intent folder, and they read the spec and the plan from the base tree, so a correction in the same pull request has no effect. A change with the base status `closed` gives none of them.
  - SD915 reports a delta line that the head does not apply (an `ADD` or `CHANGE` requirement with another text, a `REMOVE` key that is still there). For a change with no delta, it reports a diff that changes no `Files` path and no requirement hash. A pull request that sets `closed` and deletes the folder gives SD915, because the base status is `accepted`.
  - SD916 reports an `ADD` or `CHANGE` requirement with no `covers:` tag and no `verified-by`. For a change with no delta, it checks each requirement whose hash differs from the base.
  - SD920 also reports a decision of `Decisions` that is absent or is not `accepted`.
  - SD911 (`change-stale`) reports an `accepted` change that has had no commit for more than `stale-days` of the `intent` layer. It compares the committer dates of the merge base and of the newest commit that touched the folder or a `Files` path. It never reads the clock or the time zone. It is a state code: `check --all` shows it, and it is never new. It needs a real merge base. A shallow clone can hide a stale change, and it never reports a false one.
  - `check` reads the `covers:` tags also when only SD916 needs them.
- `spec-dna finish <slug> [--dry-run] [--base <rev>]` ends a change ([docs/commands.md](docs/commands.md#finish)). It runs 6 steps in memory and stops at the first one that fails, then it writes nothing:
  - Step 0: a document outside the folder links into it (a Markdown link, `[[id]]`, `depends-on`, `supersedes` or `from`). The locked text of an accepted decision, a stub and a generated index do not count. This stop also applies in the profile `lite`.
  - Step 1: the statuses (`accepted` intent, `approved` plan, and `approved` spec for a `higher-risk` change). An intent with the status `closed` runs steps 0 and 1, and then `finish` deletes the folder.
  - Steps 2 and 3: SD919, SD922, SD920 and SD914, whatever the profile, the rules and the policy. SD914 also covers a delta that differs from the approved delta of the base tree, and an interface of `Interfaces` that the diff does not change.
  - Steps 4 and 5: it applies `ADD`, `CHANGE` and `REMOVE` by byte range, with CRLF, a BOM, markers and nested lists kept, and then it checks the evidence (SD916). The result may add no error, except SD305 for a new document.
  - The write is one pass: the files, then the deletion of the tracked files of the folder, then `git add -A` of each path. A generated index that lists a deleted file is written again. `finish` makes no commit.
  - `--dry-run` prints the diff of each file and `remove <path>`. `--json` has `slug`, `ok`, `dry_run`, `step`, `diagnostics` and `files`. Exit codes: 1 a stop, 2 usage, 3 pin, base or configuration, 4 a write error, 5 no folder.
  - `finish` is the one command that deletes documents. It deletes only the files of the intent folder.
- A `covers:` tag that names the requirement of an `ADD` line of an `accepted` change gives no SD105.
- The field `from` of an intent is a reference: it gives SD101 if it names no document, and SD901 follows the layers.
- The check SD901 (`link-direction`, warning). A document of a layer X references a document of a layer Y, X is more stable than Y, and Y does not list X in `linked-from`. A reference is a Markdown link, `[[id]]`, `depends-on` or `supersedes`. A link of a locked decision that gives SD611 does not give SD901.
- The check SD902 (`layer-limit`, warning). A layer has more documents than `max-docs`, or more body characters than `max-chars`. The diagnostic is on `spec-dna.toml`. The default graph has limits. Thus a project with no `[layers]` table gets SD902 when a layer is over its limit, for example with more than 30 documents of the type `rule`.
- The check SD903 (`history-in-requirements`, warning). A requirement of a `rule`, `requirements` or `interface` document has a date, `round <n>`, `iteration <n>`, or a history phrase ("previously", "formerly", "no longer", "used to", "was changed", "before this change", "after this change"). `[lint] history-terms` replaces the phrases. `language` does not turn it off.
- The check SD904 (`term-forbidden`, warning). A requirement uses a forbidden synonym of the table of a `terms` document.
- The check SD921 (`truth-changed-without-change`, warning in `standard` and error in `strict`). A requirement of a `forever` layer changed, and no file of the `intent` folder did. It runs on the lifecycle base (the merge base of `--base`, or of `origin/HEAD`, with `HEAD`), and only with a `[change] policy` other than `none`. A move that keeps the text of each requirement gives nothing. A document that the `Files` of a plan lists gives nothing, if the change is in the diff, or if the base tree has the change with an `accepted` intent and an `approved` plan.
- The profiles `lite`, `standard` and `strict` set SD901 to SD904 to off, warning and warning, and SD921 to off, warning and error. The rule groups are `layers` and `changes`. A 0.5 project gets the new warnings SD901 to SD904 at the first `check` after the update. The gate against the base reports only the new warnings.
- The parser stores the rows of each table with the header cells `Term` and `Forbidden synonyms`. spec-dna rebuilds the index once after the update, because the parsed data changed.

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
- The harness files, with their removals, are in the `init --dry-run` diffs and in the `diff` of each entry of `--json` `files`, as the other files are. The preview starts with 10 answers: question 9 (the level) and question 10 (the models). Question 10 says `writes nothing` if the level is `none`.
- `check --staged` reads the harness files of the git index. An edit that is not staged does not count. The upgrade path of `init` changes the hook line of 0.4.1 and the harness files in one run.
- `spec-dna hook context-meter`: tells an agent its context size once at each level.
- `spec-dna hook fanout-cap`: asks the person before a workflow whose agent count can exceed the cap.
- SD504 `harness-missing` and SD505 `harness-settings-invalid`. SD501 and SD502 also apply to the harness files.

### Upgrade notes

- A configuration of 0.4 or older has no `[harness]` table, so `init` adds no harness on upgrade. Run `spec-dna init --reconfigure` to add it.
- The hooks need spec-dna 0.5.0 or later on the `PATH` of each teammate. Without it, the hooks do nothing.
- Each new `init`, also with `--non-interactive`, writes the `lean` harness: Opus at `xhigh` for the main session, the Fable advisor, the agent files, a deny rule and 2 hooks. For no harness, answer `none` to question 9, or use an answers file with `harness = "none"`.

## 0.4.2 - 2026-11-14

Fixes from the use of 0.4.1 in a second real project.

### Added

- `check --staged` checks the git index, the tree that the next commit records. Untracked files and edits that are not staged do not count. An unmerged path gives exit code 4, and a missing `spec-dna.toml` in the index gives exit code 3. The hook that `init` writes (pre-commit, husky, lefthook) now runs `spec-dna check --staged`. Before this change, a commit could pass while the committed tree failed with SD101. ADR 0001 has an amendment.
- SD611 `decision-links-change` (warning): an accepted decision links a change document in its frozen text or in `depends-on`. A link in an amendment, history or live section is allowed. [docs/types.md](docs/types.md#links-between-types) gives the link direction of each type.
- `init` and `init --dry-run` print the 8 answers before the preview. Each line says whether the answer is the default and what it writes (`writes nothing` for an answer that writes no key or file). With `--json`, `answers` has the same data.
- The preview of `init`, and `init --dry-run` for a project that has a `spec-dna.toml`, show the diff of each file: `AGENTS.md`, `CLAUDE.md`, the skill and the hook. With `--json`, each entry of `files` has a `diff`.

### Changed

- `cargo install --locked --git <url> --tag v0.4.2` works without a package name. The benchmark packages and the brownfield fixture have no binary now (they use examples and a library), so the repository has one package with a binary. The command for v0.4.1 and earlier still needs `spec-dna` at the end. The README and the quickstart give both forms.
- The skill that `init` writes lists the `types` of the project, not all 10 types. `check` compares the skill with the types of the project. After you change `types`, run `spec-dna init` to update the skill.
- `check --lint` runs the prose lint (SD801 to SD805) in every profile, even when `lint` is off. The findings are warnings, so they never fail the gate. Without `--lint`, nothing changes.
- A requirement key that is a template placeholder (`{{key}}`) gives SD306 only. It no longer gives SD203 too.
- SD804 does not report "many" after "how" ("how many").
- SD304 for `proposed` on a spec, a contract or an invariant names `draft`. `draft` is the proposal state of these types, and `proposed` is for a decision and a change. The namespace `reference/` stays singular. [docs/types.md](docs/types.md#statuses-and-namespaces) gives the reason and the namespace of each type.
- `spec-dna new` says "replace each {{...}} placeholder". The templates use that form.
- A command that writes to a closed pipe (`spec-dna check --json | head -1`) ends with its own exit code, with no panic. A non-interactive `init` with a closed stdout now writes its files.

### Upgrade notes

- A project on 0.4.1 must change the pin to 0.4.2 and its `[spec-dna-sha256]` table (see [docs/quickstart.md](docs/quickstart.md#upgrade)), install the new binary and run `spec-dna init`. That run updates the skill. It also updates the hook that 0.4.1 wrote (`entry: spec-dna check`, or the line `spec-dna check` of `.husky/pre-commit`) to `--staged`. `init` changes only that exact line. For lefthook, change `run: spec-dna check` by hand.
- SD502 reports the old skill until `init` runs.

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
