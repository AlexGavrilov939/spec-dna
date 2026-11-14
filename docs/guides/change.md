# One change from start to finish

This guide takes one change through its documents, its approvals and `finish`. It uses the policy `three-class` and the gate `block`, which are the defaults of a new project. [Concepts](../concepts.md#the-change-lifecycle) explains the model.

The example change adds a requirement: when a moisture sensor times out, the controller closes the valve of its bed. The slug of the change is `sensor-timeout`.

## Choose the class

| Class | Use it when | Documents | Who approves the plan |
| --- | --- | --- | --- |
| `small-fix` | The change keeps each requirement as it is | None: a pull request only | Not applicable |
| `routine` | The change edits requirements outside the high-risk layers | `intent.md`, `plan.md` | The agent |
| `higher-risk` | The change edits a rule or an interface, or a path of `[change] high-risk-paths` | `intent.md`, `spec.md`, `plan.md` | The owner |

If the class of an intent is too low for its diff, `check` gives SD917. The high-risk layers are `interfaces` and `rules` by default (`[change] high-risk-layers`).

## A higher-risk change

Each document needs the approval of the document before it in the base tree. Thus a higher-risk change has 4 pull requests or more: the intent, the spec, the plan, and the code with `finish`. To approve a document, set its status in the pull request. The merge records the approval. With `[change] gate = "warn"`, one pull request can hold all of them, and `check` gives a warning.

### 1. Write the intent

```sh
spec-dna new intent sensor-timeout
```

This command writes `docs/intent/sensor-timeout/intent.md` with the status `draft`. Write what you want and why, in your own words. Set the class in the front matter:

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
spec-dna show requirements/valves/closing
```

`show` prints each requirement of the document with its hash. The `Delta` section of `spec.md` lists the edits of the requirements. `ADD` names a new key. `CHANGE` and `REMOVE` name a key and the hash of the text that you read:

```md
## Delta

- ADD requirements/valves/closing#sensor-timeout: When a moisture sensor times out, the controller SHALL close the valve of its bed.
- CHANGE requirements/valves/closing#flush-time@3f2a9c1b7e0d4a11: The controller SHALL flush a valve line for 30 seconds before it closes the valve.
```

The spec has 3 more sections:

- `Interfaces`: each interface that the change edits, as `CHANGE <interface id>: <text>`.
- `Decisions`: each decision that the change needs, as `[[decisions/<slug>]]`.
- `Concerns`: a task list. Resolve each concern (`- [x]`) and write its resolution before the approval (SD919).

Open a pull request. The owner sets `status: approved` and merges it.

### 3. Write the plan

```sh
spec-dna new plan sensor-timeout
```

`plan.md` has 4 sections:

- `Files`: each path or glob that the change edits, in a code span. This list is the scope of the change (SD913).
- `Order`: the steps, in order.
- `Risks`: what can go wrong, and what limits it.
- `Proof`: the tests, with their `covers:` ids, and the commands that show the result.

Open a pull request. The owner sets `status: approved` and merges it.

### 4. Write the code and the tests

Write the code in the files of the plan. Link each test to the requirement that it proves:

```rust
// covers: requirements/valves/closing#sensor-timeout
```

A `covers:` tag can name an `ADD` requirement before `finish` writes it. If the work differs from the plan, update `plan.md` in the same commit as the work.

The code can come in more than one pull request. Run `finish` in the last one. The pull request of `finish` deletes the folder, so the checks read the spec and the plan from the base tree. Correct `Files` or the delta in an earlier pull request.

Before you open the pull request, run:

```sh
spec-dna check
spec-dna finish sensor-timeout --dry-run
spec-dna finish sensor-timeout
```

`finish` checks the approvals, the concerns, the decisions and the hashes of the delta. Then it applies the delta to `requirements/valves/closing`, checks the evidence of each new or changed requirement (SD916), and deletes the folder of the change. `--dry-run` prints the diff and writes nothing. `finish` does not commit. Commit its result in the pull request of the code.

## A routine change

A routine change has no spec and no delta. It has 3 pull requests:

1. The intent, with `class: routine` and `status: accepted`.
2. The plan. The agent sets `status: approved`.
3. The code. Edit the requirements in the same pull request as the code, then run `spec-dna finish <slug>`.

For a routine change, `finish` checks the evidence of each requirement whose hash is different from the base.

## A small fix

A small fix has no folder and no stage. It is a pull request only. It must keep each requirement of `rules`, `requirements` and `interfaces` as it is. If the diff changes such a requirement and touches no intent folder, `check --base` gives SD921. Then write an intent.

## Drop a change

1. Set `status: closed` in `intent.md` in a pull request, and merge it.
2. In a later pull request, run `spec-dna finish <slug>`. For a closed intent, `finish` checks only the status and deletes the folder.

A pull request that deletes the folder of a change that is not closed and not finished gives SD915.

## See the state of the changes

```sh
spec-dna status                 # each change, its class, its stage and its next gate
spec-dna next sensor-timeout    # the checklist to the next gate of one change
spec-dna log                    # the finished changes, from the git history
```

`next` gives the edit that approves the current document on its last line.

## When check stops you

| Code | What happened | What to do |
| --- | --- | --- |
| SD910 | A document or code changed, and the base has no approval of the document before it | Merge the approval first, in its own pull request |
| SD913 | The diff touches a file that `Files` of the plan does not list | Add the file to `Files` before the pull request of `finish`, or remove the edit |
| SD914 | A delta line is not valid, or its hash is not the hash of the current text | Run `spec-dna show` and correct the line in its own pull request before `finish` |
| SD916 | A new or changed requirement has no evidence | Add a test with a `covers:` tag, or a `verified-by` path |
| SD917 | The class of the intent is too low for the diff | Set a higher class, and add the documents of that class |
| SD919 | An approved spec has an open concern | Resolve the concern and write its resolution |
| SD921 | A requirement changed, and no intent folder changed | Write an intent, or keep the requirement as it is |

[diagnostics.md](../diagnostics.md) gives each code and its fix. [config.md](../config.md#the-table-change) gives the keys of `[change]`.
