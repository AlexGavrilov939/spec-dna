---
status: proposed
---

# 0025. Independent acceptance and the acceptance record

## Context and Problem Statement

The agent that writes the code also says that the code is done. `finish` then applies the delta and deletes the folder of the change, so no file keeps what was tried. The owner decided on 2026-11-18: each change that has a plan needs an independent acceptance. An agent in a new session, not the author, tries to break the change and writes a record, and `finish` does not pass without a valid record.

Where does the record stay after `finish`, what can "not the author" mean in a deterministic gate, and who is the root of the trust?

Constraints:

- `check` has no network and no model. A skill is never a precondition of a gate ([ADR 0006](0006-change-lifecycle.md) and [ADR 0001](0001-information-model.md)).
- No committed registry: 2 branches that edit one file give a conflict (ADR 0001, "Concurrency").
- A finished change leaves no file in the hot path. Agents search the repository, and an old plan is a search hit next to the current truth (ADR 0006, "Rejected", option 1).
- A squash merge keeps the net diff, and the title of the pull request replaces the commit messages.
- An upgraded repository with no open change gets no new diagnostic.

Evidence ([design 0.8.0](../design/0.8.0.md#1-evidence-and-sources)):

- A path in the folder of a change needs no approved plan for SD910 (F7).
- When the diff deletes the folder, each check reads the artifacts from the base tree (F8). A file written in the pull request of `finish` is then not in the folder that the checks read.
- The input of a hook has a session id, and a transcript is a local file (F12). CI has neither. A session id in a record is a claim.
- `finish --json` prints the `step` of a stop, and 6 on success (F6).

## Considered Options

For the place of the record after `finish`:

1. One file per change in a hidden folder outside the roots: `.spec-dna/accepted/<slug>.toml`.
2. One append-only log file.
3. The commit message of `finish`.
4. A document of a new type in a layer with no `context`.

For "not the author":

- (a) Claims in the record, with 2 checks that git can show: distinct sessions, and the record in commits of its own.
- (b) A session trailer on each commit of the builder, from a hook of the agent, compared with the record.
- (c) The author e-mail of the commits.
- (d) No check.

## Decision Outcome

Option 1 and option (a). The design page gives the detail in [D3](../design/0.8.0.md#d3-the-acceptance-record).

### When a record is needed

- Each change that `finish` applies needs one record, if a commit added its plan at a pin of 0.8.0 or later. `finish` finds that commit with `git log -1 --diff-filter=A <base>`, as B5 of [design 0.7.1 part B](../design/0.7.1-b.md#b5-a-plan-of-an-earlier-version-and-sd923). Only a commit that is an ancestor of the lifecycle base counts. A plan that `base..HEAD` added always needs a record, so a branch cannot set the pin back for one commit to skip it. No commit (a plan that is not committed, or a shallow clone) gives the stricter result: a record is needed. In a shallow clone, SD927 names the cause, as reason 5 of SD910. `next` names the step `accept` only for a plan that needs a record.
- A closed change and a small fix need none.
- A `routine` change needs 1 acceptor. A `higher-risk` change needs `[change] higher-risk-acceptors` (default 1). Each role of `[owners]` that matches a touched path needs an acceptor with that role ([ADR 0027](0027-owners-invariants-and-write-back.md)). One entry can count for both.

### The record

- One TOML file. Before `finish` it is `acceptance.toml` in the folder of the change. `finish` moves it to `.spec-dna/accepted/<slug>.toml`, or `<slug>-2.toml` when a change with the same slug finished before.
- The fields: `format`, `change`, `scope`, `paths`, `tree`, `result` (`accepted` or `rejected`), `acknowledged-by`, and the tables `[[acceptor]]` (role, session, model, result), `[[proof]]`, `[[invariant]]`, `[[deviation]]`, `[[attempt]]` and `[[finding]]` (acceptor number, severity, path, line, text). 2 acceptors can have the same role, so an attempt or a finding names the number of its acceptor.
- The rules: no unknown key, at most 16,384 bytes, no answer `pending`, at least one attempt for each acceptor, and a path for each finding. `paths` is not empty when a task of the plan has `Files:` other than `none`. The top `result` is `accepted` only when each acceptor accepted, no `important` finding exists, no invariant is `violated` and no deviation is `rejected`.
- `spec-dna accept <slug> [--role <name>]` writes the skeleton with the computed fields. A person can write the same file by hand, also with the harness level `none`. The gate reads the file, not the skill.

### The tree of the record

- The record stores its scope: `scope` holds the paths and globs of the scope of the plan and of `[change] shared-paths`. `paths` holds the code paths of [ADR 0024](0024-test-report-as-evidence.md) of the diff of the change against the lifecycle base that `scope` matches, sorted. Thus the number of open changes in the diff does not change the list. When the diff has no code path in `scope`, `accept` takes the code paths since the commit that added the plan. `tree` is the hash of these paths only.
- The record is stale (SD929) when the hash differs, or when the diff of the change has a code path that `scope` matches and `paths` does not list. `finish` and the checks after it read `scope` and `paths` from the record, not from the plan, so an edit of `Files` during the build changes nothing. After `finish`, the message of SD929 gives the one fix: revert the commit of `finish`, merge the base, accept again, and run `finish` again.
- In CI, the checkout is the test merge of the branch into the base. A base commit that edits a listed path is a real conflict of meaning, and the record is stale. A base commit to other paths, or a Markdown edit, changes nothing. Code of an earlier pull request of the change is not listed, so the acceptance runs in the last pull request with code.
- `check` checks only the record alone (SD928, SD931, SD932) where the diff adds or changes it. It checks the presence (SD927), the tree (SD929), the result (SD930), the count of acceptors (SD933), the acknowledgement (SD934) and the answers (SD938) only in 2 cases. One case is a diff that deletes the folder, as SD915 and SD916. The other case is a change that the branch added and deleted. Thus the first of 2 acceptors can commit, and a fix of the builder after a `rejected` record passes a hook that runs `check`. SD927 needs the archive file in the head, so a folder deleted with no `finish` gives SD927, also when the base folder holds a record. A later change never makes an old record stale.

### `finish`

- `finish` checks the record in step 5, after the reports (part 5.1) and before the evidence (part 5.4). The step numbers of 0.7.1 and the success value 6 of `finish --json` keep their meaning.
- It stops on an absent (SD927), invalid (SD928), stale (SD929) or rejected (SD930) record. It stops on one session in 2 entries (SD931), and, with `branch`, on a commit that breaks the rule of the record commits (SD932). It stops on an absent acceptor (SD933) and, with `branch`, on an absent acknowledgement (SD934).
- It adds the archive file to its writes. A diff that changes or deletes an archive file gives SD935.

### "Not the author"

- spec-dna checks 2 facts. The acceptor entries have distinct sessions (SD931). With `approval = "branch"`, a commit that adds or edits the record changes no other path (SD932). The commit of the result of `finish` is the exception.
- The skill `accept` tells the agent to stop when its session wrote code of the change. The agent `acceptor` has no instruction to edit code.
- spec-dna cannot show that the acceptor is not the builder, which model ran, or that the session was new.

### Who writes, and who acknowledges

- The acceptor writes the record and commits it alone, in each approval mode. The builder never edits it. An agent never adds an entry to `acknowledged-by`. The rule of the skills says so, and the hook `protect-approvals` denies such a commit. The hook allows the commit of `finish`, which moves the same bytes to the archive. Rule A of the hook already keeps `spec-dna approve` for a person.
- The record needs an acknowledgement when it holds a deviation, or when a person of `[owners]` matches a touched path:
  - `base`: the merge of the pull request of `finish`.
  - `branch`: `spec-dna approve <slug> acceptance` adds the e-mail of its author to `acknowledged-by` and commits that file alone. The walk counts the entry only when that commit added it, its author has that e-mail, and no later commit changes the record, except another such commit or the commit of `finish`. The commit of `finish` moves the same bytes to the archive. A record commit of the acceptor empties `acknowledged-by`. Any other commit that changes the record and keeps an entry gives SD932, so the person approves again. An approval commit and the commit of `finish` are exceptions to each clause of SD932. Thus an approver never acknowledges a text that changed after the approval.
  - `review`: an approving review of the head commit ([ADR 0026](0026-approval-through-the-code-host.md)).

### The trust root

A forged record costs one file and one commit. The root of the trust is the person who acknowledges: the merge, the approval commit or the review. The evals of the skill `accept` measure whether acceptors find planted defects, and the cost of one run. The record makes the work of the acceptor visible, binds it to the code, and keeps it.

### Rejected

- Option 2: each pull request appends to one file, so 2 branches conflict (ADR 0001).
- Option 3: the squash title replaces the message, `finish` must validate the record before a commit exists, and `check` cannot read a message on the default branch.
- Option 4: a new document type changes the type table, the layers and the templates. A Markdown document is a search hit and counts for `max-docs`. TOML in a hidden folder is skipped by the default search tools of agents.
- Option (b): a `SessionStart` hook that sets a variable has a reported defect after `/clear`. The owner can add it later (design, Q10).
- Option (c): an agent often commits with the e-mail of its owner ([ADR 0018](0018-approval-by-a-person.md)), so each honest change would fail.
- Option (d): the owner asked for a check and its limits.
- A record for each acceptor in its own file: 2 files for one change, and a second place to read.
- A new step number for the record: `finish --json` prints the step, so a new number changes its meaning.

### Consequences

- Good: each change keeps one record of what was tried, with its findings, after `finish`.
- Good: an edit of the code after the acceptance stops `finish`.
- Good: the record is the carrier of the deviations, the invariant answers and the proof states, so no other store exists.
- Bad: each change costs one more agent run. A `higher-risk` change in `strict` costs 2.
- Bad: the session is a claim, and a forged record passes the gate. Only a person or the host stops it.
- Bad: the archive grows by one file of 1 to 4 KB per change. `git grep` still finds it.
- Neutral: a repository of 0.7.1 with no open change gets no new diagnostic. An open change with a plan of 0.7.1 finishes with no record.

## Amends

ADR 0006, ADR 0012 and ADR 0018 are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0006, "Finish", the sentence "`spec-dna finish <slug>` checks the change, applies the delta, checks the evidence and deletes the folder": it also checks the acceptance record and moves it to `.spec-dna/accepted/`.
- ADR 0012, "The rule", condition 3 of an approval commit: a commit can also add the e-mail of its author to `acknowledged-by` of `acceptance.toml`, and change nothing else.
- ADR 0018, "Decision Outcome", the hook `protect-approvals`: it also denies a `git commit` that adds an entry to `acknowledged-by` of a record in a change folder, except the commit of `finish`, which moves the same bytes to the archive. The rule of the skills also says "never add an entry to `acknowledged-by`".

## Amendments
