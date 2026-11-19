---
status: proposed
---

# 0026. Approval through the code host

Moved to 0.8.1. On 2026-11-19 the owner moved the mode `review` and the persons of `[owners]` from 0.8.0 to 0.8.1. 0.8.0 does not depend on this ADR. The status stays `proposed`. The [design note of 0.8.1](../design/0.8.1-note.md) keeps the detail.

## Context and Problem Statement

[ADR 0012](0012-approval-in-the-branch.md) counts an approval commit when the e-mail of its author is in `[change] approvers`. The e-mail is no proof. [ADR 0018](0018-approval-by-a-person.md) adds that an agent often commits with the e-mail of its owner, so `check` cannot tell its approval commit from a commit of the owner. The code host knows which account approved a pull request, and the host authenticates that account. spec-dna does not read it.

The owner decided on 2026-11-18: a third mode, `approval = "review"`. The approval is a review by an owner that approves the pull request. The commit modes stay for a repository with no code host. Which command reads the reviews? How does CI make that command a required check? How does the review map to the intent, the spec and the plan? What happens on a squash merge?

Constraints:

- `check` has no network ([ADR 0006](0006-change-lifecycle.md), ADR 0012). A command that needs the host is a separate command.
- No new Rust dependency. `gh` runs as a child process, as `claude` does for the evals.
- The same commits and the same reviews give the same result.
- `base` and `branch` keep their rules.

