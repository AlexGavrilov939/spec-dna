---
status: approved
---
# Each subagent gets a card and ends with a state, an agent stops after 3 failed fixes, and a change lists its lessons: plan

## Files

- `docs/adr/0004-harness.md` The amendment of 2026-11-25, in the pull request of the plans.
- `src/harness.rs` The 2 keys, the 4 settings entries, the version of the hooks, the section of the team page, and the function check for the entries.
- `src/config.rs` The keys of [harness] in the test of the schema.
- `schema/spec-dna.schema.json` The 2 keys.
- `src/hook.rs` The 3 hooks.
- `src/harness_text.rs` The card and the new text of the skill execute.
- `src/init.rs` The preset of the 2 keys in a new configuration.
- `src/finish.rs` The text of the note on the lessons.
- `src/main.rs` The print of the note in the report of finish, and the help of hook.
- `tests/hook.rs` The tests of the 3 hooks.
- `tests/harness.rs` The tests of the settings entries.
- `tests/harness_roles.rs` The test of the section of the team page.
- `tests/harness_check_step4.rs` The findings of check for the entries.
- `tests/harness_skills.rs` The step 2 of execute in the test of the named texts.
- `tests/init_profiles.rs` The preset of each profile, and the guard of lite.
- `tests/eval_run.rs` The guard that the harness arms write no key and no entry.
- `tests/finish.rs` The tests of the note.
- `tests/cli.rs` The hook note of init on a terminal.
- `tests/upgrade_cli.rs` The 2 keys in the list of keys that upgrade does not write.
- `tests/golden/harness/settings-additions.json` The settings with the 3 hooks (new).
- `tests/golden/harness/model-team-additions.md` The team page with the card (new).
- `tests/golden/harness/settings-lite.json` The settings of a new project of lite, with the bytes of 0.8.0 (new).
- `tests/golden/harness/settings-standard.json` The settings of a new project of standard, with the entries.
- `tests/golden/harness/settings-strict.json` The settings of a new project of strict, with the entries.
- `tests/golden/harness/model-team-no-advisor.md` The team page of a new project of standard, with the card.
- `tests/golden/harness/model-team.md` The team page of a new project of the level full, with the card.
- `tests/golden/harness/skills/spec-dna-execute.md` The new text of execute.
- `docs/config.md` The 2 keys.
- `docs/commands.md` The hook section and the note of finish.
- `docs/harness.md` The keys, the entries, the 3 hooks, the card, the brief, the states, the lessons, and what each cannot prove.
- `docs/init.md` The preset of the 2 keys and the version of the hook note.
- `docs/profiles.md` The rows of the 2 keys.
- `docs/evals.md` The harness arms write no key of the 3 hooks.
- `docs/guides/change.md` The lessons at the end of a change.
- `docs/glossary.md` The terms subagent card, task brief, return state, lesson and fix limit.
- `CHANGELOG.md` The entry of the change.
- `docs/design/0.8.1-harness-additions.md` The deviations of the build, because finish deletes this plan.

## Change sets

The change sets of `.claude/agents/ownership.md` that apply. `architect` is A of each, except where the row names the owner.

- Information model: the amendment of ADR 0004 of 2026-11-25. The owner accepted it by the word of 2026-11-25.
- Hook: `src/hook.rs`, the `hook` section of `docs/commands.md`, the hooks section of `docs/harness.md` and `tests/hook.rs`. R: `init-engineer`. C: `test-engineer`.
- Harness role or generated file: `src/harness.rs`, `src/harness_text.rs`, `docs/harness.md`, `tests/golden/harness/`, the harness tests and `CHANGELOG.md`. R: `init-engineer`, and `diagnostics-engineer` for the function `check`.
- Config key: `src/config.rs`, `docs/config.md`, `schema/spec-dna.schema.json` and `CHANGELOG.md`. R: `core-engineer` and `init-engineer`.
- Command, option, help text or exit code: `src/main.rs`, the `hook` and `finish` sections of `docs/commands.md`, and `CHANGELOG.md`. R: `core-engineer` for the help, `hygiene-engineer` for `finish`.
- Eval file form or runner: `tests/eval_run.rs` and `docs/evals.md`, for the guard of the harness arms. R: `core-engineer`.

