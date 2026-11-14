---
status: proposed
---

# 0006. Change lifecycle, gates, classes and finish

## Context and Problem Statement

In 0.5, a change is one document of the type `change`, with the states `proposed`, `accepted`, `done` and `rejected`. A `done` change stays in the tree until the owner deletes it. No rule ties a change to the stages of a team, and no check compares the work with an approved plan.

The owner asks for the stages of the [AI-native SDLC playbook](https://claude.com/resources/articles/the-ai-native-sdlc-playbook): Plan, Design, Build, Test, Deploy and Maintain. How does spec-dna model a change from its intent to its merge? A gate must need only git and spec-dna. A finished change must leave no clutter in the hot path, which is the text that each task loads.

Constraints:

- A gate MUST run from the repository with only spec-dna and git ([ADR 0001](0001-information-model.md), "Layers"). Thus spec-dna uses no network and no API of a code host.
- The result of a check must not depend on the machine or on the clock.
- The rules must work with merge commits, squash merges and rebase merges.
- Each fact is in one place, and the repository has no committed registry (ADR 0001, "Concurrency").
- The terms follow the playbook. Where the playbook names nothing, spec-dna uses a plain STE word.
- An upgraded project gets no new gate.

Evidence (the data of earlier versions stays outside this repository):

- The gate against the base gave the measured gain. Retrieval and the split of big documents did not.
- Agents search the repository. They seldom follow a tree of links.
- Large files and process documents in the text that each task loads cost tokens and give no gain.
- A mature corpus removed a gate on human-typed dates after false alarms (ADR 0001, "Context and Problem Statement").
- The playbook says: "A stage ends by committing an artifact with the commit initiating the next stage." (section "Plays").
- The playbook says: "Approval comes from a human through branch protection, informed by the findings." (Stage 5, "AI in the PR review loop").

## Considered Options

1. Keep the `change` type of 0.5: one file for each change, kept after `done`.
2. Keep the state of a change in the code host or in a tracker, and read it through an API.
3. A committed registry of changes, with a stage for each change.
4. An intent folder with up to 3 artifacts and a derived stage. Gates read the base tree, and `finish` applies the delta and deletes the folder.

| Option | Source of state | Needs only git and spec-dna | Clutter after the change | Merge conflicts | Fits any repository |
| --- | --- | --- | --- | --- | --- |
| 1 | Front matter | Yes | A `done` file stays in the tree | Low | Yes |
| 2 | Code host or tracker | No: network, token, one adapter for each host | Low | Low | No: it fits one host |
| 3 | A registry file | Yes | The registry grows | High: each change edits one file | Yes |
| 4 | Front matter and git | Yes | None: `finish` deletes the folder | Low: one folder for each change | Yes |

## Decision Outcome

Option 4. The tables are in the design page, sections [Change lifecycle](../design/0.6.0.md#change-lifecycle), [Checks](../design/0.6.0.md#checks) and [Commands](../design/0.6.0.md#commands). This section gives the rules and their reasons.

### Artifacts

- A change is a folder `docs/intent/<slug>/` with `intent.md`, `spec.md` and `plan.md` ([Artifacts](../design/0.6.0.md#artifacts)). [ADR 0005](0005-layout-and-layers.md) gives the `intent` layer.
- `intent.md` moves from `draft` to `accepted` or `closed`, and from `accepted` to `closed`. `closed` means that the change does not continue: the owner did not accept it, or the team dropped it.
- `spec.md` and `plan.md` move from `draft` to `approved`. A status moves only forward (SD401). spec-dna allows a text edit of an approved spec or plan, and the status stays `approved`. The playbook says: "When implementation departs from the plan, update `plan.md` in the same commit." (Stage 3).
- `spec.md` has the sections `Delta`, `Interfaces`, `Decisions` and `Concerns` ([The spec](../design/0.6.0.md#the-spec)). `plan.md` has `Files`, `Order`, `Risks` and `Proof` ([The plan](../design/0.6.0.md#the-plan)).
- A `routine` change has no `spec.md`, so it has no delta. Its author edits the requirements in the pull request of the code. `finish` checks the evidence of each requirement whose hash differs from the base.

### Stages

The stage of a change has no field. `status` and `next` derive it from the artifacts, their statuses and git ([Stage derivation](../design/0.6.0.md#stage-derivation)). The stage is the first row of that table that matches, from the top. A `closed` intent has the stage Closed, and its next step is `finish`.

| Stage | The commit that ends it | Playbook section |
| --- | --- | --- |
| Plan | `intent.md` with `accepted` or `closed` | Stage 1, "Capture as intent.md" |
| Design | `spec.md` with `approved`, for `higher-risk` only | Stage 2, "Requirements and design" |
| Build | `plan.md` with `approved`, then the code | Stage 3, "Claude Code plan mode as the default starting point" |
| Test | The `covers:` evidence of each requirement that the change adds or changes | Stage 4, "Give Claude a feedback loop" |
| Deploy | The merge of the pull request that holds the code and the result of `finish` | Stage 5, "AI in the PR review loop" |
| Maintain | An incident with an open follow-up, which gives a new `intent.md` and an eval | Stage 6, "Closing the loop", and Stage 4, "Continuous evals in CI" |

The playbook gives the order of the gates: "An accepted `intent.md` triggers the requirements and design pass, an approved `spec.md` triggers plan mode, a merged PR triggers the pipeline, and a breached control band in production writes the next `intent.md` and so the loop continues." (section "Plays").

### Approval and merge

- To approve is to set the status `accepted` or `approved` in a commit. The approval counts after the merge, when the default branch has that status. The playbook says: "The product owner approves, and the accept or reject decision that sends the intent into Stage 2: Design is recorded as the merge or the closing review." (Stage 1). spec-dna uses only the merge ([Rejected](#rejected)).
- "Merged" means that the base tree has the artifact with that status. The base tree is the merge base of the pull request with the default branch. A squash merge or a rebase merge writes a new commit, so spec-dna does not search the history for the approval commit. A gate reads the base tree. Thus an approval in the same pull request as the work does not count ([Approval and merge](../design/0.6.0.md#approval-and-merge)).
- Without `--base`, the lifecycle checks, `status`, `next` and `finish` use the merge base with the default branch that `origin/HEAD` names. If git cannot name that branch, they print a note that spec-dna cannot decide SD910 locally.
- The gate trusts the base that CI gives with `--base`. A stacked pull request, or a base other than the default branch, can hold an approval that the default branch does not have.
- Review findings are the comments of a review on a pull request. They are data of the code host, and spec-dna reads none of them. The playbook says: "Findings do not approve or block a PR on their own, and branch protection still requires approval from a code owner." (Stage 5). Thus the Deploy stage needs only the delta, the evidence and the deletion of the folder. `REVIEW.md` gives the review its policy ([ADR 0008](0008-harness-and-evals.md)).

### Gates and scope

- `[change] gate` sets the severity of SD910: `block` (an error), `warn` or `off` ([Gates](../design/0.6.0.md#gates)). When `[change]` has no `gate`, SD910 takes the severity of the profile. SD921 follows the profile only. `[rules]` can override both codes, as for any code.
- The order of artifacts is `intent.md`, `spec.md` (for `higher-risk`), `plan.md`, then code. SD910 reports an artifact that changes while the artifact before it is not approved in the base tree.
- SD910 also reports a diff that touches a path of `Files` in a plan of the tree. It applies when the base tree has no approved version of that plan. Thus code under a plan waits for its approval, even when the diff does not touch the folder.
- A diff that touches no intent folder and no `Files` path is a `small-fix`. SD921 reports a `small-fix` that changes a requirement hash in `rules`, `requirements` or `interfaces`, or that adds or removes a key there. SD921 applies the aliases first, so a move gives no SD921.
- Each check reads the artifacts of a change from the head tree. If the diff deletes the folder, the check reads them from the base tree. Thus an edit of the spec or the plan in the pull request of `finish` has no effect on its checks. A correction of a delta or of `Files` needs its own pull request before `finish`.
- The scope of SD913 always has the intent folder and the documents that the delta names. For a `routine` change, a requirement document is in scope only if `Files` lists it. The plan is the only approved text that names the truth that a `routine` change edits.

### Who approves

- The owner decision: the agent approves a `routine` plan, and the owner approves a `higher-risk` plan. The playbook says: "Routine changes are approved by the engineer, and anything the organization classes as higher risk goes to a tech lead or architect." (Stage 3). The owner decision differs for `routine`: the agent sets the status, not a person. The merge of the plan stays under the branch protection of the repository.
- spec-dna does not check the identity of the approver. It reads no git author, no e-mail and no signature for an approval. The code host enforces who may merge: branch protection, required reviews and code-owner rules. A repository without them has no enforcement of the approver.
- A path rule cannot find the class of a plan, because both classes use the path `docs/intent/<slug>/plan.md`. Thus `init` can print a code-owner line only for `docs/intent/*/spec.md`, which only a `higher-risk` change has. The process of the repository must require the owner for a `higher-risk` plan.

### Change classes and policy

- The playbook does not define change classes. `routine` and `higher-risk` come from the words of Stage 3. `small-fix` is a plain STE word. The playbook says: "A small, well-bounded fix arrives as a PR through the review gate, and anything larger is written up as `intent.md` for Stage 1: Plan, at which point the loop starts feeding itself." (Stage 6). A `small-fix` is such a pull request.
- [Change classes](../design/0.6.0.md#change-classes) gives the artifacts of each class and each policy. `[change] policy` selects `three-class`, `two-class` or `none`. `init` writes `three-class` for a new project. A project with no `[change]` table gets `none`, and `none` sets SD910 to SD921 to `off`.
- Under `two-class`, each change has an intent by the rule of the team. SD921 reports one kind of break: a changed requirement with no intent folder. No check reports other diffs with no intent folder, because a diff alone does not show whether it is a fix or a change.
- The author sets `class` in the front matter of `intent.md`. SD917 reports a `class` below the minimum class of the change.
- A change is at least `higher-risk` when a delta line targets a layer of `[change] high-risk-layers` (default `interfaces` and `rules`). For a `routine` change, a requirement hash that the diff changes in such a layer has the same effect. A path of the plan or of the diff that matches `[change] high-risk-paths` has the same effect. A `small-fix` with such a path gives SD917 too.
- ADR 0003 says that a new rule starts as a warning in a minor release. SD905 and SD910 to SD921 can be errors from the start, as the harness codes of ADR 0004 are. The reason is the same as in ADR 0004. These codes apply only with a `[change]` table and a policy other than `none`. No older configuration has that table. Thus an upgrade gets no new error from them.

### Delta

- A delta line names a requirement id: `ADD <doc-id>#<key>: <sentence>`, `CHANGE <doc-id>#<key>@<hash>: <sentence>` or `REMOVE <doc-id>#<key>@<hash>` ([The spec](../design/0.6.0.md#the-spec)).
- The hash is the requirement hash of ADR 0001 ("Change-risk classes"). `context` and `show` already print it, so an agent copies it and does not compute it.
- The hash fixes the text that the author read. When the requirement changed after the spec, SD914 stops `finish`. The author corrects the line in its own pull request before `finish`.
- In `check`, SD914 is an error for a spec that the diff touches, and a warning for another spec. Thus an edit of a target on the default branch does not fail the pull requests of other changes. `finish` always stops on SD914. SD914 also reports an interface of `Interfaces` when the diff changes neither its document nor a file that it cites.
- The reference rule of ADR 0001 stays: a `[[id#key@hash]]` reference still ignores its hash. Only a delta line checks the hash. A `covers:` tag can name an `ADD` requirement before `finish` writes it. SD105 does not report the tag while an `accepted` change in the tree has that `ADD` line.
- `Decisions` lists the decisions that the change needs. Each must be `accepted` when `finish` runs (SD920). `Concerns` is a task list, and a spec with an open concern cannot be `approved` (SD919).

### Finish

- `spec-dna finish <slug>` stops at the first failure ([Finish](../design/0.6.0.md#finish)). First, it lists each link into the folder from unlocked text outside the folder, and it stops if one exists. A reference in the locked text of a decision does not stop `finish`, and SD101 does not report it later. That text cannot change, and SD611 already reported the link.
- Then `finish` checks the statuses, the concerns, the decisions and the hashes. It applies the delta, checks the evidence and deletes the intent folder. A new target document is minimal and has no placeholder.
- `finish` runs in the working tree of the pull request that holds the code. Thus the review sees the code and the new current truth in one diff. `finish` does not commit. The author reads the result and commits it. `--dry-run` prints the diff and writes nothing.
- For an intent with the status `closed`, `finish` checks only that status. Then it deletes the folder. To drop a started change, one pull request sets `closed`, and a later pull request runs `finish`.
- SD915 reports a deleted folder whose base status was not `closed`, when the head tree does not match the delta of the base. The head hash of an `ADD` or `CHANGE` target must be the hash of the delta sentence, and a `REMOVE` target must be absent. For a `routine` change, which has no delta, SD915 reports a deleted folder when the diff changes no `Files` path and no requirement hash. Thus a pull request that sets `closed` and deletes the folder of a change with a delta gives SD915. SD916 reports an `ADD` or `CHANGE` requirement without evidence.

### Fan-out

- SD905 (`fan-out`) warns when one change touches more requirement documents than `[change] max-fanout`. The documents are the targets of the delta, or for a `routine` change the requirement documents of the diff. The default of 8 is a proposed value, because no measurement exists yet. The warning asks the author to split the change.

### Staleness and limits

- A started change has an `accepted` `intent.md` and a folder in the tree. SD911 reports a started change with no commit for `stale-days` days. SD912 reports more started changes than `max-started`. ADR 0005 gives both keys as limits of the `intent` layer. The default graph of the design page sets 3 and 14.
- SD911 reads no clock and no date that a person types. It compares the committer date of the newest commit that touched the folder or a `Files` path with the committer date of the base commit. Thus the same commits give the same result on any machine and on any day.
- SD911 needs the history, not only 2 trees. No profile makes it an error, so the 2-tree rule of ADR 0001 for a gate stays true. A project that makes it an error with `rules` accepts a gate that reads the history.
- In a shallow clone, git gives the boundary commit as the newest commit that touched the folder. Its date is the same or later, so a shallow clone can hide a stale change but never reports a false one.
- SD913 reports a diff that touches a file outside the scope of the change. It is a warning, because a plan can omit a file and the review decides.

### Log and history

- `spec-dna log` reads the commits of the default branch that delete an `intent.md`. It reads the class, the title, the status and the delta from the parent tree of each commit ([Log and history](../design/0.6.0.md#log-and-history)).
- The status `accepted` in the parent tree gives a finished change, and `closed` gives a closed change. The close and the deletion must be in 2 pull requests, so the parent tree has `closed`, also after a squash merge.
- A squash merge writes one commit with the net diff of the pull request. If one pull request adds and deletes a folder, the default branch never has the folder. Thus the intent folder must reach the default branch in an earlier pull request than the code. `gate = "block"` makes this the normal path, because code under `Files` needs an approved plan in the base tree.
- With `gate = "warn"` or `"off"`, a project can put the intent, the approvals and the code in one pull request. After a squash merge, `log` has no entry for that change, or reports it as closed if the parent tree has `draft`.

### Incidents

- An `incident` document gives a new `intent.md` with `from:`, which names the incident. The lesson moves to `procedures/` or `rules/` through that change. The sentence of the section "Plays" in [Stages](#stages) gives this loop from production to a new `intent.md`.
- An incident links a requirement, a rule, a decision or a procedure (SD609). It never links an intent folder, because `finish` deletes the folder.
- Each incident adds an eval. SD918 reports a change that adds an incident and adds or edits no file in `[evals] dir`. It applies only when the configuration has `[evals]`. The playbook says: "Each production incident gets an eval, written by the team that owned the incident, and stays in the suite as a regression test." (Stage 4).
- The stage skills and `REVIEW.md` are harness files of [ADR 0008](0008-harness-and-evals.md). ADR 0001 says that a skill MUST NOT be a precondition of a gate, so no gate of this ADR needs them.

### Rejected

- A `stage` field in the front matter. It is a second copy of a fact that the files and git give. ADR 0001 ("Lifecycle") rejects typed progress states, because they become out of date. The `class` field stays: it records a choice of the author, and SD917 checks it.
- Staleness from the system clock. The result changes with the day of the run. A second CI run on the same commit can then give a different result.
- Staleness from human-typed dates. ADR 0001 says that no check reads a human-typed date. A person forgets to update a date, and false alarms follow.
- Option 2, the API of a code host for the merged state and the approved state. A gate then needs the network, a token and one adapter for each host. A fork or a mirror has no such state.
- A check of the approver by the git author or the e-mail. Any person or agent can set these values in a commit. A squash merge can change the author. The code host already checks the identity at the merge.
- A list of approvers in the configuration. spec-dna cannot match a name with the person who merges. Thus the list has no effect in a check.
- An approval in the same pull request as the work. The work then starts before the approval, against the playbook rule of the section "Plays". A project that accepts this risk sets `gate = "warn"` or `"off"`.
- A new approval for each edit of an approved spec or plan. The playbook asks for the edit of `plan.md` with the work. The merge of the pull request with the correction records its review.
- The spec and the plan of the head tree in the pull request of `finish`. The diff deletes them, and an edit there would change the approved scope in the same review as the code.
- A closed pull request as the rejection of an intent. The playbook accepts "the closing review", but that state is in the code host, not in git. A pull request also closes for other reasons, such as a replacement pull request.
- The close and `finish` in one pull request. The parent tree of the deletion then has `accepted`, and `log` reports a dropped change as finished.
- Option 3, a committed registry or database of changes. ADR 0001 forbids committed registries. If 2 branches edit one registry file, a conflict follows. A database file has no readable diff.
- `finish` as a separate pull request after the code. Between the 2 merges, the default branch has code that the requirements do not describe. The intent folder stays in the hot path, and a team can forget the second pull request.
- A delta by line number. Each edit on the default branch moves the lines, so the line number is wrong before `finish` runs.
- A delta with no hash. `finish` cannot see that a requirement changed after the author read it. It can then replace a newer text with no finding.
- Classes from the size of the diff. Size does not measure risk: a small change of an interface can break its users, and a large rename can be safe. A size limit also makes authors split a change to get a lower class.
- A task list file (`tasks.md`) next to `plan.md`. The playbook names no task artifact. One more file adds a type, a status and tokens. A progress list becomes out of date, and `Order` in `plan.md` already gives the steps.
- Option 1, a `done` status that keeps the documents of the change. Agents search the repository, so an old plan is a search hit next to the current truth. Process documents in the hot path cost tokens and gave no gain.
- An automatic commit by `finish`. The author must read the result before the commit. A commit runs the hooks of the author and carries the signature of the author. `upgrade` does not commit either ([ADR 0009](0009-upgrade-from-0-5.md)).

- SD905 as an error: no data shows the number of documents at which a split helps, so a block would stop valid work. A limit on the lines of a delta is also rejected, because lines do not measure the number of owners that must review.

### Consequences

- Good: each gate needs only git and spec-dna. The same commits give the same result on any machine and on any day.
- Good: a finished change leaves no file in the hot path. Git keeps the record, and `log` reads it.
- Good: spec-dna derives the stage, so the stage cannot disagree with the files.
- Bad: with `gate = "block"`, a `routine` change needs 3 pull requests: intent, plan, and code with `finish`. A `higher-risk` change needs 4. A correction of a delta or of `Files`, and the close of a dropped change, each add 1 more.
- Bad: spec-dna cannot prove who approved. The approval rule depends on the branch protection of the repository.
- Bad: SD911 and `log` need the history. A shallow clone can hide a stale change, and `log` sees only the fetched commits.
- Bad: spec-dna does not see review findings. The indicators that need them print `n/a` ([Indicators](../design/0.6.0.md#indicators)).
- Neutral: an upgraded project keeps `policy = "none"` until the owner selects a policy.

## Amends

ADR 0001 is accepted, and its text is locked. This ADR does not edit it. When the owner accepts this ADR, these rules replace the named text of ADR 0001:

- "Lifecycle", the row `change`: the statuses of `intent`, `spec` and `plan` in this ADR replace it, and the status `done` ends. [ADR 0005](0005-layout-and-layers.md) replaces the type.
- "Lifecycle", the sentence "spec-dna never deletes a document": `finish` is the one command that deletes documents. It deletes only the intent folder, and only after its checks. Git is the archive.
- "Lifecycle", the condition "when nothing cites it": it stays for unlocked text, because `finish` stops while such text links into the folder.
- "Lifecycle", the sentence "Each deletion batch needs explicit approval from the owner": for an intent folder, the merge of the `finish` pull request is the approval.
- "References and requirements", the sentence "Explicit references MUST resolve": a reference to a deleted intent folder in the locked text of a decision is an exception. SD101 does not report it.
- "Change-risk classes": the name becomes "diff risk", so it does not clash with a change class. R0, R1 and R2 do not change.
- "Metadata": the type `intent` adds the field `class` and the optional fields `from` and `areas`.

## Amendments
