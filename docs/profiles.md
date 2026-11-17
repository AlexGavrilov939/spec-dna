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
| `lint` | SD801 to SD805 (BCP 14 keyword, EARS pattern, sentence length, vague terms, passive voice) | off | off | warning |
| `layers` | SD901 (link direction), SD902 (layer limit), SD903 (history in requirements), SD904 (forbidden term) | off | warning | warning |
| `changes` | SD905 (fan-out), SD911 (stale change), SD913 (outside the plan), SD918 (incident with no eval) | off | warning | warning |
| `changes` | SD912 (started limit), SD917 (class too low), SD921 (truth changed without a change), SD923 (task incomplete), SD924 (task too large), SD925 (checkbox missing) | off | warning | error |
| `changes` | SD910 (gate approval missing), SD914 (delta invalid), SD915 (finish incomplete), SD916 (delta evidence missing), SD919 (concern open), SD920 (decision needed, not accepted), SD922 (question open) | warning | error | error |

The other codes keep the severity of [docs/diagnostics.md](diagnostics.md) in every profile. For example, SD405 (the locked text of an accepted decision changed) is always an error. SD506 (an eval file that is not valid, or an absent `check.sh`) is in no group, so it is an error in every profile. Only a rule for SD506 in `[rules]` can change it. SD612 (a link into the folder of a change) is also in no group: it is a warning in each profile, and a rule for SD612 changes it.

The rule groups `layers` and `changes` are new in 0.6.0. The word `rules` still names the severity table of the file. A layer named `rules` is not that table: `[rules]` is the severity table, and `[layers.rules]` is the layer.

## The change codes

The codes of the group `changes` depend on the `[change]` table:

- Without a `[change]` table, the policy is `none`. With `policy = "none"`, each code of the group is off, and no rule enables it.
- SD918 also needs the table `[evals]`.
- SD910 follows `[change] gate` when the key is present. `block` makes it an error, `warn` makes it a warning, and `off` disables it. Then the profile column does not apply. If `gate` is absent, SD910 takes the severity of its profile column. When `init` writes the `[change]` table, it also writes `gate`.
- SD910 reads the approvals of the base tree. With `[change] approval = "branch"`, it also reads the approval commits of the branch ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)). `strict` reads the approvals of the base only, so `approval = "branch"` with `profile = "strict"` is an error (exit code 3). `init` writes `approval = "branch"` for `lite` and `standard`, and `approval = "base"` for `strict`.
- A rule for SD910 in `[rules]` has priority over the gate. `[rules]` can also override SD921, as it overrides each code.

SD923, SD924 and SD925 are new in 0.7.0. SD923 and SD924 read the tasks of an `approved` plan only, so a draft plan of 0.6.1 gives no new diagnostic. In `standard` they are warnings, and a warning never fails `check`. In `strict` they are errors, and the gate reports only a new error. SD923 and SD924 read one tree, as SD925 does.

These codes compare with the lifecycle base: SD905 for a change with no delta, SD910, SD913, SD915, SD916, SD917 and SD918. SD920 for a decision that is not `accepted` and SD921 do the same. This base is the merge base of `--base`, of `origin/HEAD` or of the branch of `[change] base`, with `HEAD`. These codes do not run if git cannot name it.

SD911 reads the base and the history. It is silent without a base. SD911 is a state code: it is never new, and `check` shows it only with `--all`.

SD914 has one exception to the profile column. `check` lowers an error of SD914 to a warning when the diff does not touch the spec. It does the same when git cannot name the lifecycle base.

SD912 is on `spec-dna.toml`, so the profile and the rules of a root do not apply to it, as for SD902 ([diagnostics.md](diagnostics.md#checks-of-the-change-lifecycle)).

Choose a profile for the stage of the project:

- `lite`: adoption. Nothing fails `check`, except the structural errors of the model, such as invalid front matter, a changed accepted decision or an invalid eval.
- `standard`: daily work. Broken references and ids fail `check`. A document that grows over its hard budget fails `check`. A break of the gate or of the finish rules of a change fails `check`. Drift and requirements without evidence give warnings.
- `strict`: a mature corpus. Drift, requirements without evidence and every budget diagnostic fail `check`. Too many started changes, a class that is too low and a truth that changed without a change fail `check` too. The requirement lint gives warnings.

In `standard`, a mature corpus that links no tests yet gets one SD608 for each active requirement. `check` prints more than 10 warnings of one code as one count. The gate against the base reports only the new warnings. To disable them until the project links tests, set `[rules] evidence = "off"`.

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
