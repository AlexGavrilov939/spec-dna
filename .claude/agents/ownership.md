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
- The advisor (Fable) and `fallback-advisor` give advice at a decision point. They are C when the main session asks them.
- `Explore` searches on request. It owns no task and no path.

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
| Harness role or generated file | `src/harness.rs` and `src/harness_text.rs`; `docs/harness.md`; `tests/golden/harness/`; the harness tests; `CHANGELOG.md` | ARC | INI, and DIA for the function `check` | COR, TST, DOC | REL |
| Hook | `src/hook.rs`; the `hook` section of `docs/commands.md`; the hooks section of `docs/harness.md`; `tests/hook*.rs` | ARC | INI | TST | REL |
| Eval file form or runner | `src/evals.rs` and `src/eval_run.rs`; the `eval` section and the exit codes of `docs/commands.md`; `tests/eval_run.rs`; SD506 in `docs/diagnostics.md` | ARC | COR | DIA, INI, TST | REL |
| Eval skeleton or workflow | `src/skeleton.rs`; question 14 in `src/init.rs`; `tests/golden/evals/`; `docs/init.md` | ARC | INI | COR, DIA, REL | DOC |
| New dependency | `Cargo.toml` and `Cargo.lock`; the dependency table of `CONTRIBUTING.md` | OWN | ARC (the assessment), COR (the change) | REL | All |
| Convention | `AGENTS.md`, `CONTRIBUTING.md`, `README.md` and `.github/pull_request_template.md`; lint and editor configs | OWN | ARC | DOC, REL | All |
| Release | `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`; the version in `README.md`, `docs/quickstart.md` and `docs/config.md`; the checkout SHA and the Claude Code version that `init` writes into a workflow (`src/init.rs`, `src/skeleton.rs`); the tag | OWN | REL | ARC, TST | All |
| CI or hooks | `.github/` except the PR template, `.pre-commit-config.yaml`, `scripts/cross-build.sh`, `scripts/smoke.sh`, `scripts/release-assets.sh` and `src/release.rs` | ARC | REL | TST | OWN |
| Agent team | `.claude/` and `CLAUDE.md`; the pointers in `AGENTS.md` and `CONTRIBUTING.md` | OWN | ARC | DOC | All |

## Lifecycle of a change

| Step | R | A | C | Output |
| --- | --- | --- | --- | --- |
| 1. Scope | OWN | OWN | ARC | The task and its limits |
| 2. Route | The main session | The main session | This file | The responsible agent and the change sets |
| 3. Design, if a contract or 2 or more modules change | ARC | ARC | The responsible agents, the advisor | A decision, or the draft of an ADR amendment |
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
| The same error occurs 2 times | The advisor; if it is not available, `fallback-advisor` | At once |
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
src/lifecycle.rs                     DIA: the checks of a change (SD905, SD910 to SD917, SD919 to SD922) and the lifecycle base
src/finish.rs                        HYG: `finish`. It edits documents, so HYG owns it
src/stage.rs                         COR: the stage derivation, `next` and `status`, the skill names
src/sdlc.rs                          COR: `log` and `stats --sdlc`
src/evals.rs                         COR: the eval file form and its rules, which SD506 and `eval run` share
src/eval_run.rs                      COR: `eval run`
src/harness.rs                       INI: the roles, the generated files, the settings merge. DIA: the function `check` (SD501, SD502, SD504, SD505)
src/harness_text.rs                  INI: the texts of the stage skills and `REVIEW.md`
src/hook.rs                          INI
src/skeleton.rs                      INI: the starter files of the evals and the eval workflow
src/json.rs                          COR: the writer. TRN: the transcript reader
src/check.rs, src/diag.rs            DIA
src/drift.rs, src/cite.rs            DIA. CTX is C for src/cite.rs
src/evidence.rs, src/lint.rs         DIA
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
docs/init.md, docs/quickstart.md     INI. REL: the version in docs/quickstart.md
docs/harness.md                      INI. DIA: the section "Check"
docs/demo.md                         INI
docs/transcript.md                   TRN
docs/evidence.md                     ARC (OWN)
docs/adr/                            ARC (OWN)
docs/research/                       ARC
tests/cli.rs                         The agent of each command: its cases. TST: the structure and the shared helpers
tests/config_layers.rs, tests/place.rs, tests/new_place.rs, tests/layer_cmd.rs, tests/templates_new.rs   COR
tests/sd202.rs                       DIA
tests/layers_checks.rs, tests/sd921.rs, tests/lifecycle_checks.rs, tests/lifecycle_base.rs, tests/lifecycle_finished.rs, tests/lifecycle_additions.rs, tests/review_fixes_step2.rs   DIA
tests/finish.rs                      HYG
tests/change_model.rs, tests/change_config.rs, tests/git_lifecycle.rs, tests/lifecycle_commands.rs, tests/lifecycle_log.rs, tests/review_fixes.rs, tests/config_step2.rs   COR
tests/common/mod.rs, tests/common_helpers.rs   TST: the shared git fixtures. Every agent uses them
tests/init_change.rs, tests/init_language.rs   INI
tests/init_graph.rs                  INI
tests/harness.rs, tests/hook.rs      INI
tests/harness_roles.rs, tests/harness_skills.rs, tests/hook_protect.rs, tests/init_evals.rs   INI
tests/harness_check_step4.rs, tests/evals_checks.rs   DIA
tests/config_step4.rs, tests/eval_run.rs, tests/stats_evals.rs   COR
tests/context_layers.rs              CTX
tests/golden/                        The agent of the command. INI: tests/golden/harness/ and tests/golden/evals/
tests/fixtures/corpus/               TST
tests/fixtures/brownfield/           INI
tests/fixtures/layouts/              INI
tests/fixtures/demo/                 INI
tests/fixtures/transcripts*/         TRN
bench/tool/                          TST
bench/retrieval/                     CTX
bench/harness/                       TST
bench/results/tool-*.json            TST
bench/results/harness-*.json         TST
bench/results/retrieval-*.json       CTX
bench/results/ (other files)         ARC: the evidence of ADR 0002
bench/go/, bench/rust/, bench/gen/   ARC
bench/git-access/, bench/run.py      ARC
bench/README.md, bench/.gitignore    ARC
scripts/demo-replay.sh               INI
scripts/cross-build.sh               REL
scripts/smoke.sh                     REL
scripts/release-assets.sh            REL
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
LICENSE                              OWN
```
