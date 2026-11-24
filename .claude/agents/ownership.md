# RACI matrix

This file gives the responsible agent and the approver of each task and each path in this repository. The agent files in this folder must agree with it. If an agent file and this file disagree, this file is correct. Change both in the same commit.

## Legend

| Letter | Role | Meaning |
| --- | --- | --- |
| R | Responsible | Makes the change. A row can have more than one R. |
| A | Accountable | Approves the result. Each row has exactly one A. |
| C | Consulted | Gives input before the decision or the work. |
| I | Informed | Gets the result after the decision. |
| r, c, i | Conditional | The same role, only when the task touches the files of that agent. |
| - | None | No role. |

`R/A` means that one agent makes the change and approves it. The agent that writes a change never verifies it: `checker` does.

## Roles

| Code | Role | Model |
| --- | --- | --- |
| OWN | The owner: the human maintainer | - |
| ARC | `architect` | opus |
| COR | `core-engineer` | sonnet |
| DIA | `diagnostics-engineer` | sonnet |
| CTX | `context-engineer` | sonnet |
| HYG | `hygiene-engineer` | sonnet |
| INI | `init-engineer` | sonnet |
| TRN | `transcript-engineer` | sonnet |
| TST | `test-engineer` | sonnet |
| DOC | `docs-editor` | sonnet |
| REL | `release-engineer` | sonnet |
| FAL | `failure-analyst` | opus |
| CHK | `checker` | opus |

These roles have no row in the matrix:

- The main session builds a small task itself. It then acts as the R of the task and keeps the invariants of the responsible agent. It also makes each commit and pull request, after the approval of the owner.
- The advisor (Fable) and `fallback-advisor` give advice at a decision point of a `higher-risk` change. They are C when the main session asks them.
- `Explore` searches on request. It owns no task and no path.
- `acceptor` writes the acceptance record of a `higher-risk` change, from the brief of `spec-dna accept --brief`, and commits it alone. It writes `acceptance.toml` only, and it edits no code. It is not `checker`: `checker` verifies the work of an agent before the owner accepts it, and `acceptor` writes the record that the gate of `finish` reads.

## Work areas

| Area | OWN | ARC | COR | DIA | CTX | HYG | INI | TRN | TST | DOC | REL | FAL | CHK |
| --- | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: | :-: |
| Scope and priority of the work | R/A | C | - | - | - | - | - | - | - | - | C | - | - |
| Information model and ADRs | A | R | C | c | c | c | c | c | C | C | I | - | - |
| Core: parse, model, config, hashes, cache, git access, templates, `new`, the CLI | - | A | R | c | c | c | c | c | c | - | I | - | - |
| `check`: controls, gating, drift, citations, evidence, lint, codes | - | A | c | R | c | - | i | - | c | - | I | - | - |
| `context` and `show`: the section plan, budgets, the rank | - | A | c | c | R | - | i | - | c | - | I | - | - |
| Hygiene: `mv`, the split, `history`, `index`, `--stub` files | - | A | c | c | - | R | - | - | c | - | I | - | - |
| `init`: detection, adapters, skill text, quickstart, demo | - | A | c | c | c | - | R | - | c | C | I | - | - |
| `transcript`: formats, counts, privacy | - | A | c | - | c | - | - | R | c | - | I | - | - |
| Test strategy, shared test code, golden and fixture policy | - | C | c | c | c | c | c | c | R/A | - | i | - | - |
| Benchmarks: tool speed and retrieval | I | A | c | - | R | - | - | - | R | - | - | - | - |
| Prose in ASD-STE100 | - | c | c | c | c | c | c | c | - | R/A | c | - | - |
| Conventions: `AGENTS.md`, `CONTRIBUTING.md`, the PR template, lint and editor configs | A | R | I | I | I | I | I | I | I | C | C | I | I |
| Agent team: `.claude/` and `CLAUDE.md` | A | R | I | I | I | I | I | I | I | C | I | I | I |
| `CHANGELOG.md` and the version | A | C | r | r | r | r | r | r | - | C | R | - | - |
| CI, hooks, Dependabot, cross-builds | I | A | - | - | - | - | - | - | C | - | R | - | - |
| A new dependency | A | R | R | c | c | c | c | c | - | c | I | - | - |
| The cause of a failure | - | A | c | c | c | c | c | c | c | - | c | R | - |
| Verification of finished work | A | I | i | i | i | i | i | i | i | i | i | - | R |
| Approval of a step that leaves the working tree | R/A | I | - | - | - | - | - | - | - | - | C | - | C |

