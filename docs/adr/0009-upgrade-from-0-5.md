---
status: proposed
---

# 0009. Upgrade and migration from 0.5

## Context and Problem Statement

0.6.0 renames document types, statuses, configuration keys, folders, commands and harness roles at once ([ADR 0005](0005-layout-and-layers.md), [ADR 0008](0008-harness-and-evals.md)). The binary keeps no runtime alias. A 0.6 binary also refuses a 0.5 configuration, because the pin differs ([ADR 0003](0003-distribution.md), amendment "phase 2", item "Pin first"). How does a 0.5 project move to 0.6 in one reviewable change, and keep each id, each hash and the gate?

Constraints:

- The owner decisions for 0.6.0 are final: renames at once, no runtime alias, and `spec-dna upgrade` with a dry-run.
- [ADR 0001](0001-information-model.md): ids and hashes stay, and spec-dna never deletes a document. Each deletion batch needs the approval of the owner. A gate needs only spec-dna, git and 2 trees.
- ADR 0003: the pin is exact, an upgrade is one reviewable commit, and a revert undoes it. A new rule starts as a warning in a minor release.
- [ADR 0004](0004-harness.md): a hand edit stays, and spec-dna never replaces a file that it did not generate.
- A brownfield project keeps its folders where the model allows it (ADR 0005, "Layout"). spec-dna uses no network.

Evidence (the data of earlier versions stays outside this repository):

- The gate against the base gave the measured gain. Thus the upgrade must keep the gate, also for the pull request of the upgrade.
- Agents search the repository. They seldom follow a tree of links. Thus an agent copies an old name that stays valid in the files.
- The upgrades from 0.1 to 0.5 changed no document. `init` updated the adapters after a hand edit of the pin.

## Considered Options

Option 5, `spec-dna upgrade`, is a command of the binary. It plans the change in memory, proves it, and then writes it.

| Option | Old name valid after the upgrade | Reviewable diff | Proof before the write | Gate of the upgrade pull request | New tool or runtime |
| --- | --- | --- | --- | --- | --- |
| 1. Runtime aliases in each command | Yes, always | No diff | None | Yes | No |
| 2. Both names for 2 minor versions | Yes, for 2 releases | Only at the end of the period | None | Yes | No |
| 3. Migration inside `init --reconfigure` | No | Mixed with new answers | Possible | Possible | No |
| 4. A separate script or binary | No | Yes | Possible | No: CI has only the binary | Yes |
| 5. `spec-dna upgrade` | No | Yes: renames only | Yes | Yes | No |

## Decision Outcome

