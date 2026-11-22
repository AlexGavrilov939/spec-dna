# Profiles

This page gives the profiles, the severity of each diagnostic code in each profile, and the rules that override a severity. It is for the author of `spec-dna.toml`.

A profile sets the default severity of each diagnostic code. Set it in `spec-dna.toml`:

```toml
profile = "standard"            # "lite", "standard" (default) or "strict"
```

An error fails `check` when the error is new against the base. A warning never fails `check`. The value `off` disables the diagnostic.

## The table

| Rule group | Codes | lite | standard | strict |
| --- | --- | --- | --- | --- |
| `references` | SD101, SD102, SD104, SD106 | warning | error | error |
| `ids` | SD201, SD202, SD203, SD204 | warning | error | error |
| `budgets` | SD702 (a new document over the hard budget, a document that was under it in the base, or a document over it that grew) | warning | error | error |
| `budgets` | SD701 (over the soft budget), SD703 (over the soft budget and grew), SD704 (over the hard budget, no growth) | warning | warning | error |
| `drift` | SD606, SD607 | off | warning | error |
| `evidence` | SD103 (a `verified-by` path is not in the git tree), SD105 (a `covers:` tag names no requirement) | warning | error | error |
| `evidence` | SD608 (an active requirement has no evidence) | off | warning | error |
| `reports` | SD613 (report missing), SD614 (report stale), SD615 (report unreadable), SD617 (change unproven) | off | error | error |
| `reports` | SD616 (an active requirement is unproven by the reports) | off | warning | error |
| `proof` | SD618 (a proof debt and no evidence) | off | warning | warning |
| `proof` | SD619 (a proof marker that is not valid or stale) | off | warning | error |
| `proof` | SD620 (a requirement is below the level `reliable`) | off | error | error |
| `acceptance` | SD927 (acceptance missing), SD928 (acceptance invalid), SD929 (acceptance stale), SD930 (acceptance rejected), SD931 (self-accepted), SD932 (record commit invalid: with `approval = "branch"` only), SD933 (acceptor missing), SD934 (ack missing: with `approval = "branch"` only), SD935 (record edited), SD941 (acceptance rerun: with `approval = "branch"` only) | off | error | error |
| `acceptance` | SD938 (record incomplete), SD939 (deviation invalid), SD940 (deviation unresolved) | off | error | error |
| `acceptance` | SD936 (invariant invalid), SD937 (invariant stale). They run only with `[change] invariants = true`, also when a rule for the group or the code turns them on | off | error | error |
| `rollup` | SD641 (summary stale), SD642 (summary incomplete), SD643 (pin invalid), SD644 (summary of no area) | off | error | error |
| `rollup` | SD645 (a re-pin with the same text, against the base) | off | warning | warning |
| `lint` | SD801 to SD805 (BCP 14 keyword, EARS pattern, sentence length, vague terms, passive voice) | off | off | warning |
| `layers` | SD901 (link direction), SD902 (layer limit), SD903 (history in requirements), SD904 (forbidden term) | off | warning | warning |
| `changes` | SD905 (fan-out), SD911 (stale change), SD913 (outside the plan), SD918 (incident with no eval) | off | warning | warning |
| `changes` | SD912 (started limit), SD917 (class too low), SD921 (truth changed without a change), SD923 (task incomplete), SD924 (task too large), SD925 (checkbox missing) | off | warning | error |
| `changes` | SD926 (an agent or bot address in `approvers`) | warning | warning | warning |
| `changes` | SD910 (gate approval missing), SD914 (delta invalid), SD915 (finish incomplete), SD916 (delta evidence missing), SD919 (concern open), SD920 (decision needed, not accepted), SD922 (question open) | warning | error | error |

