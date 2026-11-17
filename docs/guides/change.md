# One change from start to finish

This guide takes one change through its documents, its approvals and `finish`. It uses the policy `three-class` and the gate `block`, which are the defaults of a new project. [Concepts](../concepts.md#the-change-lifecycle) explains the model.

The example change adds a requirement: when a moisture sensor times out, the controller closes the valve of its bed. The slug of the change is `sensor-timeout`.

## Choose the class

| Class | Use it when | Documents | Who approves the plan |
| --- | --- | --- | --- |
| `small-fix` | The change keeps each requirement as it is | None: a pull request only | Not applicable |
| `routine` | The change edits requirements outside the high-risk layers | `intent.md`, `plan.md` | The agent |
| `higher-risk` | The change edits a rule or an interface, or a path of `[change] high-risk-paths` | `intent.md`, `spec.md`, `plan.md` | The owner |

If the class of an intent is too low for its diff, `check` reports SD917. The high-risk layers are `interfaces` and `rules` by default (`[change] high-risk-layers`).

## A higher-risk change

Each document needs the approval of the document before it. With `[change] approval = "base"`, the base tree must show the approval, so a higher-risk change has 4 pull requests or more: the intent, the spec, the plan, and the code with `finish`. To approve a document, set its status in the pull request. The merge records the approval. With `[change] gate = "warn"`, one pull request can hold all the steps, and `check` reports a warning.

With `[change] approval = "branch"`, an approval commit in the branch also counts, so the approvals and the work can be in one pull request. [The approval commit](#the-approval-commit) gives the rule. The floor stays 2 pull requests, because `finish` follows in a later pull request.

### 1. Write the intent

```sh
spec-dna new intent sensor-timeout
```

This command writes `docs/intent/sensor-timeout/intent.md` with the status `draft`. Write what you want and why, in your own words. Set the class in the front matter first, and commit it before the approval. An approval commit changes the `status` line alone, so it cannot also add the class. An intent with no class stays in the stage `plan`, and `next` asks for the class before the approval:

```md
---
status: draft
class: higher-risk
---
```

Open a pull request. When the owner agrees with the intent, set `status: accepted` in that pull request and merge it.

### 2. Write the spec

```sh
spec-dna new spec sensor-timeout
spec-dna show requirements/valves/closing#flush-time
```

`show` with a requirement id prints that requirement with its hash. Run it for each requirement that the change edits or removes. The `Delta` section of `spec.md` lists the edits of the requirements. `ADD` names a new key. `CHANGE` and `REMOVE` name a key and the hash of the text that you read:

```md
## Delta

- ADD requirements/valves/closing#sensor-timeout: When a moisture sensor times out, the controller SHALL close the valve of its bed.
- CHANGE requirements/valves/closing#flush-time@3f2a9c1b7e0d4a11: The controller SHALL flush a valve line for 30 seconds before it closes the valve.
```

The spec has 4 more sections:

- `Interfaces`: each interface that the change edits, as `CHANGE <interface id>: <text>`.
- `Decisions`: each decision that the change needs, as `[[decisions/<slug>]]`.
- `Concerns`: a task list. Resolve each concern (`- [x]`). Write its resolution before the approval (SD919).
- `Open questions`: a task list. Answer each question (`- [x]`), and write the answer after it, before the approval (SD922).

Open a pull request. The owner sets `status: approved` and merges it.

### 3. Write the plan

```sh
spec-dna new plan sensor-timeout
```

`plan.md` has 4 sections:

- `Files`: each path or glob that the change edits, as a code span or a plain path.
- `Order`: a task list. Write each task on one line with an id, for example `- [ ] t1: Close the valve on a timeout. Files: src/valves.rs. Verify: the test passes.` A line with no id is not a task. `next` names the next open task.
- `Risks`: what can fail, and what reduces the risk.
- `Proof`: the tests, with their `covers:` ids, and the commands that show the result.

The items of `Files` and the `Files:` paths of the tasks are the scope of the change (SD913).

Open a pull request. The owner sets `status: approved` and merges it.

### 4. Write the code and the tests

Write the code in the files of the plan. Link each test to the requirement that it proves:

```rust
// covers: requirements/valves/closing#sensor-timeout
```

A `covers:` tag can name an `ADD` requirement before `finish` writes it. If the work differs from the plan, update `plan.md` in the same commit as the work.

The code can come in more than one pull request. Run `finish` in the last pull request. The pull request of `finish` deletes the folder, so the checks read the spec and the plan from the base tree. Correct `Files` or the delta in an earlier pull request.

Before you open the pull request, run:

```sh
spec-dna check
spec-dna finish sensor-timeout --dry-run
spec-dna finish sensor-timeout
```

`finish` checks the approvals, the concerns, the open questions, the decisions and the hashes of the delta. Then it applies the delta to `requirements/valves/closing`. It checks the evidence of each new or changed requirement (SD916). It deletes the folder of the change.

`--dry-run` prints the diff and writes nothing. `finish` does not commit. Commit its result in the pull request of the code.

## The approval commit

This section is for a project with `[change] approval = "branch"`. `check` then walks the commits of the branch from the oldest. A commit that edits an artifact or a path of `Files` needs the approval of the artifact before it. The approval is in the base tree, or in an approval commit that comes before this commit.

An approval commit meets all these conditions:

1. It is not a merge commit.
2. The author e-mail is in `[change] approvers` (for example `owner@example.test`). The compare ignores case.
3. It changes one artifact file only, and the `status` line only: `accepted` for an intent, `approved` for a spec or a plan.
4. The file is an artifact of a change that the branch started or continued.

The steps of a higher-risk change in one branch:

1. Commit the intent as a `draft`, with its class.
2. An approver commits `status: accepted` in `intent.md`, in a commit of its own.
3. Commit the spec as a `draft`. An approver commits `status: approved` in it. Do the same for the plan.
4. Commit the code.

A `draft` spec or plan needs no approval of the artifact before it. The approval commit of a plan fixes its `Files`: a path that a later commit adds to `Files` has no approved plan (SD910, reason 8). This holds for a plan that an approval commit of the branch approved. A plan that the base tree already approves keeps the rule of 0.6: its scope is the `Files` of the head, and SD913 warns on a widened scope. This is for a plan that an approval commit of the branch approved. A plan that the base tree already approves keeps the rule of 0.6: its scope is the `Files` of the head, and SD913 warns on a widened scope.

- The e-mail of an author is not a proof. A person can set any e-mail. The control is the code host: `CODEOWNERS` for `docs/intent/`, a protected default branch and required signed commits.
- In this repository, an agent writes an approval commit only when the owner says so explicitly in the chat ([AGENTS.md](../../AGENTS.md)). This is the rule of this repository, not advice for each project: another project decides for itself. List in `approvers` the e-mails of the persons that approve, and list each e-mail that one person uses. A person that hides the e-mail on the code host has a `noreply` address too.
- The check needs the full history. Use `fetch-depth: 0` in the step of the checkout in your workflow. A shallow clone gives no approval, and `check` reports SD910 as with `approval = "base"`. If the clone is so shallow that the merge base is not reachable, `check` reports SD910 with reason 5 for each change that needs an approval, and it does not skip the check. `init` writes `fetch-depth: 0` in the workflows that it generates.
- A merge commit that merges another branch into the branch stops the rule. Rebase the branch, or merge the base into it. A required check must also run on the event `merge_group` of a merge queue.
- A squash merge keeps the net diff only, so a change that starts and ends in one pull request leaves no record. `check` reports SD910 (reason 7) for a branch that adds a change and finishes it. Merge the artifacts and their approvals in a pull request first, and `finish` in a later one.
- `status` shows the source of each approval, `base` or `branch <short sha>`. `check --json` lists the approval commits in `approvals`.

## A routine change

A routine change has no spec and no delta. With `approval = "base"`, it has 3 pull requests:

1. The intent, with `class: routine` and `status: accepted`.
2. The plan. The agent sets `status: approved`.
3. The code. Edit the requirements in the same pull request as the code. Then run `spec-dna finish <slug>`.

With `approval = "branch"`, the intent, its approval commit, the plan, its approval commit and the code can be in one pull request. `finish` follows in a later pull request ([the approval commit](#the-approval-commit)).

For a routine change, `finish` checks the evidence of each requirement whose hash is different from the base.

### Work with no files

Some work edits no file. An example is a research task: the result is a document in the plan, or a decision of the team. Write this in the `Files` section of the plan. Use one item only, and start it with the word `none`, in any case:

```md
## Files

- None: this change is research.
```

The plan then lists no file, and `next` and `status` do not ask for paths. The stage `build` ends when each task of `Order` is done, or when `Order` has no task. A diff against the base is not needed. Then the steps of test, verify, review and finish follow. The last line of `next` tells you to run `spec-dna finish <slug>`.

This rule is for a routine change only. A higher-risk change has a delta, so its plan must list the files of the work. If a task of `Order` lists a path with `Files:`, the plan lists files, and the rule does not apply ([Files and paths](../types.md#files-and-paths)).

## A small fix

A small fix has no folder and no stage. It is a pull request only. It must keep each requirement of `rules`, `requirements` and `interfaces` as it is. If the diff changes such a requirement and touches no intent folder, `check` reports SD921. Then write an intent. With the policy `three-class`, trivial work that keeps each requirement needs no intent.

## Drop a change

1. Set `status: closed` in `intent.md` in a pull request. Merge the pull request.
2. In a later pull request, run `spec-dna finish <slug>`. For a closed intent, `finish` checks the status and the links into the folder. Then it deletes the folder.

`check` reports SD915 for a pull request that deletes the folder of a change that is not closed and not finished.

## See the state of the changes

```sh
spec-dna status                 # each change, its class, its stage and its next gate
spec-dna next sensor-timeout    # the checklist to the next gate of one change
spec-dna log                    # the finished and the closed changes, from the git history
```

The last line of `next` gives the edit or the command that ends the current stage.

## When check reports a code

Some of these codes are warnings in some profiles. [profiles.md](../profiles.md) gives the severity of each code.

| Code | What happened | What to do |
| --- | --- | --- |
| SD910 | A `spec.md` or a `plan.md` that is not a `draft`, or code, changed, and no approval of the document before it counts | With `approval = "base"`, merge the approval first, in its own pull request. With `"branch"`, the message names one of 8 reasons and its fix ([the approval commit](#the-approval-commit)) |
| SD913 | The diff touches a file that `Files` of the plan does not list | Add the file to `Files` before the pull request of `finish`, or remove the edit |
| SD914 | A delta line is not valid, or its hash is not the hash of the current text | Run `spec-dna show`. Correct the line in its own pull request before `finish` |
| SD916 | A new or changed requirement has no evidence | Add a test with a `covers:` tag, or a `verified-by` path |
| SD917 | The class of the intent is too low for the diff | Set a higher class. Add the documents of that class |
| SD919 | An approved spec has an open concern | Resolve the concern. Write its resolution |
| SD922 | An approved spec has an open question | Answer the question (`- [x]`). Write the answer after it |
| SD921 | A requirement changed, and no intent folder changed | Write an intent, or keep the requirement as it is |

[diagnostics.md](../diagnostics.md) gives each code and its fix. [config.md](../config.md#the-table-change) gives the keys of `[change]`.