Option 5. The design page holds the tables: [Upgrade](../design/0.6.0.md#upgrade), [Mapping from 0.5](../design/0.6.0.md#mapping-from-05), [Names and aliases](../design/0.6.0.md#names-and-aliases), [Configuration](../design/0.6.0.md#configuration) and [Checks](../design/0.6.0.md#checks). This ADR gives the rules and their reasons.

### Source versions

- `upgrade` accepts a pin from 0.4.0 to the version of the binary. It reads the pin with the line scan of "Pin first", because the 0.6 schema rejects the keys and type names of a 0.5 file.
- The reason for 0.4.0: each binary from 0.4.0 has the "Pin first" rule. After the upgrade, a teammate with the old binary gets the install command, not a parse error. A 0.4 configuration is a 0.5 configuration with no `[harness]` table, so one reader serves both.
- An older pin moves to 0.4.2 first, in a pull request of its own, by the steps of ADR 0003. The repository does not show that an older configuration has another form. The owner confirms this minimum version at acceptance.
- The target is the version of the binary, because the binary embeds the standard. Thus `upgrade` has no `--to <version>`. It is also the command of ADR 0003 for each later change of the pin.

### Steps

`spec-dna upgrade [--dry-run] [--layout] [--pin-file <file>]` does these steps. It stops at the first failure, and it writes nothing before step 4.

1. Read. Refuse a staged change, an unstaged change and an untracked file that git does not ignore. `--dry-run` has the same rule, so its diff is the diff of a write.
2. Plan. Compute the new configuration, front matter, adapters and harness files in memory. Also compute the moves of the changes, and with `--layout` the other moves.
3. Proof. Run the proof below on the plan.
4. Write. Write the files, and move with `git mv`. `--dry-run` prints the diff of each file and stops before this step.
5. Report. Print the report and the `git add` command. Never commit.

A second run finds nothing to change and exits with code 0. A run with the pin of the binary rewrites only the 0.5 names that remain. Thus a branch that merges the default branch after the upgrade runs `upgrade` again for its own new documents.

`upgrade` uses the exit codes of [commands.md](../commands.md#exit-codes):

- 0 after a write, or when nothing changes. 1 when it refuses a tree that is not clean, or when the proof fails. 2 for a usage error, and 4 for a git error.
- 3 for a configuration error: no `spec-dna.toml`, a pin before 0.4.0 or after the binary, or a file that neither schema reads. `[spec-dna-sha256]` with no valid `--pin-file` also gives 3.
- `--json` has `from`, `to`, `files`, `proof` and `report`. Each entry of `files` has `path`, `action` (`write`, `move`, `remove` or `keep`) and `diff`, as in `init --json`.
- The report lists the deletion proposals and each decision with no `Status` section. It lists each locked decision that cites a migrated change, because `finish` does not stop on a reference in locked text. It also lists each kept harness file, each ended role with its model, and the changes of behavior that the [design page](../design/0.6.0.md#upgrade) lists. Last, it gives the count of each error code of `check --all` on the new tree.

### Configuration

- `upgrade` keeps the comments and the order of keys in `spec-dna.toml`, so the diff shows only the renames and the new tables. The method is open: the `toml` crate gives source spans, so `upgrade` can rewrite a value in place, and `toml_edit` is a new dependency that needs the decision of the owner. The recommendation is to use the spans, append the new keys as a block, and add no new dependency.
- It renames each 0.5 type in `types`, `roots.type`, `roots.default-type` and the keys of each table by type. It renames each `change` status in `roots.status` and `roots.status-map`.
- When 2 old types with explicit keys merge into one, it keeps the weaker value. That is the larger budget, the required sections that both share, and the union of other section names.
- It writes an explicit key where a 0.6 default would change a 0.5 document: its type, required sections, live sections or budget. The keys are these:
  - `default-type = "research"` for a root with the 0.5 default `note`;
  - `required-sections = { procedure = ["Steps"] }` for a root of type `runbook`;
  - `live-sections = {}` for a root with none, so no 0.5 decision gets a live section by default;
  - `required-sections = { intent = [] }`, because a `change` had no required section;
  - `[budgets.intent]` with 10,000 and 20,000 (the 0.5 budget of a `change`), when a migrated change is over the 0.6 budget of `intent`.
- The field `class` of a migrated `intent` is optional, so `upgrade` writes none.
- A root of type `note` becomes a `mixed` root with `default-type = "research"`. A root of type `research` adds a namespace to each id (ADR 0001, amendment of 0.3.0), so the `mixed` root keeps the ids.
- It writes `[change] policy = "none"`, and `[layers]` for the types in use without the limit keys. A project with no `[layers]` table would get the limits of the default graph.
- No new code gives an upgraded project an error. SD901 to SD904 apply to each project, so they are warnings at most (ADR 0003).
- SD506, SD905 and SD910 to SD921 apply only with `[evals]`, or with `[change]` and a policy other than `none`. Thus they can be errors from the start, as ADR 0004 made its harness codes. SD911 and SD912 also need a policy.
- It sets the pin to the version of the binary. If the file has `[spec-dna-sha256]`, `--pin-file` must name the `spec-dna-pin.toml` of that version, which the person verifies with `gh attestation verify`. `upgrade` copies its sums, because the binary cannot know its own sums.

### Documents, harness and layout

- `upgrade` rewrites front matter: each 0.5 `type`, and each status that the mapping changes. It changes a body only for the link destinations of a move.
- A `change` becomes an `intent`: `proposed` gives `draft`, `accepted` stays, and `rejected` gives `closed`. The type `intent` is valid only in the `intent` layer, so `upgrade` always moves the change to `intent/<slug>/intent.md` with `git mv`.
- The move writes the old id and each old legacy alias into `aliases`, because the file stem `intent` matches no `legacy-ids` pattern. It changes the relative links, as `mv` does.
- `upgrade` also writes the 0.6 adapters, as `init` does: the line of each instruction file, and the skill with the new types.
- A `done` change becomes a `research` document in place, because [ADR 0006](0006-change-lifecycle.md) rejects a `done` state. `upgrade` removes its `status` line and writes `was-type: change`.
- A `note` becomes a `research` document with `was-type: note`. The marker keeps the 0.5 right to delete such a document (see "Deletion proposals").
- `type: spec` outside the `intent` layer is a 0.5 spec, so `upgrade` writes `requirements`. A 0.5 spec that a branch brings after the upgrade gives SD303, and the message names `requirements`. `upgrade` also writes `type:` where the place now gives another type.
- The harness follows ADR 0008, section "Rename, `upgrade` and `check`". The roles get their new names, the new keys add no file, and the team stays the 0.5 team.
- `upgrade` deletes an unedited `checker.md` and `spec-writer.md`, and it writes `verifier.md`. An agent file is not a document, so the rule against deletion of ADR 0001 does not apply.
- An edited generated file stays, and `check` gives SD501 for it before and after the upgrade. A file with no marker at a role path stays, and its role becomes `none`. Thus `upgrade` adds no SD501, SD502 or SD504.
- Without `--layout`, only the changes move, and `[[roots]]` maps the other brownfield folders to the new types. `--layout` moves each file of the other 0.5 namespace folders (`contracts`, `invariants`, `guides`, `runbooks`, `reference`, `notes`) into the folder of its 0.6 type. It writes the old id into `aliases` and changes the relative links, as `mv` does.
- A `done` change stays, because it is a deletion proposal. A 0.5 spec stays, because only a person can choose its area. If 2 files get one new path, the plan stops.

### Proof

The proof reuses the preservation check of `mv` (ADR 0001, amendments "batch split" and "rulings after checkpoint 7"). It compares the plan with the tree of `HEAD`:

- Each old id resolves to the same object: each document, section and requirement id, each alias and each legacy id.
- Each requirement and each section has the same hash. The hash excludes link destinations, because a move changes links.
- Each document has the 0.6 type that the mapping gives for its 0.5 type.
- The new configuration parses, and the layer graph has no cycle.

If one item fails, `upgrade` writes nothing. It names each failure and exits with code 1.

### First pull request

- The base of the first pull request after the upgrade has a pin before 0.6.0. Its documents use 0.5 types and statuses, which 0.6 rejects, so a direct comparison gives false findings. In this case, `check` runs the plan of `upgrade` on the base tree in memory, without `--layout`. The plan takes the pin and the sums of the head. Then `check` compares the head with the result, as usual.
- A finding that the upgrade made is in both trees, so it is not new. A moved document keeps its old id as an alias, so the fingerprints match after a move too.
- `check` reads the base pin with the line scan of "Pin first". A pin change inside 0.6, such as 0.6.0 to 0.6.1, keeps the usual comparison.
- SD503 does not apply in this case, so one pull request holds the configuration and the documents. The gate still compares each document.
- A base pin before 0.4.0, or a base that the plan cannot upgrade, stops `check` with exit code 3. Thus CI checks the upgrade with only the binary, git and 2 trees.
- The [test plan](../design/0.6.0.md#test-plan) proves this case against a 0.5 base, with and without `--layout`, and with a decision that cites a migrated change.

### Names and aliases

- The [table](../design/0.6.0.md#names-and-aliases) gives the rule for each kind of name. A runtime alias is an old name that the binary accepts as valid input, and 0.6 has none.
- The binary keeps the 0.5 names in one place: the plan of `upgrade`. `upgrade` reads it, also in its run on the base for `check`. The binary also reads it to name the new name in an error message.
- SD202 also compares the field `was-type` with 2 words, `note` and `change`. This is data that `upgrade` wrote, as `aliases` is. No use makes an old name valid in the head.
- A document id is data, not a word of the standard. A move writes the old id into `aliases`, and the id model of ADR 0001 reads it. Thus SD202 and the alias rule of `mv` stay.
- Without `--layout`, only the changes move, and each keeps its old id in `aliases`. A requirement key never changes. `upgrade` renames no decision file, so `legacy-ids` still gives `ADR-NNN`.
- Types, statuses, configuration keys, folder names, commands and roles get no alias. They are words of the standard, and no document cites them as an id.

### Deletion proposals, `init` and rollback

- `upgrade` lists a `done` or `rejected` change as a proposal only when no document cites it. The owner confirms the second condition of ADR 0001: the decisions of the change exist as decision documents.
- The owner approves each batch, and a later pull request deletes it. A closed intent is in an `until-finished` layer, so its deletion gives no SD202.
- SD202 does not report the deletion of a document with `was-type: note` or `was-type: change`, as 0.5 did not. This is the only exemption of a `forever` layer. It is narrow: a new `research` document has no marker, and a former `reference` never had the exemption.
- `init` and `init --reconfigure` do not run when the pin differs, and on a 0.5 project their message names `spec-dna upgrade`. After the upgrade, `init --reconfigure` offers the new parts: a change policy, the gate, the stage skills, `REVIEW.md`, the evals and the layer limits.
- Before the commit, `git reset --hard` and `git clean -fd` restore the tree, because `upgrade` starts from a clean tree. After the merge, the rollback is a `git revert` and the install of the old binary (ADR 0003).

### Rejected

- A compatibility mode that reads 0.5 names at run time: it is a runtime alias. Agents copy the old name, so the mode stays forever. ADR 0005 rejects the same table.
- A dual-read period over 2 minor versions: each check and test handles 2 forms for 2 releases. The renames still need `upgrade` at the end, so the cost occurs twice.
- Folder moves by default: a move changes each id and needs aliases, and its diff hides the renames. The changes are the one exception, because `intent` is valid only in its layer.
- A change that keeps its path as an `intent` document: it would be an intent outside an intent folder. No command of a change could read it, and the place rule of ADR 0005 would need an exception.
- Automatic deletion of `done` changes: ADR 0001 requires the approval of the owner for each batch, and a cited change must stay.
- The migration inside `init --reconfigure`: `init` never edits a document (ADR 0001, amendment of 0.2.0). It mixes new choices with renames, and it drops the comments of the file.
- A separate migration script or binary: it needs a second runtime, or a second release asset with its own sums. CI then has no plan for the base of the first pull request.
- A rewrite of git history: it changes each commit id and breaks each clone and open branch. It also removes the archive of ADR 0001.
- A transform of the base in memory for every `check`, forever: it is a dual-read period with no end. The base case applies only to a base pin before 0.6.0.
- "Report nothing as new" in the first pull request: the pull request that touches each document passes with no gate. CI cannot see a local proof.
- An `upgrade` that commits by itself: the author must read the diff first. A commit runs the hooks of the author and carries the signature of the author.
- An `upgrade` that drops `[spec-dna-sha256]`: the CI job of `init` then builds from source, and the verification of the binary stops with no notice.
- The limits of the default graph on an upgraded project: a 0.5 corpus has no data for a limit. Each limit gives findings that the owner did not choose.
- A `done` change as an `intent` with the status `accepted` or `closed`: an open change and a finished change would look the same. `closed` means that the change is not going on.

### Consequences

- Good: one command and one diff move a project to 0.6. The proof shows that each id and each hash stays, and the gate stays active for the upgrade pull request.
- Good: an upgraded project gets no new gate and no new generated file. The owner adds each new part with `init --reconfigure`.
- Bad: the 0.6 binary keeps the reader of the 0.5 configuration and the plan of `upgrade`, and `check` uses them for one case.
- Bad: 0.6.0 renames types, keys and commands, so this minor release breaks compatibility. Semantic Versioning allows this before 1.0, and ADR 0003 limits only the severity of a new rule.
- Bad: the binary of the old pin checks a revert after the merge. Its SD503 reports the configuration and the documents together, so a revert takes 2 pull requests. SD202 reports each moved id in the second one: each migrated change, and with `--layout` each moved document.
- Bad: a branch that started before the upgrade needs a merge and a second run of `upgrade`.

## Amends

ADR 0001, ADR 0003 and ADR 0004 are accepted, and their text is locked. Read them with these changes:

- ADR 0001, "Lifecycle", row `change`: the proposal of a migration is the report of `upgrade`. A `done` change is a `research` document with `was-type: change` until the deletion.
- ADR 0001, "Metadata", field table: one new optional field, `was-type`. Only `upgrade` writes it, and SD202 reads it. `new` removes it from its copy of a template, so a new document never gets the exemption. The prefix `x-` stays free for fields that spec-dna ignores.
- ADR 0001, "Identifiers": SD202 does not report a deleted document with `was-type: note` or `was-type: change`.
- ADR 0001, "Growth controls": when the base pin is before 0.6.0, the gate first runs the plan of `upgrade` on the base tree.
- ADR 0001, amendment of 0.3.0, item "Configuration changes": SD503 does not apply when the base pin is before 0.6.0.
- ADR 0001, amendment of 0.2.0, item "Adoption": `init` still never edits a document. After the proof, `upgrade` edits front matter and the link destinations of each move.
- ADR 0003, "Decision Outcome", item "Upgrade": `upgrade` has no `--to`. It writes the pin of the binary, and it copies the sums from `--pin-file`. In the amendment "phase 2", item "Upgrade", it replaces step 2 and the `init` of step 4.
- ADR 0003, amendment of 2026-11-08, "Upgrade in phase 1": `init` still does not run when the pin differs. `upgrade` is the one command that runs with an older pin.
- ADR 0004, "`init` and `check`": `upgrade` also removes an unedited generated file of an ended role, as ADR 0008 states.

## Amendments
