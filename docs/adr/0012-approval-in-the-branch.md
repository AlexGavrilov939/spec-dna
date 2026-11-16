---
status: accepted
---

# 0012. Approval in the branch

## Context and Problem Statement

[ADR 0006](0006-change-lifecycle.md) counts an approval only when the base tree has it. Thus each artifact of a change waits for its own merge. With `gate = "block"`, a `routine` change needs 3 pull requests, and a `higher-risk` change needs 4. How can a branch hold its own approvals, and keep the order of the stages and the `log` record?

Constraints:

- A gate runs with only spec-dna and git ([ADR 0001](0001-information-model.md), "Layers"). spec-dna uses no network and no API of a code host (ADR 0006).
- The same commits and the same clone give the same result on any machine and on any day.
- The rules work with merge commits, squash merges and rebase merges.
- A squash merge writes only the net diff. Thus `log` needs the intent folder on the default branch before the code (ADR 0006, "Log and history").
- An upgraded project gets no new gate, and a current corpus passes `check` with no edit.

Evidence (the data stays outside this repository, [Measurement](../design/0.7.0.md#measurement)):

- A lab script builds a scratch repository with a bare origin, `gate = "block"` and the 0.6.1 binary. It squash-merges each branch, as GitHub does.
- In the lab, a `routine` change needs 3 pull requests, and a `higher-risk` change needs 4. One branch gives exit code 1 with 3 SD910 errors for a `routine` change and 4 for a `higher-risk` change. With `finish` in the same branch it gives 1 SD910 error.
- The real change of 0.6.1 had 4 pull requests in 7 hours. They were the intent (#34), the plan with the design (#35), the code with `finish` (#37) and the pin (#40). [ADR 0013](0013-self-hosting.md) treats the pin.
- A model of D11 ran on a real branch with 7 commits. It clears an SD910 finding of the 0.6.1 binary when an approval commit covers it.
- The model is not product code. The build can find a case that it misses.

| Branch variant | SD910 findings | Left after the rule | Statuses set outside an approval commit |
| --- | --- | --- | --- |
| `ok`: the approvers approve each artifact before its successor | 4 | 0 | 0 |
| `late`: the approval of the plan comes after the code | 4 | 2 (the code files) | 0 |
| `stranger`: the approval commits come from an author that is not an approver | 4 | 4 | 3 |
| `mixed`: the approval commit of the intent also edits the text | 4 | 1 (the spec) | 1 (the intent) |

## Considered Options

1. Keep ADR 0006: the base tree holds each approval.
2. Read the review state of the code host through its API.
3. A trailer `Approved-by:` in a commit message.
4. Signed approval commits, and spec-dna verifies each signature.
5. Approval commits in the branch. `check` walks `base..HEAD` and accepts a status commit of an author in `[change] approvers`. A key turns the rule on, and `strict` keeps option 1.

## Decision Outcome

Option 5. The design page gives the detail in [The approval in the branch](../design/0.7.0.md#6-the-approval-in-the-branch), D11 to D17. This ADR gives the rules and their reasons.

### The rule

- `check` walks the commits of `base..HEAD` from the oldest ([D11](../design/0.7.0.md#d11-the-rule)). A commit that touches an artifact or a path of `Files` needs the approval of the earlier artifact.
- That approval is in the base tree, as in 0.6, or in an approval commit before this commit in the walk.
- The class selects the artifact that needs the approval, as in ADR 0006. Only the source of the approval changes. The scope of a plan is its `Files` at the approval commit. A path that a later commit adds has no approved plan, and `check` gives SD910. SD913 is only a warning.

An approval commit meets all 4 conditions:

1. It is not a merge commit.
2. The e-mail of its author is in `[change] approvers`. The compare ignores case and needs an exact match.
3. It changes one file only: `intent.md`, `spec.md` or `plan.md`. It changes only the `status` line, to `accepted` for an intent and `approved` for a spec or a plan.
4. The file is an artifact of a change that the branch started or continued.

The rule also covers the last approval. Take each artifact whose status is `accepted` or `approved` and differs from the base tree. An approval commit must have set that status, and a merge commit does not count. Else `check` gives SD910, also when no later commit depends on it. Else nothing checks the last approval: after the squash, the next pull request reads it from the base tree with no author.

### The identity of the approver

- The condition on the e-mail is hard: an approval commit of an author outside the list does not count ([D12](../design/0.7.0.md#d12-the-identity-of-the-approver)).
- The e-mail is not a proof. A person or an agent can set any e-mail.
- The check proves the order of the commits and a match of the e-mail. It does not prove that a person read the text.
- The real control is the code host: `CODEOWNERS` for `docs/intent/`, a protected default branch, and required signed commits.
- For a `higher-risk` change, the owner should list only the e-mail of a human, and the agent must not commit with it. The owner then makes the approval commits in the web editor of the host. A hidden e-mail becomes a `noreply` address there, so the list holds that address too.
- Owner answer Q5: `approvers` of this repository lists 2 addresses of the owner. An agent approves only when the owner says so explicitly in the chat. This replaces the previous bullet here.

### Fail closed

The rule fails closed ([D13](../design/0.7.0.md#d13-the-history-must-be-complete)). In the first 3 cases below, the branch gives no approval, and SD910 reports as in 0.6:

- The clone is shallow, and the merge base is not reachable. The message says "shallow clone". `init` writes `fetch-depth: 0` in each workflow that it generates and that runs `check`.
- A merge commit in the range merges another branch into this one. It is neutral in 2 cases only:
  - Its second parent is an ancestor of the base tip: the branch took the base.
  - Its first parent is the base tip: the test merge of a host.
- A merge queue makes a commit with another first parent, so the rule stops. A required check must also report on `merge_group`.

In the fourth case, a commit has an approval and another change in the same file. It gives no approval. The `mixed` variant shows this. If the base is unknown, no change check runs, as in 0.6.1.

### Keys and profiles

- `[change] approval` is `base` or `branch`, and the default is `base` ([D15](../design/0.7.0.md#d15-keys)). `branch` turns on the rule.
- `[change] approvers` is a list of e-mails. It must have one e-mail or more when `approval = "branch"`. Else the configuration is an error (exit code 3).
- `approval = "branch"` with `profile = "strict"` is an error (exit code 3). `strict` reads approvals from the base tree only, so each approval is a merge that the host controls.
- `init` writes `approval = "branch"` and `approvers = ["<git user.email>"]` for `lite` and `standard`. It writes `approval = "base"` for `strict`.
- An upgraded project keeps `base`, so it gets no new gate. A change of these keys is a configuration change (SD503).

### A change that starts and ends in one branch

- A branch that adds an intent and deletes it with `finish` still gives SD910, also with approval commits ([D16](../design/0.7.0.md#d16-a-change-that-starts-and-ends-in-one-branch)).
- `log` reads the deletion of `intent.md` on the default branch. A squash merge writes only the net diff, so such a change leaves no record.
- The floor is 2 pull requests. The first holds the artifacts and their approvals. The second holds the code with `finish`. The guide advises the design pull request first, but `check` does not enforce this.
- Q2 of the design: no key turns the record off.

### Message and output

- SD910 keeps its code and its severity ([D17](../design/0.7.0.md#d17-the-message)). The message names one reason of 8:
  1. No approval in the base.
  2. No approval commit before the work.
  3. An author that is not an approver.
  4. A commit that changes more than the status.
  5. A shallow clone.
  6. A merge commit that stops the rule.
  7. The change was added and finished in this branch (D16).
  8. A path that is not in the `Files` of the plan at the approval commit.
- The `fix` text gives the fix for that reason.
- `check --json` gets the field `approvals`. It lists each approval commit that cleared a gate, with `commit`, `path` and `author`.
- `status` shows the source of each approval: `base` or `branch <short sha>` (D8).

### Result

The rules give these counts for `gate = "block"` ([Result, per profile](../design/0.7.0.md#result-per-profile)). The product does not exist yet, so nobody measured them.

| Change | Before | After: `lite` and `standard` | After: `strict` |
| --- | --- | --- | --- |
| `routine`: intent, plan, code with `finish` | 3 | 2 | 3 |
| `higher-risk`: intent, spec, plan, code with `finish` | 4 | 2 | 4 |

With `gate = "warn"`, the default of `lite`, SD910 is a warning. Then the guide advises the floor of 2, and `check` does not stop one pull request.

### Answers to the rejections of ADR 0006

ADR 0006 rejects 3 items that this rule uses. Each answer applies only to `approval = "branch"`.

- "An approval in the pull request of the work". ADR 0006 says that the work then starts before the approval. The walk now checks the order: each commit of the work comes after the approval commit that it needs.
  - The order of commits does not replace a review by people. A merged pull request of the design gives that review before the work starts.
  - A change in one pull request loses that review. The floor of 2 does not force it. The guide recommends a design pull request first for a `higher-risk` change, as a convention.
- "A check of the approver by the git author or the e-mail". ADR 0006 says that anyone can set them, and a squash merge can change the author.
  - The check runs on the branch before the squash, on the author of the approval commit. Thus the squash does not change the result.
  - The other reason stays: anyone can set an e-mail. This ADR states that limit, and the host stays the control.
- "A list of approvers". ADR 0006 says that spec-dna cannot match a name with the person who merges. The list matches the author of the approval commit, and the host still controls the merge.

### Rejected

- Option 1 as the only rule: each stage costs one pull request. `strict` keeps it.
- Option 2, the review API of a host: a gate then needs the network, a token and one adapter for each host.
- Option 3, a trailer `Approved-by:`: any author can write it. It has the same limit as the e-mail, and it adds a second place for the approval.
- Option 4, signed commits as a condition: CI has no keys to verify a signature, and a list of keys is a second registry. The host keeps this control.
- The wildcard `*` in `approvers`: it matches each author, so the condition is empty.
- A new diagnostic code for the branch rules: SD910 already reports the absent approval, and its message gives the reason.
- Add and `finish` in one pull request: `log` then has no record of the change (D16, Q2).

### Consequences

- Good: with `gate = "block"` in `lite` and `standard`, a `routine` change and a `higher-risk` change each take 2 pull requests.
- Good: the gate needs only git and spec-dna. The same commits give the same result on any machine.
- Bad: the e-mail is not a proof. A person or an agent that sets the e-mail of an approver meets the condition.
- Bad: an agent often commits with the identity of its owner. Then the list cannot tell the agent from the owner, and the host is the control of a `higher-risk` change.
- Bad: one list serves both classes, and ADR 0006 lets the agent approve a `routine` plan. A list with only a human e-mail makes the human approve each plan, or the change takes 3 pull requests.
- Bad: the check reads the order of the commits, not the time. A rebase can reorder them.
- Bad: the rule needs the full history in CI. A shallow clone gives the stricter result of ADR 0006, never a false pass.
- Bad: the first pull request holds all approvals, so the reviewer reads more at once.
- Bad: a status that the base already holds has no check of its author. The merge is the control, as in 0.6.
- Neutral: an upgraded project keeps `approval = "base"` and gets no new gate.

## Amends

ADR 0001 and ADR 0006 are accepted and locked, so this ADR does not edit them. These rules replace the named text for `approval = "branch"`:

- ADR 0006, "Approval and merge", the sentence "Thus an approval in the pull request of the work does not count": an approval commit counts when it meets the 4 conditions and comes before the work in the walk.
- ADR 0006, "Who approves", the sentence "spec-dna does not check the identity of the approver": `check` compares the author e-mail of an approval commit with `[change] approvers`. The compare is a condition, not a proof.
- ADR 0006, "Rejected", the items "An approval in the pull request of the work", "A check of the approver by the git author or the e-mail" and "A list of approvers": the section [Answers to the rejections of ADR 0006](#answers-to-the-rejections-of-adr-0006) gives the new reasons. With `approval = "base"`, they stay rejected.
- ADR 0006, "Limits", the sentence "so the 2-tree gate rule of ADR 0001 stays true": SD910 also reads the history, as the last item states.
- ADR 0006, "Consequences", the line about 3 and 4 pull requests: each class needs 2 with `gate = "block"`. `strict` keeps 3 and 4.
- ADR 0006, "Consequences", the line "SD911 and `log` need the history": SD910 needs it too.
- ADR 0001, "Layers", the sentences "Every gate needs only two trees" and "A depth-1 fetch of the base commit is sufficient": SD910 also reads the commits of `base..HEAD`. With 2 trees only, the gate still runs and gives the result of ADR 0006. Thus less history never gives a pass.

## Amendments

### 2026-11-16

See the amendment of ADR 0006.
