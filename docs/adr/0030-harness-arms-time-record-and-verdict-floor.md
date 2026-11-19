---
status: proposed
---

# 0030. Eval runner: harness arms, the time of a run and the floor of a verdict

## Context and Problem Statement

The runner of [ADR 0008](0008-harness-and-evals.md) and [ADR 0016](0016-skill-evals.md) has 4 gaps for 0.8.0:

- `eval run --ab` has only the arms `with` and `without` of a skill. The copy takes `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and `.claude/*` from the project that runs the command, so each run has the team of the host. The committed records list the advisor model in 755 of 770 runs, and they do not record the team. The model-team measurement of 0.8.0 needs arms that differ in the team only.
- No run records its time. A plain `eval run` records no tokens and no models. The owner wants the cost of one acceptance run in USD, tokens and time.
- `decide` has no minimum sample. One pair with a pass in the arm `with` and a fail in `without` gives `on`. A lab simulation gave `on` in 29% of trials for one pair with no true gain.
- The runner commits all files of a fixture in one base commit. An eval that reviews or accepts a change needs a diff against `main`.

Constraints:

- The runner keeps its rules: the standard library, git, `claude`, the cost cap, and no prompt or output text in a result.
- A record has no path, no text and no clock time ([ADR 0021](0021-eval-records-and-the-cap.md)).
- The table of the release stays equal to the committed records.
- A result of 0.7.1 stays readable.

## Considered Options

1. Edit the `.claude` files of the host project before each batch.
2. Built-in harness arms that the runner writes into each copy with the generator of `init`.
3. A file of arm definitions with any `[harness]` table.

## Decision Outcome

Option 2.

### Harness arms

- An arm is a name: `none`, `lean`, `lean-no-advisor`, `full`, or `project` (the files of the project that runs the command, as in 0.7.1). For a built-in arm, the runner writes a `[harness]` table into the `spec-dna.toml` of the copy and runs the generator of `init` on the copy. It copies no `.claude/*`, no generated line and no `REVIEW.md` from the project.
- A built-in arm has `skills = []`, `review-md = false` and `protect-tests = "off"`, so the arms differ only in the level and the roles. `lean-no-advisor` is `lean` with `advisor = { claude-code = "none" }`.
- Each arm gets the adapter line of the binary.
- The runner passes `--model` and `--effort` with the orchestrator values of `lean` to each arm, so the main model is the same in each arm.
- `spec-dna eval run --arms <arm>,<arm>[,<arm>] [<id>...]` runs 2 or 3 arms on the plain evals. It runs in waves. Wave `r` runs each eval once in each arm, and the order of the arms in one eval rotates by `(eval index + r) mod arms`. A stop by the cap keeps the complete waves, and the analysis drops a partial wave. The result file is `arms-<stamp>.json`, and `--resume` works as for `--ab`.
- `--harness <arm>` gives one arm to a plain `eval run`. With `--ab`, the copy gets the team of the arm and the skills that `init` proposes for the profile `standard` at that level. Then the edit of `--ab` adds or removes the measured skill. When the role of the skill has an agent file at that level, the arm `without` also loses that file and the sentence of the role in the line of `CLAUDE.md`. The skill `accept` and the agent `acceptor` are an example.
- A batch with `--harness` is a new condition, not the condition of a row of 0.7.1. Without the option, `--ab` keeps the copy of 0.7.1.
- `--arms` and `--harness` isolate the runs (`--setting-sources project`). `--no-isolate` with them is a usage error.
- `spec-dna eval verdict --compare <treatment>,<control> <result file>` gives the verdict of 2 arms of a result of `--arms`.

### The time and the team of a run

- Each run gets `seconds`, the wall time of the agent process that the runner measures, and `turns`, the `num_turns` of the result of `claude` or `null`.
- A plain `eval run` result gets `tokens`, `models`, `turns` and `seconds` for each run.
- A record gets 3 keys after its 9 keys: `harness` (the arm of the copy), `turns` and `seconds`. For a batch of `--arms`, `skill` is `harness` and `arm` is the name of the arm. The test of the table of the release skips the name `harness`, so such a batch never enters the table. A record of 9 keys stays valid and reads as `project`, `null`, `null`.

### The floor of a verdict

- A verdict needs at least 5 evals and 15 complete pairs. Below that, the verdict is `inconclusive`, the item has `"below_floor": true`, and the text names the floor.
- Each row of the table of 0.7.1 has 15 or more pairs over 5 evals, so the table does not change.
- A test with `#[ignore]` runs the simulation of the floor on the real bootstrap.

Lab estimates of the share of trials with `on`:

| Evals x runs | Pairs | No true gain | True gain of 0.20 |
| --- | --- | --- | --- |
| 1 x 1 | 1 | 0.29 | 0.44 |
| 1 x 3 | 3 | 0.09 | 0.22 |
| 3 x 3 | 9 | 0.04 | 0.14 |
| 5 x 3 | 15 | 0.03 | 0.21 |

### The change of an eval

An eval file gets the optional key `change`: a folder under the eval folder. After the base commit, the runner copies its files over the copy. Then the runner makes one commit on the branch of the key `branch`, with the neutral identity of the runner. `change` needs `branch`. SD506 checks the key as it checks `fixture`.

### Rejected

- Option 1: a batch depends on the machine and on manual edits of a person, and the result cannot show the team.
- Option 3: a file of arms is a new format and a new place of truth. The built-in names cover the questions of 0.8.0.
- A clock time in a record: a duration gives the cost in time, and a date can tell when a person worked.
- A floor above 15 pairs: the rows `tdd` and `post-mortem` would become `inconclusive`, and `tdd` would go off by default ([ADR 0023](0023-default-skills-from-the-release-table.md)).
- A separate binary for the measurement: a second copy of the runner.

### Consequences

- Good: a batch records its team, and a reader can repeat it on another machine.
- Good: the cost of a run has its time and its turns.
- Good: one pair no longer turns a skill on.
- Bad: a record has 12 keys. A reader of 9 keys must accept 3 more.
- Bad: a project result file of 0.7.1 below the floor now reads `inconclusive`.
- Neutral: `--ab` without `--harness` keeps the copy of 0.7.1.

## Amends

ADR 0008, ADR 0016 and ADR 0021 are accepted and locked, so this ADR does not edit them. These rules apply in place of the named text:

- ADR 0008, "Evals": `eval run` gets `--arms` and `--harness`. A result of a plain `eval run` has `tokens`, `models`, `turns` and `seconds` for each run. An eval file can have the key `change`.
- ADR 0016, "The rule for a default": a verdict needs 5 evals and 15 complete pairs. `eval verdict` gets `--compare`.
- ADR 0021, "Decision Outcome": a record has 12 keys. The 3 new keys follow the 9 keys in the order `harness`, `turns`, `seconds`. "No time" means no clock time and no date. `arm` can be the name of a harness arm.

## Amendments
