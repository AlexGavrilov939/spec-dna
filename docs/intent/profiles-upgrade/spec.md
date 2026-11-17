---
status: approved
---
# Profiles at init and the upgrade from 0.6: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the defaults that `init` writes, the field `default` of `skill list`, the rule of SD507 and the names that `upgrade` writes. [ADR 0023](../../adr/0023-default-skills-from-the-release-table.md) and section 15 of [the design](../../design/0.7.0.md) state the rules, and the docs of the build state them.

## Interfaces

- None: no interface document describes `init`, `upgrade`, `skill list` or `check`.

## Decisions

- [[adr/0009-upgrade-from-0-5]]
- [[adr/0016-skill-evals]]
- [[adr/0017-profiles]]

## Concerns

- [x] With the constant `true` and the table of the pilot, `init` proposes only `tdd` for `standard` and `strict`, and no skill for `lite`. Resolved: no release has this state. The change `release-070` sets the constant and the table after the full run, and `next` and `skill show` still give the text of each skill.
- [x] A test that names the default skills changes with each table. Resolved: each such test reads the default from `skill list --json` or from a fake table, or it pins `harness-skills`. The proof runs `cargo test` with the constant `true` and with `false`.
- [x] `upgrade` can leave out a renamed skill, and D41 says that a project keeps its skills. Resolved: ADR 0023 amends D41. The report names each left-out name and the line to add.
- [x] SD507 changes for a skill of D2. Resolved: ADR 0023 states it. SD507 is off in `lite` and `standard`, and a warning in `strict`.
- [x] `strict` gets no eval pack, and ADR 0017 names one. Resolved: the pack has 812 files (3.2 MB). ADR 0023 replaces it with a note.
- [x] The fixture needs the 0.6.1 binary, and CI has none. Resolved: `tests/fixtures/upgrade/make.sh` records the files and the result of `check`, as for 0.4.2 and 0.5.0. The tests run no old binary.
- [x] Until the version is 0.7.0, the pin of the fixture equals the version of the binary, so the pin does not move. Resolved: one case sets the pin to 0.6.0. The change `release-070` runs the same tests after the version change.
- [x] ADR 0023 is `proposed`, and SD920 stops `finish` for a needed decision that is not `accepted`. Resolved: the spec links ADR 0023 in its text and not in `Decisions`, as `approver-safety` does for ADR 0018. The owner accepts ADR 0023 before the tag.

## Open questions

- [x] Does `standard` get a skill outside D2 with the verdict `on`? Answer: yes. A verdict `on` shows a gain, and `strict` and `standard` differ in the approval and the severities, not in the skills (ADR 0023).
- [x] Does the level `full` follow the verdicts? Answer: no. `full` proposes the 11 skills, as D39 says. A person chooses it.
- [x] Does `--reconfigure` apply the preset of a new profile? Answer: no. The current values stay. A part that the file does not set gets the default of its profile.
- [x] Does `upgrade` remove a valid name such as `review` when its default is off? Answer: no. `upgrade` changes only the names that ended.
- [x] Which value of the constant ships? Answer: the change `release-070` sets it from the word of the owner after the full run. Until then it is `true`.
