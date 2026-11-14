# Glossary

Each term has one meaning in spec-dna. The pages of this documentation use these terms and plain words only. The column "Name in 0.5" gives the old name of a term that version 0.6.0 renamed.

| Term | Meaning | Name in 0.5 | Page |
| --- | --- | --- | --- |
| adapter | The part of spec-dna for one agent: its instruction file, and for some agents a harness, evals and a transcript reader | | [agents](agents.md) |
| advisor | A second, stronger model that the main model consults before a decision | | [harness](harness.md) |
| alias | An earlier id of a document or a requirement. A reference to an alias resolves to the current id | | [concepts](concepts.md#ids-and-requirements) |
| amendment section | A section of an accepted decision that can grow after the acceptance. The default name is `Amendments` | | [config](config.md) |
| approve | Set the status `accepted` or `approved` in a commit, and merge the commit with a pull request | | [change guide](guides/change.md) |
| area | A topic folder under `requirements/` | | [config](config.md) |
| base | The commit that `check` compares with the head. Without `--base`, it is `HEAD`. With `--base <rev>`, it is the merge base of `<rev>` and `HEAD` | | [concepts](concepts.md#the-gate) |
| base tree, head tree | The files at the base commit and at the head of a change. A gate reads an approval from the base tree | | [concepts](concepts.md#the-change-lifecycle) |
| budget | The soft and the hard limit of the body characters of one document | | [config](config.md#budgets) |
| change | The work that one intent folder describes, from the intent to `finish` | | [change guide](guides/change.md) |
| change class | The weight of a change: `small-fix`, `routine` or `higher-risk` | | [change guide](guides/change.md#choose-the-class) |
| change policy | The change classes that a project uses: `three-class`, `two-class` or `none` (`[change] policy`) | | [config](config.md) |
| code citation | A file path or a symbol in a code span of a document. `check` reports a citation that does not resolve | | [diagnostics](diagnostics.md#code-citations) |
| concern | An item of the task list in the `Concerns` section of a spec. An approved spec has no open concern (SD919) | | [change guide](guides/change.md) |
| `covers:` tag | A comment in a test, such as `# covers: valves#shutoff`, that links the test to a requirement | | [concepts](concepts.md#ids-and-requirements) |
| current truth | The requirements, rules and interfaces of the project, in `requirements/`, `rules/` and `interfaces/`. A delta changes them, and SD921 guards them | | [concepts](concepts.md#layers-and-the-graph) |
| decision | A MADR record of a choice. After the acceptance, its text is locked, except its amendment, history and live sections | | [types](types.md) |
| delta | The list of `ADD`, `CHANGE` and `REMOVE` lines in a spec. Each line names a requirement id | | [change guide](guides/change.md) |
| diff risk | The class R0, R1 or R2 of a change of requirement text | change-risk class | [ADR 0001](adr/0001-information-model.md) |
| document id | The path of a document in its root, without `.md` | | [concepts](concepts.md#ids-and-requirements) |
| eval | A task with checks that tests the agent setup of a project | | [evals](evals.md) |
| evidence | A test with a `covers:` tag, or a `verified-by` path, that proves a requirement | | [config](config.md#evidence) |
| `finish` | The command that applies the delta of a change, checks the evidence and deletes the intent folder | | [change guide](guides/change.md) |
| foundation | The layer of the top files `product.md`, `system.md` and `terms.md` | | [concepts](concepts.md#layers-and-the-graph) |
| front matter | The YAML block at the start of a document, with fields such as `type` and `status` | | [types](types.md) |
| gate | A rule that blocks the next stage of a change until the document of the stage before has approval. spec-dna also calls `check` a gate | | [concepts](concepts.md#the-gate) |
| group | A named set of areas in the configuration. It is a view and never a folder | | [config](config.md) |
| harness | The agent setup that an adapter writes: roles, skills, hooks and `REVIEW.md`. Today only the Claude Code adapter writes a harness | | [harness](harness.md) |
| hash | The first 16 hexadecimal digits of SHA-256 over the normalized text of a requirement or a section | | [concepts](concepts.md#ids-and-requirements) |
| `higher-risk` | A change class with an intent, a spec and a plan. The owner approves the plan | | [change guide](guides/change.md#choose-the-class) |
| hook | A command that the agent runs on an event, such as `PreToolUse` of Claude Code | | [harness](harness.md#hooks) |
| incident | A post-mortem record of one event, in `incidents/` | | [types](types.md) |
| indicator | A measure of one stage of the playbook that `stats --sdlc` prints. It is leading or lagging | | [concepts](concepts.md#the-change-lifecycle) |
| instruction file | The file that an agent reads at the start of each session, such as `AGENTS.md` or `CLAUDE.md` | | [agents](agents.md) |
| intent | `intent.md`: the request of one change, in the words of its originator | type `change` | [change guide](guides/change.md) |
| intent folder | `docs/intent/<slug>/`: the folder of one change, with `intent.md`, `spec.md` and `plan.md`. `finish` deletes it | | [change guide](guides/change.md) |
| interface | An API, an event or a tool schema, in `interfaces/` | type `contract` | [types](types.md) |
| layer | A named set of document types with a place in the graph of stability | | [concepts](concepts.md#layers-and-the-graph) |
| level | The longest `moves-to` path from a layer to a layer with no outgoing edge. Level 0 is the most stable | | [concepts](concepts.md#layers-and-the-graph) |
| live section | A section of an accepted decision that can change without an amendment. The default is `Status` | | [config](config.md) |
| locked | The state of the text of an accepted decision outside its amendment, history and live sections. SD405 checks it | frozen | [diagnostics](diagnostics.md) |
| mixed root | A root with documents of more than one type. The place or the front matter gives the type of each document | | [types](types.md#place-gives-the-type) |
| `moves-to` | An edge of the layer graph: the text of the source layer matures into the target layer | | [concepts](concepts.md#layers-and-the-graph) |
| pass rate | The share of evals that pass | | [evals](evals.md) |
| pending | The layer of the top files `roadmap.md` and `questions.md` | | [concepts](concepts.md#layers-and-the-graph) |
| pin | The version of spec-dna in `spec-dna.toml` (`spec-dna = "<version>"`), with the sums of its binaries as an option | | [config](config.md#the-pin-comes-first) |
| plan | `plan.md`: the approved plan of one change, with its files, order, risks and proof | | [change guide](guides/change.md) |
| procedure | A guide or a runbook, in `procedures/` | types `guide`, `runbook` | [types](types.md) |
| profile | A set of default severities of the diagnostic codes: `lite`, `standard` or `strict` | | [profiles](profiles.md) |
| requirement | A list item with a bold key and a BCP 14 keyword, such as SHALL | | [concepts](concepts.md#ids-and-requirements) |
| requirement id | `<document id>#<key>`: the global form of a requirement | | [concepts](concepts.md#ids-and-requirements) |
| `requirements` | The type of a document that holds the requirements of one capability | type `spec` | [types](types.md) |
| research | Material that agents read only on request, in `research/` | types `note`, `reference` | [types](types.md) |
| `REVIEW.md` | The review policy of the repository. A code review service, such as Claude Code Code Review, reads it | | [harness](harness.md) |
| role | A job in the harness, such as `verifier`, with its model and its agent file | | [harness](harness.md#roles) |
| root | A folder that `spec-dna.toml` maps to document types. Document ids are relative to it | | [config](config.md) |
| `routine` | A change class with an intent and a plan. The agent approves the plan | | [change guide](guides/change.md#choose-the-class) |
| rule | A project-wide requirement with a stable id, in `rules/` | type `invariant` | [types](types.md) |
| section id | `<document id>#<heading slug>`: the unit that `context` gives to an agent | | [concepts](concepts.md#context) |
| skill | A `SKILL.md` folder that gives a procedure to the agent | | [harness](harness.md) |
| slug | The name of a change or a document in an id, in lowercase words with hyphens, such as `sensor-timeout` | | [change guide](guides/change.md) |
| `small-fix` | A change class with no intent: a pull request only | | [change guide](guides/change.md#choose-the-class) |
| spec | `spec.md`: the delta, the interfaces, the decisions and the concerns of one higher-risk change. It never holds the current truth | | [change guide](guides/change.md) |
| stable layer | A layer of level 0, such as `rules` or `decisions`. The current truth is in stable layers | | [concepts](concepts.md#layers-and-the-graph) |
| stage | One of Plan, Design, Build, Test, Deploy and Maintain. spec-dna derives the stage of a change from its files and git | | [change guide](guides/change.md) |
| standard document | A document form that a team already knows, such as an ADR or a PRD | | [types](types.md) |
| subagent | An agent with its own context, which the main agent starts. The Claude Code adapter defines them in `.claude/agents/` | | [harness](harness.md) |
| template | The text that `spec-dna new` writes for a new document | | [types](types.md#templates) |
| `terms` | The type of `terms.md`: a table of terms, meanings and forbidden synonyms. SD904 reads it | | [types](types.md) |
| top file | A document of a layer that has a fixed file name, such as `terms.md` or `roadmap.md` | | [types](types.md#place-gives-the-type) |
| `verified-by` | A front matter field with the paths of the tests or verification records of a document | | [config](config.md#evidence) |
| `verifier` | The subagent that runs the work and reports. It fixes nothing | role `reviewer` | [harness](harness.md#roles) |
