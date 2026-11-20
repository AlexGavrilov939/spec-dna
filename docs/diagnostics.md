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

SD613, SD614, SD615 and SD617 are always new, because the base has no report. SD616 and SD618 are state codes and never new.

The codes SD401 to SD405, SD202, SD605, SD606, SD607, SD702, SD703 and SD921 compare the base tree with the head tree. Thus these diagnostics are always new. SD704 is new only if the root does not enforce budgets and the document is new or grew.

Without `--base`, the change codes that need a base use the merge base of `origin/HEAD` with `HEAD`, or, if there is no `origin/HEAD`, of the branch of `[change] base` with `HEAD` ([the checks of the change lifecycle](#checks-of-the-change-lifecycle)). If git cannot name this base, `check` prints a note and runs only the checks of one tree.

## Code ranges

| Range | Subject | Section |
| --- | --- | --- |
| SD101 to SD106 | References, links and evidence | [References and evidence](#references-and-evidence-sd101-to-sd106) |
| SD201 to SD204 | Document ids and requirement keys | [Ids and keys](#ids-and-keys-sd201-to-sd204) |
| SD301 to SD306 | Front matter, types, statuses and sections | [Front matter, types and sections](#front-matter-types-and-sections-sd301-to-sd306) |
| SD401 to SD405 | Status changes, supersede links and locked text | [Status and locked text](#status-and-locked-text-sd401-to-sd405) |
| SD501 to SD507 | Adapters, harness files, configuration, evals and skills | [Adapters, harness, configuration and evals](#adapters-harness-configuration-and-evals-sd501-to-sd507) |
| SD601 to SD612 | Links, drift, evidence and quality | [Links, drift and quality](#links-drift-and-quality-sd601-to-sd612) |
| SD613 to SD617 | Test reports | [Test reports](#test-reports-sd613-to-sd617) |
| SD618 to SD620 | Proof levels and the proof debt | [Proof levels and the proof debt](#proof-levels-and-the-proof-debt-sd618-to-sd620) |
| SD701 to SD704 | Budgets | [Budgets](#budgets-sd701-to-sd704) |
| SD801 to SD807 | Requirement text | [Requirement text](#requirement-text-sd801-to-sd807) |
| SD901 to SD904 | Layers | [Layers](#layers-sd901-to-sd904) |
| SD905, SD910 to SD926 | Changes | [Changes](#changes-sd905-and-sd910-to-sd926) |
| SD927 to SD935 | The acceptance record | [The acceptance record](#the-acceptance-record-sd927-to-sd935) |

## References and evidence (SD101 to SD106)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes`, `depends-on` or `from` value names no document, section or requirement. An accepted decision gives no SD101 for a reference in its locked text ([the checks of the change lifecycle](#checks-of-the-change-lifecycle)). |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree. An accepted decision gives no SD102 for a link in its locked text. |
| SD103 | evidence-missing | error | A `verified-by` path (in the front matter, or in the inline `<!-- verified-by: ... -->` marker of a requirement) is not in the git tree. |
| SD104 | ambiguous-reference | error | A reference names an alias that more than one document has. |
| SD105 | covers-unresolved | error | A `covers:` tag in a test file names no requirement. The document, the key or the requirement does not exist, or the id names a section. A tag that names the requirement of an `ADD` line of an `accepted` change is not reported: `finish` writes the requirement later. A partial tag (`covers: partial <doc-id>#<key>`) follows the same rule. |
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

## Adapters, harness, configuration and evals (SD501 to SD507)

SD501, SD502, SD504 and SD505 check the files that `spec-dna init` writes: the adapters, and the harness files of the Claude Code adapter ([harness.md](harness.md)). SD506 checks the eval files ([evals.md](evals.md)). SD506 is new in 0.6.0. SD507 reports a skill that has no measured gain (new in 0.7.0).

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD501 | adapter-modified | error | A generated adapter (the line in `AGENTS.md` or `CLAUDE.md`, or the skill block) has a manual edit. A generated harness file of the Claude Code adapter has a manual edit, or text after its marker. These files are an agent file, a skill of the workflow pack, `REVIEW.md`, `model-team.md` and the model-team line of `CLAUDE.md`. An edited skill file of 0.6 (`spec-dna-intent`, `spec-dna-spec` or `spec-dna-plan`) also gives SD501: `init` keeps it and names it, and you delete it. A file with a harness marker that the configuration does not generate also gives SD501 when it has a manual edit: delete the file, or delete its marker line to keep it as a file of the project. The scan covers `.claude/agents/`, `.claude/skills/` and `REVIEW.md`, but not the adapter skill `.claude/skills/spec-dna/SKILL.md`. |
| SD502 | adapter-outdated | error | A generated adapter differs from the adapter that this version generates. The multi-line block of an earlier version also gives SD502: `spec-dna init` replaces it with one line. A generated harness file or line of the Claude Code adapter differs from the text for the configuration. A `.claude/settings.json` entry of the harness is absent or different (the message names its JSON path). A file with a harness marker is no longer in the configuration, and it has no edit (`spec-dna init --reconfigure` removes it). The scan is the one of SD501. An old `checker.md` or `spec-writer.md` is such a file. `CLAUDE.md` has a second model-team line (delete it). The skill lists the types of the layers, so `spec-dna layer add` also gives SD502 until `spec-dna init` runs again. |
| SD503 | config-not-alone | error | The change edits `spec-dna.toml` and other files. A configuration change must be a change of its own. In a tree binary, a change of the pin line alone does not count. In each binary, `spec-dna.toml` does not count when it differs from the base only in the pin line and in `[spec-dna-sha256]` (also in the names that `upgrade` adds to the list `skills` of `[harness]`), and the base pins an earlier version. [Files that SD503 counts](#files-that-sd503-counts) lists the paths that do not count. |
| SD504 | harness-missing | error | `[harness]` has a level that is not `none`, and a file that the harness of the Claude Code adapter needs is absent: an agent file, a skill of the workflow pack (`.claude/skills/spec-dna-<skill>/SKILL.md`), `REVIEW.md`, `.claude/agents/model-team.md`, or the model-team line of `CLAUDE.md`. A file at the path of one of these files, with no harness marker, also gives SD504: spec-dna does not replace a file that it did not generate. The message gives the ways to fix it: move the file, set the role of an agent file to `none`, remove the skill from `skills`, or set `review-md = false`. |
| SD505 | harness-settings-invalid | error | `.claude/settings.json` of the harness of the Claude Code adapter is not valid JSON, or its root is not an object, or a key that spec-dna writes into has a value of the wrong type (the message names it). `check` cannot compare the harness entries. |
| SD506 | evals-invalid | error | The table `[evals]` is in the configuration, and a `*.json` file directly in `[evals] dir` is not a valid eval, or the file `<dir>/check.sh` is absent. The message names the broken rule, and the target is its key (for example `task`, `fixture` or `checks`). The key `skill` takes the name of one of the 11 skills, or `agents-line`. `spec-dna eval run` uses the same rules. SD506 reads no text of `check.sh` or of a fixture, and no file in a subfolder. It is an error in each profile. It does not depend on `[change] policy` ([the checks of the evals](#checks-of-the-evals)). |
| SD507 | skill-unproven | warning | A skill in `[harness] skills` is off by default ([ADR 0023](adr/0023-default-skills-from-the-release-table.md)). The default of a skill follows the rule that `spec-dna skill list` shows: the 8 skills of the design (the 7 of D2 and `accept`), the verdict, and the constant of the release. A skill of the 8 with the verdict `off` is reported in each release, and a skill of the 8 with no verdict `on` is reported when the constant needs the verdict `on`. A skill outside the 8 is reported unless its verdict is `on`. The verdict comes from the newest result file of `eval run --ab` in `<[evals] dir>/results` that holds the skill, and then from the table of the release ([evals.md](evals.md#the-verdict)). A file of a stopped run gives no verdict. The code needs no `[evals]` table. It is in no rule group. It is off in `lite` and `standard`, and a warning in `strict`. It is never an error. The diagnostic is on the line of `skills` in `spec-dna.toml`, and its target is the name of the skill. |

### Files that SD503 counts

SD503 reads paths only, with 2 exceptions: it reads the text of `spec-dna.toml` in a tree binary, and for the edit of `upgrade` in each binary (see below). It compares the working tree with the base (with `--base`, tracked files; without it, also each untracked file that git does not ignore, except with `--staged`). It reads the content of no other file, and its lists do not depend on the harness level. A manual edit of one of the files below does not count.

`check` does not examine a new `spec-dna.toml` (an adoption). SD503 does not apply if the base pins a version before 0.6.0. Thus one pull request can hold the configuration and the documents of an upgrade. A base from 0.6.0 keeps SD503, also for a change of the pin, except in a tree binary and except for the edit of `upgrade` (see below).

The edit of `upgrade` is the second exception, and it holds in each binary. `spec-dna.toml` does not count when its text differs from the base text only in the pin line, in the table `[spec-dna-sha256]` and in new names of the list `skills` of `[harness]`, and the pin of the base is an earlier version than the pin of the head. The list must be on one line, each old name stays, and each new name is the name of a skill (ADR 0025). A hook that runs `check` refuses each commit while the pin is older than the binary, so the upgrade is the first commit of a branch. With this rule, the same pull request can also hold other work. Each of these cases keeps SD503: another line of `spec-dna.toml` changes, the 2 pins are equal, the pin goes back to an earlier version, or a pin is not 3 numbers. The compare reads `\r\n` as `\n`. The pin stays exact: a pin that differs from the version of the binary still stops `check` ([ADR 0019](adr/0019-limits-of-0-7-0-part-a.md)).

With `--base`, SD503 uses the same base as the change checks, so it does not count a merged pull request that edited `spec-dna.toml` ([commands.md](commands.md#check)).

A tree binary is a binary that the checkout of spec-dna itself builds. The binary is a tree binary when both facts are true:

- The file `Cargo.toml` at the top level of the git repository that holds `spec-dna.toml` has `name = "spec-dna"` in `[package]`.
- Its `version` is equal to the version of the binary that runs.

A `Cargo.toml` that is a symbolic link makes no tree binary, in the working tree and with `--staged`. The 2 modes read a link in different ways, and the target can be outside the repository, so SD503 then counts the pin line ([ADR 0019](adr/0019-limits-of-0-7-0-part-a.md)).

A tree binary needs no key. It reads the head tree: the working tree, or the git index with `--staged`. In a tree binary, SD503 does not count a change of the line `spec-dna = "..."` of `spec-dna.toml`. For this, SD503 compares the text of `spec-dna.toml` in the base with its text in the head tree. The compare reads `\r\n` as `\n` in both texts, so a working tree with CRLF line ends and a base with LF line ends differ in the pin line only. A change of the line ends alone is no pin change. If the 2 texts differ in the pin line only, `spec-dna.toml` has no change. A pin change with a document therefore gives no SD503: the document is the only counted file, and SD503 needs `spec-dna.toml` and another file. A change of any other line counts, as before. Thus a pin change with another line of `spec-dna.toml` and a document gives SD503. A release binary in another repository never has this mode. The exact pin rule stays for each rule: a pin that differs from the version of the binary stops `check`, `finish` and `eval run` with exit code 3 ([ADR 0013](adr/0013-self-hosting.md)).

These paths do not count. Each one is a file that `init` writes with the configuration:

- `spec-dna.toml`
- The adapters: `AGENTS.md` (with its generated line of 400 bytes or fewer), `CLAUDE.md` and `.claude/skills/spec-dna/SKILL.md` (with the block of the disciplines).
- The files of the harness: `.claude/settings.json`, `.claude/agents/model-team.md` and `REVIEW.md`.
- The agent files: `.claude/agents/fallback-advisor.md`, `.claude/agents/verifier.md`, `.claude/agents/acceptor.md`, `.claude/agents/Explore.md`, `.claude/agents/implementer.md` and `.claude/agents/code-simplifier.md`. The agent files of 0.5 also do not count: `.claude/agents/checker.md` and `.claude/agents/spec-writer.md`.
- The skills of the workflow pack: `.claude/skills/spec-dna-brainstorm/SKILL.md`, `.claude/skills/spec-dna-specify/SKILL.md`, `.claude/skills/spec-dna-clarify/SKILL.md`, `.claude/skills/spec-dna-write-plan/SKILL.md`, `.claude/skills/spec-dna-execute/SKILL.md`, `.claude/skills/spec-dna-tdd/SKILL.md`, `.claude/skills/spec-dna-review/SKILL.md`, `.claude/skills/spec-dna-verify/SKILL.md`, `.claude/skills/spec-dna-accept/SKILL.md`, `.claude/skills/spec-dna-finish/SKILL.md`, `.claude/skills/spec-dna-debug/SKILL.md` and `.claude/skills/spec-dna-post-mortem/SKILL.md`. The skill files of 0.6 for the names that ended also do not count: `.claude/skills/spec-dna-intent/SKILL.md`, `.claude/skills/spec-dna-spec/SKILL.md` and `.claude/skills/spec-dna-plan/SKILL.md`.
- The hook files: `.pre-commit-config.yaml` and `.husky/pre-commit`.
- The starter files of the evals, if the head configuration has `[evals]`: `<dir>/check.sh`, `<dir>/example.json`, `<dir>/fixtures/example/notes.txt` and `.github/workflows/agent-evals.yml`. `<dir>` is `[evals] dir`. Without `[evals]`, these files count.
- Each path whose file name starts with `spec-dna`, such as `.github/workflows/spec-dna.yml`. This rule has no folder limit, so a document `docs/spec-dna-notes.md` does not count.

Each other path that differs from the base counts. Examples are documents, code, lint configurations such as `.markdownlint-cli2.yaml`, `CODEOWNERS`, other workflows and a deleted file. A move gives 2 paths, the old path and the new path, because the diff has no rename detection.

## Links, drift and quality (SD601 to SD612)

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document. |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target. |
| SD603 | cross-repo-unpinned | warning | spec-dna cannot check a `<repo>:<id>` reference without a pinned index. |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document. |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word. |
| SD606 | cited-path-removed | warning | A document cites a file or a directory that the change removed or renamed. The message names the new path of a file with the same content. A link gives SD102 instead. A layer folder of changes (`docs/intent`) that held only change folders (with their files at any depth) is not reported: git has no empty folder, so `finish` removes it with the last change, and the next change goes into it again. |
| SD607 | cited-symbol-removed | warning | A document cites an identifier in an inline code span. The change removed the identifier from a file, and no other file that is not Markdown has it. If the file has exactly one new identifier, the message names it as a possible new name. |
| SD608 | requirement-unverified | warning | A requirement of an active `requirements`, `interface` or `rule` document has no evidence: no `verified-by` and no full `covers:` tag. A tag `covers: partial <doc-id>#<key>` is not evidence: the message names the first such tag, and the requirement still gives SD608. |
| SD609 | incident-unlinked | warning | An incident links no requirement, rule, decision or procedure. |
| SD610 | index-stale | warning | A generated index (`spec-dna index`) does not match its entries. A change added, removed or renamed an entry document, or changed its title or status, or edited the rows manually. |
| SD611 | decision-links-intent | warning | An accepted decision links an `intent` document in its locked text, or in `depends-on`. You can delete a finished intent (SD202 allows it), but you cannot remove the link from the locked text (SD405). SD611 does not report a link in an amendment, history or live section. SD901 reports it, so name the intent as plain text there. After `spec-dna finish` deletes the intent, the link is dead, and `check --all` names it in an [info line](#locked-references). [types.md](types.md#links-between-types) gives the link directions. |
| SD612 | link-into-change | warning | A document outside the folder of a change has a reference into that folder: a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` or `from`. `spec-dna finish <slug>` stops on it at step 0 ([Stops of `finish`](#stops-of-finish)), late, in the pull request of the code. SD612 shows the same reference early. It reads the references and the exceptions of step 0: a stub, a generated index and the locked text of an accepted decision are not sources. It reports a reference that SD611 or SD901 reports in the same run (the same document, line and target) once, as SD611 or SD901. When SD901 is off (`lite`, or a rule), SD612 reports the reference. It is a warning in each profile, it is in no rule group, and the policy `none` does not turn it off: the folder of a change exists with that policy too. `[rules] SD612` changes its severity. Name the change as plain text. It is new in 0.7.0. |

## Test reports (SD613 to SD617)

These codes are new in 0.8.0. They run only with a `[evidence] reports` key that is not empty, in each profile. `check` reads the reports only with `--reports`, and `finish` always reads them when the key is set ([Test reports](config.md#test-reports)). They are in the rule group `reports`, so a rule for the group `evidence` does not change them. In `lite`, each of them is off. A rule for the group `reports`, or for one code, turns them on there. The tables give the severity in `standard`. In `strict`, SD616 is an error.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD613 | report-missing | error | No file matches `[evidence] reports`, or a report has no stamp (`<report>.stamp`). Run the tests, then `spec-dna evidence stamp`. |
| SD614 | report-stale | error | The stamp of a report names another code tree, or the bytes of the report changed after the stamp. A change of code makes a report stale. A change of Markdown, of the folder of a change or of `.spec-dna/` does not. |
| SD615 | report-unreadable | error | spec-dna cannot read a report: an unknown format, a parse error, a `testsuite` whose `tests` count differs from its `testcase` elements (a cut file), a file over 64 MiB, a `<!DOCTYPE`, a `testcase` inside a `testcase`, a stamp with bad text, or a report or a stamp that git tracks. A committed report can be a hand-written one. |
| SD616 | requirement-unproven | warning | An active requirement has a full `covers:` tag and no `verified-by`, and no test case that binds to the tag passed in the reports. The message gives one reason: no bound case, a skipped case, or a failed case, with the name of the case. It is a state code: `check --reports --all` shows it, and it is never new. It is an error in `strict`. |
| SD617 | change-unproven | error | The same fact for a requirement that a change adds or changes. It runs where SD916 runs: when the diff deletes the folder of the change, and at step 5 of `finish`. |

- SD613, SD614 and SD615 report the state of the files on the disk. The base has no report, so each of them is new. The path of the diagnostic is the report. SD613 for a missing file has the path `spec-dna.toml`.
- SD613 to SD615 give the cause. When no report is fresh, each full tag is unbound, so SD616 and SD617 still fire in the profile that has them on. Thus a setting that makes SD613 to SD615 a warning or turns them off does not make an unproven requirement pass. A report that is in no root takes the severity of the project for SD613 to SD615, and the requirement keeps the severity of its own root.
- A `verified-by` record is a check by a person. The reports do not apply to it, so a requirement with a `verified-by` gives no SD616 and no SD617.
- A tag that binds to no case gives SD616 or SD617 with the reason "no case of a fresh report binds to the tag". Put the tag above the test, or add `case:<name>`.
- SD608 and SD916 keep their meaning: no tag and no `verified-by`.

## Proof levels and the proof debt (SD618 to SD620)

These codes are new in 0.8.0. They are in the rule group `proof`, so a rule for the group `evidence` or `reports` does not change them. In `lite`, each of them is off. A rule for the group `proof`, or for one code, turns them on there. The tables give the severity in `standard`. [Proof levels and the proof debt](config.md#proof-levels-and-the-proof-debt) describes the states and the marker.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD618 | proof-owed | warning | An active requirement has a proof debt (`<!-- proof-owed: <reason> -->` with a reason of the list) and no evidence. It replaces SD608 for that requirement. It is a state code: `check --all` shows it, and it is never new. It is a warning in `strict` too. |
| SD619 | proof-owed-invalid | warning | The reason of a `proof-owed` marker is not `manual`, `environment`, `external` or `pending`, or an active requirement has the marker and has evidence (a `covers:` tag or a `verified-by`). It is an error in `strict`. The marker of an invalid reason is no debt, so the requirement keeps SD608. |
| SD620 | proof-not-reliable | error | With `[evidence] reports` and `[evidence] reliable-for`, a requirement that a covered change adds or changes passed fewer times than `[evidence] reliable-runs`. It runs where SD916 and SD617 run: when the diff deletes the folder of the change, and at step 5 of `finish`. |

- SD618 and SD619 run on a marker. No key turns them on. The marker is an act of the author on one requirement, as `verified-by` is.
- SD620 runs only when `check` reads the reports (`--reports`) and `reliable-for` covers the change. A requirement with no passed case gives SD617 or SD916, not SD620. When SD617 is off, a requirement with a tag and no passed case gives SD620. A requirement with a `verified-by` record and a tag whose bound case failed, was skipped or passed too few times gives SD620 too. A `verified-by` record with no tag is exempt.
- A debt is never proof. SD916, SD617 and SD620 stop `finish` for a requirement that has a debt, as they stop one that has none.

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

## Changes (SD905 and SD910 to SD926)

These codes are new in 0.6.0, except SD923 to SD926, which are new in 0.7.0. They apply only to a project with a `[change]` table whose `policy` is not `none`. SD918 also needs an `[evals]` table. With `policy = "none"`, SD905 and SD910 to SD926 are off, and no rule enables them. The rule group `changes` sets their severity ([profiles.md](profiles.md)). The change lifecycle is in [change.md](guides/change.md).

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD905 | fan-out | warning | One change touches more requirement documents than `[change] max-fanout`: the documents of the delta of its spec, or, for a change with no delta line, the requirement documents of its diff. Off in `lite`. |
| SD910 | gate-approval-missing | error | A `spec.md` or a `plan.md` that is not a `draft` changes while no approval of the artifact before it counts. Code under the `Files` of a plan that no approval covers counts as an artifact. With `[change] approval = "base"`, only the base tree gives an approval. With `"branch"`, an approval commit of an approver in the branch also counts. The message names one of 9 reasons. `[change] gate` sets the severity: `block` is an error, `warn` is a warning, `off` disables the code. |
| SD911 | change-stale | warning | A started change has had no commit for more than `stale-days` days (a key of the `intent` layer). It is a state code: `check --all` shows it, and it is never new. Off in `lite`. |
| SD912 | started-limit | warning | More changes than `max-started` of the `intent` layer have an `accepted` intent. The diagnostic is on `spec-dna.toml`. It is new when the head has more started changes than the lifecycle base. Off in `lite`, an error in `strict`. |
| SD913 | outside-plan | warning | A change of the diff has a plan, and the diff touches a file that no item of `Files` matches. The intent folder (with its `acceptance.toml`), the files of `.spec-dna/accepted/` and the documents that the delta names are always in scope. A file that SD503 does not count is always in scope: an adapter, a harness file, a hook file, a starter file of the evals and a file whose name starts with `spec-dna`. A path that a glob of [`[change] shared-paths`](config.md#the-table-change) matches is in scope too. Before a plan exists, SD913 gives nothing. Known limit: a path that another open change lists in its own `Files` hides SD913 for that path in this change, because SD913 asks whether any plan of the diff matches the path. No check reports it. Off in `lite`. |
| SD914 | delta-invalid | error | A delta line has an invalid form, names an absent target, targets a layer that `moves-to` does not allow, or has a hash that does not match the requirement. SD914 also reports an `Interfaces` item that is not `CHANGE <interface id>: <text>`, `ADD <interface id>: <text>` or `none`, that names an absent interface document, that has the place of another type, or that names an interface that the diff does not change. It is a warning for a spec that the diff does not touch, and with no lifecycle base. |
| SD915 | finish-incomplete | error | The diff deletes an intent folder whose base status is not `closed`, and the head does not have the result of its delta. For a change with no delta, the diff changes no `Files` path and no requirement hash. |
| SD916 | delta-evidence-missing | error | An `ADD` or `CHANGE` requirement has no full `covers:` tag and no `verified-by` when `finish` runs, or when the diff deletes the intent folder. For a change with no delta, the same applies to each requirement whose hash differs from the base. |
| SD917 | class-too-low | warning | The `class` of an intent is lower than the minimum that the diff gives. Off in `lite`, an error in `strict`. |
| SD918 | incident-eval-missing | warning | The diff adds a document of type `incident`, and adds or changes no `*.json` file directly in `[evals] dir`. It needs `[evals]` and a `[change] policy` other than `none`. Off in `lite`. A move of an incident is not an add. |
| SD919 | concern-open | error | A spec with the status `approved` has an open concern (`- [ ]` in `Concerns`). |
| SD920 | decision-needed-unaccepted | error | A decision that `Decisions` of an `approved` spec names does not exist. Also, when `finish` runs or the diff deletes the intent folder, a decision that `Decisions` names is absent or is not `accepted`. |
| SD921 | truth-changed-without-change | warning | A requirement of a `rule`, `requirements` or `interface` document of a `forever` layer changed, was added or was removed, and no file under the folder of the `intent` layer changed. It runs on the lifecycle base, and only with a `[change] policy` other than `none`. A plan that covers the document gives an exemption. Off in `lite`, an error in `strict`. |
| SD922 | question-open | error | A spec with the status `approved` has an open question (`- [ ]` in `Open questions`). |
| SD923 | task-incomplete | warning | A task of an `approved` plan has no `Files:` or no `Verify:`, its `Files:` has a glob or a folder, or its `Skip:` has no reason. A glob has `*` or `?`. A `draft` plan is not checked. A plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0 is skipped. Off in `lite`, an error in `strict`. |
| SD924 | task-too-large | warning | A task of an `approved` plan lists more paths in `Files:` than `[change] max-task-files` (default 3). A plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0 is skipped. Off in `lite`, an error in `strict`. |
| SD925 | checkbox-missing | warning | An item of `Concerns` or `Open questions` of a spec has no task box, so SD919 and SD922 cannot see if it is open. A spec of each status gives it, except the spec of a `closed` change. Off in `lite`, an error in `strict`. |
| SD926 | approver-agent | warning | An item of `[change] approvers` is the address of an agent or a bot, so a commit of this author can come from an agent and it proves no approval of a person. The addresses are `noreply@anthropic.com` and each address that ends with `[bot]@users.noreply.github.com`. The diagnostic is on the line of the key `approvers` in `spec-dna.toml`, and the target is the item. One diagnostic for each item, with `approval = "branch"` and with `"base"`. A warning in each profile. A project that wants an agent identity as an approver sets `[rules] SD926 = "off"`. |

## The acceptance record (SD927 to SD935)

These codes are new in 0.8.0. They are in the rule group `acceptance`, so a rule for the group `changes` does not change them. In `lite`, each of them is off. A rule for the group `acceptance`, or for one code, turns them on there. With `[change] policy = "none"` they are off, and no rule enables them. The severity of each code is an error in `standard` and `strict`. [The acceptance record](config.md#the-acceptance-record) describes the key, and [accept](commands.md#accept) describes the command and the record.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD927 | acceptance-missing | error | A change that needs a record has none: the diff deletes its folder and the diff adds no archive file of the change (a record in the folder of the base does not count), or `finish` finds no `acceptance.toml` in the folder. In a shallow clone the commit that added the plan can be absent, so the message names the clone: fetch the full history (`fetch-depth: 0`) |
| SD928 | acceptance-invalid | error | The record breaks a rule of a valid record: the format, the change name, the scope and the paths, a value `pending`, an acceptor field, the number of an acceptor in an attempt or a finding, a missing reason, the result logic, or the size limit of 16,384 bytes. Also: the `brief` of an entry is not the brief of the commit that added the entry. The message says to remove the entry and accept again from a new brief. A shallow clone that cuts `base..HEAD` gives SD928 with the fix `fetch-depth: 0` |
| SD929 | acceptance-stale | error | The hash of `paths` differs from the code now, or the diff has a code path that `scope` matches and `paths` does not list. The fix: revert the commit of `finish`, merge the base, accept again, and run `finish` again. `finish` itself says: delete the record, accept again |
| SD930 | acceptance-rejected | error | The top `result` of the record is `rejected` |
| SD931 | self-accepted | error | 2 acceptor entries have the same `session` and the same `agent`. The value `unknown` matches no value, and `accept` writes `agent = "unknown"`, so the code fires only for a value that a person or an agent writes by hand. It cannot catch a forger |
| SD933 | acceptor-missing | error | The record has fewer acceptor entries than the change needs: 1 for a `routine` change, `[change] higher-risk-acceptors` for a `higher-risk` change, and one entry for each role of `[owners]` that matches a path of the record |
| SD935 | record-edited | error | The diff changes or deletes a file of `.spec-dna/accepted/`. The folder is the archive of the records of finished changes. Also: a file that the diff adds is not the record that `finish` moves. The content of an added archive file of a change that needs a record must be the content of its folder record (line ends CRLF and LF count the same): the newest text of `<folder>/acceptance.toml` in the commits of `base..HEAD`, else its text in the base. If the content differs, or no such text exists, SD935 reports the archive file. The rule covers a later commit that edits an archive file that the branch added, because the diff holds the last content only. An added archive file of another change, with no need for a record, is no finding |

A change needs a record when the acceptance is on (SD927 is not off) and its plan is new. A closed change needs none, and a small fix has no plan. One rule serves `finish`, `check` and `next`:

- A plan that the lifecycle base does not have is new: a commit of `base..HEAD` added it, or no commit did. Thus a branch cannot set the pin back for one commit to skip the record.
- A plan that the base has is new when the newest commit that added it, and that is an ancestor of the base, pinned 0.8.0 or later in `spec-dna.toml`. A pin before 0.8.0 needs no record, and `finish` prints the note ``note: `<plan>` was added when spec-dna.toml pinned <pin>, before 0.8.0, so `finish` needs no acceptance record`` on stderr. A shallow clone has no such commit, and a commit with no pin gives no version: both need a record, as the stricter result.
- After `upgrade` alone, a repository of 0.7.1 gets no new diagnostic. The codes run only in `finish` and where a diff touches a record, an archive file or a deleted folder.

Where each code is raised:

- `finish`, step 5.2: it reads `<folder>/acceptance.toml` of the head and stops at the first code that is not off, in this order: SD927, SD928, SD929, SD930, SD931, SD933. Step 5.5 moves the record to `.spec-dna/accepted/<slug>.toml`, or to `<slug>-2.toml`, `-3` and so on if the file exists. It moves each record that the folder has, also in `lite`, with `acceptance` off, and for a plan of a version before 0.8.0. A rule that needs the record decides only whether `finish` stops: with no rule and no record, `finish` writes no archive file, as before. It stops before it writes if git ignores the path, and the message names the rule (`git check-ignore -v`).
- `check`, where the diff adds or changes a record, in the folder of a change or as an archive file: SD928 and SD931 only. Thus the first of 2 acceptors can commit, and a code fix after a `rejected` record passes `check --staged`. The compare of the brief runs for an entry that the branch adds to the record of the folder. It does not run for an entry that the base already has, nor on an archive file.
- `check`, where the diff deletes the folder of a change that needs a record, and for a change that the branch added and deleted: SD927, and, for the archive file that the diff adds, SD929, SD930 and SD933. A branch-born change also gets SD910 (reason 7). `check --staged` reads the index, so the hash is the hash of the staged code.
- `check`, for each diff: SD935. It reports a changed or a deleted archive file, and an added archive file with the wrong content (see the row of SD935). The compare of the content does not run in a shallow clone that cuts `base..HEAD`: SD928 reports the cut.
- SD913 never reports `acceptance.toml` in the folder of a change or a file of `.spec-dna/accepted/`.
- The compare of the brief reads the commits of `base..HEAD` (`git rev-list --reverse --first-parent`). The adding commit of an entry is the oldest commit of the latest unbroken run of commits whose record holds an entry with the same role, session and brief. A later commit of the record does not move it. An entry that no commit has takes the index with `--staged`, else the working tree.

- `accept` and `accept --brief` run in each profile. The codes decide only whether `finish` and `check` need a record.
- The record proves that the work is visible and bound to the code. It does not prove that the acceptor is not the builder: `session` and `agent` are claims, and the brief is a text that anyone can compute.
- The table `[[proof]]` of the record is a snapshot. No check compares it with the proof states at `finish`.
- The compare of the brief (SD928) reads `spec-dna.toml` of `HEAD`, not the one of the adding commit. The text of the brief does not read the configuration yet, so this has no effect. Slices S5 and S6 add the owners and the invariants: the compare then needs the configuration of the adding commit.

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

- SD905 (the delta part), SD912 (the count), SD914, SD919, SD920 (the absent decision), SD922 and SD925 read one tree. They need no base and no history.
- SD923 and SD924 read one tree. They need no base. For a plan with a finding, they also read the history of that plan: the commit that added it, and the pin of `spec-dna.toml` in that commit ([SD923](#checks-of-the-change-lifecycle)).
- SD926 reads `spec-dna.toml` only. It needs no base and no history. The hidden address of a person, such as `1234+name@users.noreply.github.com`, is not on the list. The target is the item as the file writes it, and the compare of the address ignores case. SD926 is not a state code. It is new when the configuration of the base does not list the address. Against a base that lists it already, `check` shows it only with `--all`.
- These codes compare the head with the lifecycle base. They do not run if git cannot name this base:
  - SD905 (the part for a change with no delta);
  - SD910, SD913, SD915, SD916, SD917 and SD918;
  - SD920 (a decision that is not `accepted`);
  - SD921;
  - SD927 to SD931, SD933 and SD935 ([the acceptance record](#the-acceptance-record-sd927-to-sd935)).
- SD911 reads the base and the history.

The lifecycle base is the merge base of `--base` (or, without it, of `origin/HEAD`, or of the branch of `[change] base`) with `HEAD`. The later merge base with `origin/HEAD` replaces an older `--base` ([commands.md](commands.md#check)), so the walk of SD910 does not read merged pull requests. The diff is the working tree (with `--staged`, the git index) against it. The diagnostics of the checks of the diff are always new. A check reads the artifacts of a change from the head tree. If the diff deletes the folder of a change, it reads them from the base tree. With `[change] approval = "base"`, it reads each approval from the base tree only, so an approval in the same pull request does not count. With `"branch"`, SD910 also reads the commits of `base..HEAD` (see SD910 below).

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
    - The scope of a plan that an approval commit of the branch approved is its `Files` at that commit. A path that a later commit adds to `Files` has no approved plan (reason 8). A plan that the base tree approves keeps the rule of 0.6: its scope is the `Files` of the head, and SD913 warns on a widened scope.
    - Each status that the branch sets (`accepted` or `approved`) needs an approval commit that set it, also when no later work depends on it.
    - The check needs the full history. A shallow clone, and a merge commit that merges another branch, stop the rule, and SD910 then reports as with `approval = "base"`. If the base is known and the merge base is not reachable in a shallow clone, SD910 reports reason 5 for each change that needs an approval. An unknown base (no `origin/HEAD` and no `--base`) gives no change check and a note, as in 0.6.1. A merge of the base into the branch, and the test merge of a code host, do not stop it.
    - A change that the branch adds and finishes gives SD910, also when approval commits exist. The record of `finish` needs the approvals in an earlier pull request.
    - An approval commit of the branch holds only while the artifact keeps its approval status at the head (the working tree, or the index with `--staged`). A later commit that sets the artifact back, for example a plan back to `draft` after the code, gives SD910 with reason 9 to the work that used the approval. Work after that commit and before a new approval commit gives reason 9 too. A new approval commit clears the work after it. An artifact that the head does not have, such as a folder that `finish` deleted, keeps its approval.
    - An approval from the base tree does not follow this rule: SD401 reports the step back, and the work before it stays clear.
    - With `approval = "base"`, a `draft` plan in the head gives SD910 for its code, because the base tree has no `approved` plan.
    - An author e-mail is not a proof. The code host stays the control: `CODEOWNERS`, a protected default branch and required signed commits.
  - The message of SD910 names one reason of 9:
    1. The base tree has no approval.
    2. No approval commit comes before the work.
    3. The author of the approval commit is not an approver.
    4. The commit changes more than the `status` line.
    5. The clone is shallow.
    6. A merge commit stops the rule.
    7. The branch added and finished the change.
    8. The path is not in the `Files` of the plan at its approval commit.
    9. A later commit set the artifact back after its approval. The message names the first commit that changed the status after the approval commit, or the working tree. For the working tree, the message also says how the commit of the step back passes a hook that runs `spec-dna check`: skip only that hook for this commit, name the skip in the body, and make the approval commit next. A commit of the history gets no such text, because its hook has already run.
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
  - If a plan lists no path, and it has an item that is not a path, the message names the first such item. If its `Files` section has a paragraph and no list item, and the paragraph is not in the `none` form: the first word is `none`, followed by `:`, `.` or the end of the paragraph ([Files and paths](types.md#files-and-paths)), the message says: "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`". SD915 gives the same text.
  - For a change with no delta, a requirement document is in scope only if `Files` lists it.
  - SD913 gives one diagnostic for each path outside the scope of every change of the diff, also for a path that is deleted. A generated index is not reported. A file of the tool (the list of SD503) and a path of `shared-paths` are not reported.
  - SD913 runs only when at least one change of the diff has a `plan.md`. A `draft` plan is a plan, and so is a plan with `- None.` under `Files`. A change also has a plan if the base tree has its `plan.md`, so a branch that deletes the plan does not turn SD913 off. Before a plan exists, a change has no `Files`, so it has no scope to check against. Thus an edit of another document (such as the questions or the roadmap) or of a spike gives no SD913 during `clarify`.
  - The check reads the plan in the tree that SD913 reads for the change: the head tree, the base tree or the newest branch text for a folder that the diff deletes, or the newest branch text of a change that the branch added and finished. A change that its `Files` matched has a plan.
  - When a change of the diff has a plan, the rule applies to each path outside the scope of every change of the diff. The scope of a change with no plan (its folder, and the documents that its delta and `Interfaces` name) still counts. An edit that belongs to a change with no plan, next to a change with a plan, still gives SD913. Keep one change in a branch.
  - The warnings come back when the plan arrives in the same branch, because the diff against the base still holds the edits of `clarify`. List such a path in `Files`, or merge the edits of `clarify` in a pull request of their own.
  - Code that a branch edits before any plan exists gets no signal: SD910 needs `Files`, and SD913 needs a plan. The review of the pull request is the control.
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
  - An item of `Interfaces` has one of 3 forms:
    - `CHANGE <interface id>: <text>`. The id must name a document of the type `interface`.
    - `ADD <interface id>: <text>`, for a new interface. The id names an `interface` document, or it names no document and its place gives the type `interface`. Else the message says "the place of `<id>` gives the type `<type>`, not `interface`".
    - One item whose first word is `none`, followed by `:`, `.` or the end of the text ([the `none` rule](types.md#the-none-rule)). The section then states "no interface", and it gives no SD914.
  - When `finish` runs, or when the diff deletes the folder of the change, an `ADD` item needs the document in the head tree, or a delta line `ADD <id>#<key>` that adds it. Else SD914 says "the spec adds the interface `<id>`, and the tree has no such document; write it in this pull request, or add its requirements with `ADD <id>#<key>` lines". A document that the diff adds is a change of the interface, so the rule "the diff changes the interface" holds.
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
- SD919. Each open concern (a task list item with an empty box) of an `approved` spec gives one diagnostic. A resolved concern, an item with no box, and a spec with another status give none. One item whose first word is `none`, followed by `:`, `.` or the end of the text ([the `none` rule](types.md#the-none-rule)), such as `- [ ] None.`, is no open concern.
- SD922. Each open question (a task list item with an empty box in `Open questions`) of an `approved` spec gives one diagnostic. A resolved question (`- [x]`), an item with no box, and a spec with another status give none. One item whose first word is `none`, followed by `:`, `.` or the end of the text, such as `- [ ] None.`, is no open question. `Concerns` and `Open questions` are 2 sections: an open concern gives SD919, and an open question gives SD922.
- SD923:
  - The check reads each task of the section `Order` of a plan whose status is `approved`. A `draft` plan is not checked, so a plan of 0.6.1 gives no new diagnostic until it is approved. The plan of a change whose intent is `closed` is not checked. A task whose id an earlier task has is not checked.
  - A task needs 2 labels:
    - `Files:` lists the exact paths of the task, or `none` (`Files: none.`). The paths follow the rules of [Files and paths](types.md#files-and-paths). `[` and `]` are plain characters, so `app/[id]/page.tsx` is an exact path. A path with `*` or `?`, a path that ends with `/`, and a path that names a folder of the tree are not exact: the check reports them as a glob or a folder. A label with no path and with text that is not in the `none` form (the first word is `none`, followed by `:`, `.` or the end of the text) is also not valid.
    - `Verify:` has a command in a code span, or the check in words.
  - A third label is a problem if it is empty: `Skip:` needs a reason (`Skip: the store keeps the old alert.`). A task with an empty `Skip:` is not skipped. A skipped task still needs `Files:` and `Verify:`, because the plan was approved with them. `Hours:` is not checked here: `finish` reports an invalid value.
  - One diagnostic names each problem of the task, and the fix of each. Its target is the id of the task. An empty `Skip:` gives the problem ``its `Skip:` has no reason`` and the fix ``write the reason after `Skip:` ``.
  - A plan of 0.6 can have the labels already, but the text does not show the version, so the history does. For an `approved` plan with a finding of SD923 or SD924, `check` reads the newest commit that added the plan (`git log -1 --diff-filter=A --no-renames`) and the pin of `spec-dna.toml` in that commit. A pin before 0.7.0 skips both codes for the plan. A plan that is not committed, a clone that is shallow and a commit with no pin give the current form. When a skipped diagnostic would show (it is new, or `--all` is set), `check` prints one note on stderr: the plan, the pin, and the number of tasks with a finding. The note is not in `--json`. To check the plan, give each task exact paths in `Files:` and one `Verify:`.
- SD924. The check reads the same tasks as SD923. It counts the distinct paths of the label `Files:` of a task, and it gives one diagnostic if the count is above `[change] max-task-files`. The target is the id of the task. A task of 3 paths or less and one `Verify:` is the size that a person can review.
- SD925. The check reads the items of `Concerns` and `Open questions` of each spec, in each status. An item with no task box gives one diagnostic. The target is the text of the item (80 characters at most). A section with one item whose first word is `none`, followed by `:`, `.` or the end of the text gives none. The message of `Open questions` names SD922, and the message of `Concerns` names SD919. The spec template writes a box.
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
    - A full `covers:` tag of a test file names it.
    - It has a `verified-by` marker.
    - Its document has `verified-by` in the front matter.
  - A tag `covers: partial <doc-id>#<key>` is not evidence, also next to no other evidence. A full tag or a `verified-by` of the same requirement is evidence, also next to a partial tag. If only a partial tag names the requirement, the message names the first such tag and says so.
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
| 0 | A document outside the folder links into it: a Markdown link or image, `[[id]]`, `depends-on`, `supersedes` or `from`. `finish` reports each reference as SD101 (an id) or SD102 (a link), and `check` reports it as SD612. A stub, a generated index and the locked text of an accepted decision are not sources. The folder is not tracked by git, or it differs from `HEAD` (an edit that is not committed, or an untracked file): this stop has a reason and no diagnostic |
| 1 | The intent is not `accepted`, the plan is not `approved`, or the spec is not `approved` (for a `higher-risk` change, and for a spec that has a delta line). A `closed` intent passes: `finish` then deletes the folder and does nothing else |
| 2 | SD919 (an open concern), SD922 (an open question) or SD920 (a needed decision that is absent or not `accepted`) |
| 3 | SD914: a delta line is not valid, its target is absent or in a layer that `moves-to` does not allow, or its hash is not the hash of the requirement. Also SD914: the delta differs from the delta that the base tree has (an approved delta must not change). Also SD914: an interface of `Interfaces` that the diff does not change, or an `ADD` item of `Interfaces` whose document the tree does not have and no delta line adds |
| 4 | The result would not give the result of a delta line (`check` would report SD915), or two lines edit the same lines of a file |
| 5 | SD613, SD614 or SD615 (step 5.1: with `[evidence] reports`, a test report is missing, stale or unreadable), SD916 (a requirement that the change adds or changes has no evidence), SD617 (step 5.4: it has a tag, and no bound test case passed in the reports), SD620 (step 5.4: with `[evidence] reliable-for`, a requirement passed fewer times than `[evidence] reliable-runs`), SD927, SD928, SD929, SD930, SD931 or SD933 (step 5.2: a change that needs a record has none, or the record is not valid, stale, rejected, self-accepted or short of acceptors), a path of the archive file that git ignores or that has a link (step 5.5), or the result adds an error. The only exception is SD305 for a new document: the author adds the sections that the root requires |

The base of step 3 is the merge base of `--base` (or of `origin/HEAD`, or of the branch of `[change] base`) with `HEAD`, for each policy. An `accepted` change needs it, and without it `finish` exits with code 3. A change that the base tree does not have was added in the same branch: its delta has no approved text to compare.

### Locked references

A reference in the locked text of an accepted decision gives no SD101 and no SD102. This text cannot change (SD405). A reference to a finished intent must stay in it (SD611 warned when the author wrote it). The locked text is each part of the body outside the amendment, history and live sections.

`check --all` prints one info line for each such reference whose target no longer exists, after the diagnostics: ``<path>:<line>: info: the locked text links `<target>`, which no longer exists; add an amendment that names `<target>` and points to <pointer>``. The pointer is the change if the target was in the folder of an intent: ``the change `<slug>` (finished in commit <short id> on <date>)``, or "closed in" for a closed change. The commit is the newest commit that [`log`](commands.md#log) gives for the folder. `check --all` and `check --json` read the history only when such a link exists. For each folder of the `intent` layer that holds a change that a dead link names, `check` runs one `git log` over the deleted `intent.md` files, then one `git diff-tree` and one read of the parent `intent.md` for each deleted change that a dead link names. It reads no other change. If the history has no such commit (a shallow clone, or a deletion that is not committed yet), the pointer is ``the change `<slug>` (`spec-dna log` names its commit)``. For another target, the pointer is "the new place of the text, or the reason for its removal". The line is not a diagnostic: it has no code and no severity, it is not in the counts, and it does not change the exit code. An absent anchor in a link of the locked text stays SD602. `check --json` always has the field `locked_links`, a list of `{path, line, target, pointer, commit}`. `commit` is the full id of the commit that the pointer names, or `null`.

An amendment section of the same decision (`amendment-sections`, with its sub-sections) clears the line when it names the target. A code span, or a word of prose with a `/`, on a line of the section must equal one of these: the target as written, with no `#` fragment; the id or the path of the place that is gone, from the root of the documents or of the repository, with or without `.md`. For a target in the folder of a change, the slug of the change in a code span also clears the line. A cleared link is not in `locked_links`. A link to the dead target in the amendment still gives SD102, because that text can change.

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
