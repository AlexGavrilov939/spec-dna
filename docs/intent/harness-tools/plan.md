---
status: approved
---
# `transcript --cost` gives the USD of a change, and `outline` gives the line ranges of the definitions of a code file: plan

## Files

- `docs/adr/0004-harness.md` The amendment of 2026-11-25 for the price table, in the pull request of the plans.
- `src/prices.rs` The price table of the release and the reader of a prices file (new).
- `src/transcript.rs` The mode --cost, the tie of a session to a change, and the name outline in the counted commands.
- `src/main.rs` The modules, the options of transcript, the command outline, the help and the exit codes, and the profile of the adapter skill in check and in layer add.
- `src/outline.rs` The line rules and the ranges of outline (new).
- `src/adapters.rs` The profile of the adapter skill, the body of 0.7.1 for lite, and the sentence on outline.
- `src/init.rs` The profile of the adapter skill in init.
- `src/upgrade.rs` The profile of the adapter skill in upgrade, and the text of Explore in its test.
- `src/harness.rs` The sentence on outline in Explore.
- `src/harness_text.rs` The body of Explore of 0.7.1 for lite.
- `tests/cli.rs` The cases of transcript --cost and of the count of outline.
- `tests/outline.rs` The cases of the command outline (new).
- `tests/harness_roles.rs` The guard of the adapter skill of lite.
- `tests/fixtures/transcripts-cost/session-2.jsonl` A synthetic session with an advisor call, the split of the cache writes, a call of the tool Agent, a model with no price and the slug of a change (new).
- `tests/fixtures/transcripts-cost/session-2/subagents/agent-a1b2c3d4e5f6a7b8.jsonl` The synthetic subagent of that session, with its type (new).
- `tests/fixtures/transcripts-cost/session-2/subagents/agent-a1b2c3d4e5f6a7b8.meta.json` The type of the subagent, only if t1c finds the type in a file of its own (new).
- `tests/fixtures/transcripts-unsafe/cost-unsafe.jsonl` Secret text in each field that the cost report reads (new).
- `tests/fixtures/outline/sample.rs` The input of the golden file of Rust (new).
- `tests/fixtures/outline/sample.py` The input of the golden file of Python (new).
- `tests/fixtures/outline/sample.go` The input of the golden file of Go (new).
- `tests/fixtures/outline/sample.ts` The input of the golden file of JavaScript and TypeScript (new).
- `tests/fixtures/outline/Sample.java` The input of the golden file of Java (new).
- `tests/golden/outline/rust.txt` The outline of sample.rs (new).
- `tests/golden/outline/python.txt` The outline of sample.py (new).
- `tests/golden/outline/go.txt` The outline of sample.go (new).
- `tests/golden/outline/typescript.txt` The outline of sample.ts (new).
- `tests/golden/outline/java.txt` The outline of Sample.java (new).
- `tests/golden/harness/skills/spec-dna.md` The adapter skill with the sentence on outline.
- `tests/golden/harness/skills-0.7.1/spec-dna.md` The adapter skill of lite, with the bytes of 0.8.0 (new).
- `tests/golden/harness/Explore.md` The text of Explore with the sentence on outline.
- `docs/transcript.md` The mode --cost, and what it cannot prove.
- `docs/commands.md` The sections transcript and outline, the summary and the exit codes.
- `docs/harness.md` The sentence on outline in Explore.
- `docs/glossary.md` The term outline.
- `docs/evals.md` The new texts of Explore and of the adapter skill in the copies of the harness arms.
- `.claude/agents/ownership.md` The rows of the new files.
- `.claude/agents/release-engineer.md` The step that compares the price table with the public price page.
- `README.md` The row of outline in the table of commands.
- `CHANGELOG.md` The entry of the change.
- `docs/design/0.8.1-harness-additions.md` The deviations of the build, because finish deletes this plan.

## Change sets

The change sets of `.claude/agents/ownership.md` that apply. `architect` is A of each, except where the row names the owner.

