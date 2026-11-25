---
status: draft
---
# Each subagent gets a card and ends with a state, an agent stops after 3 failed fixes, and a change lists its lessons: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It adds 3 hooks (`subagent-card`, `return-state` and `fix-limit`), 2 keys of `[harness]` (`subagent-card` and `fix-limit`), the entries of the hooks in `.claude/settings.json`, a section of the team page, a new text of the skill `execute` and one note of `finish`. Items 18 to 20 of the [design](../../design/0.8.1-harness-additions.md) state the contract, and [ADR 0004](../../adr/0004-harness.md) holds the amendment of 2026-11-25. The plan gives each agent-facing text in full, with its bytes.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0004-harness]]
- [[adr/0017-profiles]]
- [[adr/0032-acceptance-in-one-pass]]

## Concerns

- [x] The room of the skill `execute`. E12 of the design gives a body of 2,174 bytes and 326 bytes of room. The body now has 2,263 bytes, because the change `lifecycle-fixes` added a sentence, so the room is 237 bytes. Resolved: the plan gives a body of 2,485 bytes. The brief template is the 5 labels of a brief, and the actions of 3 states are one step. The sentence on the hook `protect-tests` leaves the part `Check:`, because the hook gives the person its own reason. The test of 2,500 bytes fixes the limit.
- [x] The merge of `.claude/settings.json` finds a handler by its event and its name (`apply_want` and `satisfied` in `src/harness.rs`), and writes the matcher only for a new group. 2 entries of `fix-limit` on `PostToolUse` thus become one. Resolved: `fix-limit` has one `PostToolUse` entry with the matcher `Bash|Edit|Write`, and the hook reads `tool_name`. So it has 2 entries, not the 3 of the design. The behavior is the same.
- [x] `lite`. Design steps 10 and 7 of items 18 and 19 make `init` write `subagent-card = false` and `fix-limit = 0` for a new `lite` configuration. Resolved: the plan follows the design, so `spec-dna.toml` of a new `lite` project has 2 more lines. Each other generated file of `lite` keeps the bytes of 0.8.0: the settings, the agent files, the skills, the team page, the adapter skill and the lines of `CLAUDE.md` and `AGENTS.md`. A golden test proves it. A key that a person sets to `true` in `lite` writes the hooks, because the key decides, not the profile ([ADR 0017](../../adr/0017-profiles.md)).
- [x] A current configuration. Resolved: `init` writes the 2 keys only in a new configuration. `upgrade`, `init` on a current file and `init --reconfigure` keep the keys of the file, and a file with no key has the effect of off. So a project of 0.8.0 gets no new hook and no new finding of `check`. `Harness::toml_section` writes a key only if the file has it, as for `protect-approvals`, so a file of 0.8.0 keeps its bytes.
- [x] The reason of `return-state`. The design gives "End your report with one line: `State: <state>`." Resolved: the reason names the 4 values, so a subagent that wrote another word gets the right words in its one more request.
- [x] The card of the design has the reference "(item 20)" and no form of a lesson. Resolved: the card has no reference to the design, and line 5 gives the form of a lesson line. The card has 788 bytes, and 791 bytes with the line of the role `acceptor`.
- [x] The matcher of `SubagentStart`. The design gives `.+` only for `SubagentStop`. Resolved: both entries have `.+`, so a subagent with an empty `agent_type` gets no card and no block, and both hooks also give no output for an empty `agent_type`.
- [x] The facts that the build confirms on a real run (section 2 of the design): a shell command with an exit code other than 0 gives `PostToolUseFailure` and no `PostToolUse` (E3), and the input of `SubagentStop` for an internal agent of a session that starts with `--agent <name>` (E1). Resolved: tasks t1a and t1b confirm them before the code of the hooks, with counts and key names only. A live session costs less than 1 USD, and the owner decides who runs it. For each other result, each task names the change, inside the files of the plan.
- [x] The note of `finish` follows the profile: it prints in `standard` and `strict`, and not in `lite`. Resolved: the amendment of ADR 0004 of 2026-11-25 records this effect of the profile. The note prints only after a success that writes, not with `--dry-run`, so it prints once for each change. It goes to stderr in the text output, as the other notes, so the output of `--json` keeps its bytes.
- [x] The frozen texts: `specify`, `finish`, `write-plan`, `review` and `acceptor.md` (`docs/evals.md`, section "The freeze of the skill texts for the reruns"). Resolved: no task edits them. The card reaches the agent `acceptor` as context, not in its file, and adds nothing to the output of `accept --brief`. The pack evals start no subagent, and the harness arms of `eval run` keep both keys off. The golden files of the frozen texts stay as they are (guard).
- [x] The text of `execute` changes, and `execute` is not in the freeze table. Resolved: the paid reruns of `execute` and `review` in this part start after the merge of this change, so they measure the new text.

## Open questions

- [x] Q1 to Q9 of the design. Answer of the owner (word of 2026-11-25, the recommended option of each): Q1 (b), the hook `subagent-card` for each subagent. Q2 (b), the hook `return-state`. Q3 (a), the skill `execute`, and the full form in `docs/harness.md`. Q4 (b), a hook with a state file. Q5 (a), the same command. Q6 (a), the message only. Q7 (a), nowhere in the repository. Q8 (a), a note of `finish`. Q9 (a), no hook.
