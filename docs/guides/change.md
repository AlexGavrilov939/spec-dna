# One change from start to finish

This guide takes one change through its documents, its approvals and `finish`. It uses the policy `three-class` and the gate `block`, which are the defaults of a new project. [Concepts](../concepts.md#the-change-lifecycle) explains the model.

The example change adds a requirement: when a moisture sensor times out, the controller closes the valve of its bed. The slug of the change is `sensor-timeout`.

## Choose the class

| Class | Use it when | Documents | Who approves the plan |
| --- | --- | --- | --- |
| `small-fix` | The change keeps each requirement as it is | None: a pull request only | Not applicable |
| `routine` | The change edits requirements outside the high-risk layers | `intent.md`, `plan.md` | The agent, when the owner says so |
| `higher-risk` | The change edits a rule or an interface, or a path of `[change] high-risk-paths` | `intent.md`, `spec.md`, `plan.md` | The owner |

If the class of an intent is too low for its diff, `check` reports SD917. The high-risk layers are `interfaces` and `rules` by default (`[change] high-risk-layers`).

## A higher-risk change

Each document needs the approval of the document before it. With `[change] approval = "base"`, the base tree must show the approval, so a higher-risk change has 4 pull requests or more: the intent, the spec, the plan, and the code with `finish`. To approve a document, run `spec-dna approve <slug> <artifact>` in that pull request ([commands.md](../commands.md#approve)). The merge records the approval. With `[change] gate = "warn"`, one pull request can hold all the steps, and `check` reports a warning.

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

The spec has 4 more sections, and a fifth with the key `invariants`:

- `Interfaces`: each interface that the change edits, as `CHANGE <interface id>: <text>`, and each new interface, as `ADD <interface id>: <text>`. Write `- none` if the change touches no interface.
- `Decisions`: each decision that the change needs, as `[[decisions/<slug>]]`.
- `Concerns`: a task list. Resolve each concern (`- [x]`). Write its resolution before the approval (SD919).
- `Open questions`: a task list. Answer each question (`- [x]`), and write the answer after it, before the approval (SD922).
- `Invariants`: only with `[change] invariants = true` (`new spec` then writes it with `- none`). List each rule that the change must keep true, as `- <doc-id>#<key>@<hash>`. This is the id of a requirement of a `rule` document, with the hash that `spec-dna show` prints. Keep `- none` if no rule applies. Write each item as a list item. A paragraph or an empty section gives SD936. A rule that the delta of the same spec adds or removes is no invariant (SD936). If the delta changes the rule with a `CHANGE` line, the item has the hash of that line (SD937). The acceptor answers each item in the record.

```md
## Invariants

- rules/safety#valve-closed-on-fault@a1b2c3d4e5f60718
```

Open a pull request. The owner runs `spec-dna approve sensor-timeout <artifact>` in it, and merges it.

### 3. Write the plan

```sh
spec-dna new plan sensor-timeout
```

`plan.md` has 4 sections:

- `Files`: each path or glob that the change edits, as a code span or a plain path.
- `Order`: a task list. Write each task on one line with an id, for example `- [ ] t1: Close the valve on a timeout. Files: src/valves.rs. Verify: the test passes. Hours: 1-2.` A line with no id is not a task. `Hours:` is optional: write a number or a range. `next` names the next open task. The stage stays `build` while a task is open, and `finish` stops at step 0 and lists the open tasks. A task is closed when its box is `- [x]`, or when it has a reason in `Skip:`.
- `Risks`: what can fail, and what reduces the risk.
- `Proof`: the tests, with their `covers:` ids, and the commands that show the result.

The items of `Files` and the `Files:` paths of the tasks are the scope of the change (SD913). A plan has no default size budget, because it grows with the number of tasks. `[budgets.plan]` sets one.

#### A spike

For a spike, write one task for each way that you try. When the spike ends, add `Skip: <reason>` to each task that is not needed, and tick it (`- [x]`). `next`, `status` and the counts then treat the task as closed, and `finish` leaves its hours out. A commit that adds `Skip:` to an approved plan adds no path to `Files`, so it needs no new approval. A `Skip:` with no reason does not skip the task: SD923 reports it.

Open a pull request. The owner runs `spec-dna approve sensor-timeout <artifact>` in it, and merges it.

### 4. Write the code and the tests

Write the code in the files of the plan. Link each test to the requirement that it proves:

```rust
// covers: requirements/valves/closing#sensor-timeout
```

A `covers:` tag can name an `ADD` requirement before `finish` writes it. If the work differs from the plan or the spec, add an item to the section `Deviations` of `plan.md` in the same commit as the work. Write the item as `- <id> (plan): <text>`, or `- <id> (spec): <text>. Delta: <verb> <doc-id>#<key>` (a line of the delta of the spec) or `Follow-up: <slug>` (a change folder of the tree). The acceptor copies each deviation into the record and gives it a verdict. `finish` prints it. With `approval = "base"`, the merge of the pull request acknowledges the deviations. With `"branch"`, an approver acknowledges them (see below). If the work needs a file that `Files` does not list, the agent stops and asks the person (the skill `execute`). With a yes, add the path to `Files` of `plan.md` in the commit of the edit. A plan with an approval commit in this branch needs a new approval commit first, because a path that a later commit adds to `Files` gives SD910 (reason 8): set the plan back to `draft`, add the path, and approve it again (see [The approval commit](#the-approval-commit)).

Write the test first, and commit it alone before the code. The `tdd` skill gives the way. The commit of the red test has the subject `test: <what the test checks> (red)`. If a hook of the project runs the tests, that hook rejects the commit, because the test fails. Skip only that hook, for that commit only:

- Use the skip option of the hook runner, so that the lint and format hooks still run.
- If the runner has no option for one hook, use `git commit --no-verify`. Run the lint and format hooks manually before the commit.
- Name the skip in the body of the commit, for example `Skipped hook: the hook that runs the tests. The test is red by design.`
- The commit with the code (the green commit) runs all hooks. Never skip a hook for it.

The evals of `tdd` read the order of the commits, not their subjects. A test commit before the code commit passes them.

The code can come in more than one pull request. Run `finish` in the last pull request. The pull request of `finish` deletes the folder, so the checks read the spec and the plan from the base tree. Correct `Files` or the delta in an earlier pull request.

Before you open the pull request, run `spec-dna check`. Run `finish` after the last task, and after the review of the owner if the plan has a review task (see below).

A change that needs an acceptance record needs it before `finish` ([accept](../commands.md#accept)). The key `[change] acceptance-for` decides which changes need one: in `standard`, a `higher-risk` change (a `routine` change then follows the review of 0.7.1), and in `strict`, each change with a new plan ([the rule](../config.md#the-acceptance-record)). The class that counts is the class for the acceptance: a branch cannot lower the class of the base to skip the record. This is the step "accept". When the tasks are done and the code is committed, an agent that did not write the code tries to break the change. It writes `acceptance.toml` in the folder of the change:

For a change that needs a record, `spec-dna next sensor-timeout` names this step as its one item `review`: ``start one agent `acceptor` with the prompt `cd '<root>' && spec-dna accept sensor-timeout --brief`, and add no word``. It names the item again after a code edit, because the record is then stale. After a fix of a note, it names a follow-up. After 2 rejected records, it waits for the owner. `init` and `upgrade` write the skill `review` and the agent `acceptor` (`.claude/agents/acceptor.md`) in `standard` and `strict`, and not in `lite`. A person can write the same record by hand, also with the harness level `none`.

1. The builder starts one agent `acceptor` with the prompt line of `next`, and adds no word of its own. The acceptor runs the command and reads the brief as its output. For a `higher-risk` change in `strict`, the skill tells the builder to start the second acceptor with `claude --agent acceptor '<prompt>'` in a new session. This is advice only: the gate cannot check a session.
2. The acceptor runs the proof, tries to break the change, and runs `spec-dna accept sensor-timeout --session "$CLAUDE_CODE_SESSION_ID"`. It fills in the record, copies the hash of the last line of its brief into `brief`, and commits the record alone. It edits no code.
3. `finish` reads the record in step 5.2. It stops in the cases below. When it succeeds, step 5.5 moves the record to `.spec-dna/accepted/sensor-timeout.toml`. Commit that file with the rest of the result.
    - The record is absent, not valid, stale or `rejected` (SD927 to SD930).
    - 2 acceptors have one session and one agent (SD931).
    - With `approval = "branch"`, a commit of the current life of the record is not valid (SD932).
    - The record has too few acceptors, or no entry for a role of `[owners]` (SD933).
    - With `approval = "branch"`, the record comes after 2 rejected records, and no approval commit of `plan.md` comes after the second (SD941).
    - An item of `Invariants` or `Deviations` is not valid or is stale (SD936, SD937, SD939 and SD940).
    - The record has no answer for an invariant or no entry for a deviation (SD938).
    - With `approval = "branch"`, a deviation of the record has no acknowledgement of an approver (SD934). This stop comes last, in part 5.3.

A finding of the record is a `blocker` (it breaks a requirement, a delta line, an invariant, a rule of the project or a command of `Proof`, loses data, or opens a security hole) or a `note` (each other finding). Only a reason gives `rejected`, and a note never does.

A role of [`[owners]`](../config.md#the-table-owners) that matches a path of the change needs a record, also for a `routine` change. Without `--role`, `accept --brief` names each matched role in its first line. One acceptor entry with `roles = ["db-reviewer", "security-reviewer"]` answers all of the roles. SD933 names each role that no entry has. `--role <name>` names a role of the entry, and it can come more than once. An entry `@<login>` or an e-mail in `[owners]` names a person: it gives a warning and has no effect in 0.8.0.

With `approval = "branch"`, a record that has a deviation needs the acknowledgement of an approver, and `finish` stops with SD934 until the approver gives it. After the acceptor commits the record, the approver runs `spec-dna approve sensor-timeout acceptance` in a terminal of their own. The command adds the e-mail of its author to `acknowledged-by` and commits the record alone. With `base`, the merge of the pull request acknowledges the deviations.

SD932 checks that a commit of the current life of the record, which adds or edits the record, changes the record alone. A commit that deletes the record, and moves it nowhere, ends the life. SD932 does not read a commit of an earlier life. To clear a mixed commit, delete the record in a commit of its own, then accept again. No rewrite of the history is needed. To clear a mixed commit, delete the record in a commit of its own, then accept again. No rewrite of the history is needed. SD932 also checks that each entry of `acknowledged-by` comes from an approval commit of its author. A later commit that changes the record leaves the entry stale, for example a second acceptor, a follow-up or an edit of a finding. `accept` writes `acknowledged-by = []` for a new acceptor or a follow-up. The approver then runs `approve sensor-timeout acceptance` again, and the command first removes the stale entries in a commit of its own.

A code edit after the acceptance makes the record stale (SD929), so accept again. A Markdown edit does not. `check` runs the same codes in the pull request. The first acceptor of 2 can commit, and a fix after a `rejected` record passes `check --staged`. A plan that an earlier version added, before 0.8.0, needs no record, and `finish` prints a note. In `lite`, no change needs a record. The compare of the brief covers a record in the folder of a change only. An archive file that a person or an agent writes by hand is not compared with the brief. Its history is not read for the rule "a rejected record cannot become accepted". The honest path is `finish`, which moves the record that passed step 5.2 to the archive folder.

`finish` checks the approvals, the concerns, the open questions, the decisions and the hashes of the delta. Then it applies the delta to `requirements/valves/closing`. It checks the acceptance record, if the change needs one, and the evidence of each new or changed requirement (SD916). It deletes the folder of the change.

`--dry-run` prints the diff and writes nothing. `finish` does not commit. Commit its result in the pull request of the code.

If the project has summaries, `next` prints a note that names the summaries that the delta makes due. Edit the text of each summary whose meaning changes, in a commit of the build. If the text still holds, name the summary in `finish`, for example `spec-dna finish sensor-timeout --holds summaries/valves`. Give one `--holds` for each summary ([finish](../commands.md#finish)).

`finish` writes the new pins of a summary that you edited, also if the edit merged in an earlier pull request. It writes the pins of a summary that you name with `--holds`, and `check` then gives SD645, a warning that shows the claim. It leaves each other due summary as it is, and it prints the summary as stale. `check` then gives SD641 until you edit the summary and run `spec-dna refresh <id>`, or run `spec-dna refresh <id> --holds`.

`finish` adds the `Hours:` labels (the estimates) of the tasks that are not skipped, and the `Actual:` labels (the real hours) of each task, also of a skipped one, and prints the line `Hours: <estimate>, actual <n>`. Without `Actual:` the line is `Hours: <estimate>`. The estimate is a number, or a range `<low>-<high>` if a task has a range. When a task is done, add `Actual: <n>` to it. Put the line in the commit message of the change, and keep it in the squash message. A squash merge keeps only the message of the last pull request of the change, so the line is the only record of the hours. `log` reads the last such line of the commit that deletes `intent.md`; `stats --sdlc` reads the estimate.

`finish` never waits for a task that only `finish` can do. A task whose text names `spec-dna finish <slug>` with the slug of its change closes with `finish`: `next`, `status` and step 0 of `finish` treat it as closed, and its estimate counts. Write no such task. If a plan has one, for example ``- [ ] t9: Run `spec-dna finish sensor-timeout` and commit the result``, it does not hold `finish`. A task that names `spec-dna finish` with no slug, or with another slug, is a task of real work, and `finish` stops at step 0 with a hint.

The review by the owner is optional. A project whose owner reviews the pull request at the merge writes no review task. If the owner reviews the code before `finish`, end the plan with a task for it: `- [ ] t9: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. By: person. Hours: 1.` The task has `Files: none` and a `Verify:`, so SD923 accepts it. The label `By: person` makes it a task of a person: `next` and `status` end its line with `[person]`, and the last line of `next` is `wait for a person: ...`. A plan of 0.7.1 has the title `Review by the owner` with no label, and `next` reads it the same way. Only the owner ends this task. An agent never ticks it and never writes `Skip:` for it. It is a task of a person, so `finish` stops while it is open, and the last steps of the change have this order:

1. Open the pull request with the code. The task `Review by the owner` is open.
2. The owner reviews the pull request.
3. A person ticks the task (`- [x]`), writes its `Actual:` hours, and commits.
4. Run `spec-dna finish sensor-timeout --dry-run`, then `spec-dna finish sensor-timeout`. Commit the result in the same pull request.
5. The owner approves the merge.

The result of `finish` is the last commit of the pull request, so the owner can read it before step 5. A project that wants the owner to review the result of `finish` first can run `finish` after the review, in a last commit of the same pull request.

### A delta that depends on a spike

A spike is a short task that tries or measures one option before you decide. Sometimes the text of a delta line depends on its result. `finish` applies the delta of the base tree, and it stops when the pull request of `finish` changes the delta (SD914, step 3). Thus the edit of the delta must merge before the pull request of `finish`.

- Run the spike before the approval of the spec when you can. Before a plan exists, a spike gives no SD913. When the plan comes in the same branch, SD913 reports the file of the spike: list it in `Files`, or merge the spike in a pull request of its own.
- An approved spec stays `approved` after a text edit. `check` asks for no new approval of the edit.
- With `approval = "branch"`, run the spike in the first pull request, after the approval commit of the plan. Edit the delta in the same pull request. The count stays 2 pull requests.
- With `approval = "base"`, edit the delta in a pull request before the pull request of `finish`, for example the pull request of the spike.
- If the approver must read the new delta, set the spec to `draft` in the commit of the edit. Then the approver commits `status: approved` alone. With `approval = "branch"`, that commit is the approval. With `"base"`, only the merge records the review. If the base tree already has the spec `approved`, `check` gives SD401 for the commit with `draft` until the approval commit follows.
- To add a path to `Files` of a plan that an approval commit approved, use the same 2 commits. Else the new path gives SD910 (reason 8).
- Do not edit the delta in the pull request of `finish`. A new approval commit there does not help, because the squash merge keeps no commit of the branch.

### Test reports in CI

A tag is a claim of the author. With `[evidence] reports`, a tag proves a requirement only when a test case that binds to it passed ([Test reports](../test-report.md)). `init` writes `reports = []` and no `--reports`, because its CI job runs no tests and does not know your test command. To use the reports, make 3 steps:

1. Set the globs of the report files of your runner in `spec-dna.toml`, for example `reports = ["target/nextest/ci/junit.xml"]`. Add the folder of the reports to `.gitignore`. A report that git tracks gives SD615.
2. In the CI job that runs the tests, run the tests so that they write the report. Then, in the same job, run `spec-dna evidence stamp`. The stamp binds the report to the code tree.
3. Run `spec-dna check --reports --base origin/HEAD` after the stamp. A skipped test, a failed test and a tag that binds to no test case now give SD617 when the pull request finishes the change.

```sh
cargo nextest run --profile ci      # the runner writes the JUnit XML report
spec-dna evidence stamp             # bind the report to this tree
spec-dna check --reports --base origin/HEAD
```

Run the same 2 commands on your machine before `finish`: the tests, then `spec-dna evidence stamp`. Stage new files with `git add` first, because an untracked file is not in the code tree. `finish` reads the reports when the key is set. It stops at step 5 if a report is missing, stale or unreadable (SD613 to SD615) or if a tag has no passed test case (SD617). A change of code after the stamp makes the report stale (SD614). A change of Markdown does not. The hook `check --staged` needs no report.

A runner with no JUnit output can write the line format: `spec-dna-report 1`, then one line `pass`, `fail` or `skip`, a tab, a path or `-`, a tab and the name of the case. Put the tag in a comment on the line above the test, or above its decorators: a tag binds to the test directly below it ([How a tag binds to a test case](../test-report.md#how-a-tag-binds-to-a-test-case)). If a layout does not allow this, add the whole name of the case to the tag: `// covers: requirements/valves/closing#sensor-timeout case:closes_on_timeout`. A pre-commit hook that runs the tests needs the stamp in its entry ([A hook that runs the tests](../test-report.md#a-hook-that-runs-the-tests)).

## The approval commit

This section is for a project with `[change] approval = "branch"`. `check` then walks the commits of the branch from the oldest. A commit that edits an artifact or a path of `Files` needs the approval of the artifact before it. The approval is in the base tree, or in an approval commit that comes before this commit.

An approval commit meets all these conditions:

1. It is not a merge commit.
2. The author e-mail is in `[change] approvers` (for example `owner@example.test`). The compare ignores case.
3. It changes one artifact file only, and the `status` line only: `accepted` for an intent, `approved` for a spec or a plan.
4. The file is an artifact of a change that the branch started or continued.

The steps of a higher-risk change in one branch:

1. Commit the intent as a `draft`, with its class.
2. An approver runs `spec-dna approve sensor-timeout intent`. The command sets `status: accepted` and commits it alone.
3. Commit the spec as a `draft`. An approver runs `spec-dna approve sensor-timeout spec`. Do the same for the plan.
4. Commit the code.

A `draft` spec or plan needs no approval of the artifact before it. The approval commit of a plan fixes its `Files`: a path that a later commit adds to `Files` has no approved plan (SD910, reason 8). This holds for a plan that an approval commit of the branch approved. A plan that the base tree already approves keeps the rule of 0.6: its scope is the `Files` of the head, and SD913 warns on a widened scope.

- An approval holds while the artifact keeps its approval status at the head. If a later commit sets an approved artifact back (a plan back to `draft` after the code), SD910 gives reason 9 to the work that used the approval, and to the work between the step back and the next approval commit. To rework an approved plan, set it back to `draft`, change it, and approve it again with an approval commit before new work. The same holds for a spec. A hook that runs `spec-dna check` rejects the commit of the step back, with `status: draft` (SD910, reason 9). Skip only that hook, for this commit only, and name the skip in the body of the commit. Then the approver makes the approval commit, with all hooks on. The message of reason 9 for the working tree says the same. An approval from the base tree follows the rule of 0.6: SD401 reports the step back of a spec or a plan, and its message and its fix give these steps too.
- The e-mail of an author is not a proof. A person can set any e-mail. The control is the code host: `CODEOWNERS` for `docs/intent/`, a protected default branch and required signed commits.
- An agent writes an approval commit only when the owner says so explicitly in the chat. This is the default rule of each project: `init` writes it into the generated line of `AGENTS.md`, and each skill repeats it. Each skill also tells an agent never to add an entry to `acknowledged-by` of an acceptance record. A person approves with `spec-dna approve <slug> <artifact>` in a terminal of their own ([commands.md](../commands.md#approve)). List in `approvers` the e-mails of the persons that approve, and list each e-mail that one person uses. A person that hides the e-mail on the code host has a `noreply` address too. Do not list the address of an agent or a bot: `check` warns about it (SD926).
- The check needs the full history. Use `fetch-depth: 0` in the step of the checkout in your workflow. A shallow clone gives no approval, and `check` reports SD910 as with `approval = "base"`. If the clone is so shallow that the merge base is not reachable, `check` reports SD910 with reason 5 for each change that needs an approval, and it does not skip the check. `init` writes `fetch-depth: 0` in the workflows that it generates.
- A merge commit that merges another branch into the branch stops the rule. Rebase the branch, or merge the base into it. A required check must also run on the event `merge_group` of a merge queue.
- A squash merge keeps the net diff only, so a change that starts and ends in one pull request leaves no record. `check` reports SD910 (reason 7) for a branch that adds a change and finishes it. Merge the artifacts and their approvals in a pull request first, and `finish` in a later one.
- `status` shows the source of each approval, `base` or `branch <short sha>`. `check --json` lists the approval commits in `approvals`.

### Change the approval rule

- Stricter: set `[harness] protect-approvals = true` in `spec-dna.toml`. The profile `strict` has it. A hook of the agent then denies each commit that sets an approval status, each commit that adds an entry to `acknowledged-by` of an acceptance record in a change folder, and each call of `spec-dna approve` by the agent, also on the word of the owner. The commit of `finish`, which moves the record to `.spec-dna/accepted/`, passes. A person runs `spec-dna approve` ([harness.md](../harness.md#spec-dna-hook-protect-approvals)).
- Less strict: do not edit the generated line, because `check` reports the edit (SD501). Write the rule of the project in `AGENTS.md`, after the generated line, and name the exception. For example: "In this project, an agent approves a routine plan with no word of the owner." spec-dna has no key for this direction. The generated line and the skills keep the default sentence, so the agent reads both texts.

## A routine change

A routine change has no spec and no delta. With `approval = "base"`, it has 3 pull requests:

1. The intent, with `class: routine` and `status: accepted`.
2. The plan. The owner runs `spec-dna approve <slug> plan`, or tells the agent to approve it.
3. The code. Edit the requirements in the same pull request as the code. Then run `spec-dna finish <slug>`.

With `approval = "branch"`, the pull requests have 2 orders ([the approval commit](#the-approval-commit)):

- The code in the first pull request. It holds the intent, its approval commit, the plan, its approval commit and the code. `finish` follows in a later pull request. That pull request can hold no code when each task of the plan is closed (SD915). A task that names `spec-dna finish <slug>` closes with `finish`.
- The code in the pull request of `finish`. The first pull request holds the intent, the plan and their approval commits. The code and the result of `finish` follow in a second pull request.

For a routine change, `finish` checks the evidence of each requirement whose hash is different from the base. Put the edits of requirements in the pull request of `finish`: `finish` and SD916 check the evidence of each requirement whose hash differs from the base, and no check reads the evidence of an edit that merged before. With the code in the first pull request, tick each task of the plan there.

### Work with no files

Some work edits no file. An example is a research task: the result is a document in the plan, or a decision of the team. Write this in the `Files` section of the plan. Use one item only, and start it with the word `none`, in any case:

```md
## Files

- None: this change is research.
```

The plan then lists no file, and `next` and `status` do not ask for paths. The stage `build` ends when each task of `Order` is closed, or when `Order` has no task. A diff against the base is not needed. Then the steps of test, verify, review and finish follow. The last line of `next` tells you to run `spec-dna finish <slug>`.

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
spec-dna log                    # the finished and the closed changes, with their hours, from the git history
```

The last line of `next` gives the edit or the command that ends the current stage.

## When check reports a code

Some of these codes are warnings in some profiles. [profiles.md](../profiles.md) gives the severity of each code.

| Code | What happened | What to do |
| --- | --- | --- |
| SD910 | A `spec.md` or a `plan.md` that is not a `draft`, or code, changed, and no approval of the document before it counts | With `approval = "base"`, merge the approval first, in its own pull request. With `"branch"`, the message names one of 9 reasons and its fix ([the approval commit](#the-approval-commit)) |
| SD612 | A document outside the folder of a change links into the folder, and `finish` would stop on it | Name the change as plain text, or link the document that the change moves its text into |
| SD913 | A change of the diff has a plan, and the diff touches a file that no `Files` of the diff lists. Before a plan exists, SD913 gives nothing | List the file in `Files`, also an edit of `clarify` such as the questions, or move the edit to a pull request of its own |
| SD914 | A delta line is not valid, or its hash is not the hash of the current text | Run `spec-dna show`. Correct the line in its own pull request before `finish` ([a delta that depends on a spike](#a-delta-that-depends-on-a-spike)) |
| SD916 | A new or changed requirement has no full evidence. A `covers: partial` tag alone is not evidence | Add a test with a `covers:` tag that covers all of it, or a `verified-by` path |
| SD927 | A change that needs a record has none: `finish` finds no `acceptance.toml`, or the pull request deletes the folder and adds no archive file | Run the step "accept" ([above](#4-write-the-code-and-the-tests)): start an agent `acceptor` with the prompt line of `spec-dna next` (`cd '<root>' && spec-dna accept <slug> --brief`), or `claude --agent acceptor '<prompt>'` for a second acceptor in a new session. In a shallow clone, fetch the full history (`fetch-depth: 0`) |
| SD928 | The record breaks a rule (also an id that 2 `[[invariant]]` or 2 `[[deviation]]` entries share, a `rejected` result with no reason, or a record of format 1), or the `brief` of an entry is not the brief of its adding commit. The compare does not run on an archive file that a person or an agent writes by hand, and its history is not read: only `finish` checks the brief and the history before it archives the record | Fix the record. For a wrong `brief`, remove the entry and accept again from a new brief. For a record of format 1, delete the record, commit, and accept again |
| SD929 | The code that the record lists changed after the acceptance | Before `finish`: delete the record and accept again. After `finish`: run `git revert --no-commit` on the commit of `finish`, delete `<folder>/acceptance.toml`, commit, merge the base, accept again, and run `finish` again |
| SD930, SD931, SD933 | SD930: the record says `rejected`. SD931: 2 entries have one session and one agent. SD933: the record has too few acceptors, or no entry for a matched role of `[owners]` | Fix each finding and accept again. Use another acceptor, or run `spec-dna accept <slug> --role <name>` for each missing acceptor or role |
| SD932, SD934 | With `approval = "branch"` only. SD932: a commit of the current life of the record also changes another path, or an entry of `acknowledged-by` does not count. SD934: a record that has a deviation has no acknowledgement of an approver | For a commit that changes another path: delete the record in a commit of its own, then accept again. For SD934 and an entry that does not count: an approver runs `spec-dna approve <slug> acceptance` |
| SD941 | With `approval = "branch"` only: the record comes after 2 rejected records of the change, and no approval commit of `plan.md` by an approver comes after the second | Commit `status: draft` of `plan.md` alone. Skip the commit hook for that commit only, and name the skip in the body of the commit: the draft commit gives SD401 and SD910 reason 9 until the approval commit. Then the approver runs `spec-dna approve <slug> plan`. In a shallow clone, fetch the full history (`fetch-depth: 0`) |
| SD936, SD937 | With `[change] invariants`: an item of `Invariants` is not valid (it names no rule, or a rule that the delta adds or removes), or the section has no list item, or its hash is not the hash of the rule now (or of the `CHANGE` line) | Fix the item. Write each item as a list item, or `- none`. Remove an item for a rule that the change adds or removes. Run `spec-dna show <doc-id>#<key>` for the new hash |
| SD938 | The record has no answer for an invariant of the spec, or no entry with the same text for a deviation of the plan | The acceptor adds the entry that the message names, and commits the record alone |
| SD939, SD940 | An item of `Deviations` is not `- <id> (<kind>): <text>`, or a `spec` item names no line of the delta and no change folder | Write the kind `plan` or `spec`. Add `Delta: <verb> <doc-id>#<key>` or `Follow-up: <slug>` to a `spec` item. A code span reads as its text. The message names the form that it found |
| SD935 | The diff changes or deletes a file of `.spec-dna/accepted/` | Revert the edit. The file is the record of a finished change |
| SD613 to SD615, SD617 | With `[evidence] reports`: a test report is missing, stale or unreadable, or a tag has no passed test case | Run the tests, then `spec-dna evidence stamp`. Put the tag above the test, or add `case:<name>` ([test reports in CI](#test-reports-in-ci)) |
| SD917 | The class of the intent is too low for the diff | Set a higher class. Add the documents of that class |
| SD919 | An approved spec has an open concern | Resolve the concern. Write its resolution |
| SD922 | An approved spec has an open question | Answer the question (`- [x]`). Write the answer after it |
| SD921 | A requirement changed, and no intent folder changed | Write an intent, or keep the requirement as it is |
| SD923 | A task of an approved plan has no `Files:` or no `Verify:`, its `Files:` has a glob or a folder, or its `Skip:` has no reason | Write the exact paths and the check of the task, or the reason after `Skip:` |
| SD924 | A task of an approved plan lists more paths than `max-task-files` | Split the task |
| SD925 | An item of `Concerns` or `Open questions` has no task box | Write `- [ ]` or `- [x]` at the start of the item |

[diagnostics.md](../diagnostics.md) gives each code and its fix. [config.md](../config.md#the-table-change) gives the keys of `[change]`.
