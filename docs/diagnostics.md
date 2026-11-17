# Diagnostics

This page gives each diagnostic code of `check`, with its name, its severity and its meaning. A line of `check` output reads `<path>:<line>: <severity> <code> <name>: <message>`. Use the page when `check` reports a code that you do not know.

## Read the output

```text
docs/requirements/valves/closing.md:3: error SD103 evidence-missing: `verified-by` path `tests/valves.rs` is not in the git tree
SD103 fix: Fix the `verified-by` path. The path must exist in the git tree.
```

Every diagnostic has a stable code. A code keeps its meaning. spec-dna does not use a retired code again. An error fails `check` when the error is new against the base. A warning never fails `check`.

The text output has this order: the errors, the drift warnings, the other warnings, and one count line for each prose lint code. Then it gives the fix of each listed code once (`SD102 fix: ...`). Thus an agent needs no lookup. With `--all`, the output also has one info line for each dead link in locked text, after the diagnostics ([locked references](#locked-references)). An info line is not a diagnostic.

`check --json` prints `code`, `name`, `severity`, `new`, `doc`, `path`, `line`, `target`, `message` and `fix` for each diagnostic. It also prints `locked_links` (see [locked references](#locked-references)) and `documents`: the id, path, type, status, body characters and number of recognized requirements of each document.

The tables below give the severity of each code in the `standard` profile. The profile and the rules of the root can change the severity or disable the code ([profiles.md](profiles.md)).

## The gate

The gate compares each diagnostic with the base by its fingerprint (code, document id, target). For a moved document, the comparison uses its old id, which the document keeps as an alias.

The codes SD401 to SD405, SD202, SD605, SD606, SD607, SD702, SD703 and SD921 compare the base tree with the head tree. Thus these diagnostics are always new. SD704 is new only if the root does not enforce budgets and the document is new or grew.

Without `--base`, the change codes that need a base use the merge base of `origin/HEAD` with `HEAD`, or, if there is no `origin/HEAD`, of the branch of `[change] base` with `HEAD` ([the checks of the change lifecycle](#checks-of-the-change-lifecycle)). If git cannot name this base, `check` prints a note and runs only the checks of one tree.

## Code ranges

| Range | Subject | Section |
| --- | --- | --- |
| SD101 to SD106 | References, links and evidence | [References and evidence](#references-and-evidence-sd101-to-sd106) |
| SD201 to SD204 | Document ids and requirement keys | [Ids and keys](#ids-and-keys-sd201-to-sd204) |
| SD301 to SD306 | Front matter, types, statuses and sections | [Front matter, types and sections](#front-matter-types-and-sections-sd301-to-sd306) |
| SD401 to SD405 | Status changes, supersede links and locked text | [Status and locked text](#status-and-locked-text-sd401-to-sd405) |
| SD501 to SD506 | Adapters, harness files, configuration and evals | [Adapters, harness, configuration and evals](#adapters-harness-configuration-and-evals-sd501-to-sd506) |
| SD601 to SD611 | Links, drift, evidence and quality | [Links, drift and quality](#links-drift-and-quality-sd601-to-sd611) |
| SD701 to SD704 | Budgets | [Budgets](#budgets-sd701-to-sd704) |
| SD801 to SD807 | Requirement text | [Requirement text](#requirement-text-sd801-to-sd807) |
| SD901 to SD904 | Layers | [Layers](#layers-sd901-to-sd904) |
| SD905, SD910 to SD922 | Changes | [Changes](#changes-sd905-and-sd910-to-sd922) |

## References and evidence (SD101 to SD106)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes`, `depends-on` or `from` value names no document, section or requirement. An accepted decision gives no SD101 for a reference in its locked text ([the checks of the change lifecycle](#checks-of-the-change-lifecycle)). |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree. An accepted decision gives no SD102 for a link in its locked text. |
| SD103 | evidence-missing | error | A `verified-by` path (in the front matter, or in the inline `<!-- verified-by: ... -->` marker of a requirement) is not in the git tree. |
| SD104 | ambiguous-reference | error | A reference names an alias that more than one document has. |
| SD105 | covers-unresolved | error | A `covers:` tag in a test file names no requirement. The document, the key or the requirement does not exist, or the id names a section. A tag that names the requirement of an `ADD` line of an `accepted` change is not reported: `finish` writes the requirement later. |
| SD106 | convention-unresolved | error | A pointer of `conventions` in spec-dna.toml names a document or a Markdown file that is not in the git tree. Or it names a section that the document or file does not have. |

## Ids and keys (SD201 to SD204)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD201 | duplicate-id | error | More than one document has the same id or alias. |
| SD202 | id-removed | error | The head tree does not have an id of the base tree, and no document keeps it as an alias. The layer of the base document decides: a layer with `keep = "until-finished"` (the `intent` layer by default) allows the deletion. In a `forever` layer, only the field `was-type: note` or `was-type: change` allows it. The status gives no exemption. |
| SD203 | key-format | error | A requirement key in a sentence with a BCP 14 keyword is not a slug and not a legacy key. A key that is a template placeholder (`{{key}}`) gives SD306 only. |
| SD204 | key-duplicate | error | A requirement key occurs 2 times in one document. |

## Front matter, types and sections (SD301 to SD306)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD301 | front-matter-syntax | error | The front matter is not in the supported YAML subset. |
| SD302 | front-matter-field | error | A known field has a value of the wrong kind. The `class` of an `intent` must be `routine` or `higher-risk`: another value, such as `small-fix`, gives SD302 (a small fix has no intent). A `was-type` value other than `note` or `change` also gives SD302. |
| SD303 | type-invalid | error | The `type` is unknown (the message names the new type of a type of 0.5), it does not match a single-type root, or it is not in the `types` list of the project. The document has no type: its front matter has none, its place gives none, and the root has no `default-type`. The type is valid only in the `intent` layer (`intent`, `spec` or `plan`), and the document is at another place. A type that no layer has also gives SD303, with the hint to run `spec-dna layer add`. In the first 3 cases (an unknown type, no type, and an intent-only type at a wrong place), the document has no valid type. It gets no SD304 for its status and no SD305 for the sections of a type, and no budget applies to it. |
| SD304 | status-invalid | error | After the `status-map` of the root applies, the status is absent, or the type does not allow it. The types `requirements`, `interface`, `rule`, `procedure` and `intent` use `draft` for a proposal. `proposed` is for a decision. On an `intent`, the message names `closed` for `rejected` and `research` for `done`. |
| SD305 | section-missing | error | A document has no section with a heading from one group of `required-sections` of the root for its type. The defaults are: decision `Decision Outcome` or `Decision`; rule `Rules`; incident `Summary` and `Root cause`; intent `Intent`, `Problem` or `Summary`; spec `Delta`; plan `Files`. |
| SD306 | placeholder-left | error | A template placeholder (`{{text}}`, from `spec-dna new`) is still in the document. |

## Status and locked text (SD401 to SD405)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD401 | status-transition | error | A status moved back, or it left a final state. |
| SD402 | superseded-without-successor | error | A superseded document has no active (or accepted) successor that names it in `supersedes`. |
| SD403 | supersede-cycle | error | The `supersedes` chain returns to its start. |
| SD404 | depends-on-inactive | error | `depends-on` names a superseded or retired document. |
| SD405 | decision-locked | error | Text outside the amendment sections (`amendment-sections`, default `## Amendments`), the history sections and the live sections (`live-sections`, default `Status`) changed after the decision got the status `accepted`. |

## Adapters, harness, configuration and evals (SD501 to SD506)

SD501, SD502, SD504 and SD505 check the files that `spec-dna init` writes: the adapters, and the harness files of the Claude Code adapter ([harness.md](harness.md)). SD506 checks the eval files ([evals.md](evals.md)). SD506 is new in 0.6.0.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD501 | adapter-modified | error | A generated adapter (the line in `AGENTS.md` or `CLAUDE.md`, or the skill block) has a manual edit. A generated harness file of the Claude Code adapter has a manual edit, or text after its marker. These files are an agent file, a stage skill, `REVIEW.md`, `model-team.md` and the model-team line of `CLAUDE.md`. A file with a harness marker that the configuration does not generate also gives SD501 when it has a manual edit: delete the file, or delete its marker line to keep it as a file of the project. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill `.claude/skills/spec-dna/SKILL.md`. |
| SD502 | adapter-outdated | error | A generated adapter differs from the adapter that this version generates. The multi-line block of an earlier version also gives SD502: `spec-dna init` replaces it with one line. A generated harness file or line of the Claude Code adapter differs from the text for the configuration. A `.claude/settings.json` entry of the harness is absent or different (the message names its JSON path). A file with a harness marker is no longer in the configuration, and it has no edit (`spec-dna init --reconfigure` removes it). The scan is the one of SD501. An old `checker.md` or `spec-writer.md` is such a file. `CLAUDE.md` has a second model-team line (delete it). The skill lists the types of the layers, so `spec-dna layer add` also gives SD502 until `spec-dna init` runs again. |
| SD503 | config-not-alone | error | The change edits `spec-dna.toml` and other files. A configuration change must be a change of its own. In a tree binary, a change of the pin line alone does not count. [Files that SD503 counts](#files-that-sd503-counts) lists the paths that do not count. |
| SD504 | harness-missing | error | `[harness]` has a level that is not `none`, and a file that the harness of the Claude Code adapter needs is absent: an agent file, a stage skill (`.claude/skills/spec-dna-<skill>/SKILL.md`), `REVIEW.md`, `.claude/agents/model-team.md`, or the model-team line of `CLAUDE.md`. A file at the path of one of these files, with no harness marker, also gives SD504: spec-dna does not replace a file that it did not generate. The message gives the ways to fix it: move the file, set the role of an agent file to `none`, remove the skill from `skills`, or set `review-md = false`. |
| SD505 | harness-settings-invalid | error | `.claude/settings.json` of the harness of the Claude Code adapter is not valid JSON, or its root is not an object, or a key that spec-dna writes into has a value of the wrong type (the message names it). `check` cannot compare the harness entries. |
| SD506 | evals-invalid | error | The table `[evals]` is in the configuration, and a `*.json` file directly in `[evals] dir` is not a valid eval, or the file `<dir>/check.sh` is absent. The message names the broken rule, and the target is its key (for example `task`, `fixture` or `checks`). `spec-dna eval run` uses the same rules. SD506 reads no text of `check.sh` or of a fixture, and no file in a subfolder. It is an error in each profile. It does not depend on `[change] policy` ([the checks of the evals](#checks-of-the-evals)). |

### Files that SD503 counts

SD503 reads paths only, with one exception: in a tree binary, it reads the text of `spec-dna.toml` (see below). It compares the working tree with the base (with `--base`, tracked files; without it, also each untracked file that git does not ignore, except with `--staged`). It reads the content of no other file, and its lists do not depend on the harness level. A manual edit of one of the files below does not count.

`check` does not examine a new `spec-dna.toml` (an adoption). SD503 does not apply if the base pins a version before 0.6.0. Thus one pull request can hold the configuration and the documents of an upgrade. A base from 0.6.0 keeps SD503, also for a change of the pin, except in a tree binary.

A tree binary is a binary that the checkout of spec-dna itself builds. The binary is a tree binary when both facts are true:

- The file `Cargo.toml` at the top level of the git repository that holds `spec-dna.toml` has `name = "spec-dna"` in `[package]`.
- Its `version` is equal to the version of the binary that runs.

A tree binary needs no key. It reads the head tree: the working tree, or the git index with `--staged`. In a tree binary, SD503 does not count a change of the line `spec-dna = "..."` of `spec-dna.toml`. For this, SD503 compares the text of `spec-dna.toml` in the base with its text in the head tree. If the 2 texts differ in the pin line only, `spec-dna.toml` has no change. A pin change with a document therefore gives no SD503: the document is the only counted file, and SD503 needs `spec-dna.toml` and another file. A change of any other line counts, as before. Thus a pin change with another line of `spec-dna.toml` and a document gives SD503. A release binary in another repository never has this mode. The exact pin rule stays for each rule: a pin that differs from the version of the binary stops `check`, `finish` and `eval run` with exit code 3 ([ADR 0013](adr/0013-self-hosting.md)).

These paths do not count. Each one is a file that `init` writes with the configuration:

- `spec-dna.toml`
- The adapters: `AGENTS.md`, `CLAUDE.md` and `.claude/skills/spec-dna/SKILL.md`.
- The files of the harness: `.claude/settings.json`, `.claude/agents/model-team.md` and `REVIEW.md`.
- The agent files: `.claude/agents/fallback-advisor.md`, `.claude/agents/verifier.md`, `.claude/agents/Explore.md`, `.claude/agents/implementer.md` and `.claude/agents/code-simplifier.md`. The agent files of 0.5 also do not count: `.claude/agents/checker.md` and `.claude/agents/spec-writer.md`.
- The stage skills: `.claude/skills/spec-dna-intent/SKILL.md`, `.claude/skills/spec-dna-spec/SKILL.md`, `.claude/skills/spec-dna-plan/SKILL.md` and `.claude/skills/spec-dna-review/SKILL.md`.
- The hook files: `.pre-commit-config.yaml` and `.husky/pre-commit`.
- The starter files of the evals, if the head configuration has `[evals]`: `<dir>/check.sh`, `<dir>/example.json`, `<dir>/fixtures/example/notes.txt` and `.github/workflows/agent-evals.yml`. `<dir>` is `[evals] dir`. Without `[evals]`, these files count.
- Each path whose file name starts with `spec-dna`, such as `.github/workflows/spec-dna.yml`. This rule has no folder limit, so a document `docs/spec-dna-notes.md` does not count.

Each other path that differs from the base counts. Examples are documents, code, lint configurations such as `.markdownlint-cli2.yaml`, `CODEOWNERS`, other workflows and a deleted file. A move gives 2 paths, the old path and the new path, because the diff has no rename detection.

## Links, drift and quality (SD601 to SD611)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document. |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target. |
| SD603 | cross-repo-unpinned | warning | spec-dna cannot check a `<repo>:<id>` reference without a pinned index. |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document. |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word. |
| SD606 | cited-path-removed | warning | A document cites a file or a directory that the change removed or renamed. The message names the new path of a file with the same content. A link gives SD102 instead. |
| SD607 | cited-symbol-removed | warning | A document cites an identifier in an inline code span. The change removed the identifier from a file, and no other file that is not Markdown has it. If the file has exactly one new identifier, the message names it as a possible new name. |
| SD608 | requirement-unverified | warning | A requirement of an active `requirements`, `interface` or `rule` document has no evidence: no `verified-by` and no `covers:` tag. |
| SD609 | incident-unlinked | warning | An incident links no requirement, rule, decision or procedure. |
| SD610 | index-stale | warning | A generated index (`spec-dna index`) does not match its entries. A change added, removed or renamed an entry document, or changed its title or status, or edited the rows manually. |
| SD611 | decision-links-intent | warning | An accepted decision links an `intent` document in its locked text, or in `depends-on`. You can delete a finished intent (SD202 allows it), but you cannot remove the link from the locked text (SD405). SD611 does not report a link in an amendment, history or live section. SD901 reports it, so name the intent as plain text there. After `spec-dna finish` deletes the intent, the link is dead, and `check --all` names it in an [info line](#locked-references). [types.md](types.md#links-between-types) gives the link directions. |

## Budgets (SD701 to SD704)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD701 | budget-soft | warning | A document is over its soft budget. |
| SD702 | budget-hard | error | A document is over its hard budget. Against the base, it is new, it was under the hard budget, or it grew by more than `[budgets] growth` characters (default 4,000). |
| SD703 | budget-grew | warning | A document over its soft budget grew against the base. It is under its hard budget, or over it by at most `[budgets] growth` characters. |
| SD704 | budget-hard-kept | warning | A document is over its hard budget, but it was over it in the base and did not grow. Or a split moved it out of a document over its hard budget (its section aliases name that document). In a root with `enforce-budgets = false`, every document over its hard budget gives SD704. |

## Requirement text (SD801 to SD807)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD801 | requirement-keyword | warning | A requirement has no uppercase BCP 14 keyword. |
| SD802 | ears-pattern | warning | An `If` requirement has no `then`. |
| SD803 | sentence-length | warning | A requirement sentence is longer than `lint.max-words`. |
| SD804 | vague-term | warning | A requirement uses a term from the vague-term list. The word "many" after "how" ("how many") is a question about a number, so it is not vague. |
| SD805 | passive-voice | warning | A requirement uses a passive construction (heuristic). |
| SD806 | requirements-unrecognized | warning | A `requirements`, `interface` or `rule` document has bare tokens that match a `legacy-keys` pattern, but no recognized requirement. Or spec-dna recognizes some requirements, but a list item or a table row starts with a `legacy-keys` token. This token is not the key of one of them. The legacy keys can be in a syntax that spec-dna does not read, such as a table. |
| SD807 | requirement-colon-missing | warning | In a `requirements`, `interface` or `rule` document, a list item starts with a bold slug key and has a BCP 14 keyword. No colon follows the key (`- **key** [1.1]: sentence`). Thus the item is not a requirement. |

The requirement lint (SD801 to SD805) is off in the `lite` and `standard` profiles. It is a warning in `strict`. The rule `lint` changes this.

The requirement lint (SD801 to SD805) and SD904 read English word lists. A `language` with a primary subtag other than `en` disables them, also with `check --lint`. SD903 does not depend on `language`.

The [research matrix](research/matrix.md), section "Controlled-language lint", describes the lint warnings SD803 to SD805 and their sources.

## Layers (SD901 to SD904)

These codes are new in 0.6.0. The rule group `layers` sets their severity ([profiles.md](profiles.md)). [Checks of the layers and of the requirement text](#checks-of-the-layers-and-of-the-requirement-text) gives their details.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD901 | link-direction | warning | A document of a layer X references a document of a layer Y. X is more stable than Y (X is reachable from Y by `moves-to` edges), and Y does not list X in `linked-from`. A reference is a Markdown link to a document, `[[id]]`, `[[id#key]]`, `depends-on`, `supersedes` or (in an intent) `from`. A document in no layer, a layer with no path to the other, an unresolved target, an image, a stub and a generated index give no diagnostic. SD901 gives one diagnostic for each document and target. A link of a locked decision that gives SD611 does not give SD901. |
| SD902 | layer-limit | warning | A layer has more documents than `max-docs`, or more body characters than `max-chars`. The documents of all roots count. A stub and a generated index do not count. The diagnostic is on `spec-dna.toml`, so the rules and the profile of a root do not apply. |
| SD903 | history-in-requirements | warning | A recognized requirement of a `rule`, `requirements` or `interface` document has a calendar date, `round <n>`, `iteration <n>`, or a phrase of `[lint] history-terms`. |
| SD904 | term-forbidden | warning | A recognized requirement of a `rule`, `requirements` or `interface` document uses a forbidden synonym of a `terms` document. The check is off when `language` is not English. |

## Changes (SD905 and SD910 to SD922)

These codes are new in 0.6.0. They apply only to a project with a `[change]` table whose `policy` is not `none`. SD918 also needs an `[evals]` table. With `policy = "none"`, SD905 and SD910 to SD922 are off, and no rule enables them. The rule group `changes` sets their severity ([profiles.md](profiles.md)). The change lifecycle is in [change.md](guides/change.md).

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD905 | fan-out | warning | One change touches more requirement documents than `[change] max-fanout`: the documents of the delta of its spec, or, for a change with no delta line, the requirement documents of its diff. Off in `lite`. |
| SD910 | gate-approval-missing | error | A `spec.md` or a `plan.md` that is not a `draft` changes while no approval of the artifact before it counts. Code under the `Files` of a plan that no approval covers counts as an artifact. With `[change] approval = "base"`, only the base tree gives an approval. With `"branch"`, an approval commit of an approver in the branch also counts. The message names one of 8 reasons. `[change] gate` sets the severity: `block` is an error, `warn` is a warning, `off` disables the code. |
| SD911 | change-stale | warning | A started change has had no commit for more than `stale-days` days (a key of the `intent` layer). It is a state code: `check --all` shows it, and it is never new. Off in `lite`. |
| SD912 | started-limit | warning | More changes than `max-started` of the `intent` layer have an `accepted` intent. The diagnostic is on `spec-dna.toml`. It is new when the head has more started changes than the lifecycle base. Off in `lite`, an error in `strict`. |
| SD913 | outside-plan | warning | The diff of a change touches a file that no item of `Files` matches. The intent folder and the documents that the delta names are always in scope. Off in `lite`. |
| SD914 | delta-invalid | error | A delta line has an invalid form, names an absent target, targets a layer that `moves-to` does not allow, or has a hash that does not match the requirement. SD914 also reports an `Interfaces` item that names an absent interface document, or an interface that the diff does not change. It is a warning for a spec that the diff does not touch, and with no lifecycle base. |
| SD915 | finish-incomplete | error | The diff deletes an intent folder whose base status is not `closed`, and the head does not have the result of its delta. For a change with no delta, the diff changes no `Files` path and no requirement hash. |
| SD916 | delta-evidence-missing | error | An `ADD` or `CHANGE` requirement has no `covers:` tag and no `verified-by` when `finish` runs, or when the diff deletes the intent folder. For a change with no delta, the same applies to each requirement whose hash differs from the base. |
| SD917 | class-too-low | warning | The `class` of an intent is lower than the minimum that the diff gives. Off in `lite`, an error in `strict`. |
| SD918 | incident-eval-missing | warning | The diff adds a document of type `incident`, and adds or changes no `*.json` file directly in `[evals] dir`. It needs `[evals]` and a `[change] policy` other than `none`. Off in `lite`. A move of an incident is not an add. |
| SD919 | concern-open | error | A spec with the status `approved` has an open concern (`- [ ]` in `Concerns`). |
| SD920 | decision-needed-unaccepted | error | A decision that `Decisions` of an `approved` spec names does not exist. Also, when `finish` runs or the diff deletes the intent folder, a decision that `Decisions` names is absent or is not `accepted`. |
| SD921 | truth-changed-without-change | warning | A requirement of a `rule`, `requirements` or `interface` document of a `forever` layer changed, was added or was removed, and no file under the folder of the `intent` layer changed. It runs on the lifecycle base, and only with a `[change] policy` other than `none`. A plan that covers the document gives an exemption. Off in `lite`, an error in `strict`. |
| SD922 | question-open | error | A spec with the status `approved` has an open question (`- [ ]` in `Open questions`). |

## Checks of the layers and of the requirement text

[profiles.md](profiles.md) gives the severity of each code in each profile. The rule group is `layers`.

- SD901. The graph of the layers gives the direction. A document can link a layer that its own layer moves into. It can also link a layer that lists its own layer in `linked-from`.
- A rule that links an intent gives SD901, because an intent moves into a rule, and a finished intent is deleted. An accepted decision that links an intent in its locked text gives SD611, not SD901. In an amendment section, a link to an intent gives SD901. Name the intent as plain text there.
- SD902. `max-docs` and `max-chars` of a layer set the limits. The default graph gives limits to `foundation`, `rules`, `intent` and `pending`. A project that has no `[layers]` table gets these limits.
- SD903. The check reads the text of each recognized requirement. It ignores link destinations, autolinks, bare URLs, code spans and `[[id]]` references. It matches whole words, in any case.
- SD903 dates. A date is `YYYY-MM-DD`, `YYYY/MM/DD`, `D.M.YYYY`, `D/M/YYYY`, `D Month YYYY`, `Month D, YYYY` or `Month YYYY`. The month has its English name, and the year is from 1900 to 2099. A duration such as "30 days" is not a date.
- SD903 phrases. The default phrases are "previously", "formerly", "no longer", "used to", "was changed", "before this change" and "after this change". `[lint] history-terms` replaces them. The key does not change the dates. `used to` after a form of "be" ("SHALL be used to") is not history.
- SD903 false matches. The match does not use the meaning of the sentence. "no longer than 2 seconds" matches "no longer", and "round 2 decimals" matches `round <n>`. To avoid such a match, rewrite the sentence, or list your own phrases in `history-terms`.
- SD904. The table of terms has the header cells `Term` and `Forbidden synonyms` (in any order). spec-dna splits a cell of synonyms at `,` and `;`. A synonym that matches a whole phrase of the requirement text gives SD904. The target of the diagnostic is `<key>:<synonym>`.

## Checks of the change lifecycle

The checks of the change codes read these trees:

- SD905 (the delta part), SD912 (the count), SD914, SD919, SD920 (the absent decision) and SD922 read one tree. They need no base and no history.
- These codes compare the head with the lifecycle base. They do not run if git cannot name this base:
  - SD905 (the part for a change with no delta);
  - SD910, SD913, SD915, SD916, SD917 and SD918;
  - SD920 (a decision that is not `accepted`);
  - SD921.
- SD911 reads the base and the history.

The lifecycle base is the merge base of `--base` (or, without it, of `origin/HEAD`, or of the branch of `[change] base`) with `HEAD`. The diff is the working tree (with `--staged`, the git index) against it. The diagnostics of the checks of the diff are always new. A check reads the artifacts of a change from the head tree. If the diff deletes the folder of a change, it reads them from the base tree. With `[change] approval = "base"`, it reads each approval from the base tree only, so an approval in the same pull request does not count. With `"branch"`, SD910 also reads the commits of `base..HEAD` (see SD910 below).

The changes of a diff are these:

- The folders of the `intent` layer that the diff touches.
- The changes that a commit of the branch added and the branch finished, and that neither tree has (a branch-born change). The check reads their newest artifacts from the history.
- The changes whose `Files` match a changed path.

A diff with no change is a small fix. SD910 and SD913 do not apply to it.

A change whose intent has the status `closed` does not proceed, so its spec gives none of these diagnostics.

- SD905:
  - The check counts the documents that the valid delta lines of a spec name. A document that the lines name more than once counts once. A document that does not exist yet (an `ADD` line) counts.
  - More than `[change] max-fanout` (default 8) documents give one diagnostic for the spec.
  - For a change with no delta line, the check counts the changed requirement documents of its diff. The diagnostic is on its intent.
  - With more than one change in the diff, a document belongs to the change whose `Files` match it.
- SD912:
  - The limit is `max-started` of the layer that has the type `intent` (3 in the default graph). A layer with no `max-started` has no limit.
  - The check counts the intents with the status `accepted`.
  - The diagnostic is on `spec-dna.toml`, so the rules of a root do not apply. Its target is `<layer>:max-started`.
  - The diagnostic is new if the head has more such intents than the lifecycle base has, and the head is over the limit.
- SD910:
  - The order of artifacts is `intent.md`, then `spec.md` (for `higher-risk`), then `plan.md`, then code. Code is a changed file that no layer owns.
  - The check applies these rows:
    - `intent.md` needs nothing.
    - `spec.md` needs an `accepted` intent.
    - `plan.md` needs an `approved` spec for a `higher-risk` change, and an `accepted` intent for another change.
    - A path that a `Files` item matches needs an `approved` plan. If more than one change matches the path, one approved plan is enough.
  - A `spec.md` or a `plan.md` with the status `draft` needs no approval of the artifact before it. Code under `Files` still needs an `approved` plan.
  - With `approval = "base"`, the rows read the base tree. With `approval = "branch"`, they read the base tree and the approval commits of the branch.
  - A branch-born change gives one diagnostic on its intent, because the base tree has no approval for it.
  - SD910 follows `[change] gate`, and a rule in `[rules]` has priority over the gate. A deleted artifact is not an edit.
  - The approval commit (`approval = "branch"`):
    - `check` walks the commits of `base..HEAD` from the oldest. The order of the commits counts, and their date does not.
    - An approval commit is not a merge commit, and the e-mail of its author is in `[change] approvers` (the compare ignores case). It changes one artifact file only, and it changes the `status` line only, to `accepted` for an intent and to `approved` for a spec or a plan.
    - The scope of a plan is its `Files` at its approval commit. A path that a later commit adds to `Files` has no approved plan.
    - Each status that the branch sets (`accepted` or `approved`) needs an approval commit that set it, also when no later work depends on it.
    - The check needs the full history. A shallow clone, and a merge commit that merges another branch, stop the rule, and SD910 then reports as with `approval = "base"`. A merge of the base into the branch, and the test merge of a code host, do not stop it.
    - A change that the branch adds and finishes gives SD910, also when approval commits exist. The record of `finish` needs the approvals in an earlier pull request.
    - An author e-mail is not a proof. The code host stays the control: `CODEOWNERS`, a protected default branch and required signed commits.
  - The message of SD910 names one reason of 8:
    1. The base tree has no approval.
    2. No approval commit comes before the work.
    3. The author of the approval commit is not an approver.
    4. The commit changes more than the `status` line.
    5. The clone is shallow.
    6. A merge commit stops the rule.
    7. The branch added and finished the change.
    8. The path is not in the `Files` of the plan at its approval commit.
  - `check --json` has the field `approvals`: the approval commits that cleared a gate ([commands.md](commands.md#check)).
- SD913:
  - The scope of a change has these parts:
    - its folder;
    - the documents that its delta and `Interfaces` name;
    - the paths of its `Files`;
    - the `Files:` paths of the tasks of its `Order`.
  - A `Files` item is a glob from the repository root. It is in a code span, or it is a plain path ([Files and paths](types.md#files-and-paths)). A final `/` matches the folder.
  - If the diff deletes the folder, the scope has the paths of the base tree. It also has the paths of the newest version of the plan and the spec on the branch before the deletion. So a plan that the pull request of `finish` adds or edits is in scope.
  - The scope reads committed branch text only. These 3 cases give the scope of the base tree alone, and SD913 can then give a diagnostic:
    - a partial clone without the blobs;
    - an edit that no commit holds;
    - an edit that a side branch made and merged after `finish`.
  - If a plan lists no path, and it has an item that is not a path, the message names the first such item.
  - For a change with no delta, a requirement document is in scope only if `Files` lists it.
  - SD913 gives one diagnostic for each path outside the scope of every change of the diff, also for a path that is deleted. A generated index is not reported.
  - A change with no `plan.md` has no `Files`, so each path outside its folder gives SD913.
- SD917:
  - The minimum class is `higher-risk` in these cases:
    - A delta line targets a document of a layer of `[change] high-risk-layers`.
    - For a change with no delta line, the diff changes, adds or removes a requirement of such a layer.
    - The diff, or `Files`, has a path that a glob of `[change] high-risk-paths` matches.
  - With one change in the diff, the whole diff belongs to it. With more, a path belongs to the change whose `Files` match it.
  - An intent with the class `routine` (also an absent `class`) gives one diagnostic on the intent.
  - A diff with no change has no intent. It gives SD917 for each path of `high-risk-paths`, and for each document that has a changed requirement in a layer of `high-risk-layers`.
- SD914:
  - The check reads each item of `Delta` in the order of the file, and it reports the first problem of an item:
    - The form is not `ADD <id>#<key>: <text>`, `CHANGE <id>#<key>@<hash>: <text>` or `REMOVE <id>#<key>@<hash>`. Or the item has a nested list, or a second line names the same key.
    - The document of the target does not exist (an `ADD` line can name a new document), or its id names several documents.
    - The document has no valid type, or its type holds no requirement. Only `requirements`, `interface` and `rule` hold requirements.
    - The layer of the document is not in the `moves-to` list of the `intent` layer. A new document gets its layer from its id, as `finish` writes it at `<id>.md`.
    - The key exists for `ADD`, or it does not exist for `CHANGE` and `REMOVE`.
    - The hash is not the hash that `show` prints. It is also not the hash without link destinations (a `mv` that rewrote a link keeps the line).
  - The check skips a line that the head already applies. For an `ADD` or `CHANGE` line, the requirement has the text of the line. For a `REMOVE` line, the base has the key and the head has not.
  - An item of `Interfaces` must be `CHANGE <interface id>: <text>`, and the id must name a document of the type `interface`.
  - The target of the diagnostic is `<id>#<key>`. For an `Interfaces` item it is the id, and for an invalid form it is the text of the item.
  - `check` reports SD914 as an error (in the profiles that make it an error) for a spec that the diff touches. It reports a warning for another spec. It also reports a warning for each spec when git cannot name the lifecycle base (the merge base with `--base`, with `origin/HEAD` or with the branch of `[change] base`).
  - The check that the diff changes an interface of `Interfaces` is part of SD914 too. The pull request must change the interface document, or a file that it cites.
  - `finish` runs this check. `check` runs it for each change that the diff deletes and whose base status is not `closed`. It reads the spec of the base tree, and the diff of the head against the lifecycle base.
- SD918. A change that adds an incident must add an eval for it.
  - The check lists the incidents that the diff adds. Such an incident is a document of type `incident` that has these 3 properties:
    - The diff adds its path.
    - Git does not detect it as the target of a rename.
    - The lifecycle base has neither its id nor its aliases (`spec-dna mv` writes the old id as an alias).
  - So a move of an incident, also to a path with another id, gives no SD918.
  - An untracked file in the working tree is not a rename target. So a move that is not staged and not committed gives SD918, unless the new file keeps the old id as an alias.
  - The diff must also add or change a `*.json` file directly in `[evals] dir`. A changed eval counts. A deleted eval does not count. A file in a subfolder (`results/`, a fixture) does not count.
  - The check gives one diagnostic for each added incident, on line 1 of the incident, with its id as the target. The diagnostic is always new.
  - `policy = "none"`, the profile `lite` and a configuration with no `[evals]` disable it.
- SD919. Each open concern (a task list item with an empty box) of an `approved` spec gives one diagnostic. A resolved concern, an item with no box, and a spec with another status give none.
- SD922. Each open question (a task list item with an empty box in `Open questions`) of an `approved` spec gives one diagnostic. A resolved question (`- [x]`), an item with no box, and a spec with another status give none. `Concerns` and `Open questions` are 2 sections: an open concern gives SD919, and an open question gives SD922.
- SD920. Each `[[id]]` of `Decisions` in an `approved` spec must name a document. A fragment and a label are not part of the id. If the diff deletes the folder of the change, or `finish` runs, each such decision must also have the status `accepted`. The check reads the spec from the base tree, and the decision from the head tree.
- SD915:
  - The check applies to a change that the diff deletes: the base tree has its folder, and the head tree has not.
  - A change whose base status is `closed` gives no SD915, SD916 and SD920, because it is dropped, not finished.
  - The check reads the delta and `Files` from the base tree. So an edit of the spec or the plan in the same pull request has no effect.
  - A change with a delta gives one diagnostic for each line that the head does not apply. An `ADD` or `CHANGE` line needs the requirement with the text of the line. A `REMOVE` line needs the key to be absent. A `REMOVE` line for a key that neither tree has gives no SD915.
  - A change with no delta line gives one diagnostic on its intent if the diff changes none of these:
    - a requirement hash;
    - a path that the plan matches (outside the folder).
  - The paths of the plan are the section `Files` and the `Files:` paths of its tasks.
  - The plan of the base tree can list no path, and have an item of `Files` that is not a path. Then the message names the first such item.
  - A pull request that sets `closed` and deletes the folder gives SD915, because the base status is not `closed`. Set `closed` in an earlier pull request, and merge it.
- SD916:
  - The check applies to the same deleted change, and `finish` calls the same code.
  - With a delta, it checks the requirement of each `ADD` and `CHANGE` line that the head applies. A line that the head does not apply gives SD915.
  - With no delta, it checks each requirement that the diff changes or adds, in a document that `Files` lists. With one change in the diff, it checks a document of any path.
  - A requirement has evidence in one of 3 cases:
    - A `covers:` tag of a test file names it.
    - It has a `verified-by` marker.
    - Its document has `verified-by` in the front matter.
  - The diagnostic is on the requirement, and its target is `<id>#<key>`.
- SD911:
  - A started change is a change with an `accepted` intent.
  - The `stale-days` key of the `intent` layer sets the limit (14 in the default graph). A layer with no `stale-days` has no check.
  - The check compares 2 committer dates:
    - the date of the lifecycle base;
    - the date of the newest commit that touched the folder of the change or a `Files` path of its plan.
  - One `git log` pass reads all the dates. The result never reads the clock or the time zone.
  - A change with no commit (an uncommitted folder) is not stale.
  - The check needs a real merge base. It is silent when git cannot name one, and the note about the base appears.
  - SD911 is a state code: it reports the state of the tree, so it is never new. `check` shows it only with `--all`, and the command `status` shows it.
  - A shallow clone has fewer commits, and its oldest commit shows every file as added. Thus a shallow clone can hide a stale change. It never reports a change that is not stale in the full repository.
- SD921:
  - The check compares each requirement of the base with the requirement of the head that has the same key. It applies the aliases of `mv`. A requirement has changed if the hash of its text, without link destinations, is different.
  - A key that has no match on the other side was added or removed. A move or a split with `spec-dna mv` keeps the keys through aliases, so it gives nothing. A split by hand with no alias gives 2 diagnostics (an added key and a removed key).
  - SD921 gives one diagnostic for each document.
  - It compares with the lifecycle base (the merge base of `--base`, of `origin/HEAD` or of the branch of `[change] base`, with `HEAD`). It does not run when git cannot name that base.
  - A document that the `Files` of a plan lists gives nothing in 2 cases:
    - The change is in the diff.
    - The base tree has the change with an `accepted` intent and an `approved` plan.
  - The diagnostic is also off in the `lite` profile and with `[change] policy = "none"` (no rule enables it).

### Checks of the evals

SD506 needs the table `[evals]`, with any `[change] policy`. It reads the files of `[evals] dir`:

- Each `*.json` file directly in the folder goes through the validator of `spec-dna eval run`. The first broken rule gives one diagnostic for the file, on line 1. The diagnostic has the path of the file as its `doc`. The key of the rule is its target (`file` for a rule of the whole file). So 2 files that break the same rule give 2 findings.
- The file `<dir>/check.sh` must exist. If it is absent, one diagnostic is on `<dir>/check.sh` with the target `check.sh`.
- The check reads the working tree, with the untracked files that git does not ignore. With `--base`, it reads tracked files only, and with `--staged`, the git index.
- The base findings use the `[evals]` table of the base, so only a new finding fails the gate. A base with no table has no findings.

### Stops of `finish`

`spec-dna finish <slug>` ([commands.md](commands.md#finish)) stops at the first step that fails, and then it writes nothing. A stop does not follow the profile, a `[rules]` entry or the policy `none`: each diagnostic of a stop is an error. The steps and their reasons are these:

| Step | Stop |
| --- | --- |
| 0 | A document outside the folder links into it: a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` or `from`. `finish` reports each reference as SD101 (an id) or SD102 (a link). A stub, a generated index and the locked text of an accepted decision are not sources. The folder is not tracked by git, or it differs from `HEAD` (an edit that is not committed, or an untracked file): this stop has a reason and no diagnostic |
| 1 | The intent is not `accepted`, the plan is not `approved`, or the spec is not `approved` (for a `higher-risk` change, and for a spec that has a delta line). A `closed` intent passes: `finish` then deletes the folder and does nothing else |
| 2 | SD919 (an open concern), SD922 (an open question) or SD920 (a needed decision that is absent or not `accepted`) |
| 3 | SD914: a delta line is not valid, its target is absent or in a layer that `moves-to` does not allow, or its hash is not the hash of the requirement. Also SD914: the delta differs from the delta that the base tree has (an approved delta must not change). Also SD914: an interface of `Interfaces` that the diff does not change |
| 4 | The result would not give the result of a delta line (`check` would report SD915), or two lines edit the same lines of a file |
| 5 | SD916 (a requirement that the change adds or changes has no evidence), or the result adds an error. The only exception is SD305 for a new document: the author adds the sections that the root requires |

The base of step 3 is the merge base of `--base` (or of `origin/HEAD`, or of the branch of `[change] base`) with `HEAD`, for each policy. An `accepted` change needs it, and without it `finish` exits with code 3. A change that the base tree does not have was added in the same branch: its delta has no approved text to compare.

### Locked references

A reference in the locked text of an accepted decision gives no SD101 and no SD102. This text cannot change (SD405). A reference to a finished intent must stay in it (SD611 warned when the author wrote it). The locked text is each part of the body outside the amendment, history and live sections.

`check --all` prints one info line for each such reference whose target no longer exists, after the diagnostics: ``<path>:<line>: info: the locked text links `<target>`, which no longer exists; add an amendment that points to <pointer>``. The pointer is the change (``the change `<slug>` (`spec-dna log` names its commit)``) if the target was in the folder of an intent. Otherwise it is "the new place of the text, or the reason for its removal". The line is not a diagnostic: it has no code and no severity, it is not in the counts, and it does not change the exit code. An absent anchor in a link of the locked text stays SD602. `check --json` always has the field `locked_links`, a list of `{path, line, target, pointer}`.

The front matter is not locked, so a broken `supersedes` or `depends-on` still gives SD101. The same reference in one of these sections still gives SD101 or SD102, because the author can change that text. A decision that is not accepted has no locked text.

## Code citations

SD606 and SD607 use the code citations of a document ([ADR 0001](adr/0001-information-model.md), amendment "code citations"). `context <code path>` uses the same citations. A document cites code in these ways:

- A link to a file that is not a document. The text of the link is not a second citation.
- A path in an inline code span or in prose, with an optional location: `src/loans.py`, `src/loans.py:42`, `src/loans.py#L3-L9`. A path is relative to the repository root or to the document. A file name or a path tail that only one file has also resolves, for example `loans.py`.
- A distinctive identifier in an inline code span:
  - a name with an inner `_` (`max_loans`);
  - a name with a lowercase letter before an uppercase letter (`loanBook`);
  - a called name with 4 characters or more (`renew()`).

  Words such as `status` or `HTTP` are not identifier citations. spec-dna also excludes a short list of common call names (`read()`, `main()`) and product names (`GitHub`, `macOS`).

An identifier matches a file if it occurs in the file as a whole word, even in a comment or a string. A citation that still resolves after the change, such as a file name after a move, gives no SD606.

The locked text of an accepted decision cannot change (SD405). If this text cites a removed path or identifier, an amendment section of the decision clears SD606 and SD607 for that decision. The amendment section must cite new code. New code is a path that the head tree has (the rename target, if git found one), or an identifier that the code has. The base text of the decision does not cite it. A citation of removed code in the amendment section itself still gives a warning, because that text can change.

The rule `drift` sets the severity of SD606 and SD607 (`warning` in `standard`), or disables them. Only current documents give SD606 and SD607. These documents can cite code that does not exist:

- a superseded, retired or closed document
- an incident
- a research document
- a roadmap
- a list of questions
- a spec and a plan
