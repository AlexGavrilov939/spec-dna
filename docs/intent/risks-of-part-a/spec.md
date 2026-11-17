---
status: approved
---
# Risks of part A: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the rule and the message of SD910, the glob rule of SD923, the pin compare and the tree binary of SD503, and the git access of the walk. Section 15 of [the design](../../design/0.7.0.md) and the docs of the build state the rules.

## Interfaces

- None: no interface document describes `check` or the git access.

## Decisions

- [[adr/0002-language]]
- [[adr/0012-approval-in-the-branch]]
- [[adr/0013-self-hosting]]

## Concerns

- [x] Reason 9 changes the text of SD910, and D17 names 8 reasons. Resolved: ADR 0019 states 9 reasons. ADR 0012 is at its hard budget, so a new ADR holds the rule. The code, its severity and the JSON fields stay.
- [x] The head rule makes `check` stricter for a branch that sets an approved artifact back. Resolved: the same net diff already fails with `approval = "base"` (SD910, reason 1), so the branch rule no longer passes where the base rule fails. A new approval commit clears the work after it.
- [x] A batched walk could read a path as a commit header. Resolved: the parse uses `-z`. The 2 tests of forged headers in `tests/branch_approval.rs` stay green, and a unit test compares the batched paths with the paths of each commit.
- [x] The CRLF rule could hide a real change of `spec-dna.toml`. Resolved: TOML reads `\r\n` as `\n`, so the meaning is the same. A change of the line ends alone has no pin change, so it still counts.
- [x] A symlinked `Cargo.toml` is no tree binary. Resolved: the checkout of spec-dna has a real file. Each mode gives the same answer, and SD503 counts the pin.

## Open questions

- [x] What is the target of the batched walk? Answer: on the fixture of 500 commits, `check` with `approval = "branch"` takes at most 2 times the time with `approval = "base"`, on the same machine. Before the change it is 5.7 times with a direct git, and 11.9 times with the macOS shim `/usr/bin/git`.
- [x] Does SD913 change for a plan that is `draft` at the head? Answer: no. SD913 reads the `Files` of the plan at the head, whatever its status.
- [x] Does an approval from the base tree get reason 9? Answer: no. SD401 reports an `approved` plan that goes back to `draft`, as in 0.6.
- [x] Does the head rule apply to an artifact that the head does not have, such as a folder that `finish` deleted? Answer: no. A deletion takes no approval away, as in the walk of part A. The rule reads only an artifact that exists at the head.