Evidence ([design note of 0.8.1](../design/0.8.1-note.md#2-evidence)):

- gh 2.101.0 fills `{owner}` and `{repo}` from the remote, and `gh api --paginate --slurp` gives one JSON array (F18).
- A review of the REST API has `user.login`, `state` and `commit_id`. The documentation of the host says: "Pull request authors cannot approve their own pull requests" (F19).
- gh exits 0 on success, 1 on an API or network failure and 4 with no login. A gh command can print nothing and exit 0 (the release check of 0.7.1).

## Considered Options

1. `check` reads the reviews.
2. A command `spec-dna reviews` that runs `gh` and that CI makes a required check. `check` stays offline.
3. Only the branch protection of the host: required reviews and code owners. spec-dna reads nothing.
4. An app or an action of the host that writes the approval into the repository.

## Decision Outcome

Option 2. The design note of 0.8.1 gives the detail in [D4](../design/0.8.1-note.md#d4-approval-through-a-review-of-the-host).

### The mode

- `[change] approval` is `base`, `branch` or `review`. With `review`, the approval of an artifact is a review by an approver that approves the pull request on its head commit.
- `approvers` items have 2 forms. An e-mail matches the author of a commit. `@<login>` matches an account of the host. A hidden address `<id>+<login>@users.noreply.github.com` gives the login too. With `review`, the list needs one login, or the exit code is 3.
- The `status` lines stay. A person runs `spec-dna approve`, or the agent sets the status on the word of the owner. The status says what was approved, and the review says who approved it. With `review`, `approve` makes the status commit with no check of its author.

### `check` with `review`

- SD910 reads the statuses with no author and no commit order. An artifact can follow an earlier artifact whose approval status the same diff sets.
- Code under `Files` needs a plan that the base tree approves, as with `base` (reason 1). Thus the work waits for a merged plan, and the floor stays 2 pull requests: the design, then the code with the record and `finish`.
- SD910 keeps its code, its severity and its 9 reasons.

### The command `spec-dna reviews`

- `spec-dna reviews [--pr <n>] [--json]`. With no `--pr`, it reads `pull_request.number` from `GITHUB_EVENT_PATH`.
- It runs `gh api repos/{owner}/{repo}/pulls/<n>` for the head commit, the base commit and the author, and `gh api --paginate --slurp repos/{owner}/{repo}/pulls/<n>/reviews` for the reviews.
- The items that need an approval come from the git diff of the merge base and the head commit:
  - each artifact whose approval status the diff sets needs an approver;
  - the acceptance record, when it needs an acknowledgement ([ADR 0025](0025-independent-acceptance.md)), needs an approver;
  - each group of persons of `[owners]` that matches a touched path needs one person of the group (D5 of the [design note of 0.8.1](../design/0.8.1-note.md#4-the-persons-of-owners-item-5-of-080)).
- For each login, the last review with the state `APPROVED`, `CHANGES_REQUESTED` or `DISMISSED` is its state. It counts when it is `APPROVED` and its `commit_id` is the head commit. A review of the author of the pull request never counts.
- Exit codes, in the table of the commands:
  - 0: each approval is present.
  - 1: an approval is absent, and the output names the item and who can give it.
  - 2: usage, also no pull request number.
  - 3: configuration, also no `@<login>` item in `approvers`, no remote, or no `gh`, as `eval run` treats an absent `claude`.
  - 4: the call of the host failed (`gh` has no signed-in account, an API or network error, an unknown pull request, or the head commit is not in the clone).
- Code 4 gets this meaning beside a git or file error. The command fails closed: an empty output of `gh` is never a pass.
- On `merge_group`, it exits 0 with a note, because the queue takes only a pull request whose required checks passed.
- A stub `gh` on `PATH` tests each exit code offline.

### CI

With `review`, `init` writes `.github/workflows/spec-dna-reviews.yml`. The workflow runs on `pull_request`, `pull_request_review` and `merge_group`. It has `permissions: {}` at the workflow level and `contents: read` and `pull-requests: read` for the job. It pins the checkout to the SHA that `src/init.rs` uses, with `fetch-depth: 0` and `persist-credentials: false`. It passes `GH_TOKEN: ${{ github.token }}`. The person makes the job a required check of the default branch.

### Squash merge, no remote, `strict`

- A squash merge keeps the net diff. After it, `check` on the default branch reads the statuses of the base tree, as with `base`, and the record of the archive. It cannot show that a review existed. The host keeps the reviews.
- A repository with no remote cannot use `review`. `reviews` exits 3.
- With no answer of the owner, `review` with `strict` is exit code 3, as `branch` with `strict`. The design asks the owner to allow this case (Q1), because the host authenticates the reviewer.

### What the gate proves

- `reviews` proves one fact: at the time of the job, an account of the list approved the head commit for each item that needs it.
- The branch protection of the host must do the rest. The check is required. The merge waits for it on the last head. No administrator skips it. The host knows who holds the account.

### Rejected

- Option 1: `check` then needs the network and a token, and the same commits can give another result on another day.
- Option 3: the host does not know the artifacts, the record or the roles. spec-dna cannot name what the review must cover.
- Option 4: one app for each host, and a bot commit is one more e-mail to trust.
- A review of an older commit: the code after it has no review. Run `finish` before the review.
- A new key for the logins: 2 lists of the same persons.
- A host other than GitHub: no second adapter exists yet. A new host is a new reader of its command line tool.

### Consequences

- Good: the approval rests on an account that the host authenticates, not on an e-mail.
- Good: `check` stays offline and deterministic.
- Bad: the agent and the approver need 2 accounts. A person whose agent opens pull requests with the account of that person cannot use `review`.
- Bad: a push after the review needs a new review, and so does the commit of `finish`.
- Bad: the result rests on the branch protection of the host, which spec-dna cannot read.
- Neutral: `base` and `branch` do not change, and an upgraded project keeps its value.

## Amends

On 2026-11-19, this ADR amends ADR 0006, ADR 0012 and ADR 0018. They are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text, for `approval = "review"` only:

- ADR 0006, "Rejected", option 2, and ADR 0012, "Rejected", option 2 ("the review API of a host"): the command `reviews` reads the reviews through `gh`. The gate `check` still needs no network, no token and no adapter.
- ADR 0012, "Keys and profiles", the sentence "`[change] approval` is `base` or `branch`": it is `base`, `branch` or `review`.
- ADR 0012, "Keys and profiles", the sentence "`[change] approvers` is a list of e-mails": an item is an e-mail or `@<login>`. With `review`, the list needs one login.
- ADR 0012, "The rule": with `review`, SD910 reads no commit and no author. An artifact can follow an earlier artifact that the same diff approves, and code needs a plan that the base tree approves.
- ADR 0018, "Decision Outcome", the sentence "With `approval = "branch"`, it stops when the author is not an approver": with `review`, `approve` checks no author, because the review is the approval.

## Amendments
