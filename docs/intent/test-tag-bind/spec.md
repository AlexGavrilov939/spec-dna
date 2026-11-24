---
status: draft
---
# A tag binds the test under it: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It changes the rule of the bind of a `covers:` tag, adds the code SD621, the option `show --reports` and the field `binds` of `show --json` and `check --reports --json`, and adds the page `docs/test-report.md`. It also adds the note of a test hook to `init` and `upgrade`, and a new text of SD614. The [design of evidence and context](../../design/0.8.1-a-evidence-and-context.md) states the contract in D82 and D85, and the [design of lifecycle and harness](../../design/0.8.1-a-lifecycle-and-harness.md) in D89. [ADR 0024](../../adr/0024-test-report-as-evidence.md) holds the amendment of 2026-11-24.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0024-test-report-as-evidence]]

## Concerns

- [x] A tag that bound by the rule of 0.8.0 and binds no test now leaves its requirement unproven, so a current corpus with `[evidence] reports` can need edits. Resolved: SD616 is a state code, and SD617 stops only the `finish` of a change that touches the requirement. Each message names the reason and the fix, and `CHANGELOG.md` gives the steps of the upgrade. The owner accepted the amendment of ADR 0024 on 2026-11-24.
- [x] The forms of a test read lines, not a syntax tree, so an unusual layout gives the reason `no-test`. Resolved: `case:"<name>"` is the escape, and each message names it.
- [x] jest-junit with its defaults writes no `file`, so each bind of such a project is a bind by name. Resolved: SD621 is a warning in `standard`, and its message names `addFileAttribute: "true"`. It is an error in `strict`, as D82 decides.
- [x] The slice table of the design does not list `src/lifecycle.rs` and `src/finish.rs` for SD621. Resolved: SD621 is new where SD617 runs, in the diff that deletes the folder of a change and at step 5.4 of `finish`, so the plan lists both files.
- [x] D92 of the change `lifecycle-fixes` says that its text on `stats` goes into the page of the test reports. Resolved: the rule of `stats` is a detail of one command, so it goes into the section `stats` of `docs/commands.md`, and the new page links that section. Thus neither change waits for the other.
- [x] The changes `test-tag-bind`, `lifecycle-fixes` and `summary-tier` edit some of the same files. Resolved: no contract of one change depends on another, so the change that merges second rebases on the first.
- [x] The skill `tdd` keeps its text, so an agent does not learn the place of the tag from the skill. Resolved: the messages of SD616, SD617 and SD621 give the place and `case:`. The text of `tdd` waits for a rerun of its evals.

## Open questions

- [x] Does the new rule need a transition level, for example a warning for a tag that only the rule of 0.8.0 binds? Answer of the design (D82, "Upgrade"): no. A warning level keeps the false proof. SD616 is a state code, and SD617 stops only the `finish` of a change that touches the requirement.
- [x] Where does the reference of the test reports go? Answer of the design (D85): the new page `docs/test-report.md`. `docs/evidence.md` keeps the results of the agent experiments.
- [x] Does SD621 stay an error in `strict`? Answer of the design (D82, section 10): yes in 0.8.1. A report from a project with the defaults of jest-junit can show later that a warning is the better level.
