---
status: approved
---
# A tag binds the test under it: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the rule of the bind of a `covers:` tag, adds the code SD621, the option `show --reports` and the field `binds` of `show --json` and `check --reports --json`, and adds the page `docs/test-report.md`. It also adds the note of a test hook to `init` and `upgrade`, and a new text of SD614. The [design of evidence and context](../../design/0.8.1-a-evidence-and-context.md) states the contract in D82, as its amendment of the build changes it, and in D85. The [design of lifecycle and harness](../../design/0.8.1-a-lifecycle-and-harness.md) states it in D89. The page `docs/test-report.md` gives the rule as built. [ADR 0024](../../adr/0024-test-report-as-evidence.md) holds the amendments of 2026-11-24 and 2026-11-25.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0024-test-report-as-evidence]]

## Concerns

- [x] A tag that bound by the rule of 0.8.0 and binds no test now leaves its requirement unproven, so a current corpus with `[evidence] reports` can need edits. Resolved: SD616 is a state code, and SD617 stops only the `finish` of a change that touches the requirement. Each message names the reason and the fix, and `CHANGELOG.md` gives the steps of the upgrade. The owner accepted the amendment of ADR 0024 on 2026-11-24.
- [x] The forms of a test read lines, not a syntax tree. A line that matches no form gives the reason `no-test`. A test whose chain or title the scanner cannot read gives a weak bind (SD621, cause `no-identity`). Resolved: `case:"<name>"` is the escape for both, and the message of each names it. It is no escape for the causes `by-name` and `not-unique`, and their messages do not offer it.
- [x] jest-junit with its defaults writes no `file`, so each bind of such a project is a bind by name. Resolved: SD621 is a warning in `standard`, and its message names `addFileAttribute: "true"`. It is an error in `strict`, as D82 decides.
- [x] The slice table of the design does not list `src/lifecycle.rs` and `src/finish.rs` for SD621. Resolved: SD621 is new where SD617 runs, in the diff that deletes the folder of a change and at step 5.4 of `finish`, so the plan lists both files.
- [x] D92 of the change `lifecycle-fixes` says that its text on `stats` goes into the page of the test reports. Resolved: the rule of `stats` is a detail of one command, so it goes into the section `stats` of `docs/commands.md`, and the new page links that section. Thus neither change waits for the other.
- [x] The changes `test-tag-bind`, `lifecycle-fixes` and `summary-tier` edit some of the same files. Resolved: no contract of one change depends on another, so the change that merges second rebases on the first.
- [x] The skill `tdd` keeps its text, so an agent does not learn the place of the tag from the skill. Resolved: the reasons of SD616 and SD617, and the fix text of both codes, give the place of the tag and `case:`. SD621 gives the fix of its cause. The text of `tdd` waits for a rerun of its evals.
- [x] The build found false proofs in rules of D82 and of its amendment of the build that let one case take the place of another. These rules are the precedence of a fit, the shared set by name, the last segment of a name, and a class name with no read of the `package` or `namespace` line. Resolved: the rule of the identity takes their place. A bind is silent only when each of its cases has the full identity that spec-dna builds from the definition under the tag, and no other definition builds that identity. A case leaves the set only by a full field that differs. Each other case that names the test is a weak link: SD621 reports it, and the worst case decides. The owner accepted the new rule on 2026-11-25, and ADR 0024 holds it in the amendment of 2026-11-25.

## Open questions

- [x] Does the new rule need a transition level, for example a warning for a tag that only the rule of 0.8.0 binds? Answer of the design (D82, "Upgrade"): no. A warning level keeps the false proof. SD616 is a state code, and SD617 stops only the `finish` of a change that touches the requirement.
- [x] Where does the reference of the test reports go? Answer of the design (D85): the new page `docs/test-report.md`. `docs/evidence.md` keeps the results of the agent experiments.
- [x] Does SD621 stay an error in `strict`? Answer of the design (D82, section 10): yes in 0.8.1. A report from a project with the defaults of jest-junit can show later that a warning is the better level.
