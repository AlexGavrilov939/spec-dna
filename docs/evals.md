# Evals

This page gives the eval file, the runner, the cost cap and the CI job of the evals. Read it when you test the agent setup of a Claude Code project. [commands.md](commands.md#eval) gives the command.

An eval is a task with checks that tests the agent setup of a project: its instruction files, skills, hooks and roles. `spec-dna eval run` runs the evals and reports the pass rate. Thus a team can see when a change of the setup makes the agent worse. The runner uses the Claude Code adapter and starts headless Claude Code. [agents.md](agents.md) lists the adapters.

## Start

For a new project, question 14 of `init` proposes `skeleton`. [init.md](init.md#evals) gives the rules of `init`. `skeleton` writes these files:

| File | Content |
| --- | --- |
| `[evals]` in `spec-dna.toml` | The table that enables `eval run`, SD506 and SD918. It is directly before `[harness]` |
| `<dir>/check.sh` | The functions `tests_unchanged`, `file_has` and `spec_dna_clean` for the checks. It has no shebang, because `eval run` sources it. The default pathspecs of `tests_unchanged` are the globs of `[evidence] tests` |
| `<dir>/example.json` | An example eval: a task that edits one file, and one check |
| `<dir>/fixtures/example/notes.txt` | The fixture of the example eval: one line, `status: draft` |
| `.github/workflows/agent-evals.yml` | The CI job. `init` writes it only for the CI answer `github` |

`<dir>` is `[evals] dir`. The folder is `evals`. If git lists a file under `evals/`, the folder is `.spec-dna/evals`, and a note says so. The starter needs no build tool.

`init` writes each starter file once and never again. The CI job runs nothing until the owner sets the repository variable `SPEC_DNA_EVALS` to `true`. An upgraded project has no evals. To add them, run `spec-dna init --reconfigure`. Choose `skeleton`.

## Configuration

```toml
[evals]
dir = "evals"         # the eval files, check.sh and the fixtures
min-pass-rate = 0.8   # the share of evals that must pass, from 0 to 1
runs = 3              # the runs of each eval, unless the eval sets its own
max-total-usd = 20    # the cost cap of one eval run command
```

The table is optional. A project with no table has no evals. [config.md](config.md#the-table-evals) gives the rules of each key. These rules apply:

- `dir` is a path relative to the repository root. An absolute path, a `..` or `.` segment, a `\` and a `:` are errors (exit code 3).
- `dir` and `templates.dir` must not overlap.
- The folder is never part of a root, also for a root `.`. `check` does not read a file there as a document, so a fixture can hold Markdown files.
- `runs` is an integer from 1 to 100. `max-total-usd` is a number above 0.
- `pass-at` is a key of an eval file only. In `[evals]` it is an unknown key.
- `--runs`, `--min-pass-rate` and `--max-total-usd` of `eval run` replace these values for one command.

## An eval file

An eval is the file `<dir>/<id>.json`. A JSON file directly in `<dir>` is an eval. A file in a subfolder is not. The file has at most 64 KiB, it is UTF-8, and it is one JSON object. A key that the table does not list is an error. An integer is a JSON number with no fraction.

`eval run` reads the working tree: the tracked files, and the untracked files that git does not ignore. It checks the keys in the order of the table, and it reports the first rule that fails.

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

| Key | Required | Default | Rule |
| --- | --- | --- | --- |
| `id` | yes | - | `[a-z0-9][a-z0-9-]{0,63}`, equal to the name of the file without `.json` |
| `task` | yes | - | The request that the agent gets. Not empty, at most 8,000 bytes, and it does not start with `-` |
| `fixture` | yes | - | A folder under `<dir>/`, but not under `<dir>/results/`. It has `/` separators and no `..`, `.`, empty segment or `\`. It holds at least one file |
| `branch` | no | none | A git branch name of at most 100 characters of `A-Z`, `a-z`, `0-9`, `.`, `_`, `/` and `-`. It starts with a letter or a digit, has no `..` or `//`, and does not end with `/` or `.lock`. The branch `main` is the base branch, so it gives no switch |
| `allowed-tools` | no | `[]` | A list of strings. Each one is not empty, does not start with `-`, and has no control character |
| `max-turns` | no | `20` | An integer from 1 to 1,000 |
| `max-budget-usd` | yes | - | The budget of one run. A number above 0, and at most `[evals] max-total-usd` |
| `runs` | no | `[evals] runs` | An integer from 1 to 100. `--runs` overrides it |
| `pass-at` | no | the runs of the eval | The runs that must pass for the eval to pass. An integer from 1 to the runs of the eval |
| `checks` | yes | - | A list that is not empty. Each item has exactly `name` (`[a-z0-9][a-z0-9-]{0,63}`, once in the file) and `run` (not empty). A check passes on exit code 0 |

On a `fix/` branch, the hook `protect-tests` can ask before an edit of a test file ([harness.md](harness.md#spec-dna-hook-protect-tests)). A run with `claude -p` cannot answer, so Claude Code denies the edit.

A check can call a function of `<dir>/check.sh`, such as `tests_unchanged`. The runner reads `check.sh` from the project, never from the copy that the agent edits. Thus the agent cannot change its own checks.

`eval run` stops with exit code 3 if `check.sh` is absent, or if `<dir>` holds no eval. SD506 reports the same cases, and it reports an eval file that breaks a rule of the table ([diagnostics.md](diagnostics.md)).

## Run the evals

```text
spec-dna eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate]
```

The command needs the table `[evals]`. A pin that differs from the version of the binary stops the command, as it stops `check`. An unknown id is a usage error.

| Option | Meaning |
| --- | --- |
| `<id>...` | The evals to run. With no id, the command runs each eval, in the order of their ids |
| `--runs <n>` | Replaces the `runs` of each selected eval and `[evals] runs`. An integer from 1 to 100 |
| `--min-pass-rate <r>` | Replaces `[evals] min-pass-rate`. A number from 0 to 1 |
| `--max-total-usd <n>` | Replaces `[evals] max-total-usd` for the cost cap. A number above 0 |
| `--out <dir>` | Writes the result to `<dir>/eval-<YYYYMMDD>T<HHMMSS>Z.json` (the UTC time of the start). If the file exists, the name gets the next free suffix from `-2`. See [Results](#results) |
| `--dry-run` | Checks the evals and prints each eval with its runs and the estimate. It starts no process and writes no file. Exit code 0, or 3 if the estimate is above the cap |
| `--isolate` | Adds `--setting-sources project --strict-mcp-config` to `claude`. It is off by default, so CI and a laptop run the same command |
| `--json` | Prints the result as one JSON object, and no text |

`--isolate` has these effects, as a test with Claude Code 2.1.325 showed:

- A run does not load the user and local settings. It loses the user skills, the user agents and the plugins that the user settings enable.
- `--strict-mcp-config` removes the MCP servers of the account of the person.
- The flag keeps the plugins that `CLAUDE_CODE_PLUGIN_DIRS` names. To remove them, unset the variable before the command.
- A clean `CLAUDE_CONFIG_DIR` also removes the login.
- The proof that the user `CLAUDE.md` is not loaded is weak, because it uses the answer of the model.

### The steps

1. The runner reads the configuration and checks each eval file. An invalid file gives exit code 3, and the message names the path and the rule.
2. It computes the estimate: the sum of `runs x max-budget-usd` of the selected evals. If the estimate is above `max-total-usd`, it stops with exit code 3 and starts no process.
3. It runs `claude --version` once. If `claude` is absent or cannot start, it stops with exit code 3.
4. For each eval and each run, it does steps 5 to 11. The loop has no early stop.
5. It makes a temporary folder, `spec-dna-eval-<pid>-<n>`, in the temporary directory of the system (mode 700 on Unix).
6. It copies the files of the fixture into the root of the folder. Then it copies `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and the files of `.claude/`, except `.claude/settings.local.json`, from the working tree. A file of the project replaces a fixture file at the same path.
7. The copy uses the list of git, so it does not copy a file that git ignores. It copies no symbolic link, and a warning names it.
8. It starts a git repository in the folder. The base commit has the author `spec-dna <eval@spec-dna.invalid>`, the date `2000-01-01T00:00:00Z`, no signature and no hook. If `branch` is set and is not `main`, it switches to a new branch of that name.
9. It starts `claude` in the folder, and then it reads `total_cost_usd` and `subtype` from the output. If the cost is absent, the spend counts `max-budget-usd` for the run.
10. It runs each check with `sh -c` in the folder. The checks also run when `claude` failed. A run passes when `claude` exits with code 0 and each check passes.
11. It deletes the folder, also after an error. It adds the cost to the spend. If the spend is above `max-total-usd` and a run remains, it stops.
12. An eval passes when its passed runs are at least `min(pass-at, runs)`. The pass rate is the share of the evals that pass, not of the runs. The suite passes when the pass rate is at least `min-pass-rate`.

Details of the steps 9 and 10:

- The arguments of `claude` are, in this order: `-p`, the task, `--allowedTools` and then `--output-format json --max-turns <n> --max-budget-usd <usd> --permission-mode dontAsk`.
- `--allowedTools` has the tools joined by commas in one argument. The runner passes it only for a list that is not empty.
- With `--isolate`, the arguments end with `--setting-sources project --strict-mcp-config`.
- The runner never passes `--bare`, because an eval must load `CLAUDE.md`, the skills and the hooks.
- The environment is the environment of `eval run`, without `CLAUDE_PROJECT_DIR`, `GIT_DIR`, `GIT_WORK_TREE` and `GIT_INDEX_FILE`. A session that starts `eval run` keeps its `CLAUDECODE`, and `claude -p` starts under it (Claude Code 2.1.325).
- The runner keeps at most 16 MiB of the output of `claude` in memory. The output is one JSON object, or a list of events that ends with the object of type `result`.
- The script of a check sources `check.sh` of the project, never the copy. Then it runs the `run` text of the check.
- The environment of a check has `SPEC_DNA_CHECK_SH`, the absolute path of `check.sh`, and `SPEC_DNA_EVAL_BASE`, the id of the base commit.

### Output

```text
eval run: 2 evals, 4 runs, estimate 7.00 USD, cap 20.00 USD
example: 1 of 1 runs passed, pass-at 1: pass, 0.12 USD
fix-keeps-tests: 1 of 3 runs passed, pass-at 2: fail, 1.80 USD
pass rate 50.0 percent (1 of 2 evals), min-pass-rate 80.0 percent: fail
spend 1.92 USD
```

The progress of each run goes to stderr. A stop by the cap adds the line `stopped: the spend <s> USD is above max-total-usd <c> USD; <n> runs did not start`. The line of the pass rate is then absent. An eval that did not run all its runs does not pass.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | The pass rate is at least `min-pass-rate`. Also `--dry-run` |
| 1 | The pass rate is lower |
| 2 | Usage error: a bad option value, an unknown id, `--out` that is `[evals] dir`, or no `run` subcommand |
| 3 | No `[evals]` table, an invalid eval file, no eval, an absent `check.sh`, a different version pin, an estimate above the cap, a stop by the cap, or a `claude` that is absent or cannot start |
| 4 | A git or file error of `eval run` itself: the temporary folder, the copy, the base commit or the result file |

## Cost

spec-dna makes no network call, but `claude` does, and each run costs money. The runner has no time limit of its own. In CI, `timeout-minutes` of the workflow limits the job. `eval run` limits the cost in 2 ways:

- Before the first run, it compares the estimate (`runs x max-budget-usd` of all selected evals) with `max-total-usd`. If the estimate is higher, it does not start.
- After each run, it adds the cost that the agent reports. When the sum is higher than `max-total-usd`, it starts no more runs and exits with code 3.

Claude Code checks `--max-budget-usd` against an estimate of the client, so the real cost can be higher than the budget. Thus the second check uses the sum of the costs that the runs report.

## Results

`--json` prints the result, and `--out <dir>` writes the same object to a file. The file is indented. `eval run` makes `<dir>` before the first run, so a bad value stops the command before any cost. `<dir>` is relative to the repository root.

`<dir>` cannot be `[evals] dir`, because SD506 reads each `*.json` file there. Use `<[evals] dir>/results`.

```json
{"spec_dna":"0.5.0","claude_code":"2.1.325","started":"2026-11-15T10:15:00Z","evals_dir":"evals","min_pass_rate":0.8,"max_total_usd":20,"estimate_usd":1,"spend_usd":0.12,"stopped":null,"pass_rate":1,"passed":true,"evals":[{"id":"example","runs":1,"pass_at":1,"max_budget_usd":1,"passed_runs":1,"passed":true,"spend_usd":0.12,"results":[{"claude_exit":0,"subtype":"success","cost_usd":0.12,"checks":[{"name":"notes-done","passed":true}]}]}]}
```

- `claude_code` is the first word of `claude --version`, or `null`. `stopped` is `null` or `"max-total-usd"`.
- After a stop, `pass_rate` is `null`, `passed` is `false`, and an eval that did not start has `"results":[]`.
- `claude_exit` is the exit code of `claude`, or -1 if a signal ended it. `cost_usd` is `null` when `claude` gave no cost. `spend_usd` is the sum that the cap uses.
- With `--dry-run`, `--json` prints one object: `spec_dna`, `dry_run` (`true`), `evals_dir`, `max_total_usd`, `estimate_usd`, and `evals`. Each item of `evals` has `id`, `runs`, `pass_at`, `max_budget_usd` and `estimate_usd`.

The result has no task text, no check command, no output text of `claude`, no path of the machine and no value of the environment. Claude Code can keep its own session files on the machine. spec-dna does not read or copy them.

A project can keep the result files in `<[evals] dir>/results`. `spec-dna stats --sdlc` reads them for the indicator "eval pass rate". It uses the newest complete file by `started`. A file is complete when it has a string `started`, a number `pass_rate` from 0 to 1, and `"stopped": null`.

## The CI job

The job runs on a pull request and a push that change `CLAUDE.md`, `AGENTS.md`, `.claude/agents/`, `.claude/skills/`, `.claude/settings.json` or `[evals] dir`. It also runs each week and on request. These rules apply:

- The job runs only when the repository variable `SPEC_DNA_EVALS` is `true`.
- A pull request from a fork gets no secrets, so the job does not run for it. The workflow never uses `pull_request_target`.
- The job has the permission `contents: read`, and it reads the key from the secret `ANTHROPIC_API_KEY`.
- The job installs the pinned spec-dna and Claude Code 2.1.325, and it runs `spec-dna eval run`. Each action in it has a full commit SHA.
- The job fails when the pass rate is lower than `min-pass-rate`.
- `init` writes the job once. The workflow has no marker, so a Dependabot update of a pinned action gives no diagnostic.

After a write that created the workflow, `init` prints the 2 commands that enable the job. They are `gh secret set ANTHROPIC_API_KEY` and `gh variable set SPEC_DNA_EVALS --body true`.

## Incidents and evals

Each incident adds an eval, so that the same failure cannot occur again without notice. SD918 warns when the diff adds an `incident` document and adds or changes no `*.json` file directly in `[evals] dir`. A file in a subfolder does not count. A deleted eval does not count.

SD918 needs `[evals]` in the configuration and a `[change] policy` other than `none`. It is off in the profile `lite`.

`spec-dna stats --sdlc` also reports the days from the commit that added an incident to the commit that added its eval. It reads the first path `<[evals] dir>/<id>.json` in the section `Eval` of the incident.