In the `CHANGELOG.md` row, the responsible agent of a change writes its entry, and `release-engineer` edits the file and the release sections. In the last row, the owner approves or does each commit, push, pull request, merge, tag and release. The main session makes a commit or a pull request only after that approval.

## Change sets

A change set is a group of files that must change together. One file without the others breaks a contract.

If a change needs a new ADR or an amendment, the row "Information model" also applies, and the owner accepts the text.

| Change | Files that change together | A | R | C | I |
| --- | --- | --- | --- | --- | --- |
| Information model | A dated amendment of ADR 0001; `src/model.rs`, `src/parse.rs`; the doc that states the rule; `CHANGELOG.md` | OWN | ARC (the amendment), COR (the code) | DIA, CTX, HYG, INI, TST | DOC, REL |
| Hash definition or parsed data | `src/hash.rs` or `src/parse.rs`; `PARSER_VERSION` in `src/parse.rs` | ARC | COR | DIA, HYG, TST | REL |
| Cache layout | `src/index.rs` and its `FORMAT` | ARC | COR | TST | REL |
| Diagnostic code: new, retired or new severity | `src/diag.rs`; the module that gives the code; the profile table and `GROUPS` in `src/config.rs`; `docs/diagnostics.md`; `docs/profiles.md`; the `check` tests and golden files; `CHANGELOG.md` | ARC | DIA, and COR for `src/config.rs` | INI, TST | DOC, REL |
| Config key | `src/config.rs`, `docs/config.md` and `schema/spec-dna.schema.json`; `src/detect.rs` if `init` proposes the key; `CHANGELOG.md` | ARC | COR, and the agent of the feature | INI, TST | DOC, REL |
| Command, option, help text or exit code | `src/main.rs`; the section of the command and the exit codes in `docs/commands.md`; the doc page of the command; the tests in `tests/cli.rs`; `CHANGELOG.md` | ARC | COR, and the agent of the command | TST, DOC | INI, REL |
| `--json` shape | The module of the command; `src/json.rs`; the tests, and the golden file if the command has one; the doc page | ARC | The agent of the command | COR, TST | REL |
| Document type or template | `DocType` in `src/model.rs`; `src/config.rs`; `src/layer.rs`; `src/templates.rs` and `docs/types.md`; `src/detect.rs` for the detection of the type | ARC | COR | INI, DIA, HYG | DOC, REL |
| Adapter or skill text | `src/adapters.rs`; `docs/init.md` and `docs/quickstart.md`; `scripts/demo-replay.sh`; the `init` tests in `tests/cli.rs` | ARC | INI | CTX, DIA, DOC, TST | REL |
| Context plan or rank | `src/context.rs` and `src/rank.rs`; the `context` section of `docs/commands.md`; `bench/results/retrieval-*.json` | ARC | CTX | DIA, TST | INI, REL |
| Move, split or generated index | `src/mv.rs`, `src/split.rs` and `src/registry.rs`; the `mv`, `history` and `index` sections of `docs/commands.md` | ARC | HYG | COR, DIA, TST | REL |
| Transcript format or output | `src/transcript.rs` and the reader in `src/json.rs`; `docs/transcript.md`; `tests/fixtures/transcripts*/` | ARC | TRN | COR, TST | REL |
| Git access | `src/git.rs` | ARC | COR | TST | REL |
| Approval in the branch (ADR 0012) | `src/approval.rs`; the function `approval_missing` in `src/lifecycle.rs` and the texts of SD910 in `src/diag.rs`; the keys `approval` and `approvers` (the row "Config key"); the field `approvals` of `check --json` and its golden file; `tests/branch_approval.rs`; the SD910 part of `docs/diagnostics.md` and `docs/profiles.md`; `CHANGELOG.md` | ARC | DIA | COR, INI, TST | DOC, REL |
| Harness role or generated file | `src/harness.rs` and `src/harness_text.rs`; `docs/harness.md`; `tests/golden/harness/`; the harness tests; `CHANGELOG.md` | ARC | INI, and DIA for the function `check` | COR, TST, DOC | REL |
| Hook | `src/hook.rs`; the `hook` section of `docs/commands.md`; the hooks section of `docs/harness.md`; `tests/hook*.rs` | ARC | INI | TST | REL |
| Eval file form or runner | `src/evals.rs`, `src/eval_run.rs`, `src/eval_ab.rs`, `src/verdict.rs` and `src/eval_records.rs`; the `eval` section and the exit codes of `docs/commands.md`; `docs/evals.md`; `tests/eval_run.rs`, `tests/eval_ab.rs` and `tests/eval_records.rs`; `bench/results/evals/`; SD506 and SD507 in `docs/diagnostics.md` | ARC | COR | DIA, INI, TST | REL |
| Default skills (ADR 0023) | `DEFAULT_ON_NEEDS_VERDICT_ON` and `RELEASE` in `src/verdict.rs`; `Skill::in_d2` in `src/harness.rs`; the preset of `src/init.rs`; `rename_skills` in `src/upgrade_config.rs`; SD507 in `src/check.rs`; `docs/profiles.md`, `docs/init.md` and `docs/commands.md` (`skill list`); `tests/init_profiles.rs` and `tests/evals_checks.rs`; `CHANGELOG.md`. A change of the constant or of `RELEASE` edits `src/verdict.rs` only | OWN (the value of the constant) | COR (`src/verdict.rs`, `src/upgrade_config.rs`), INI (`src/init.rs`, `src/harness.rs`) and DIA (`src/check.rs`) | TST | REL, DOC |
| Eval skeleton or workflow | `src/skeleton.rs`; question 14 in `src/init.rs`; `tests/golden/evals/`; `docs/init.md` | ARC | INI | COR, DIA, REL | DOC |
| Upgrade plan | `src/upgrade.rs`, `src/upgrade_old.rs`, `src/upgrade_config.rs`, `src/upgrade_docs.rs`, `src/upgrade_proof.rs` and `src/config_text.rs`; `tests/upgrade_*.rs`, `tests/fixtures/upgrade/` and `tests/golden/upgrade/`; the doc page of the command, when it exists; `CHANGELOG.md` | ARC | INI (the command), COR (the configuration) and HYG (the documents) | DIA, TST | DOC, REL |
| Import | `src/import.rs`, `src/import_config.rs`, `src/import_docs.rs`, `src/import_speckit.rs`, `src/import_openspec.rs` and `src/import_proof.rs`; `tests/common/import.rs` and `tests/import_*.rs`; the `import` section of `docs/commands.md`; the term import in `docs/glossary.md`; `CHANGELOG.md` | ARC | INI (the command), COR (the configuration), HYG (the documents, the readers and the proof) | DIA, TST | DOC, REL |
| New dependency | `Cargo.toml` and `Cargo.lock`; the dependency table of `CONTRIBUTING.md` | OWN | ARC (the assessment), COR (the change) | REL | All |
| Convention | `AGENTS.md`, `CONTRIBUTING.md`, `README.md` and `.github/pull_request_template.md`; lint and editor configs | OWN | ARC | DOC, REL | All |
| Release | `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`; the version in `README.md`, `docs/quickstart.md` and `docs/config.md`; the checkout SHA and the Claude Code version that `init` writes into a workflow (`src/init.rs`, `src/skeleton.rs`); the tag. The pin in `spec-dna.toml` of this repository moves in the release pull request, in the same commit as the version in `Cargo.toml` (ADR 0013): the tree binary does not count the pin line for SD503 | OWN | REL | ARC, TST | All |
| CI or hooks | `.github/` except the PR template, `.pre-commit-config.yaml`, `scripts/cross-build.sh`, `scripts/smoke.sh`, `scripts/release-assets.sh`, `scripts/verify-release.sh`, `tests/release_check.rs` and `src/release.rs` | ARC | REL | TST | OWN |
| Agent team | `.claude/` and `CLAUDE.md`; the pointers in `AGENTS.md` and `CONTRIBUTING.md` | OWN | ARC | DOC | All |

