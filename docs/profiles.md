# Profiles

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

The other codes keep the severity of [docs/diagnostics.md](diagnostics.md) in every profile. For example, SD405 (the frozen text of an accepted decision changed) is always an error.

Use the profiles in this order:

- `lite`: adoption. Nothing fails `check`, except the structural errors of the model, such as invalid front matter or a changed accepted decision.
- `standard`: daily work. Broken references and ids fail `check`. A document that grows over its hard budget fails `check`. Drift and requirements without evidence give warnings.
- `strict`: a mature corpus. Drift, requirements without evidence and every budget diagnostic fail `check`. The requirement lint gives warnings.

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