The other codes keep the severity of [docs/diagnostics.md](diagnostics.md) in every profile. For example, SD405 (the locked text of an accepted decision changed) is always an error. SD506 (an eval file that is not valid, or an absent `check.sh`) is in no group, so it is an error in every profile. SD507 (a skill in `skills` that is off by default, see [ADR 0023](adr/0023-default-skills-from-the-release-table.md)) is in no group too. It is off in `lite` and `standard`, and a warning in `strict`. Only a rule for SD506 or SD507 in `[rules]` can change them. SD612 (a link into the folder of a change) is also in no group: it is a warning in each profile, and a rule for SD612 changes it.

The group `reports` is new in 0.8.0. Its codes run only with `[evidence] reports` ([config.md](config.md#test-reports)). They are off in `lite`. In `standard` and `strict` they run only when the project sets the key, so `upgrade` gives a project of 0.7.1 no new error. A rule for the group `evidence` does not change them. A rule for the group `reports`, or for one code, turns them on in `lite` too. 0.8.0 adds no key to a project that `upgrade` moves.

The group `proof` is new in 0.8.0. SD618 and SD619 run on a marker `<!-- proof-owed: <reason> -->`. The marker is an act of the author on one requirement, as `verified-by` is, so no key turns it on. In `standard` its codes are warnings, so a marker never stops a change there. SD620 runs only with `[evidence] reports` and `[evidence] reliable-for` ([config.md](config.md#proof-levels-and-the-proof-debt)). The codes are off in `lite`. A rule for the group `evidence` or `reports` does not change them. A rule for the group `proof`, or for one code, turns them on in `lite` too.

The group `rollup` is new in 0.8.0. It needs no key, and it has no effect on a tree with no `summary` document and no `system` or `product` document with `rests-on`: `upgrade` gives a project of 0.7.1 no new diagnostic. The codes are off in `lite`, because each new mechanism of 0.8.0 is off there. In `standard` and `strict`, a stale summary is an error: an error fails `check` only when it is new against the base, so a stale summary of the base does not stop a later change. A rule for the group `rollup`, or for one code, turns the codes on in `lite` too. A rule for the group `budgets` does not change them. The default severity of each code is a warning, and the table sets the rest.

The group `acceptance` is new in 0.8.0. It needs no key: `finish` needs an acceptance record for a change that has a plan and that `[change] acceptance-for` selects, in `standard` and `strict`. The codes are off in `lite`. With `[change] policy = "none"` they are off in each profile, and no rule enables them. A rule for the group `changes` does not change them. A rule for the group `acceptance`, or for one code, turns them on in `lite` too. The command `spec-dna accept` runs in each profile: the codes decide only whether `finish` and `check` need a record ([accept](commands.md#accept)). A plan that a commit added on the base at a pin before 0.8.0 needs no record. A project that runs `upgrade` gets no new error for the plans that it has. A plan that the branch adds needs one ([the rule](diagnostics.md#the-acceptance-record-sd927-to-sd935)).

The rule groups `layers` and `changes` are new in 0.6.0. The word `rules` still names the severity table of the file. A layer named `rules` is not that table: `[rules]` is the severity table, and `[layers.rules]` is the layer.

## The change codes

The codes of the group `changes` depend on the `[change]` table:

- Without a `[change]` table, the policy is `none`. With `policy = "none"`, each code of the group is off, and no rule enables it.
- SD918 also needs the table `[evals]`.
- SD910 follows `[change] gate` when the key is present. `block` makes it an error, `warn` makes it a warning, and `off` disables it. Then the profile column does not apply. If `gate` is absent, SD910 takes the severity of its profile column. When `init` writes the `[change]` table, it also writes `gate`.
- SD910 reads the approvals of the base tree. With `[change] approval = "branch"`, it also reads the approval commits of the branch ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). `strict` reads the approvals of the base only, so `approval = "branch"` with `profile = "strict"` is an error (exit code 3). `init` writes `approval = "branch"` for `lite` and `standard`, and `approval = "base"` for `strict`.
- A rule for SD910 in `[rules]` has priority over the gate. `[rules]` can also override SD921, as it overrides each code.

SD923, SD924, SD925 and SD926 are new in 0.7.0. SD926 is a warning in each profile. SD923 and SD924 read the tasks of an `approved` plan only, so a draft plan of 0.6.1 gives no new diagnostic. They skip an `approved` plan that a commit added when `spec-dna.toml` pinned a version before 0.7.0, in each profile ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). In `standard` they are warnings, and a warning never fails `check`. In `strict` they are errors, and the gate reports only a new error. SD923 and SD924 read the head tree, as SD925 does, and one `git log` for each plan that they skip.

These codes compare with the lifecycle base: SD905 for a change with no delta, SD910, SD913, SD915, SD916, SD917 and SD918. SD920 for a decision that is not `accepted` and SD921 do the same. This base is the merge base of `--base`, of `origin/HEAD` or of the branch of `[change] base`, with `HEAD`. These codes do not run if git cannot name it.

SD911 reads the base and the history. It is silent without a base. SD911 is a state code: it is never new, and `check` shows it only with `--all`.

SD914 has one exception to the profile column. `check` lowers an error of SD914 to a warning when the diff does not touch the spec. It does the same when git cannot name the lifecycle base.

SD912 is on `spec-dna.toml`, so the profile and the rules of a root do not apply to it, as for SD902 ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)).

Choose a profile for the stage of the project:

- `lite`: adoption. Nothing fails `check`, except the structural errors of the model, such as invalid front matter, a changed accepted decision or an invalid eval.
- `standard`: daily work. Broken references and ids fail `check`. A document that grows over its hard budget fails `check`. A break of the gate or of the finish rules of a change fails `check`. Drift and requirements without evidence give warnings.
- `strict`: a mature corpus. Drift, requirements without evidence and every budget diagnostic fail `check`. Too many started changes, a class that is too low and a truth that changed without a change fail `check` too. The requirement lint gives warnings.

In `standard`, a mature corpus that links no tests yet gets one SD608 for each active requirement. `check` prints more than 10 warnings of one code as one count. The gate against the base reports only the new warnings. To disable them until the project links tests, set `[rules] evidence = "off"`.

## What `init` writes for each profile

`spec-dna init` expands the profile of a new project into keys, so the file shows the whole workflow (ADR 0017 and [ADR 0023](adr/0023-default-skills-from-the-release-table.md)). The profile has no hidden effect: the binary reads the keys, and the answer to each question replaces its key.

| Key | lite | standard | strict |
| --- | --- | --- | --- |
| `[change] policy` | `two-class` | `three-class` | `three-class` |
| `[change] gate` | `warn` | `block` | `block` |
| `[change] approval` | `branch`, with the e-mail of git as the approver | `branch`, as `lite` | `base` |
| `[harness] skills` at the level `lean` | `brainstorm`, `write-plan` and `finish`, each only if it is on by default | each skill that is on by default | each skill that is on by default |
| `[harness] skills` at the level `full` | the 11 skills | the 11 skills | the 11 skills |
| `[harness] review-md` | `false` | `true` | `true` |
| `[harness] protect-tests` | `off` | `fix-branches` (`off` for the link style `verified-by`) | as `standard` |
| `[evals]` (question 14) | `none` | `skeleton` | `skeleton`, and a note on `eval pack` |
| `[harness] protect-approvals` | not written | not written | `true` |
| `[evidence] reports` | not written | `[]`, with a comment | `[]`, with a comment |
| `[evidence] reliable-for` | not written | not written | `# reliable-for = "higher-risk"`, as a comment |
| `[change] acceptance-for` | not written | not written | not written |
| `[change] higher-risk-acceptors` | not written | not written | `2` |
| `[change] invariants` | not written | not written | `true` |

- The profile also sets the default of `[change] acceptance-for` at run time: `all` in `strict`, and `higher-risk` in `standard` and `lite` ([config.md](config.md#the-acceptance-record)). `init` writes nothing, so a change of `profile` moves the default. In `standard`, only a `higher-risk` change needs an acceptance record. In `strict`, each change that has a plan needs one. In `lite` the codes are off, so the key has an effect only with a rule for the group `acceptance`. A key in the file replaces the default.
- `init` detects no test runner, so it writes `reports = []`. The key turns the test reports on when the project lists the globs of its report files ([config.md](config.md#test-reports)). `--reconfigure` keeps the list.
- `strict` writes `reliable-for` as a comment under `reports`, because the key needs `reports` ([config.md](config.md#proof-levels-and-the-proof-debt)). An active key with an empty `reports` is an error (exit code 3). `--reconfigure` keeps `reliable-for` and `reliable-runs` when the file sets them.
- A skill is on by default when `spec-dna skill list` says so. The rule of [ADR 0023](adr/0023-default-skills-from-the-release-table.md) reads the 7 skills of the design (D2), the verdict of the skill in the table of the release, and a constant of the release. A skill outside the 7 can be on by default when its verdict is `on`. `full` ignores the verdicts: a person chooses it.
- The profile `lite` also selects the text of the generated files. Each skill file and `model-team.md` of `lite` has the text of 0.7.1 above the marker line, and `lite` writes no agent `acceptor`. `standard` and `strict` have the text of this release ([harness.md](harness.md#the-workflow-pack-and-reviewmd)).
- With the policy `none`, `init` writes no skill.
- `lite`, `standard` and `strict` differ in the approval, the hook and the severities, and `strict` also reads the verdicts. `standard` and `strict` have the same rule for the skills.
- `strict` writes no eval pack: the pack has 57 evals in 812 files. `init` prints a note with the command `spec-dna eval pack`, which writes the pack before `spec-dna eval run --ab`.
- An answer to question 4, 5, 12 or 14 replaces the value of the table.
- `init --reconfigure` keeps the current values. A part that the file does not set (`skills`, `review-md` or `protect-tests`) gets the default of the profile of the file. A new answer to question 1 changes the severities, and `init` prints a note. For the profile `strict`, `init` also writes `approval = "base"` (the key `approvers` stays, and the note says so) and `protect-approvals = true`, because `strict` reads the approvals of the base only (ADR 0018).
- `upgrade` applies no preset, with one exception. An upgrade from a pin before 0.7.0 adds to a list `skills` that is not empty each skill that is new in 0.7.0 (`tdd`, `execute`, `verify`, `finish`, `debug` and `post-mortem`) and that `init` proposes for the profile and the level of the project. A name of the list keeps its place, and an empty list stays empty. A project with the policy `none`, or with no `[change]` table, gets no skill, as with `init`. The report names each skill that it adds. Remove a name from the list to turn its skill off. If the list has many lines and the new names do not fit, `upgrade` makes the rename of 0.6 alone, as 0.7.0 did, gives a warning, and the project adds the names by hand ([ADR 0023](adr/0023-default-skills-from-the-release-table.md)).

## Overrides

A rule changes the severity of one rule group or of one code. Its value is `"off"`, `"warning"` or `"error"`.

```toml
profile = "standard"

[rules]                         # for every root
drift = "error"                 # a rule group
SD704 = "warning"               # one code

[[roots]]
path = "docs/legacy"
profile = "lite"                # the profile of this root
rules = { lint = "off" }        # the rules of this root
```

spec-dna finds the severity of a code for a document in this order. It uses the first match:

1. the `rules` of the root of the document: the code, then its group
2. `[rules]`: the code, then its group
3. the `profile` of the root
4. the `profile` of the project (default `standard`)

A diagnostic of a file outside every root, such as an adapter in `AGENTS.md`, uses steps 2 and 4.

The root keys `lint-severity` and `drift-severity` of 0.1 are the rules `lint` and `drift` of that root. A `rules` table of the same root has priority over them. The key `enforce-budgets = false` still works. With it, every document over its hard budget in the root gives SD704, never SD702.

## Changes from 0.1

0.1 had no profiles. Its behavior was close to `standard`, with 2 differences:

- The requirement lint gave warnings in every root. In `standard`, the lint is off. Set `[rules] lint = "warning"` to keep it.
- 0.1 had no evidence check. In `standard`, an active requirement without evidence gives SD608. Set `[rules] evidence = "off"` to disable it.
