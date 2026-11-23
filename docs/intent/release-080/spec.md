---
status: draft
---
# Release of 0.8.0: the last owner answers and the tag: spec

## Delta

None. This change adds, changes or removes no requirement. It adds one key (`[harness.roles] acceptor.advisor`), changes the text of the generated file `acceptor.md` in `standard`, and changes the table of the release. The [design](../../design/0.8.0.md) states the contract in D81, and [ADR 0032](../../adr/0032-acceptance-in-one-pass.md) holds the amendment.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0032-acceptance-in-one-pass]]

## Concerns

- [x] The key sets a line of text. Claude Code has no switch of the advisor for one subagent, so the line is a request. Resolved: the docs and the notes of the release say so, and the costs in them come from whole sessions.
- [x] The default of `standard` changes the hash of `acceptor.md` that the freeze names. Resolved: no eval runs the agent as a subagent, so the evals need no rerun, and the freeze section names the exception.
- [x] The key must survive `init --reconfigure` and `upgrade`. Resolved: the plan has a test for each case (t2 and t3).

## Open questions

- [x] Does the acceptance pass run with no advisor by default? Answer of the owner (Q54): yes in `standard`, no in `strict`.
- [x] Do `write-plan` and `finish` get a rerun to the floor? Answer of the owner (Q55): yes, up to 15 USD.
