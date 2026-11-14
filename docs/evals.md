# Evals

This page gives the eval file, the runner, the cost limits and the CI job of the evals. Read it when you test the agent setup of a Claude Code project.

An eval is a task with checks that tests the agent setup of a project: its instruction files, skills, hooks and roles. `spec-dna eval run` runs the evals and reports the pass rate. Thus a team can see when a change of the setup makes the agent worse. The runner uses the Claude Code adapter and starts headless Claude Code. [agents.md](agents.md) lists the adapters.

## Start

For a new project, question 14 of `init` proposes `skeleton`. `skeleton` writes these files:

| File | Content |
| --- | --- |
| `[evals]` in `spec-dna.toml` | The table that enables `eval run`, SD506 and SD918 |
| `evals/check.sh` | The shell functions that the checks call. `init` writes it once and never again |
| An example eval and `evals/fixtures/example/` | A task that edits one text file, and one check. It needs no build tool |
| `.github/workflows/agent-evals.yml` | The CI job. `init` does not write it when the CI answer is `gitlab` |

The CI job runs nothing until the owner sets the repository variable `SPEC_DNA_EVALS`. An upgraded project has no evals. To add them, run `spec-dna init --reconfigure`. Choose `skeleton`.

## Configuration

```toml
[evals]
dir = "evals"         # the eval files, check.sh and the fixtures
min-pass-rate = 0.8   # the share of evals that must pass
runs = 3              # the runs of each eval, unless the eval sets its own
max-total-usd = 20    # the cost limit of one eval run command
```

For a root `.`, `init` writes an `exclude` for `[evals] dir`, because a fixture can hold Markdown files.

## An eval file

An eval is a JSON file directly in `[evals] dir`:

```json
{
  "id": "fix-keeps-tests",
  "task": "Fix the failing renew test by changing the code, not the test.",
  "fixture": "evals/fixtures/renew",
  "branch": "fix/renew",
  "allowed-tools": ["Read", "Edit", "Bash(cargo test*)"],
  "max-turns": 20,
  "max-budget-usd": 2,
  "runs": 3,
  "pass-at": 2,
  "checks": [
    { "name": "tests-unchanged", "run": "tests_unchanged" },
    { "name": "tests-pass", "run": "cargo test --quiet" }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `id` | The name of the eval |
| `task` | The request that the agent gets |
| `fixture` | The folder that the run starts from |
| `branch` | Optional. The branch of the run. A `fix/` branch enables the hook `protect-tests` |
| `allowed-tools` | The tools that the agent can use |
| `max-turns` | The most turns of one run |
| `max-budget-usd` | Required. The cost limit of one run. The cost cap of the command needs it |
| `runs` | Optional. The runs of this eval. `--runs` overrides it, and it overrides `[evals] runs` |
| `pass-at` | The runs that must pass for the eval to pass |
| `checks` | Each check: a name, and a command that passes on exit code 0 |

A check can call a function of `<dir>/check.sh`, such as `tests_unchanged`. The runner reads `check.sh` from the project, never from the copy that the agent edits, so the agent cannot change its own checks. SD506 reports an eval file that is not valid, an eval with no check, an absent fixture and an absent `check.sh`.

## Run the evals

```text
spec-dna eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run]
```

For each run of each eval, the runner does these steps:

1. It copies the fixture and the harness files of the project into a new temporary directory. The harness files are `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and `.claude/`, without `.claude/settings.local.json`.
2. It starts a git repository there and commits the start state. If the eval has a `branch`, it switches to that branch.
3. It starts headless Claude Code with the task, the allowed tools, the turn limit and the cost limit of the eval.
4. It runs each check in the directory. A run passes when the agent exits with code 0 and each check passes.
5. It deletes the directory.

An eval passes when at least `pass-at` of its runs pass. The suite passes when the share of evals that pass is at least `min-pass-rate`. `eval run` prints a table of the results.

| Exit code | Meaning |
| --- | --- |
| 0 | The pass rate is at least `min-pass-rate` |
| 1 | The pass rate is lower |
| 3 | A configuration error, the cost cap, or no `claude` binary on `PATH` |

## Cost

spec-dna makes no network call, but the agent that `eval run` starts uses the network, and each run costs money. Thus `eval run` has 2 limits:

- Before the first run, it compares `runs × max-budget-usd` of all evals with `max-total-usd`. If the sum is higher, it does not start.
- After each run, it adds the cost that the agent reports. When the sum is higher than `max-total-usd`, it starts no more runs and exits with code 3.

The agent checks its own cost limit against an estimate, so the real cost can be higher.

## Results

`--json` prints the result, and `--out <dir>` writes it to a file. The result has the eval ids, the check names, the counts, the total cost and the versions. It has no prompt text and no output text, because they can hold secrets. `spec-dna stats --sdlc` reads the result files that the project keeps for the indicator "eval pass rate".

## The CI job

The job runs on a pull request and a push that change `CLAUDE.md`, `AGENTS.md`, `.claude/skills/`, `.claude/agents/`, `.claude/settings.json` or `[evals] dir`. It also runs once a week and on request. These rules apply:

- The job runs only when the repository variable `SPEC_DNA_EVALS` is set.
- A pull request from a fork gets no secrets, so the job does not run for it.
- The job has `contents: read`, and it reads the key from the secret `ANTHROPIC_API_KEY`.
- The job fails when the pass rate is lower than `min-pass-rate`.
- `init` writes the job once. A Dependabot update of a pinned action gives no SD501.

## Incidents and evals

Each incident adds an eval, so that the same failure cannot occur again without notice. SD918 warns when a change adds an `incident` document and no file in `[evals] dir`. It applies only when `[evals]` is in the configuration and the change policy is not `none`.