## Texts

Each text below is final. A task that writes it uses these bytes. A test fixes each limit.

The card, `SUBAGENT_CARD` in `src/harness_text.rs`: 788 bytes, with no final line feed. The limit is 800 bytes.

```text
Subagent card. Your agent file and your brief have priority over this card.
1. Do only the task of the brief. Change only the files of `Files`. If the task needs more, stop and report it.
2. Read the brief, then the files that it names. Run `spec-dna context <file>` before an edit. Search only for a fact that these do not give.
3. Do not commit, push or change the git state.
4. Report the result, the changed files, each command that you ran with its result, and the open issues.
5. For a rule that the project files do not state, add `Lessons:` before the last line, with one line for each: `- Target: <file>. Text: <sentence>. Evidence: <what happened>.`
6. End the report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.
```

For `agent_type = "acceptor"`, line 3 is this line, and the card then has 791 bytes:

```text
3. Commit only `acceptance.toml`, and do not push.
```

The reason of `return-state`: 149 bytes.

```text
spec-dna return state: end your report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.
```

A state line is the last line of the report that is not empty, after the removal of white space and of `*` and `` ` `` at its 2 ends. It is the word `State:`, one space and one of the 4 values, with this case.

The message of `fix-limit`, with `{n}` from `--limit`: 249 bytes for 3.

```text
spec-dna fix limit: this command failed after {n} fixes. Stop the fixes of this problem. Tell the person the command, the cause that each fix assumed, the error now, and the next step that you propose. A subagent ends its report with `State: BLOCKED`.
```

The note of `finish`, on stderr: 162 bytes.

```text
note: list the lessons of this change for the person: the target file, the text and the evidence of each; edit no target file before the person accepts the lesson
```

The skill `execute` of `standard` and `strict`: 2,485 bytes, from 2,263. The limit is 2,500 bytes. 3 edits of `EXECUTE_PARTS`, and no other edit:

1. In step 2, the first text below becomes the second text:

    ```text
    Give the subagent the task and
    Give it a brief with the lines `Task:`, `Files:`, `Out of scope:`, `Read first:` and `Done when:`, and
    ```

2. A new step 3, and the steps 3 to 5 become 4 to 6:

    ```text
    3. If the report ends with `State: <state>`, act on it. `COMPLETE_WITH_CONCERNS`: read each concern first. `NEEDS_CONTEXT`: give the fact and start again. `BLOCKED`: remove the cause or ask the person. Do not send the same brief again.
    ```

3. The part `Check:` loses its last sentence:

    ```text
    The hook `protect-tests`, if it is on, asks the person before an edit of a test file.
    ```

The team page with `subagent-card = true` gets this section after the section "Review policy", and before the section "Hooks". `{card}` is the card above.

````md
## Subagent card

The hook `subagent-card` gives each subagent this card when it starts. The agent file and the brief of the subagent have priority over it. The agent `acceptor` gets another line 3: commit only `acceptance.toml`, and do not push.

```text
{card}
```
````

The section "Hooks" of the team page gets these sentences, after the sentence of `protect-approvals`. The version in its last sentence is 0.8.1 with either key on.

- With `subagent-card = true`: `` `subagent-card` gives each subagent the card above when it starts. `return-state` asks a subagent once for the last line `State: <state>` when its report has none. ``
- With `fix-limit` above 0: `` `fix-limit` tells an agent to stop after {n} failed fixes of one shell command. ``

The settings entries. Each command ends with `2>/dev/null || true`.

| Event | Matcher | Command | Key |
| --- | --- | --- | --- |
| `SubagentStart` | `.+` | `spec-dna hook subagent-card` | `subagent-card = true` |
| `SubagentStop` | `.+` | `spec-dna hook return-state` | `subagent-card = true` |
| `PostToolUseFailure` | `Bash` | `spec-dna hook fix-limit --limit <n>`, and `--intent '<dir>'` for each folder of the `intent` layer | `fix-limit` above 0 |
| `PostToolUse` | `Bash\|Edit\|Write` | The same command | `fix-limit` above 0 |

## Order

- [ ] t1a: Confirm in the lab, on the current Claude Code and before t6, that a `Bash` call with an exit code other than 0 gives `PostToolUseFailure` and no `PostToolUse` (E3, section 2 of the design). Agent: `init-engineer`. The method: a scratch repository in the lab, outside this repository, with one logging hook on each of the 4 events `PostToolUse`, `PostToolUseFailure`, `SubagentStart` and `SubagentStop`. The hook writes only the event name, `tool_name`, the value of `agent_type`, the key names of the input, the length of `last_assistant_message` and the first word of `error`. No hook writes a prompt, a command, a message text or a path. Each session runs with `--model haiku` and costs less than 1 USD. Before the run, the owner decides if an agent runs it as a paid run of this part, or if the person runs it in a terminal of their own. Put the counts and the key names in the pull request of the code, with no text of a transcript. The session runs `false`, then `true`. The other result: if a failed call also gives `PostToolUse`, record the key of its input that marks the failure. Then t6 deletes a count only for a `PostToolUse` with no such mark. If no key marks it, stop and tell `architect` before t6. Files: none. Verify: the note of the lab gives the count of each event for each of the 2 calls, and `git status --porcelain` lists no file of the lab. Hours: 1.
- [ ] t1b: Confirm in the lab, before t4 and t5, the input of `SubagentStart` (`agent_type` and `agent_id`) and of `SubagentStop` (`agent_type`, `agent_transcript_path`, `last_assistant_message` and `stop_hook_active`, E1), and the `agent_type` of each internal agent of a session that starts with `claude -p --agent <name>` (section 2 of the design). Agent: `init-engineer`. With the same hooks as t1a, one session starts a named subagent and the built-in `general-purpose`, and one session starts with `--agent <name>`. The other results: if a key has another name, t4 and t5 use that name. If an internal agent has an empty `agent_type`, the matcher `.+` skips it, as designed. If it has the name of the session, t19 states that it gets the card and one block. Files: none. Verify: the note of the lab gives the key names of each event and each `agent_type` with its count, and `git status --porcelain` lists no file of the lab. Hours: 1.
- [x] t2: Read the keys `subagent-card` (`true` or `false`) and `fix-limit` (an integer from 0) in `Harness::parse`, as `Option` values. Write each key in `toml_section` only when the file has it. Add `card_hook_on` (the key is `true`) and `fix_hook_on` (above 0), both false at the level `none`. The unit tests fail before the code: each key parses, a wrong type gives an error that names the key, and a table with both keys reads back equal. A table of 0.8.0 renders the same bytes (guard). Agent: `init-engineer`. Files: `src/harness.rs`. Verify: `cargo test --bin spec-dna harness::`. Hours: 2.
- [x] t3: Add the 2 keys to the schema, with a description that says what each key writes and that a file with no key has the effect of off. Add them to the list of `[harness]` keys in the test of the schema, which fails before the schema edit. Document both keys in the table `[harness]` and in its example. Agent: `core-engineer`. Files: `schema/spec-dna.schema.json`, `src/config.rs`, `docs/config.md`. Verify: `cargo test --bin spec-dna config::`. Hours: 1.
- [x] t4: Add `spec-dna hook subagent-card`. With a non-empty `agent_type`, it prints `hookSpecificOutput` with `hookEventName: "SubagentStart"` and the card of the section "Texts" as `additionalContext`, with the line 3 of the acceptor for `agent_type = "acceptor"`. With an empty or absent `agent_type`, bad JSON or another event, it prints nothing. Put the card in `src/harness_text.rs` with a test of its 2 sizes (800 bytes or fewer, ASCII, no `{{`). The tests in `tests/hook.rs` fail before the code. Agent: `init-engineer`. Files: `src/hook.rs`, `src/harness_text.rs`, `tests/hook.rs`. Verify: `cargo test --test hook` and `cargo test --bin spec-dna harness_text::`. Hours: 2.
- [x] t5: Add `spec-dna hook return-state`. It reads the last line of `last_assistant_message` that is not empty. After a call of the tool `SubagentHandback`, it reads the `message` of the last such call in `agent_transcript_path`, from the end of the file, at most 4 MiB, as the context meter reads. If that line is no state line and `stop_hook_active` is false, it prints `{"decision":"block","reason":"<the reason of the section Texts>"}`. With a state line, with `stop_hook_active` true, with an empty or absent `agent_type`, or with bad input, it prints nothing. It stores nothing. The tests fail before the code: the 4 values pass, a line in `**` or backticks passes, a report with no state line gives the block, the same input with `stop_hook_active` true gives nothing, an empty `agent_type` gives nothing, and a synthetic transcript that the test writes with a `SubagentHandback` call uses its `message`. Agent: `init-engineer`. Files: `src/hook.rs`, `tests/hook.rs`. Verify: `cargo test --test hook`. Hours: 3.
- [x] t6: Add `spec-dna hook fix-limit --limit <n> --intent <dir>...`. Only `Bash`, `Edit` and `Write` count. The key is the session id, the agent (`main`, else `agent_id`) and the first 8 bytes of SHA-256 of the session id and the command, with each run of white space as one space, in hexadecimal. On `PostToolUseFailure` of `Bash`: the first failure makes a count of 0. A later failure adds 1 if the agent has an edit mark, and each failure removes the mark. At the limit, the hook prints the message of the section "Texts" once for each key, as `additionalContext` of `PostToolUseFailure`. On `PostToolUse` of `Bash`: delete the count of the command. On `PostToolUse` of `Edit` or `Write`: set the edit mark of the agent, except for a `file_path` below an `--intent` folder of the root (`CLAUDE_PROJECT_DIR`, else `cwd`). The state is one file for each key and one for each mark in `<temporary directory>/spec-dna-fix/`, with numbers only, and the hook removes a file that is older than 3 days. `--limit 0` or no `--limit` does nothing. The tests fail before the code: the sequence failure, edit, failure, edit, failure, edit, failure gives the message at the fourth failure and only once. Failures with no edit between give no message. A pass deletes the count. 2 agents have 2 counts. An edit below an `--intent` folder sets no mark. No state file holds a byte of the command. Agent: `init-engineer`. Files: `src/hook.rs`, `tests/hook.rs`. Verify: `cargo test --test hook`. Hours: 4.
- [ ] t7: Write the 4 settings entries of the section "Texts" in `wants`, and give `hooks_version` the value 0.8.1 when either key is on. The tests fail before the code: a configuration with both keys writes the 4 entries and equals the new golden file, a second `init` changes nothing, the key off removes the entries with `--reconfigure`, and a configuration with no key writes no entry. Agent: `init-engineer`. Files: `src/harness.rs`, `tests/harness.rs`, `tests/golden/harness/settings-additions.json`. Verify: `cargo test --test harness` and `cargo test --bin spec-dna harness::`. Hours: 3.
- [ ] t8: Write the section "Subagent card" and the sentences of the section "Hooks" of the team page, as the section "Texts" gives them. The test fails before the code: a configuration with both keys gives the new golden file, and the golden file passes the lint. The page with no key keeps its bytes (guard: `model-team.md`, `model-team-0.7.1.md`, and `model-team-no-advisor.md` until t11). Agent: `init-engineer`. Files: `src/harness.rs`, `tests/harness_roles.rs`, `tests/golden/harness/model-team-additions.md`. Verify: `cargo test --test harness_roles`. Hours: 2.
- [ ] t9: Make `check` report each of the 4 entries as the other hook entries: SD504 for an absent entry, SD502 for a changed command or a second handler of the name with another command, and nothing for an entry of a key that is off. Document the entries in the section "Check". Where `check` already sees an entry through `wants`, its test is a guard, and the function keeps its code. Agent: `diagnostics-engineer`. Files: `src/harness.rs`, `tests/harness_check_step4.rs`, `docs/harness.md`. Verify: `cargo test --test harness_check_step4`. Hours: 2.
- [ ] t10: Write the guard of `lite`: a new project of `lite` at the levels `lean` and `full` writes the settings of the new golden file `settings-lite.json` (the bytes of 0.8.0), the agent files, the skills and the team page of 0.7.1, and the lines of 0.8.0 in `CLAUDE.md` and `AGENTS.md`. Write the golden file from the output of the binary before t13. The test reads no line of `spec-dna.toml`, so it passes before and after t13. Agent: `init-engineer`. Files: `tests/init_profiles.rs`, `tests/golden/harness/settings-lite.json`. Verify: `cargo test --test init_profiles`. Hours: 2.
- [ ] t11: Write the expected golden files of a new project of `standard` and `strict` before the preset: the settings with the 4 entries, and the team page with the card. This is a red step: the tests of a new project in `tests/init_profiles.rs` and `tests/harness.rs` fail until t13. Make the commit as the skill `tdd` says for a red test. Agent: `init-engineer`. Files: `tests/golden/harness/settings-standard.json`, `tests/golden/harness/settings-strict.json`, `tests/golden/harness/model-team-no-advisor.md`. Verify: `cargo test --test init_profiles --test harness` fails only in the tests that read these 3 files. Hours: 1.
- [ ] t12: Write the other expectations of a new project before the preset, as a red step that ends in t13. The golden file `model-team.md` of the project of level `full` gets the card. In `tests/harness.rs`, the dry run counts 7 hooks, the hook note names 0.8.1, and `standard_init_writes_no_approval_hook` looks for the handler `protect-approvals`, not for the matcher `Bash`, which `fix-limit` now also has. In `tests/cli.rs`, the hook note of `init_asks_on_a_terminal` names 0.8.1. Agent: `init-engineer`. Files: `tests/golden/harness/model-team.md`, `tests/harness.rs`, `tests/cli.rs`. Verify: `cargo test --test harness_roles --test harness --test cli` fails only in these tests. Hours: 2.
- [ ] t13: Write the preset in `init`: in a new configuration with a level other than `none`, `subagent-card = true` and `fix-limit = 3` for `standard` and `strict`, and `false` and `0` for `lite`. `init` on a current file and `--reconfigure` keep the keys of the file, and a file with no key gets none. Add the tests of the 2 keys of each profile, of `--reconfigure` and of a project of 0.8.0, which fail before the code. Add `subagent-card` and `fix-limit` to the keys that `upgrade` leaves to the person in `the_first_upgrade_of_the_fixture_of_0_7_1_writes_the_files_of_its_profile` (guard). Agent: `init-engineer`. Files: `src/init.rs`, `tests/init_profiles.rs`, `tests/upgrade_cli.rs`. Verify: `cargo test --test init_profiles --test harness --test harness_roles --test cli` and `cargo test`. Hours: 3.
- [ ] t14: Add the guard that each built-in harness arm of `eval run` writes no key `subagent-card` or `fix-limit` and no entry of the 3 hooks into its copy, so the copy keeps the setup of 0.8.0. Say so in the table of the arms. Agent: `core-engineer`. Files: `tests/eval_run.rs`, `docs/evals.md`. Verify: `cargo test --test eval_run`. Hours: 1.
- [ ] t15: Write the new text of the skill `execute` of the section "Texts". The skill of `lite` keeps the bytes of 0.7.1 (guard: `lite_writes_each_skill_with_the_bytes_of_0_7_1`). The golden file changes first, so the tests fail before the code. In `the_golden_files_of_standard_differ_from_0_7_1_in_the_named_lines_only`, the lines of `execute` that go and that come get the new steps 2 to 6 and the new line `Check:`. The test of 2,500 bytes passes with 2,485. Agent: `init-engineer`. Files: `src/harness_text.rs`, `tests/golden/harness/skills/spec-dna-execute.md`, `tests/harness_skills.rs`. Verify: `cargo test --test harness_skills` and `cargo test --bin spec-dna harness_text::`. Hours: 1.
- [ ] t16: Print the note of the section "Texts" on stderr after a success of `finish` that writes, in the text output, when the profile is not `lite`. `--dry-run`, a stop and `--json` print no such note. The tests fail before the code: a success in `standard` prints the note once, a stop and a success in `lite` print none, and the stdout of `--json` keeps its bytes. Agent: `hygiene-engineer`. Files: `src/finish.rs`, `src/main.rs`, `tests/finish.rs`. Verify: `cargo test --test finish`. Hours: 2.
- [ ] t17: Document the 3 hooks in the `hook` section, their flags and their state, and the note in the `finish` section. Add the 3 new hooks to the help of `hook`. Agent: `init-engineer`, with `core-engineer` for the help and `hygiene-engineer` for the `finish` section. Files: `docs/commands.md`, `src/main.rs`. Verify: `cargo test --bin spec-dna` and `pre-commit run --all-files`. Hours: 2.
- [ ] t18: Document in `docs/harness.md`: the 2 keys in the example and in the table of keys, the 4 entries in the table of entries, the version 0.8.1 of the hooks, and one section for each hook with the facts of t1a and t1b. Give the full template of the task brief, with the meaning of each line and the source of `Task`, `Files` and `Done when` in `next_task` of `spec-dna next <slug> --json`. Give the table of the 4 return states and the actions. Add the lessons: the form, the rule that the person accepts each lesson in the chat, and the 3 classes of target. Agent: `init-engineer`. Files: `docs/harness.md`. Verify: `pre-commit run --all-files`. Hours: 3.
- [ ] t19: Add to `docs/harness.md` what each item cannot prove, from the lists "Does not prove" of items 18, 19 and 20 of the design, with the result of t1b for an internal agent. Add one more limit to item 18: a subagent with a structured output can get one more request from `return-state`. Agent: `init-engineer`. Files: `docs/harness.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 1.
- [ ] t20: Document the preset of each profile and the version 0.8.1 of the hook note. Agent: `init-engineer`, with `diagnostics-engineer` for `docs/profiles.md`. Files: `docs/init.md`, `docs/profiles.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t21: Add the lessons at the end of a change to the change guide, and the 5 terms to the glossary. The term `brief` keeps its meaning: the text of `accept --brief`. Agent: `architect` for the facts, `docs-editor` for the language. Files: `docs/guides/change.md`, `docs/glossary.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 1.
- [ ] t22: Write the entry of the change, with a part for an upgrade from 0.8.0: a current file gets no key and no hook, and a new project of `standard` or `strict` gets both. Agent: `init-engineer`; `release-engineer` edits the file. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t23: Add the deviations of this change to the design page, in a section of the deviations of the build: the concerns of the spec that differ from the design, each item of `Deviations` of this plan, and the results of t1a and t1b. Agent: `architect`. Files: `docs/design/0.8.1-harness-additions.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 1.

## Risks

- The frozen texts of `specify`, `finish`, `write-plan`, `review` and `acceptor.md` must not change. The golden files `tests/golden/harness/skills/spec-dna-{specify,finish,write-plan,review}.md`, `acceptor.md` and `acceptor-no-advisor.md` stay green with no edit (guard).
- The texts of the section "Texts" are agent-facing, and they freeze at the merge. The paid reruns of `execute` and `review` in this part start only after the merge of this change and of `harness-tools`.
- `harness-tools` also edits `src/harness.rs`, `src/harness_text.rs`, `src/init.rs`, `src/main.rs`, `tests/cli.rs`, `tests/harness_roles.rs`, `docs/commands.md`, `docs/harness.md`, `docs/glossary.md`, `docs/design/0.8.1-harness-additions.md` and `CHANGELOG.md`. Build this change first, and rebase `harness-tools` on it. Then run each check of `harness-tools` again.
- The facts of t1a and t1b can differ from the hooks reference. Each other result has its change in t4 to t6 or t19. A result that the plan does not cover stops the build before t4, and `architect` decides.
- A subagent with a structured output, such as an agent of a workflow with a schema, can get one more request from `return-state`, because its last text has no state line. The cost is one request for each such agent.
- An internal agent of a session that starts with `--agent <name>` can get the card and one block.
- `Explore`, `verifier`, `fallback-advisor` and `acceptor` keep the report form of their agent files, which have priority over the card. Line 6 of the card still asks each of them for a state line. A report with none costs one more request. The agent files do not change, because `acceptor.md` is frozen.
- The probe of this plan, in the lab, put the 2 presets, the entries, a section of the team page and the texts into a copy of the tree, and ran `cargo test`. The tests that it names are in `Files`. A test that the probe did not see can fail in t13. Then the builder gives it an explicit key in a task of its own before t11, and records the task in `Deviations`.
- A project of 0.8.0 that runs `init --reconfigure` keeps a file with no key, so it gets no hook. The entry of the change tells the person to add the 2 keys.

## Proof

Each test fails before the code, unless it says "guard".

- `tests/hook.rs`: a `SubagentStart` input gives the card, and with `agent_type = "acceptor"` the card with the line 3 of the acceptor. A `SubagentStop` input with no state line gives `decision: "block"`, and the same input with `stop_hook_active` true gives no output. An input with an empty `agent_type` gives no output. The sequence failure, edit, failure, edit, failure, edit, failure gives the message at the fourth failure, and only once. A pass deletes the count, an edit below an `--intent` folder sets no mark, and no state file holds a byte of the command.
- `src/harness_text.rs`: the card has 800 bytes or fewer in both forms. Each body of a skill has 2,500 bytes or fewer, and `execute` has 2,485.
- `tests/harness.rs`, `tests/harness_roles.rs`: the 4 entries and the section of the team page equal the new golden files. A configuration with no key writes no entry (guard).
- `tests/harness_check_step4.rs`: SD504 for an absent entry and SD502 for a changed command.
- `tests/init_profiles.rs`: a new project of `standard` and `strict` has both keys on, and a new project of `lite` has both keys off. The settings, the agent files, the skills, the team page and the lines of a new project of `lite` keep the bytes of 0.8.0 (guard). A project of 0.8.0 keeps a file with no key after `init` and `upgrade` (guard).
- `tests/finish.rs`: the note prints once after a success in `standard`, and not after a stop, with `--dry-run`, with `--json` or in `lite`.
- `tests/eval_run.rs`: the copy of each built-in arm has no key and no entry of the 3 hooks (guard).
- `tests/acceptance_checks.rs`: `in_lite_a_new_plan_needs_no_record_and_finish_is_as_before` stays green, so `finish` of `lite` prints no new line (guard).
- `tests/upgrade_cli.rs`: `the_first_upgrade_of_the_fixture_of_0_7_1_writes_the_files_of_its_profile` stays green: the pin is the only edit of the configuration, and in `lite` each changed line of a generated file is a marker line (guard).
- Guards with no change: the golden files of the frozen texts, `skills-0.7.1/`, `agents-0.7.1/`, `model-team-0.7.1.md`, `settings-lean.json`, and the golden files of `tests/golden/upgrade/`.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result `accepted`.