- Information model: the amendment of ADR 0004 of 2026-11-25 for the price table. The owner accepted it by the word of 2026-11-25.
- Transcript format or output: `src/transcript.rs`, `docs/transcript.md` and `tests/fixtures/transcripts*/`. R: `transcript-engineer`. C: `core-engineer`, `test-engineer`. The reader of `src/json.rs` does not change.
- Command, option, help text or exit code: `src/main.rs`, the `transcript` and `outline` sections, the summary and the exit codes of `docs/commands.md`, `tests/cli.rs`, `tests/outline.rs` and `CHANGELOG.md`. R: `core-engineer`, `transcript-engineer` and `context-engineer`.
- `--json` shape: the JSON of `--cost` and of `outline`, in `src/transcript.rs`, `src/outline.rs` and their tests. R: `transcript-engineer` and `context-engineer`.
- Adapter or skill text: `src/adapters.rs` and its callers. `docs/init.md`, `docs/quickstart.md` and `scripts/demo-replay.sh` hold no text of the body of the adapter skill, so they keep their bytes. R: `init-engineer`.
- Harness role or generated file: `src/harness.rs`, `src/harness_text.rs`, `docs/harness.md` and `tests/golden/harness/`, for `Explore`. R: `init-engineer`.
- Upgrade plan: `src/upgrade.rs`, for the profile of the adapter skill and the text of `Explore` in its test. R: `init-engineer`.
- Agent team: `.claude/agents/ownership.md` and `.claude/agents/release-engineer.md`. A: the owner. R: `architect`.
- Convention: the row of `outline` in `README.md`. A: the owner. R: `architect`.

## Texts

Each text below is final. A task that writes it uses these bytes.

The sentence of `Explore` of `standard` and `strict`: the line `- Read a large file in parts.` gets 105 bytes. `lite` keeps the line of 0.7.1.

```text
- Read a large file in parts. For a code file, run `spec-dna outline <file>` first, and read only the lines of the item that you need.
```

The sentence of the adapter skill of `standard` and `strict`: step 2 gets 109 bytes. `lite` keeps the step of 0.7.1.

```text
2. Read a listed section with `spec-dna show <id>#<anchor>` only if the task needs it. Before you read a long code file, run `spec-dna outline <file>`, and read only the line range that you need.
```

The text output of `transcript --cost`. USD values have 6 decimals. `-` is a change of `null`.

```text
cost: <usd> USD for <n> requests (list price[, estimated][, partial]); prices: built-in <date> | file [<date>]
change <slug or ->: <n> sessions, <usd> USD
  model <model id>: <n> requests, <usd> USD | no price
  role <role>: <usd> USD
  advisor: <n> calls, at least <usd> USD
