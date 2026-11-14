---
status: accepted
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

1. Runtime aliases in each command.
2. Both names for 2 minor versions.
3. The migration inside `init --reconfigure`.
4. A separate migration script or binary.
5. `spec-dna upgrade`, a command of the binary. It plans the change in memory, proves it, and then writes it.

## Decision Outcome

Option 5. The design page holds the tables and the detail: [Upgrade](../design/0.6.0.md#upgrade), [Mapping from 0.5](../design/0.6.0.md#mapping-from-05), [Names and aliases](../design/0.6.0.md#names-and-aliases), [Configuration](../design/0.6.0.md#configuration) and [Checks](../design/0.6.0.md#checks). This ADR gives the rules and their reasons.

### Source versions

- `upgrade` accepts a pin from 0.4.0 to the version of the binary. Each binary from 0.4.0 has the "Pin first" rule. Thus after the upgrade, a teammate with the old binary gets the install command, not a parse error.
- An older pin moves to 0.4.2 first, in a pull request of its own, by the steps of ADR 0003. The repository shows no older form of the configuration, so the owner confirms this minimum version at acceptance.
- The target is the version of the binary, because the binary embeds the standard. Thus `upgrade` has no `--to <version>`.

### Run

- `spec-dna upgrade [--dry-run] [--layout] [--pin-file <file>]` starts only from a clean tree. It computes the whole plan in memory and runs the proof on it. It writes nothing before the proof passes. `--dry-run` also needs a clean tree. It prints the diff of each file and writes nothing.
- `upgrade` moves with `git mv`, and it never commits. A second run finds nothing to change and exits with code 0.

### Configuration

- `upgrade` keeps the comments and the order of keys in `spec-dna.toml`, so the diff shows only the renames and the new tables. The method is [open point 9](../design/0.6.0.md#open-points-for-the-owner).
- It renames each 0.5 type and each `change` status. When 2 old types merge into one, it keeps the weaker value of each key.
- It writes an explicit key where a 0.6 default would change a 0.5 document: its type, required sections, live sections or budget.
- A `note` root becomes a `mixed` root with `default-type = "research"`, because a `research` root adds a namespace to each id (ADR 0001, amendment of 0.3.0).
- It writes `[change] policy = "none"`, and `[layers]` for the types in use with no limit keys. Without `[layers]`, the project would get the limits of the default graph.
- It writes no `[evals]`, so an upgraded project keeps the evals `none`. `init` proposes `skeleton` only for a new project.
- No new code gives an upgraded project an error ([Checks](../design/0.6.0.md#checks)).

### Documents, harness and layout

- `upgrade` rewrites front matter: each 0.5 `type`, and each status that the mapping changes.
- A `change` becomes an `intent`. The type `intent` is valid only in the `intent` layer, so `upgrade` always moves the change to `intent/<slug>/intent.md`.
- A `done` change becomes a `research` document in place, because [ADR 0006](0006-change-lifecycle.md) rejects a `done` state. `upgrade` removes its `status` line and writes `was-type: change`. A `note` becomes `research` with `was-type: note`.
- The harness follows ADR 0008, section "Rename, `upgrade` and `check`": the team stays the 0.5 team under the new names. `upgrade` deletes an unedited `checker.md` and `spec-writer.md`, and it writes `verifier.md`. An agent file is not a document, so the rule against deletion of ADR 0001 does not apply.
- An edited generated file stays and gives SD501 before and after the upgrade. A file with no marker at a role path stays, and its role becomes `none`. Thus `upgrade` adds no SD501, SD502 or SD504.
- Without `--layout`, only the changes move, and `[[roots]]` maps the other brownfield folders. `--layout` also moves the files of the other 0.5 namespace folders, with aliases.

### Proof

The proof reuses the preservation check of `mv` (ADR 0001, amendments "batch split" and "rulings after checkpoint 7"). It compares the plan with the tree of `HEAD`:

- Each old id resolves to the same object: each document, section and requirement id, each alias and each legacy id.
- Each requirement and each section has the same hash. The hash excludes link destinations, because a move changes links.
- Each document has the 0.6 type that the mapping gives for its 0.5 type.
- The new configuration parses, and the layer graph has no cycle.

If one item fails, `upgrade` writes nothing. It names each failure and exits with code 1.

### First pull request

- The base of the first pull request after the upgrade has a pin before 0.6.0. Its documents use 0.5 types and statuses, which 0.6 rejects, so a direct comparison gives false findings. Thus `check` runs the plan of `upgrade` on the base tree in memory, without `--layout`. Then it compares the head with the result, as usual.
- A finding that the upgrade made is in both trees, so it is not new. A moved document keeps its old id as an alias, so the fingerprints match after a move too.
- SD503 does not apply in this case, so one pull request holds the configuration and the documents. The gate still compares each document.
- A base pin before 0.4.0, or a base that the plan cannot upgrade, stops `check` with exit code 3. Thus CI checks the upgrade with only the binary, git and 2 trees. The [test plan](../design/0.6.0.md#test-plan) proves this case.

### Names and aliases

- The binary keeps the 0.5 names in one place: the plan of `upgrade`. The binary reads it for `upgrade`, for the run on the base in `check`, and to name the new name in an error message.
- Types, statuses, configuration keys, folder names, commands and roles get no alias. They are words of the standard, and no document cites them as an id.
- A document id is data, not a word of the standard. A move writes the old id into `aliases`, and the id model of ADR 0001 reads it. Thus SD202 and the alias rule of `mv` stay.
- SD202 also compares the field `was-type` with 2 words, `note` and `change`. This is data that `upgrade` wrote, as `aliases` is. No use makes an old name valid in the head.

### Deletion proposals, `init` and rollback

- `upgrade` lists a `done` or `rejected` change as a proposal only when no document cites it. The owner confirms the second condition of ADR 0001 and approves each batch. A closed intent is in an `until-finished` layer, so its deletion gives no SD202.
- SD202 does not report the deletion of a document with `was-type: note` or `was-type: change`, as 0.5 did not. This is the only exemption of a `forever` layer. It is narrow: a new `research` document has no marker, and a former `reference` never had the exemption.
- After the upgrade, `init --reconfigure` offers the new parts: a change policy, the gate, the stage skills, `REVIEW.md`, the evals and the layer limits.
- Before the commit, `git reset --hard` and `git clean -fd` restore the tree. After the merge, the rollback is a `git revert` and the install of the old binary (ADR 0003).

### Rejected

- Runtime aliases, or a compatibility mode that reads 0.5 names at run time: agents copy the old name, so the mode stays forever. ADR 0005 rejects the same table.
- A dual-read period over 2 minor versions: each check and test handles 2 forms for 2 releases. The renames still need `upgrade` at the end, so the cost occurs twice.
- Folder moves by default: a move changes each id and needs aliases, and its diff hides the renames. The changes are the one exception, because `intent` is valid only in its layer.
- A change that keeps its path as an `intent` document: no command of a change could read it, and the place rule of ADR 0005 would need an exception.
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
- Bad: this minor release breaks compatibility. Semantic Versioning allows this before 1.0, and ADR 0003 limits only the severity of a new rule.
- Bad: the binary of the old pin checks a revert after the merge. Its SD503 makes the revert take 2 pull requests, and SD202 reports each moved id in the second one.
- Bad: a branch that started before the upgrade needs a merge and a second run of `upgrade`.

## Amends

ADR 0001, ADR 0003 and ADR 0004 are accepted, and their text is locked. Read them with these changes:

- ADR 0001, "Lifecycle", row `change`: the proposal of a migration is the report of `upgrade`. A `done` change is a `research` document with `was-type: change` until the deletion.
- ADR 0001, "Metadata", field table: one new optional field, `was-type`. Only `upgrade` writes it, and SD202 reads it. `new` removes it from its copy of a template, so a new document never gets the exemption. The prefix `x-` stays free for fields that spec-dna ignores.
- ADR 0001, "Identifiers": SD202 does not report a deleted document with `was-type: note` or `was-type: change`.
- ADR 0001, "Growth controls", and its amendment of 0.3.0, item "Configuration changes": when the base pin is before 0.6.0, the gate first runs the plan of `upgrade` on the base tree, and SD503 does not apply.
- ADR 0001, amendment of 0.2.0, item "Adoption": `init` still never edits a document. After the proof, `upgrade` edits front matter and the link destinations of each move.
- ADR 0003, "Decision Outcome", item "Upgrade": `upgrade` has no `--to`. It writes the pin of the binary, and it copies the sums from `--pin-file`. In the amendment "phase 2", item "Upgrade", it replaces step 2 and the `init` of step 4.
- ADR 0003, amendment of 2026-11-08, "Upgrade in phase 1": `init` still does not run when the pin differs. `upgrade` is the one command that runs with an older pin.
- ADR 0004, "`init` and `check`": `upgrade` also removes an unedited generated file of an ended role, as ADR 0008 states.

## Amendments
