---
status: accepted
---

# 0006. Change lifecycle, gates, classes and finish

## Context and Problem Statement

In 0.5, a change is one document of the type `change`, with the states `proposed`, `accepted`, `done` and `rejected`. A `done` change stays in the tree until the owner deletes it. No rule ties a change to the stages of a team, and no check compares the work with an approved plan.

The owner asks for the stages of the [AI-native SDLC playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook): Plan, Design, Build, Test, Deploy and Maintain. How does spec-dna model a change from its intent to its merge? A finished change must leave no clutter in the hot path, which is the text that each task loads.

Constraints:

- A gate MUST run from the repository with only spec-dna and git ([ADR 0001](0001-information-model.md), "Layers"). Thus spec-dna uses no network and no API of a code host.
- The result of a check must not depend on the machine or on the clock.
- The rules must work with merge commits, squash merges and rebase merges.
- Each fact is in one place, and the repository has no committed registry (ADR 0001, "Concurrency").
- The terms follow the playbook. Where the playbook names nothing, spec-dna uses a plain STE word.
- An upgraded project gets no new gate.

Evidence: the facts of earlier versions in [Goal](../design/0.6.0.md#goal). A mature corpus also removed a gate on human-typed dates after false alarms (ADR 0001, "Context and Problem Statement").

## Considered Options

1. Keep the `change` type of 0.5: one file for each change, kept after `done`.
2. Keep the state of a change in the code host or in a tracker, and read it through an API.
3. A committed registry of changes, with a stage for each change.
4. An intent folder with up to 3 artifacts and a derived stage. Gates read the base tree, and `finish` applies the delta and deletes the folder.

## Decision Outcome

Option 4. The design page gives the tables and the steps in [Change lifecycle](../design/0.6.0.md#change-lifecycle), [Checks](../design/0.6.0.md#checks) and [Commands](../design/0.6.0.md#commands).

### Artifacts

- A change is a folder `docs/intent/<slug>/` with `intent.md`, `spec.md` and `plan.md` ([Artifacts](../design/0.6.0.md#artifacts)), in the `intent` layer of [ADR 0005](0005-layout-and-layers.md). A status moves only forward (SD401).
- An approved spec or plan stays `approved` after a text edit, because the playbook asks for the edit of `plan.md` in the commit of the work (Stage 3).
- A `routine` change has no `spec.md` and no delta. Its author edits the requirements in the pull request of the code.

### Stages

The stage has no field. `status` and `next` derive it from the artifacts, their statuses and git ([Stage derivation](../design/0.6.0.md#stage-derivation)). As the playbook says, a stage ends with the commit of an artifact, which starts the next stage. A breach in production writes the next `intent.md` (section "Plays").

### Approval and merge

- To approve is to set `accepted` or `approved` in a commit. The approval counts when the base tree, the merge base with the default branch, has it ([Approval and merge](../design/0.6.0.md#approval-and-merge)).
- A squash merge or a rebase merge writes a new commit, so a gate reads the base tree, not the history. Thus an approval in the pull request of the work does not count.
- spec-dna reads no review findings, because the code host holds them. The playbook says that findings do not approve or block a pull request on their own (Stage 5). Thus Deploy needs only the delta, the evidence and the deletion of the folder.

### Gates and scope

- `[change] gate` sets the severity of SD910 ([Gates](../design/0.6.0.md#gates)). SD910 reports an artifact that changes before the base tree approves the artifact before it. It also reports a change under `Files` of a plan that the base tree does not approve. Thus code waits for its plan, also when the diff does not touch the folder.
- A `small-fix` touches no intent folder and no `Files` path. SD921 reports one that changes, adds or removes a requirement in `rules`, `requirements` or `interfaces`.
- When the diff deletes the folder, each check reads the artifacts from the base tree. Thus an edit of the spec or the plan in that pull request cannot change the approved scope in the review of the code.
- SD913 is a warning, because a plan can omit a file and the review decides.

### Who approves

- The owner decision: the agent approves a `routine` plan, and the owner approves a `higher-risk` plan. The playbook gives a routine approval to the engineer (Stage 3), so for `routine` the agent sets the status, not a person.
- spec-dna does not check the identity of the approver. The code host enforces who may merge.
- A path rule cannot find the class of a plan, so the repository process must require the owner for a `higher-risk` plan ([Change classes](../design/0.6.0.md#change-classes)).

### Change classes and policy

- The playbook defines no change classes. `routine` and `higher-risk` come from Stage 3. `small-fix` names the small, well-bounded fix of Stage 6, a pull request with no `intent.md`.
- `[change] policy` selects `three-class`, `two-class` or `none` ([Change classes](../design/0.6.0.md#change-classes)). A project with no `[change]` table gets `none`, which sets SD910 to SD921 to `off`.
- Under `two-class`, SD921 reports only a changed requirement with no intent folder, because a diff alone does not show a fix from a change.
- SD917 reports a `class` below the minimum class from the delta, the diff and the high-risk keys of `[change]`.
- ADR 0003 starts a new rule as a warning in a minor release. SD905 and SD910 to SD921 can be errors from the start, as in ADR 0004, because they apply only to a project with a `[change]` table and a policy other than `none`.

### Delta

- A delta line names a requirement id. A `CHANGE` or `REMOVE` line also names its hash of ADR 0001 ("Change-risk classes") ([The spec](../design/0.6.0.md#the-spec)). `context` and `show` print the hash, so an agent copies it and does not compute it.
- The hash fixes the text that the author read. If the requirement changed after the spec, SD914 stops `finish`, and the author corrects the line in its own pull request.
- `check` reports SD914 as an error only for a spec that the diff touches, so an edit of a target on the default branch does not fail other changes.
- A `[[id#key@hash]]` reference still ignores its hash (ADR 0001). Only a delta line checks it.

### Finish

- `spec-dna finish <slug>` checks the change, applies the delta, checks the evidence and deletes the folder. It stops at the first failure ([Finish](../design/0.6.0.md#finish)).
- `finish` runs in the pull request of the code, so the review sees the code and the new current truth in one diff. It does not commit.
- A link into the folder from unlocked text stops `finish`. A link in the locked text of a decision does not, because that text cannot change ([Amends](#amends)).
- To drop a started change, one pull request sets `closed`, and a later one runs `finish`.

### Limits

- SD905 (`fan-out`) warns when a change touches more requirement documents than `[change] max-fanout`. The default of 8 is a proposed value, because no measurement exists yet.
- SD911 reports a started change with no commit for `stale-days` days. SD912 reports more started changes than `max-started`.
- SD911 reads no clock and no typed date. It compares the committer dates of the base commit and of the newest commit that touched the folder or a `Files` path.
- SD911 needs the history, not only 2 trees. No profile makes it an error, so the 2-tree gate rule of ADR 0001 stays true.

### Log and history

- A squash merge writes only the net diff, so the folder must reach the default branch before the code ([Log and history](../design/0.6.0.md#log-and-history)). `gate = "block"` makes this the normal path, because code under `Files` waits for an approved plan.

### Incidents

- An `incident` gives a new `intent.md` with `from:`, and the lesson moves to `procedures/` or `rules/` through it. This is the loop of [Stages](#stages).
- Each incident adds an eval (SD918), as the playbook asks (Stage 4).
- No gate here needs the stage skills or `REVIEW.md` of [ADR 0008](0008-harness-and-evals.md), because a skill MUST NOT be a precondition of a gate (ADR 0001).

### Rejected

- Option 1, a `done` status that keeps the documents of a change. Agents search the repository, so an old plan is a search hit next to the current truth, and it costs tokens.
- Option 2, the API of a code host. A gate then needs the network, a token and one adapter for each host, and a fork or a mirror has no such state.
- Option 3, a committed registry or database. ADR 0001 forbids it. 2 branches that edit one registry give a conflict, and a database has no readable diff.
- A `stage` field. It copies a fact of the files and git, and ADR 0001 ("Lifecycle") rejects typed progress states, because they become out of date. The `class` field stays, because it records a choice of the author.
- Staleness from the system clock: a second CI run on the same commit can give a different result. Staleness from typed dates: people forget to update them, and false alarms follow (ADR 0001).
- A check of the approver by the git author or the e-mail: anyone can set them, and a squash merge can change the author. A list of approvers: spec-dna cannot match a name with the person who merges.
- An approval in the pull request of the work. The work then starts before the approval, against the section "Plays". A project that accepts this risk sets `gate = "warn"` or `"off"`.
- A new approval for each edit of an approved spec or plan. The merge of the edit records its review.
- A closed pull request as the rejection of an intent. The playbook accepts "the closing review" (Stage 1), but the code host holds that state. A pull request also closes for other reasons.
- The close and `finish` in one pull request. The parent tree of the deletion then has `accepted`, so `log` reports a dropped change as finished.
- `finish` in a pull request after the code. Between the 2 merges, the requirements do not describe the code, and the folder stays in the hot path. A team can also forget the second pull request.
- A delta by line number: each edit on the default branch moves the lines. A delta with no hash: `finish` can replace a newer text with no finding.
- Classes from the size of the diff. A small change of an interface can break its users, and a large rename can be safe. A size limit also makes authors split a change to get a lower class.
- A task list file (`tasks.md`). The playbook names no task artifact. One more file adds a type, a status and tokens, and `Order` in `plan.md` gives the steps.
- An automatic commit by `finish`. The author must read the result first. A commit runs the hooks and carries the signature of the author. `upgrade` does not commit either ([ADR 0009](0009-upgrade-from-0-5.md)).
- SD905 as an error: no data shows the number of documents at which a split helps. A limit on delta lines: lines do not measure the number of owners that must review.

### Consequences

- Good: each gate needs only git and spec-dna. The same commits give the same result on any machine and on any day.
- Good: a finished change leaves no file in the hot path. Git keeps the record, and `log` reads it.
- Good: the derived stage cannot disagree with the files.
- Bad: with `gate = "block"`, a `routine` change needs 3 pull requests: intent, plan, and code with `finish`. A `higher-risk` change needs 4. A correction of a delta or of `Files`, and the close of a dropped change, each add 1 more.
- Bad: spec-dna cannot prove who approved. The gate trusts the base that CI gives with `--base`.
- Bad: SD911 and `log` need the history. A shallow clone can hide a stale change but never reports a false one, and `log` sees only the fetched commits.
- Bad: spec-dna does not see review findings, so the indicators that need them print `n/a` ([Indicators](../design/0.6.0.md#indicators)).
- Neutral: an upgraded project keeps `policy = "none"` until the owner selects a policy.

## Amends

ADR 0001 is accepted and locked, so this ADR does not edit it. When the owner accepts this ADR, these rules replace the named text of ADR 0001:

- "Lifecycle", the row `change`: the statuses of `intent`, `spec` and `plan` in this ADR replace it, and the status `done` ends. [ADR 0005](0005-layout-and-layers.md) replaces the type.
- "Lifecycle", the sentence "spec-dna never deletes a document": `finish` is the one command that deletes documents. It deletes only the intent folder, and only after its checks. Git is the archive.
- "Lifecycle", the condition "when nothing cites it": it stays for unlocked text, because `finish` stops while such text links into the folder.
- "Lifecycle", the sentence "Each deletion batch needs explicit approval from the owner": for an intent folder, the merge of the `finish` pull request is the approval.
- "References and requirements", the sentence "Explicit references MUST resolve": a reference to a deleted intent folder in the locked text of a decision is an exception. SD101 does not report it.
- "Change-risk classes": the name becomes "diff risk", so it does not clash with a change class. R0, R1 and R2 do not change.
- "Metadata": the type `intent` adds the field `class` and the optional fields `from` and `areas`.

## Amendments

### 2026-11-16: a draft and the gate

- This replaces, in "Gates and scope", the sentence "SD910 reports an artifact that changes before ...". SD910 reports a `spec.md` or a `plan.md` that is not `draft` when the approval of the artifact before it is absent. Code under `Files` still needs an `approved` plan.
- ADR 0012, "The rule": the walk checks an artifact only at the commits after which it is not `draft`.
- The counts of pull requests do not change.
