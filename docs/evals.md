# Evals

This page gives the eval file, the runner, the cost cap, the pack that measures each skill and the CI job of the evals. Read it when you test the agent setup of a Claude Code project. [commands.md](commands.md#eval) gives the command.

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
| `skill` | no | none | The skill that `eval run --ab` measures with this eval. One of the 11 names of the pack, such as `tdd` or `write-plan`. `eval run` without `--ab` does not read it. [The pack](#the-pack-and-the-measure-of-a-skill) gives its use |
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
- The agent and the checks run with a neutral git setup: `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME` and `GIT_COMMITTER_EMAIL` (`eval@example.invalid`), `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, and `commit.gpgsign` and `tag.gpgsign` set to `false` through `GIT_CONFIG_COUNT`. Thus a `git commit` of the agent or of a check never depends on the identity or the signing of the machine. The copy is always a git repository with one base commit.
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
{"spec_dna":"0.6.0","claude_code":"2.1.325","started":"2026-11-15T10:15:00Z","evals_dir":"evals","min_pass_rate":0.8,"max_total_usd":20,"estimate_usd":1,"spend_usd":0.12,"stopped":null,"pass_rate":1,"passed":true,"evals":[{"id":"example","runs":1,"pass_at":1,"max_budget_usd":1,"passed_runs":1,"passed":true,"spend_usd":0.12,"results":[{"claude_exit":0,"subtype":"success","cost_usd":0.12,"checks":[{"name":"notes-done","passed":true}]}]}]}
```

- `claude_code` is the first word of `claude --version`, or `null`. `stopped` is `null` or `"max-total-usd"`.
- After a stop, `pass_rate` is `null`, `passed` is `false`, and an eval that did not start has `"results":[]`.
- `claude_exit` is the exit code of `claude`, or -1 if a signal ended it. `cost_usd` is `null` when `claude` gave no cost. `spend_usd` is the sum that the cap uses.
- With `--dry-run`, `--json` prints one object: `spec_dna`, `dry_run` (`true`), `evals_dir`, `max_total_usd`, `estimate_usd`, and `evals`. Each item of `evals` has `id`, `runs`, `pass_at`, `max_budget_usd` and `estimate_usd`.

The result has no task text, no check command, no output text of `claude`, no path of the machine and no value of the environment. Claude Code can keep its own session files on the machine. spec-dna does not read or copy them.

A project can keep the result files in `<[evals] dir>/results`. `spec-dna stats --sdlc` reads them for the indicator "eval pass rate". It uses the newest complete file by `started`. A file is complete when it has a string `started`, a number `pass_rate` from 0 to 1, and `"stopped": null`.

## The pack and the measure of a skill

[ADR 0016](adr/0016-skill-evals.md) gives the reasons. Each skill of the workflow pack costs bytes in each request. The runner can show what a skill adds on the setup of your project. It runs the same evals with the skill and without it, and it compares the 2 pass rates.

### The pack

`spec-dna eval pack` writes the shipped evals to `<[evals] dir>/pack/`. The pack has 5 evals for each of the 11 skills and 2 evals of the pair `agents-line`, 57 in total. Each eval has the key `skill`, and its id is `<group>-<n>`, with `n` from 1. The group is a skill, or `agents-line`. The ids of `verify` are `verify-6` to `verify-10`: both arms of the pilot passed each run of the first 5 evals of `verify`, so they could not show a gain, and the 5 new evals replace them. Each eval runs 4 times (`"runs": 4`). The files are these:

| File | Content |
| --- | --- |
| `<dir>/pack/<id>.json` | The eval. A JSON file in a subfolder is not an eval of `eval run`, so the pack stays out of the normal runs and out of SD506 |
| `<dir>/pack/fixtures/<id>/` | The fixture of the eval: a small project with no data of a consumer project. Its `spec-dna.toml` pins the version of the binary that wrote it |

The command writes each file once. If a file exists, the command keeps it, and the output names it (`kept <path>`). Thus a project can edit its copy of an eval. The command needs `[evals]`. Without it, the exit code is 3. A write error gives exit code 4. With `--json`, it prints `{"written":[...],"kept":[...]}`.

A check of the pack is a shell command. A check that runs `spec-dna check` reads the codes that it needs, such as `SD919`, and never the exit code alone. The 2 arms differ in the generated files, so the exit code of `check` can differ with the arm (for example SD501). The `tdd` evals read the git history of the copy, so a test commit must come before the code commit.

### Run the arms

```text
spec-dna eval run --ab <skill|all|agents-line> [--runs <n>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate | --no-isolate] [--resume <result file>] [--json]
```

`--ab` takes one target:

- a skill, such as `tdd`, to run the 5 evals of that skill;
- `all`, to run each skill that the pack has (the 11 skills of a full pack, 440 runs). It does not include `agents-line`;
- `agents-line`, the pair of D34 for the line of `AGENTS.md`. It runs `tdd-1`, `debug-1` and `execute-1`, and the 2 evals `agents-line-1` and `agents-line-2` (they have the key `skill` with the value `agents-line`). `agents-line-1` is a long task that moves a document, and the line tells the agent to run `spec-dna check` before a commit. `agents-line-2` has a plan that is a draft, and the line says that an agent writes an approval only when the owner says so.

`--ab` reads `<dir>/pack/*.json` and checks each file with the rules of SD506. An invalid file, or a pack with no eval for the target, gives exit code 3. `--ab` also needs `<dir>/check.sh`, as `eval run` does. `--min-pass-rate` with `--ab` is a usage error, because the verdict has its own rules. The options `--runs`, `--max-total-usd`, `--out`, `--dry-run` and `--json` work as for `eval run`. `--runs` replaces the 4 runs of each eval. An eval of `execute`, and `tdd-1` and `debug-1`, have `max-turns` 40: the pilot showed that the line of `AGENTS.md` costs 3 to 7 turns that only run `spec-dna`, and a cap of 25 turns stopped correct work.

`--ab` isolates the runs by default: it adds `--setting-sources project --strict-mcp-config`, so the user settings, the hooks, the MCP servers and the user `CLAUDE.md` of the person do not reach a run. `--no-isolate` switches it off. `--isolate` is the default, and with `--no-isolate` it is a usage error.

The 2 arms differ in the temporary copy of each run:

| Arm | The copy |
| --- | --- |
| `with` | It has the folder of the skill, `.claude/skills/spec-dna-<name>/SKILL.md`, also when `[harness] skills` has the skill off. The text is the text of `spec-dna skill show <name>`, with a front matter |
| `without` | It has no folder of the skill. The line of `AGENTS.md` and `CLAUDE.md` has no discipline of the skill, and the adapter skill has no rule of it. For `write-plan`, the adapter skill also loses the rule that names `writing-plans`. A skill with no discipline changes no line. The command adds `--disallowedTools` with `Bash(spec-dna skill:*)` after the place of `--allowedTools`, so the agent cannot read the text of the skill. `spec-dna next` still prints the name of the skill, so the arm measures the absence of the text, not of the name |

The pair `agents-line` has no `.claude/skills/` folder in each arm, because the line serves an agent with no skill folder. `with` has the generated line in `AGENTS.md` and `CLAUDE.md`, as `init` writes it. `without` has no line and denies `spec-dna skill`.

These rules apply to both arms:

- The tools of the eval get `Skill` (`--allowedTools`), because the arm `with` needs it under `--permission-mode dontAsk`. Each pair side in the result has `permission_denials`, the number of items in the list `permission_denials` of the result of `claude`, or `null`. A denied call of the skill is then visible.
- A fixture can have its own `AGENTS.md` and `CLAUDE.md`. They keep their text. The copy adds only the generated lines of the project (the line of the adapter, and the line of the harness) after it. The arm `without` and the pair `agents-line` remove only their discipline line or lines.
- In the copy of a fixture, the runner writes the version of the binary into the pin line of `spec-dna.toml`, so a fixture does not give exit code 3 in each run.
- The `PATH` of the agent and of each check has the folder of the running binary first, so `spec-dna` in a copy is the binary under test.
- An agent that runs more than 25 minutes, and a check that runs more than 5 minutes, are killed with their process group. The run counts as failed, with the subtype `timeout`, and it is no refusal. `SPEC_DNA_EVAL_TIMEOUT` (the agent) and `SPEC_DNA_EVAL_CHECK_TIMEOUT` (a check) set the limits in seconds, from 1 to 86,400. Another value is a usage error.
- The working tree never changes.

A pair is the same eval and the same run number in both arms. For each eval and run number, `with` runs first for an odd number, and `without` runs first for an even number. The runner runs one run at a time.

The cap is checked before each run: if the spend and the `max-budget-usd` of the next run could pass `max-total-usd`, the run does not start. The command stops with `"stopped": "max-total-usd"` and exit code 3. The estimate is not a limit. If the estimate is above `max-total-usd`, the command prints a note on stderr (`note: the estimate <e> USD is above max-total-usd <c> USD; the run stops before a run that could pass the cap, and --resume goes on`) and starts. Thus a hard total can cover a plan of many targets, and `--resume` goes on from the partial result with more money. `eval run` without `--ab` keeps the refusal of [ADR 0008](adr/0008-harness-and-evals.md).

`--dry-run` prints the number of runs, the estimate and the cap, also when the estimate is above the cap. It then says that a real run starts and stops before a run that could pass the cap, and the exit code is 0. With `--json`, the key `above_cap` is `true`.

#### Stop and resume

The runner writes the result file after each pair, with a hidden temporary file and a rename, so a kill loses no paid run. Until the end, the file says `"stopped": "interrupted"`. SIGHUP, SIGINT and SIGTERM stop the command: it kills the agent and its process group, removes the copy, writes the result with `"stopped": "interrupted"`, and exits with 129 (SIGHUP), 130 (SIGINT) or 143 (SIGTERM). A run of the agent that ended before the signal is in `spend_usd`. A Ctrl-C does not reach the git helpers of the runner, so an interrupt is never an error of git.

```text
spec-dna eval run --ab <target> --resume evals/results/skills-<time>.json --max-total-usd <n>
```

`--resume` goes on in the same file, and it writes one merged result into it. It skips each pair that the file holds, and it runs the arm that a half pair lacks. The target must be the target of the file. Without `--runs`, the run uses the `runs` of the file. `--resume <file> --runs <n>` with `n` above the `runs` of the file adds the runs from `runs + 1` to `n` to each eval, and the file keeps `runs = n`. Thus a smoke run with `--runs 1` becomes run 1 of the full batch, and no paid run is lost. `n` below the `runs` of the file, and `--runs` with a file that has `"runs": null`, are usage errors (exit code 2). `--resume` takes no `--out` and no `--dry-run`. The spend, the excluded attempts and the models of the file carry over, and the cap counts the spend of the file too. A file of a finished target has nothing to run. `eval verdict` keeps reading the newest file that holds each name.

### Runs that the API refuses

The runner treats a refused attempt as no run. A refused attempt is an account limit, a credit or a login problem, 401, 403, 429, a 5xx status, a page of Cloudflare or a lost connection. A result of `claude` with a `subtype` that starts with `error_max_` (the agent ran out of turns or budget) is never a refusal. A run that exits with code 0 and has no `"is_error": true` is never a refusal. Else an attempt is refused in these cases:

1. The result object has `"is_error": true` and an `api_error_status` of 401, 403, 429 or 500 to 599. The text does not matter, and it can be long.
2. The result object has a `result` text with fewer than 600 bytes and a marker.
3. There is no result object, and the stdout is empty. The exit code is not 0. The stderr gives the class. This is a decision of the design: an empty output is a failed connection more often than a failed run.
4. There is no result object, the exit code is not 0, and the stdout is a text. A text with fewer than 600 bytes and a marker gives the class. Without a marker, the attempt is a silent error (see below): an agent never ends with a plain text.
5. The result object has `"is_error": true`, no marker, and a cost of 0 or no cost: a silent error. A real agent run never costs 0.

A marker is `API Error:` with the status 401, 403, 429 or 500 to 599, the word `429`, `hit your ... limit` (at most 40 characters between), `limit reached` together with `reset` (for example `5-hour limit reached ∙ resets 3pm`), a window of the account (`5-hour limit`, `weekly limit`, `session limit`, `opus limit`), or one of these phrases, in any case:

- account: "usage limit", "spend limit reached", "credit balance", "out of usage", "out of extra usage", "please run /login", "not logged in";
- other: "rate limit", "limit will reset", "overloaded", "Failed to authenticate", "Attention Required", "Cloudflare", "Request timed out", "Connection error", "Unable to connect", "ECONNRESET", "ETIMEDOUT", "ECONNREFUSED", "ENOTFOUND", "socket hang up", "fetch failed", "could not resolve host".

`Context limit reached` and `Token limit reached for this file` have no `reset`: they are runs. A failed check, an agent that ran out of turns or budget, and an error that has a cost and no marker are runs, and they count.

A silent error is excluded like a refusal, with the short ladder: no result counts it, and the spend gets its cost, or 0. 5 silent errors in a row stop the command with `"api-error"` (exit code 3) and the partial result. A run that counts resets the row. Thus a broken `claude`, such as one that prints a plain error, gives no result and no verdict.

The runner tries a refused attempt again with the same eval, arm and run number, in a new temporary folder. The waits depend on the class:

| Class | Waits | Total |
| --- | --- | --- |
| An account limit, credit or login (the markers of account) | 60, 120, 240, 480, 960 and 1,920 seconds, then 1,800 seconds 9 times, and a last wait of 1,620 seconds: 16 retries | 6 hours |
| Any other refusal | 60, 120, 240, 480, 960 and 1,920 seconds: 6 retries | about 63 minutes |

A 5-hour session window ends before the last retry of an account limit. `SPEC_DNA_EVAL_BACKOFF` sets the first wait in seconds, from 0 to 3,600, and the next waits scale with it. A value that is not an integer in this range is a usage error (exit code 2). The tests set it to 0. The variable has an effect only on `--ab`.

- No result counts a refused attempt: not the pass rate, not the runs, not the cost of the arm. The result file reports the count of excluded attempts of each arm.
- The charged cost of a refused attempt is the cost that `claude` reports, or 0. It goes to `spend_usd` and to the check of the cap only.
- After the last wait, the command stops with exit code 3, and `stopped` is `"api-error"`. It writes the partial result. Run the target again later with `--resume`.
- The 5 silent errors in a row are the backstop for a refusal that the markers miss.

`eval run` without `--ab` keeps the rule of [ADR 0008](adr/0008-harness-and-evals.md): a refused run is a failed run, and the runner never tries it again.

### The result file of `--ab`

The runner writes `<dir>/skills-<YYYYMMDD>T<HHMMSS>Z.json` (the UTC time of the start), where `<dir>` is `<[evals] dir>/results` or the value of `--out`. A second file in the same second gets the suffix `-2`. The runner writes the text to a hidden temporary file in the folder and then gives it its final name, so a reader never sees a half file. The file is indented JSON. A stop by the cap, by `api-error` or by a git or file error (exit code 4) writes the partial result before the command exits, so the paid runs are not lost. It has no `pass_rate` at the top level, so `spec-dna stats --sdlc` does not count it as a result of `eval run`.

| Key | Content |
| --- | --- |
| `spec_dna`, `claude_code`, `started` | The versions and the start time, as in the result of `eval run` |
| `models` | The sorted names of the models that the results of `claude` list in `modelUsage`. Names only |
| `target`, `runs`, `seed`, `resamples` | The target, the value of `--runs` (`null` if each eval kept its own runs), 24 and 2,000 |
| `max_total_usd`, `estimate_usd`, `spend_usd` | The cap, the estimate and the spend. The spend includes the excluded attempts |
| `excluded` | `{"with": n, "without": n}`, the excluded attempts of each arm |
| `stopped` | `null`, `"max-total-usd"`, `"api-error"`, `"error"` (a git or file error) or `"interrupted"` (SIGINT, SIGTERM, or a file that the runner writes during the run) |
| `error` | Only when `stopped` is `"error"`: the first line of the message of the git or file error, with no path of the machine |
| `skills` | One item for each skill, or one item `agents-line` |

An item of `skills` has `name`, `evals`, `with`, `without`, `difference`, `cost_ratio`, `verdict`, `rule` and `pairs`. An arm has `runs`, `passes`, `pass_rate`, `cost_usd` and `cost_per_correct_usd` (`null` with no pass). `difference` and `cost_ratio` have `value`, `low` and `high`, the 90% interval. `cost_ratio` is `null` when it is undefined. A pair has `eval`, `run`, `with` and `without`. Each side has `claude_exit`, `subtype` (`timeout` for a run that a time limit killed), `cost_usd`, `excluded`, `permission_denials`, `passed`, `tokens`, `models` and `checks`. `tokens` is `{"in","out","cache_read","cache_write"}`, the sums over the entries of `modelUsage` of `inputTokens`, `outputTokens`, `cacheReadInputTokens` and `cacheCreationInputTokens` (a field that is absent counts 0), or `null` if `modelUsage` is absent or a count is not an integer of 0 or more. `models` is the sorted list of the names in `modelUsage` of that run. `checks` is a list of `{"name","passed"}`, one for each check of the eval: the names and the results, with no command and no output. A result file that an older binary wrote has none of these 3 keys, and `--resume` reads such a side as `null`, an empty list and an empty list. A side is `null` when its arm did not run, because the run stopped after the other arm. The analysis counts complete pairs only. After a stop, the item of a skill that has not all its pairs has `"verdict": null`. The file has no task text, no check command, no output text of `claude` and no path of the machine.

### The verdict

The unit of the verdict is one run. The 4 runs of one eval are not independent, so the bootstrap first resamples the evals, and then the runs of each eval. For each resample it draws the index of an eval `k` times for `k` evals, and then one index for each run of each drawn eval. Both arms use the same index of a pair. The generator is SplitMix64 with the seed 24, and an index is the next value modulo the count. The interval is the 100th and the 1,900th value of the 2,000 sorted values. Thus a reader can repeat the interval from the pairs of the file.

A skill gets the verdict `on` when one rule holds:

1. The pass rate with the skill is at least 0.20 above the pass rate without it, and the low end of the interval of the difference is above 0.
2. The pass rate with the skill is at most 0.05 below the pass rate without it, the ratio of the costs per correct outcome is at most 0.90, and the high end of the interval of the ratio is below 1. The ratio is undefined when a resample has no pass in an arm, and rule 2 is then not met.

The verdict is `off` when the high end of the interval of the difference is below 0. Else it is `inconclusive`. A skill that is off by default and is `inconclusive` stays off. The result file gives the rule of an `on` verdict (`"rule": 1` or `2`).

```text
spec-dna eval verdict [--json]
```

`eval verdict` prints one line for each of the 11 skills and for `agents-line`, in this form: the name, the verdict (`on`, `off`, `inconclusive` or `unmeasured`) and the source. The source is `project <file>`, `release` or `-`. For each name, it reads the newest result file of `<[evals] dir>/results` that holds a verdict of that name. A file of a stopped run has no verdict for an incomplete skill, so it does not hide an older verdict. If no file holds the name, it reads the table of the release that the binary holds. The command starts no agent, needs no `[evals]` table, and gives exit code 3 for a bad configuration. With `--json`, it prints `{"verdicts":[{"name","verdict","source","file"}]}`.

```text
spec-dna eval verdict --release [--json]
```

`eval verdict --release` prints the table of the release: one line for each name that has a row, for the report and the release notes. A line has the name, the verdict and the rule, the passes and the runs of each arm with the pass rate, the cost per correct outcome of each arm, the difference with its 90% interval, the cost ratio (`-` if it is undefined), the batch and the version of Claude Code:

```text
tdd on (rule 1): with 10/20 0.50, without 1/20 0.05, cost per correct 1.993746 and 19.680766 USD, difference 0.45 [0.15, 0.75], cost ratio -, batch 20261117T082902Z-tdd, claude 2.1.326
```

A name that has no record has no row. `--release` reads the binary only: it needs no project and no `[evals]` table. With `--json`, it prints `{"release":[{"name","verdict","rule","batch","claude_code","models","with","without","difference","cost_ratio"}]}`. An arm has `runs`, `passes`, `pass_rate`, `cost_usd` and `cost_per_correct_usd`. `--release` is an option of `eval verdict` only.

SD507 `skill-unproven` reports a skill of `[harness] skills` that is off by default and has no verdict `on` in the project or in the release. It is a warning in the profile `strict`, and off in `lite` and `standard`. [diagnostics.md](diagnostics.md) gives the code.

### The cost

The estimate of a target is the sum of `runs x 2 arms x max-budget-usd` of its evals. The 11 skills have 55 evals, 4 runs and 2 arms: 440 runs. The pilot of the 4 skills that are off (`execute`, `tdd`, `verify` and `debug`) and the pair `agents-line` is 200 runs. `--dry-run` prints the number of runs and the estimate. The default `max-total-usd` is 20, so a real run needs `--max-total-usd`. The command prints a note if the estimate is above the cap, and it checks the spend before each run (exit code 3, `"stopped": "max-total-usd"` and the partial result).

### The records of a release

A release keeps the result of its runs as records in `bench/results/evals/`. A record is one line of compact JSON for each run of each arm. It has 9 keys, in this order, and no other key:

| Key | Value |
| --- | --- |
| `skill` | The name of the item of the result file: one of the 11 skills, or `agents-line`. The group `tdd` and the group `agents-line` both have `tdd-1`, and `skill` keeps them apart |
| `eval` | The eval id |
| `run` | The run number of the pair, from 1 to 100 |
| `arm` | `with` or `without` |
| `pass` | `true` or `false` |
| `cost` | `cost_usd` of the side, or `null` |
| `tokens` | `null`, or `{"in","out","cache_read","cache_write"}` |
| `models` | 1 to 8 sorted names of models. A name has 1 to 64 characters from `A-Z a-z 0-9 . _ : - [ ] /`, with no `..` and no `/` at the start, such as `claude-fable-5-2[1m]` |
| `claude_code` | The version of Claude Code, or `null` |

A record has no path, no text, no check name and no time. The pair is `eval` and `run`. The records of the 200 runs of the pilot have `"tokens": null`, and each of them names the models of its result file, because the runner of the pilot kept one list of models for each file. The runner of this release keeps the tokens and the models of each run.

```text
spec-dna eval records <result file>
```

`eval records` reads one result file of `--ab` and prints its records on stdout. It prints `eval records: <n> records of <k> names (<names>)` on stderr. A person writes the output to `bench/results/evals/<stamp>-<target>.jsonl`, where `<stamp>` is the stamp of the name of the result file. A file of the records is a batch.

The command builds each record from 9 values, and it never copies an object of the input. A value outside its form stops the command, and the message names the key and never the value. After the records are built, the command computes each item again from them, and it compares the result with the item of the file: the runs, the passes, the sums of the costs, the interval of the difference, the interval of the cost ratio, the verdict and the rule. A run with `cost` `null` counts the `max-budget-usd` of its eval in the pack of the binary. A mismatch stops the command, and it prints no record. The command takes one file and no option. Exit codes: 0, 2 for a usage error, and 3 for a file that is not a result of `--ab`, an item with no verdict (the run is not complete; finish it with `--resume`), a value outside its form or a failed comparison.

The binary holds the table of the release as the constant `verdict::RELEASE`, and not the records. For each name, the newest batch that has records of the name gives the row. Thus a confirmation batch must give a verdict alone: the batch of an earlier run does not add to it. A test reads the records on the disk, computes the table with the same functions, and compares it with the constant. If they differ, the test prints the text of the constant. `eval verdict --release` prints the table.

### The simulation of the rules

The rules are strict on purpose. A simulation on the bootstrap above gives the share of trials that get the verdict `on`. Each trial draws 5 evals of 4 runs for each arm, with the true pass rate 0.8 with the skill and 0.6 without it (a gain of 0.20). The runs are independent, and each run costs the same. Thus the ratio of the costs per correct outcome is the passes without the skill over the passes with it. 10,000 trials give these shares:

| Share of trials | Value |
| --- | --- |
| Rule 1 holds | 0.244 |
| Rule 2 holds | 0.177 |
| The verdict is `on` | 0.246 |

Rule 2 adds almost nothing to rule 1 when the runs cost the same. The run is `cargo test --release --bin spec-dna verdict -- --ignored --nocapture`. A real eval has clusters of runs, so the shares of a real result can be lower. Thus only a large gain gives the verdict `on`, and a smaller gain stays `inconclusive`.

### The limits

One model family runs the agents, and a result is valid for the model and the Claude Code version that the file records. The `without` arm cannot remove what the model learned in training, so the gain of the arm is a lower bound. No result is a proof for another project. The pack has no arm with another plugin installed.

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