## Lifecycle of a change

| Step | R | A | C | Output |
| --- | --- | --- | --- | --- |
| 1. Scope | OWN | OWN | ARC | The task and its limits |
| 2. Route | The main session | The main session | This file | The responsible agent and the change sets |
| 3. Design, if a contract or 2 or more modules change | ARC | ARC | The responsible agents; the advisor in a `higher-risk` change | A decision, or the draft of an ADR amendment |
| 4. Accept an ADR or an amendment | OWN | OWN | ARC | The accepted text |
| 5. Build | The responsible agent, or the main session | ARC | TST | The diff, with its tests |
| 6. Docs | The responsible agent | DOC | - | Docs that agree with the code, in STE |
| 7. Verify | CHK | OWN | - | The report of `checker` |
| 8. Record | The responsible agent, then REL | OWN | - | The `CHANGELOG.md` entry |
| 9. Commit and pull request | The main session | OWN | CHK | A commit or a pull request, after the approval |
| 10. Merge, tag, release | OWN | OWN | REL | The release |

## Escalation

| Situation | Escalate to | When |
| --- | --- | --- |
| Agents disagree on an approach | `architect` | Before the build |
| A change touches the information model or a contract of the change sets | `architect`, then the owner for an ADR | Before the build |
| A failure has no obvious cause | `failure-analyst` | Before any fix |
| The same error occurs 2 times | In a `higher-risk` change, the advisor, or `fallback-advisor` if it is not available. In other work, `failure-analyst` | At once |
| The same fix fails 3 times | `architect` and `failure-analyst` | Stop the work |
| A secret, transcript text or consumer project data is in a diff, a fixture, a doc or a commit | The owner | At once; stop the commit |
| A new dependency, language or tool | `architect`, then the owner | Before the change |
| GitHub Actions does not start the jobs | The owner; run the jobs locally | Before the merge |
| A run can exceed a cost cap or a usage limit | The owner | Before the run |
| A step leaves the working tree: commit, push, pull request edit, tag, issue | The owner | Before the step |