unpriced: <model ids, or none>
```

A session with calls of the tool `Agent` and no file of a subagent adds the line `note: <n> sessions call the tool Agent and have no file of a subagent; give the folder of the session`.

The JSON of `--json` is the JSON of item 21 of the design. `prices.source` is `built-in` or `file`. `prices.date` is the date of the table of the binary, or the key `date` of the file, else `null`.

The text output of `outline` and its JSON are those of item 22 of the design. The note for a file with no rule: `<path>: <n> lines; no outline rule for this file type; read it by ranges`. For a binary file or a file above 4 MiB, the reason is `binary file` or `file above 4 MiB`.

## Order

- [ ] t1a: Confirm on a transcript of the lab, before t4, that the usage of a request with an advisor call has no tokens of the advisor model (E7, section 2 of the design). Agent: `transcript-engineer`. The method: a script in the lab, outside this repository, reads a local Claude Code transcript of a session with an advisor call and a subagent. It prints only key paths, counts, model ids and the presence of each key, and no value of a text field. No text of a transcript goes into a file of this repository, into a report or into the pull request. The other result: if the usage holds the tokens of the advisor, t4 prices them at the rates of the advisor model in the role `advisor`, and `usd_lower_limit` is then their exact sum. Files: none. Verify: the note of the lab gives the count of advisor calls and the model ids of their usage, and `git status --porcelain` lists no file of the lab. Hours: 1.
- [ ] t1b: Confirm on the same transcript, before t2, that the usage splits the cache writes into a 5-minute part and a 1-hour part, and record the 2 key names (E7). Agent: `transcript-engineer`. Use the script of t1a. The other result: if no usage has the split, the cache writes take the 5-minute rate and the mark `estimated`, and the 2 fields are `null`. Files: none. Verify: the note of the lab gives the 2 key paths and the count of requests that have them, and `git status --porcelain` lists no file of the lab. Hours: 1.
- [ ] t1c: Confirm on the same transcript, before t2, the field that gives the type of a subagent, and where it is: in the subagent file, in a file of its own, or in the call of the tool `Agent` of the main transcript with its link to the agent id (section 2 of the design). Agent: `transcript-engineer`. Use the script of t1a. The other result: if no field gives the type, the role of a request of a subagent is `subagent`, and t2 writes no file `.meta.json`. Files: none. Verify: the note of the lab gives the key path of the type and the count of subagents that have it, and `git status --porcelain` lists no file of the lab. Hours: 1.
- [ ] t2: Write the synthetic fixtures with the key names of t1b and t1c: a main session with 2 requests of one model, an advisor call (`server_tool_use` with the name `advisor` and an `advisor_tool_result`), the 2 parts of the cache writes, a call of the tool `Agent`, a request of a model with no row, and a `Bash` call of `spec-dna next sensor-timeout`. Add its subagent with a type. Each text field holds a sentinel text, such as `SECRET-PROMPT-TEXT`, and no text of a real transcript. Agent: `transcript-engineer`, with `test-engineer` consulted. Files: `tests/fixtures/transcripts-cost/session-2.jsonl`, `tests/fixtures/transcripts-cost/session-2/subagents/agent-a1b2c3d4e5f6a7b8.jsonl`, `tests/fixtures/transcripts-cost/session-2/subagents/agent-a1b2c3d4e5f6a7b8.meta.json`. Verify: `cargo run --quiet -- transcript tests/fixtures/transcripts-cost --json` exits with 0, and its output holds no sentinel text. Hours: 2.
- [ ] t3: Add `src/prices.rs`: the table of the release, with one row for each model id of the table of model ids in `src/harness.rs` that has a public price, its date, and the address of the price page. Read the prices from that page on the day of the build. A model with no public price gets no row. Add the reader of a prices file: TOML, one table for each model id, with the keys `input`, `output`, `cache-write-5m`, `cache-write-1h` and `cache-read` and an optional top key `date`. Its rows replace the rows of the same model. An unknown key, a negative or missing rate and a bad model id are errors that name the key. The unit tests fail before the code. Agent: `transcript-engineer`, with `core-engineer` for the line `mod prices;`. Files: `src/prices.rs`, `src/main.rs`. Verify: `cargo test --bin spec-dna prices::`. Hours: 2.
- [ ] t4: Add the count of `--cost` and the options `--cost` and `--prices <file>`: the 4 token counts of each request once, by its id (E8), and the 2 parts of the cache writes in 2 new fields of the tokens. Price each part at its rate, and each request at the rate of its model. Count the advisor calls, and give each call the lower limit of the design: the context of the request with the call, at the input rate of the advisor model of `[harness.roles]`. With calls and no such model, `usd_lower_limit` is `null`, and the total has the mark `partial`. A model with no row has `usd: null`, a row in `unpriced` and the mark `partial`. The role of a request is `main`, `advisor`, or the type of the subagent from t1c. Print the text output and the JSON of the section "Texts". The cases fail before the code: `session-1.jsonl` with the prices file of the design gives the JSON of the design byte for byte, with the version of the binary in place of `0.8.1`, and `session-2.jsonl` gives the split, the advisor, the subagent role and the mark `partial`. The read report keeps its bytes (guard). Agent: `transcript-engineer`, with `core-engineer` for the options. Files: `src/transcript.rs`, `src/main.rs`, `tests/cli.rs`. Verify: `cargo test --test cli transcript` and `cargo test --bin spec-dna transcript::`. Hours: 5.
- [ ] t5: Tie each episode to a change, in the order of item 21, step 5 of the design: `--change <slug>`, the slug of a `spec-dna new intent`, `next`, `accept`, `approve` or `finish` call or of an edit below `<intent folder>/<slug>/`, the session of an acceptor entry of a record or of the archive, then the slug of the episode before it in the same agent. A subagent gets the slug of the main agent at its start. Print a slug only if it names a change folder of the head tree, an archive file under `.spec-dna/accepted/`, or a change that `sdlc::ended_changes` gives, else `null`. Without a configuration, each change is `null`. Read the records with `record::Record::parse` and the owner of an archive file with `acceptance::archive_owner`, with no edit of `src/record.rs` or `src/acceptance.rs`. The cases fail before the code: in a repository with the folder `docs/intent/sensor-timeout/`, `session-2.jsonl` gives the change `sensor-timeout`. In a repository with no such change, it gives `null`. `--change other` puts each session on `other` where that change exists. Agent: `transcript-engineer`, with `core-engineer` for the read of the head tree. Files: `src/transcript.rs`, `src/main.rs`, `tests/cli.rs`. Verify: `cargo test --test cli transcript`. Hours: 4.
- [ ] t6: Add the exit codes and the help: `--change` or `--prices` without `--cost` gives 2, a slug that is not a valid name of a change gives 2, a prices file that is not valid gives 2, and a slug that names no change gives 5. Document the mode in the `transcript` section and the codes in the table of exit codes. In a project of `lite`, `--cost` gives the same output as in `standard`. The cases fail before the code. Agent: `core-engineer`, with `transcript-engineer` for the `transcript` section. Files: `src/main.rs`, `tests/cli.rs`, `docs/commands.md`. Verify: `cargo test --test cli transcript` and `pre-commit run --all-files`. Hours: 2.
- [ ] t7: Add the privacy guard of `--cost`: a fixture with secret text in each field that the cost report reads, as `tests/fixtures/transcripts-unsafe/unsafe.jsonl` has for the read report: a model id, a role, a slug, a tool name, a command and a path. The text output and the JSON hold no byte of a secret. A role that does not match the pattern of a tool name prints as `<redacted>`. Agent: `transcript-engineer`. Files: `tests/cli.rs`, `tests/fixtures/transcripts-unsafe/cost-unsafe.jsonl`. Verify: `cargo test --test cli transcript`. Hours: 1.
- [ ] t8: Document `--cost` in `docs/transcript.md`: the options, the price table and its date, the file of `--prices`, the count of each part, the advisor, the tie to a change, the privacy rule, and the facts of t1a, t1b and t1c with the version of Claude Code. Add what it cannot prove, from the list "Does not prove" of item 21 of the design. Agent: `transcript-engineer`. Files: `docs/transcript.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 2.
- [ ] t9: In the lab, compare the total of `--cost` for one headless run of `eval run` with the `total_cost_usd` that its result file records (E6). Use the result file and the transcripts of an earlier run of `eval run`, else of the first canary of the reruns of this part, so no new paid run is necessary. If neither exists before `finish`, write `Skip:` with the reason. The difference must come only from the advisor part and from the last decimal places. Put the 2 totals and the difference in the pull request of the code, with no text of a transcript. If the difference has another cause, stop and tell `architect`. Agent: `transcript-engineer`. Files: none. Verify: the note of the lab gives the 2 totals and the cause of the difference. Hours: 1.
- [ ] t10: Add `src/outline.rs`: the items (start, end, depth, kind, name, text), the rules 3 to 9 of item 22 of the design, the limits of 4 MiB, a NUL byte in the first 8 KiB, 300 items and 120 characters, and the rule of Rust. The unit tests fail before the code: the file `src/valve.rs` of the design gives the outline of the design, each line that is not blank is in a range of depth 0, and the 301st item gives the line `1 more items`. Agent: `context-engineer`, with `core-engineer` for the line `mod outline;`. Files: `src/outline.rs`, `src/main.rs`. Verify: `cargo test --bin spec-dna outline::`. Hours: 4.
- [ ] t11: Add the golden file of Rust: a file with an `impl` block, a `trait`, a `mod` block, a doc comment, an attribute, a raw string on many lines and a block comment. A unit test compares the outline with the golden file and checks that each line that is not blank is in a range of depth 0. It fails before a rule of the fixture works. Agent: `context-engineer`, with `test-engineer` consulted. Files: `src/outline.rs`, `tests/fixtures/outline/sample.rs`, `tests/golden/outline/rust.txt`. Verify: `cargo test --bin spec-dna outline::`. Hours: 2.
- [ ] t12: Add the rule of Python and its golden file: a class with methods, a decorator, a docstring on many lines and an `async def`. The test fails before the rule. Agent: `context-engineer`. Files: `src/outline.rs`, `tests/fixtures/outline/sample.py`, `tests/golden/outline/python.txt`. Verify: `cargo test --bin spec-dna outline::`. Hours: 2.
- [ ] t13: Add the rule of Go and its golden file: `func`, a method `func (r T) Name`, `type`, a `var` block and a raw string on many lines. The fixture uses tabs, as `.editorconfig` says. The test fails before the rule. Agent: `context-engineer`. Files: `src/outline.rs`, `tests/fixtures/outline/sample.go`, `tests/golden/outline/go.txt`. Verify: `cargo test --bin spec-dna outline::`. Hours: 2.
- [ ] t14: Add the rule of JavaScript and TypeScript and its golden file: `export` and `export default`, a class with a method, `get` and a field with a modifier, an `interface`, a `type`, a `const` with `=` and a template string on many lines. The other extensions of the family use the same rule, and a unit test checks each extension. The test fails before the rule. Agent: `context-engineer`. Files: `src/outline.rs`, `tests/fixtures/outline/sample.ts`, `tests/golden/outline/typescript.txt`. Verify: `cargo test --bin spec-dna outline::`. Hours: 2.
- [ ] t15: Add the rule of Java and its golden file: a class with modifiers, a constructor, methods, an annotation, a Javadoc comment, an `enum` and a `record`. The test fails before the rule. Agent: `context-engineer`. Files: `src/outline.rs`, `tests/fixtures/outline/Sample.java`, `tests/golden/outline/java.txt`. Verify: `cargo test --bin spec-dna outline::`. Hours: 2.
- [ ] t16: Add the command `outline <path>... [--json]`: the dispatch, the help and the path rule of the spec. It reads no `spec-dna.toml`. The cases fail before the code: the file of the design gives its text and its JSON byte for byte, an absolute path below the root gives the same result, a path with `..`, a path outside the root and a missing file give 5, no path gives 2, a folder outside git gives 3, an unknown extension, a binary file and a file above 4 MiB give the note and exit code 0, and a project of `lite` gives the same output as a project of `standard`. Agent: `core-engineer`, with `context-engineer` for the cases. Files: `src/main.rs`, `tests/outline.rs`. Verify: `cargo test --test outline`. Hours: 3.
- [ ] t17: Document `outline`: its section with the table of the 5 families, the relation to `context` and `show`, the limits, and what it cannot prove, from the list "Does not prove" of item 22 of the design. Add its row to the summary, an example line and its exit codes. Agent: `context-engineer`, with `core-engineer` for the summary and the exit codes. Files: `docs/commands.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 2.
- [ ] t18: Add `outline` to the spec-dna commands that `transcript` counts. The case fails before the code: a transcript with a `Bash` call of `spec-dna outline src/a.rs` counts `outline`, not `other`. Agent: `transcript-engineer`. Files: `src/transcript.rs`, `tests/cli.rs`. Verify: `cargo test --test cli transcript`. Hours: 1.
- [ ] t19: Give the adapter skill a profile: add the profile to `skill_file` and `check` of `src/adapters.rs` through 2 new functions, and keep the old names as calls with the profile `standard`. The text is the same for each profile in this task. A unit test checks that `lite` and `standard` give the same bytes (guard, until t23). Agent: `init-engineer`. Files: `src/adapters.rs`. Verify: `cargo test --bin spec-dna adapters::`. Hours: 1.
- [ ] t20: Pass the profile of the configuration to the adapter skill in `init`, in `check` and in `layer add`. Agent: `init-engineer`, with `core-engineer` for `src/main.rs`. Files: `src/init.rs`, `src/main.rs`. Verify: `cargo test`. Hours: 1.
- [ ] t21: Pass the profile in `upgrade`, and remove the 2 old names. The tests of `src/harness.rs` that call `adapters::check` pass the profile. Agent: `init-engineer`. Files: `src/upgrade.rs`, `src/adapters.rs`, `src/harness.rs`. Verify: `cargo test` and `cargo clippy --all-targets -- -D warnings`. Hours: 1.
- [ ] t22: Add the guard of the adapter skill of `lite`: a new project of `lite` writes the adapter skill of the golden file `skills-0.7.1/spec-dna.md`, a copy of the current `skills/spec-dna.md`. The test passes before and after t23. Agent: `init-engineer`. Files: `tests/harness_roles.rs`, `tests/golden/harness/skills-0.7.1/spec-dna.md`. Verify: `cargo test --test harness_roles`. Hours: 1.
- [ ] t23: Write the sentence on `outline` in the adapter skill of `standard` and `strict`, with the body of 0.7.1 for `lite`. The golden file changes first, so the test fails before the code. Agent: `init-engineer`. Files: `src/adapters.rs`, `tests/golden/harness/skills/spec-dna.md`. Verify: `cargo test --test harness_roles` and `cargo test --bin spec-dna adapters::`. Hours: 1.
- [ ] t24: Give `lite` the body of `Explore` of 0.7.1 in `agent_body`, as `code-simplifier` has. The text is the current text, so no file changes its bytes (guard: `lite_writes_the_agent_files_of_0_7_1_at_each_level`). Agent: `init-engineer`. Files: `src/harness_text.rs`. Verify: `cargo test --test harness_roles`. Hours: 1.
- [ ] t25: Write the sentence on `outline` in `Explore` of `standard` and `strict`. The golden file and the text in the test of `src/upgrade.rs` change first, so the tests fail before the code. Agent: `init-engineer`. Files: `src/harness.rs`, `tests/golden/harness/Explore.md`, `src/upgrade.rs`. Verify: `cargo test --test harness_roles` and `cargo test --bin spec-dna upgrade::`. Hours: 1.
- [ ] t26: Say in `docs/harness.md` that `Explore` and the adapter skill of `standard` and `strict` name `outline`, and that `lite` keeps the texts of 0.7.1. Add the term `outline` to the glossary. Say in the page of the evals that the copy of a built-in harness arm of 0.8.1 gets the new texts of `Explore` and of the adapter skill, so a run of 0.8.1 and a run of 0.8.0 do not have the same texts. Agent: `init-engineer`, with `architect` for the facts, `docs-editor` for the language of the glossary and `core-engineer` for `docs/evals.md`. Files: `docs/harness.md`, `docs/glossary.md`, `docs/evals.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t27: Add the rows of `src/outline.rs` (`context-engineer`), `src/prices.rs` (`transcript-engineer`), `tests/outline.rs`, `tests/fixtures/outline/` and `tests/golden/outline/` (`context-engineer`) to the quick lookup. Add to `release-engineer.md` the step: before each release, compare the price table with the public price page, and put each new or changed row in the release pull request for the owner. Add the row of `outline` to the table of commands of `README.md`. Agent: `architect`. Files: `.claude/agents/ownership.md`, `.claude/agents/release-engineer.md`, `README.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t28: Write the entry of the change. Agent: `transcript-engineer` and `context-engineer`; `release-engineer` edits the file. Files: `CHANGELOG.md`. Verify: `pre-commit run --all-files`. Hours: 1.
- [ ] t29: Add the deviations of this change to the design page, in the section of the deviations of the build: the concerns of the spec that differ from the design, each item of `Deviations` of this plan, and the results of t1a, t1b, t1c and t9. Agent: `architect`. Files: `docs/design/0.8.1-harness-additions.md`. Verify: `pre-commit run --all-files` and `cargo run --quiet -- check --all` with no SD102. Hours: 1.

## Risks

- No text of a real transcript may enter this repository, a report or a pull request. t1a, t1b, t1c and t9 read local transcripts only for key names and counts, and the fixtures are synthetic. A secret, a prompt or a path of a consumer project in a diff stops the commit, and the owner hears of it at once.
- The prices are the list prices of the day of the build. The owner accepts each row in the release pull request, and `release-engineer` compares the table again before the release.
- The facts of t1a, t1b and t1c can differ from the design. Each other result has its change in t2 or t4. A result that the plan does not cover stops the build before t4, and `architect` decides.
- The texts of `Explore` and of the adapter skill are agent-facing, and they freeze at the merge. The paid reruns of `execute` and `review` in this part start only after the merge of this change and of `harness-reports`.
- This change and `harness-reports` both edit `src/harness.rs`, `src/harness_text.rs`, `src/init.rs`, `src/main.rs`, `tests/cli.rs`, `tests/harness_roles.rs`, `docs/commands.md`, `docs/harness.md`, `docs/glossary.md`, `docs/design/0.8.1-harness-additions.md` and `CHANGELOG.md`. Build `harness-reports` first, and rebase this change on it. Then run each check again.
- The adapter skill of a project of 0.8.0 in `standard` or `strict` gets SD502 until `init` or `upgrade` writes it again, as for each release. A project of `lite` gets no finding.
- The built-in harness arms of `eval run` use `Harness::new`, which gives the profile `standard`. So their copies get the new texts of `Explore` and of the adapter skill, and a rerun of 0.8.1 measures these texts. The freeze table does not list them, so the rule asks for no rerun, and t26 states the difference in `docs/evals.md`.
- A fixture file in `tests/fixtures/outline/` is source code of 4 languages. The pre-commit hooks read it: `.editorconfig`, `typos` and the line ends. Each fixture follows the rules of its language in `.editorconfig`.
- An `outline` range can be wrong for a signature on many lines, a macro that makes definitions, generated code or a minified file. The command gives the ranges, and the agent checks the first and the last line.

## Proof

Each test fails before the code, unless it says "guard".

- `tests/cli.rs`: `session-1.jsonl` with the prices file of the design gives the JSON of the design byte for byte, with the version of the binary in place of `0.8.1`. `session-2.jsonl` gives the 2 parts of the cache writes, the advisor calls with the lower limit, the role of the subagent, the change `sensor-timeout` in a repository with that change and `null` in a repository with none, and the mark `partial` with the model in `unpriced`. The exit codes of `--change` and `--prices`. The fixture with secret text gives no byte of a secret (guard). The read report keeps its bytes for each current fixture (guard). A call of `spec-dna outline` counts as `outline`.
- `src/prices.rs`: the table has a date and an address, and a prices file replaces the rows of its models.
- `src/outline.rs`: the 5 golden files, and in each of them each line that is not blank is in a range of depth 0.
- `tests/outline.rs`: the example of the design byte for byte in text and in JSON, an absolute path below the root, and the exit codes 0, 2, 3 and 5.
- `tests/harness_roles.rs`: the adapter skill of `lite` keeps the bytes of 0.8.0 (guard), and the adapter skill of `standard` has the sentence. `lite_writes_the_agent_files_of_0_7_1_at_each_level` stays green (guard).
- The lab check of t9: the total of `--cost` and `total_cost_usd` differ only by the advisor part and by the last decimal places. If t9 ends with `Skip:`, no run proves the total of a real session, and the proof of item 21 rests on the fixtures only.

`pre-commit run --all-files`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, `checker` reports no open blocker, and the record of the acceptor has the result `accepted`.
