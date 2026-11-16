---
status: draft
---
# Workflow pack of 11 skills: spec

## Delta

None. This change adds, changes or removes no requirement. It adds the text of 11 skills, the discipline block, a longer line of `AGENTS.md`, the command `spec-dna skill` and new values of `[harness] skills`. The rules are in [the design](../../design/0.7.0.md), D2 to D6, D39, D41 and section 15, and the docs of the build state them.

## Interfaces

None. No interface document describes the CLI, the configuration or the skill text. The new surface is this. `spec-dna skill list` prints one line for each of the 11 skills, in the order of D2. A line has the name, the kind (`stage` or `event`) and the stage, or `-` for an event skill. Then it has the state in the project (`on`, `off`, or `-` with no configuration) and the default of D2. `spec-dna skill show <name>` prints the body of the skill, with no front matter and no marker. With `--json`, `list` prints `{"skills":[{"name":"brainstorm","kind":"stage","stage":"plan","state":"on","default":"on"}]}`, with `null` for no stage or no state, and `show` prints `{"name":"write-plan","kind":"stage","stage":"build","body":"..."}`. `[harness] skills` takes the 11 names. The names `intent`, `spec` and `plan` give exit code 3, and the message names the new names and `spec-dna upgrade`.

## Decisions

- [[adr/0010-workflow-pack]]
- [[adr/0011-entry-points]]
- [[adr/0003-distribution]]
- [[adr/0017-profiles]]

## Concerns

- [x] After the rename, `Skill::ALL` no longer holds the 3 old skill paths. `init` deletes these files in the pull request of the pin, and a deleted file counts for SD503. Resolved: the 3 old paths stay in a list of paths that SD503 does not count, as the agent files of 0.5 do. A test of `tests/harness_skills.rs` proves it.
- [x] The line of `AGENTS.md` must name the 5 rules, `spec-dna skill show` and the approval rule in 400 bytes. A draft has 421 bytes with its marker. Resolved: the limit counts the whole line with its marker, because each request carries it. If the build cannot fit the text, it stops and asks `architect`.
- [x] Slice 3b (`hours-and-links`) adds the line `Hours: <total>`, and it can merge after slice 4. Resolved: the skill `finish` says "if `spec-dna finish` prints a line `Hours: <total>`, put it into the commit message, and keep it in the squash message". This text is true before and after slice 3b.
- [x] An old name in `[harness] skills` stops each command with exit code 3. Resolved: `upgrade` renames the names in the text of `spec-dna.toml` before it parses the file. The tests that use an old name change to the new names.
- [x] The listing of the 7 skills that are on has 1,250 bytes or fewer, with the prefix "In a spec-dna project" in each description. This gives about 155 bytes for each description. Resolved: a test fixes the 2 listings, and the build keeps each description to one sentence.
- [x] The rule of Q5 is a rule of this repository, but the skills ship to each project. Resolved: the skill text states it for any project. "Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session." The text names no person and no repository.

## Open questions

- [x] Where does `skill list` get the state of a skill? Answer: from `[harness] skills` of `spec-dna.toml`, if the command finds one. With no configuration, the state is `-` in the text and `null` in `--json`. A configuration error gives exit code 3. A pin of another version gives the warning of `next` on stderr.
- [x] Which skills does `init` write before slice 8? Answer: at the level `lean`, the 7 skills that D2 sets on. At the level `full`, also `execute`, `tdd`, `verify` and `debug` (D39). Slice 8 makes the list depend on the profile.
- [x] Does `skill show` take the adapter skill `spec-dna`? Answer: no. It takes the 11 names only, and each other name gives exit code 2.
- [x] Does the discipline block count in the limit of 2,500 bytes of a body? Answer: no. That limit applies to the 11 skills. The block has its own limit of 700 bytes, and the adapter skill keeps its own size.
- [x] Which skill holds the words "a spec-dna skill replaces a general skill of the same stage" (D5)? Answer: the adapter skill `spec-dna`, after the discipline block.