## Decision authority

| Question | Decides |
| --- | --- |
| What do we build, and when? | The owner |
| How does it work? Which contract changes? | `architect` |
| Is an ADR or an amendment accepted? | The owner |
| Does a diagnostic exist, and with which severity? | `architect` (A), `diagnostics-engineer` (R) |
| Which tests prove the change? | `test-engineer` for the strategy, the responsible agent for its tests |
| Is the text correct STE? | `docs-editor` |
| Why did it fail? | `failure-analyst` |
| Is the work done? | `checker` reports, and the owner accepts |
| Do we add a dependency? | The owner, after the assessment of `architect` |
| Is the release ready? | The owner, with the checklist of `release-engineer` |
| Does a change break rule zero? | The owner. If there is a doubt, treat it as a break. |

## Quick lookup by path

This table gives the responsible agent (R) of each tracked path. The work areas and the change sets give the A of each task. `(OWN)` marks a path where each change needs the approval of the owner. `test-engineer` owns the policy of `tests/golden/` and `tests/fixtures/`.

```text
src/main.rs                          COR: arguments, help, --version, dispatch, exit codes, new. A command handler: the agent of the command
src/config.rs                        COR
src/model.rs, src/parse.rs           COR
src/frontmatter.rs, src/hash.rs      COR
src/glob.rs, src/repo.rs             COR
src/git.rs, src/index.rs             COR
src/templates.rs                     COR
src/layer.rs                         COR: the layer graph, its levels, the file order and `layer add`/`layer list`
src/change.rs                        COR: the change model, the delta lines, the tasks of a plan
src/lifecycle.rs                     DIA: the checks of a change (SD905, SD910 to SD917, SD919 to SD925) and the lifecycle base
src/approval.rs                      DIA: the walk of the commits of a branch and the approval commits (ADR 0012)
src/approve.rs                       COR: approve, the status commit of a person
src/record.rs                        COR: the acceptance record (D3 of 0.8.0): the reader, the rules of a valid record, the tree hash over `paths`, the brief and the writer of the skeleton. DIA is C for the rules that SD928 reports
src/accept.rs                        COR: `accept` and `accept --brief`, the skeleton and the brief of the acceptance record
src/acceptance.rs                    DIA: the rule "a change that needs a record", the archive file, the compare of the brief, and the checks of the record that `finish` and `check` share (SD927 to SD931, SD933, SD935). HYG is C for step 5.2 and 5.5 of `finish`
src/finish.rs                        HYG: `finish`. It edits documents, so HYG owns it
src/owners.rs                        COR: the table `[owners]`, the glob match and the agent roles of a change (D5 and D77 of 0.8.0). DIA is C for SD933
src/rollup.rs                        HYG: the roll-up: the pins `rests-on`, the frontier of a summary, `refresh` and the re-pin of `finish` (D50 to D53 of 0.8.0). DIA is C for SD641 to SD645
src/stage.rs                         COR: the stage derivation, `next` and `status`, the skill names
src/sdlc.rs                          COR: `log` and `stats --sdlc`
src/import.rs                        INI: `import`, the command, its write and its report
src/import_config.rs                 COR: the plan of `spec-dna.toml` for `import` and the file of `--areas`
src/import_docs.rs, src/import_speckit.rs, src/import_openspec.rs, src/import_proof.rs   HYG: the plan of the documents, the readers of the 2 tools and the proof of `import`
src/upgrade.rs                       INI: `upgrade`, the command, its write and its report
src/upgrade_old.rs, src/upgrade_config.rs   COR: the 0.5 reader and the plan of the configuration
src/upgrade_docs.rs, src/upgrade_proof.rs   HYG: the plan of the documents (front matter, moves) and the proof
src/config_text.rs                   COR: the edits of TOML text by path
src/evals.rs                         COR: the eval file form and its rules, which SD506 and `eval run` share
src/eval_run.rs                      COR: `eval run`, and the attempt that `--ab` uses (the refused run, the back-off)
src/eval_ab.rs                       COR: `eval run --ab`: the arms, the pairs and the result file
src/verdict.rs                       COR: the bootstrap, the rules of the verdict, the table of the release, the rule of ADR 0023 and its constant, and the reader of the results (`eval verdict`, SD507)
src/eval_records.rs                  COR: `eval records`, the reader of a result file of `--ab` and the writer of the records
src/eval_pack.rs                     INI: the evals of the pack, their fixtures and `eval pack`
src/eval_pack_pilot.rs, src/eval_pack_rest.rs, src/eval_pack_fixtures_rest.rs   INI: the data of the evals of the pack (15 pilot evals of `execute`, `tdd` and `debug`, 35 other evals and their fixtures)
src/eval_pack_harder.rs              INI: the evals of part B (`verify-6` to `verify-10`, `agents-line-1`, `agents-line-2`) and their fixtures
src/eval_pack_accept.rs              INI: the 5 evals `accept-1` to `accept-5` of the acceptance pass (D62) and their fixtures
src/harness.rs                       INI: the roles, the generated files, the settings merge. DIA: the function `check` (SD501, SD502, SD504, SD505)
src/harness_text.rs                  INI: the texts of the 11 skills of the workflow pack and `REVIEW.md`
src/hook.rs                          INI
src/skeleton.rs                      INI: the starter files of the evals and the eval workflow
src/json.rs                          COR: the writer. TRN: the transcript reader
src/check.rs, src/diag.rs            DIA
src/drift.rs, src/cite.rs            DIA. CTX is C for src/cite.rs
src/evidence.rs, src/lint.rs         DIA
src/report.rs                        DIA: the 2 readers of a test report (JUnit XML subset and the line format), the stamp and the read of the reports (D1)
src/tree.rs                          COR: the code tree and its hash, the second compare of the pinned hashes (D1). DIA is C for the paths that it excludes
src/context.rs, src/rank.rs          CTX
src/mv.rs, src/split.rs              HYG
src/registry.rs                      HYG
src/init.rs, src/detect.rs           INI
src/adapters.rs                      INI
src/transcript.rs                    TRN
src/release.rs                       REL
schema/spec-dna.schema.json          COR
docs/config.md, docs/types.md        COR. REL: the version in docs/config.md
docs/commands.md                     The agent of each command: its section. COR: the summary, the exit codes and the cache
docs/diagnostics.md                  DIA
docs/profiles.md                     DIA
docs/test-report.md                  DIA
docs/init.md, docs/quickstart.md     INI. REL: the version in docs/quickstart.md
docs/harness.md                      INI. DIA: the section "Check"
docs/agents.md                       INI
docs/evals.md                        COR. INI: the starter files and the CI job
docs/demo.md                         INI
docs/transcript.md                   TRN
docs/evidence.md                     ARC (OWN)
docs/adr/                            ARC (OWN)
docs/design/                         ARC (OWN)
docs/research/                       ARC
docs/README.md, docs/concepts.md     ARC: the facts. DOC: the language
docs/glossary.md, docs/guides/       ARC: the facts. DOC: the language
docs/assets/                         ARC (OWN)
tests/cli.rs                         The agent of each command: its cases. TST: the structure and the shared helpers
tests/config_layers.rs, tests/place.rs, tests/new_place.rs, tests/layer_cmd.rs, tests/templates_new.rs   COR
tests/sd202.rs                       DIA
tests/evidence_reports.rs            DIA: the test reports, the stamp, the bind and SD613 to SD617. TST: the fixtures of the 5 runners
tests/proof_debt.rs                  DIA: the proof debt, SD618 and SD619, and the proof counts of `status` and `stats`. COR: the counts
tests/layers_checks.rs, tests/sd921.rs, tests/task_checks.rs, tests/lifecycle_checks.rs, tests/lifecycle_base.rs, tests/lifecycle_finished.rs, tests/lifecycle_additions.rs, tests/review_fixes_step2.rs   DIA
tests/finish.rs                      HYG
tests/branch_approval.rs             DIA. COR: the cases of `next` and `status`
tests/approve_cmd.rs                 COR
tests/accept_cmd.rs                  COR: `accept`, the skeleton and the brief of the record. HYG is C for the cases of `finish`
tests/accept_archive_owner.rs         COR: the archive file that belongs to a change by its field `change`
tests/accept_followup.rs, tests/accept_stop.rs   COR: the follow-up of a note (D73) and the stop rule (D74). DIA is C for SD929 and SD941
tests/acceptance_need.rs             DIA: the rule "a change that needs a record" and the key `[change] acceptance-for` (D70, ADR 0032)
tests/owners.rs                      COR: the table `[owners]`, the roles of the brief. DIA is C for SD933
tests/rollup_checks.rs, tests/rollup_refresh.rs   HYG: the roll-up checks (SD641 to SD645) and `refresh`. DIA is C for the checks
tests/acceptance_checks.rs           DIA: the record in `finish` (steps 5.2 and 5.5) and in `check`, the archive file and SD927 to SD931, SD933 and SD935. HYG is C for the cases of `finish`
tests/tree_binary.rs                 COR
tests/change_model.rs, tests/change_config.rs, tests/git_lifecycle.rs, tests/lifecycle_commands.rs, tests/lifecycle_log.rs, tests/review_fixes.rs, tests/config_step2.rs   COR
tests/common/mod.rs, tests/common_helpers.rs   TST: the shared git fixtures. Every agent uses them
tests/init_change.rs, tests/init_language.rs, tests/init_profiles.rs   INI
tests/init_graph.rs                  INI
tests/harness.rs, tests/hook.rs      INI
tests/harness_roles.rs, tests/harness_skills.rs, tests/hook_protect.rs, tests/init_evals.rs   INI
tests/hook_approvals.rs              INI
tests/accept_next.rs                 INI: the item `accept` of `next`. COR is C
tests/skill_cmd.rs                   INI: the cases of `skill list` and `skill show`. COR: the arguments and the help
tests/harness_check_step4.rs, tests/evals_checks.rs   DIA
tests/config_step4.rs, tests/eval_run.rs, tests/eval_ab.rs, tests/eval_records.rs, tests/stats_evals.rs   COR
tests/fixtures/eval-records/         COR
tests/eval_pack.rs, tests/eval_pack_rest.rs   INI
tests/eval_pack_pilot/, tests/fixtures/eval-solutions/   INI: the reference and the wrong solutions that prove the checks of the pack
tests/context_layers.rs              CTX
tests/common/import.rs               TST: the fixture projects of `import`
tests/import_cli.rs                  INI
tests/import_speckit.rs, tests/import_openspec.rs   HYG
tests/upgrade_cli.rs, tests/upgrade_review_fixes.rs   INI
tests/upgrade_config.rs, tests/upgrade_pin.rs   COR
tests/upgrade_first_pr.rs            DIA
tests/upgrade_fixtures.rs, tests/upgrade_v061.rs   TST
tests/golden/                        The agent of the command. INI: tests/golden/harness/ (also tests/golden/harness/old-skills/, the skill files that 0.6.1 wrote) and tests/golden/evals/. COR: tests/golden/upgrade/config/
tests/fixtures/corpus/               TST
tests/fixtures/brownfield/           INI
tests/fixtures/layouts/              INI
tests/fixtures/demo/                 INI
tests/fixtures/transcripts*/         TRN
tests/fixtures/upgrade/              TST: the projects of 0.4, 0.5 and 0.6.1
bench/tool/                          TST
bench/retrieval/                     CTX
bench/harness/                       TST
bench/team/                          TST: the tasks, the oracle and the tools of the model-team measurement (ADR 0029), accepted by ARC
bench/team2/                         TST: the recipes, the tasks, the oracle and the tools of protocol 2 of the model-team measurement (ADR 0033), accepted by ARC
bench/results/tool-*.json            TST
bench/results/harness-*.json         TST
bench/results/retrieval-*.json       CTX
bench/results/evals/                 COR: the records of the eval runs of a release. The main session imports them, and the owner approves
bench/results/ (other files)         ARC: the evidence of ADR 0002
bench/go/, bench/rust/, bench/gen/   ARC
bench/git-access/, bench/run.py      ARC
bench/README.md, bench/.gitignore    ARC
scripts/demo-replay.sh               INI
scripts/cross-build.sh               REL
scripts/smoke.sh                     REL
scripts/release-assets.sh            REL
scripts/verify-release.sh            REL
tests/release_check.rs               REL
.github/pull_request_template.md     ARC (OWN)
.github/                             REL: the other files
.pre-commit-config.yaml              REL
Cargo.toml, Cargo.lock               COR: a dependency (OWN). REL: the version (OWN)
CHANGELOG.md                         REL: the file and the release sections. The agent of a change: its entry (OWN)
README.md                            DOC: the language. REL: the version. ARC: the rest (OWN)
AGENTS.md, CONTRIBUTING.md           ARC (OWN). COR: the dependency table in CONTRIBUTING.md (OWN)
SECURITY.md                          ARC (OWN)
.editorconfig, .gitattributes        ARC (OWN)
.gitignore, .markdownlint-cli2.yaml  ARC (OWN)
_typos.toml                          ARC (OWN)
CLAUDE.md, .claude/                  ARC (OWN)
spec-dna.toml                        ARC (OWN): the configuration of this repository. The pin moves with the version in the release pull request (see "Release" and ADR 0013)
docs/intent/                         ARC (OWN): the intents of the open changes. `new intent` writes them and `finish` deletes them
LICENSE                              OWN
```
